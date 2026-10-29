//! VirtualBox: VBoxManage, version support by this Vagrant, and the host
//! conflicts that stop machines from running well (Hyper-V on Windows, the
//! kernel module on Linux; risk R8).

use std::path::{Path, PathBuf};

use serde::Serialize;

use vagrant_studio_platform::HostOs;
use vagrant_studio_vagrant_adapter::readiness::{Finding, Level, install_hint};

use crate::compat::{VirtualBoxSupport, virtualbox_support};
use crate::{
    Capability, Context, DetectFuture, Detection, ProviderAdapter, ProviderKind, VM_CAPABILITIES,
    tools,
};

pub struct VirtualBox;

impl ProviderAdapter for VirtualBox {
    fn id(&self) -> &'static str {
        "virtualbox"
    }
    fn name(&self) -> &'static str {
        "VirtualBox"
    }
    fn kind(&self) -> ProviderKind {
        ProviderKind::Vm
    }
    fn capabilities(&self) -> &'static [Capability] {
        VM_CAPABILITIES
    }
    fn detect<'a>(&'a self, ctx: &'a Context<'a>) -> DetectFuture<'a> {
        Box::pin(detect(ctx))
    }
}

fn find_vboxmanage(ctx: &Context<'_>) -> Option<PathBuf> {
    tools::find(
        ctx,
        "VBoxManage",
        &[
            (
                HostOs::Windows,
                r"%ProgramFiles%\Oracle\VirtualBox\VBoxManage.exe",
            ),
            (HostOs::Linux, "/usr/bin/VBoxManage"),
            (HostOs::Linux, "/usr/lib/virtualbox/VBoxManage"),
        ],
    )
}

async fn detect(ctx: &Context<'_>) -> Detection {
    let Some(exe) = find_vboxmanage(ctx) else {
        return Detection::new(
            false,
            None,
            vec![Finding::problem(
                Level::Error,
                "VirtualBox is not installed (VBoxManage not found).",
                install_hint(
                    ctx.os,
                    "VirtualBox",
                    "https://www.virtualbox.org/wiki/Downloads",
                ),
            )],
        );
    };
    let version = tools::run(ctx, &exe, &["--version"])
        .await
        .filter(|o| o.success)
        .and_then(|o| o.first_line().map(str::to_owned))
        // e.g. "7.1.18r173720"
        .map(|v| v.split('r').next().unwrap_or(&v).trim().to_owned());

    let mut findings = vec![Finding::ok(format!(
        "VBoxManage found at {}.",
        exe.display()
    ))];
    match &version {
        Some(v) => {
            findings.push(Finding::ok(format!("VirtualBox {v}.")));
            if let Some(vagrant) = ctx.vagrant_version {
                findings.push(support_finding(vagrant, v));
            }
        }
        None => findings.push(Finding::problem(
            Level::Warning,
            "VBoxManage didn't report a version.",
            "Reinstall VirtualBox if machines fail to start.",
        )),
    }
    match ctx.os {
        HostOs::Windows => findings.extend(hypervisor_finding(ctx)),
        HostOs::Linux => findings.push(kernel_module_finding(ctx)),
        _ => {}
    }
    Detection::new(true, version, findings)
}

fn support_finding(vagrant: &str, virtualbox: &str) -> Finding {
    match virtualbox_support(vagrant, virtualbox) {
        VirtualBoxSupport::Supported => Finding::ok(format!(
            "Vagrant {vagrant} supports VirtualBox {virtualbox}."
        )),
        VirtualBoxSupport::Unsupported {
            newest: (major, minor),
        } => Finding::problem(
            Level::Error,
            format!(
                "Vagrant {vagrant} doesn't support VirtualBox {virtualbox}; machines won't start."
            ),
            format!(
                "Install a Vagrant release that supports this VirtualBox version, or use VirtualBox {major}.{minor} or older."
            ),
        ),
        VirtualBoxSupport::Unknown => Finding::problem(
            Level::Warning,
            format!(
                "Vagrant Studio can't tell whether Vagrant {vagrant} supports VirtualBox {virtualbox}."
            ),
            "If Vagrant reports that this VirtualBox version is unsupported, update Vagrant or install a VirtualBox version listed in its release notes.",
        ),
    }
}

/// VirtualBox runs on top of the Windows hypervisor when one is active, which
/// is slower and doesn't work for every box.
fn hypervisor_finding(ctx: &Context<'_>) -> Option<Finding> {
    match ctx.probe.hypervisor_present()? {
        false => Some(Finding::ok("The Windows hypervisor is not running.")),
        true => Some(Finding::problem(
            Level::Warning,
            "The Windows hypervisor is running (used by Hyper-V, WSL 2, Windows Sandbox and Memory Integrity). VirtualBox then runs machines through it, which is slower, and some boxes fail to boot.",
            "If machines are slow or fail to start, see VirtualBox's notes on Hyper-V. Turning these Windows features off is a host setting Vagrant Studio doesn't change.",
        )),
    }
}

fn kernel_module_finding(ctx: &Context<'_>) -> Finding {
    if ctx.probe.path_exists(Path::new("/dev/vboxdrv")) {
        Finding::ok("The VirtualBox kernel module is loaded.")
    } else {
        Finding::problem(
            Level::Error,
            "The VirtualBox kernel module (vboxdrv) isn't loaded, so machines can't start.",
            "Run `sudo /sbin/vboxconfig` to build and load it. With Secure Boot enabled, the module must be signed first.",
        )
    }
}

/// A host network adapter VirtualBox can bridge to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgedInterface {
    /// The name `bridge:` must match (Vagrant compares it without case).
    pub name: String,
    pub wireless: bool,
}

/// Adapters a public network can bridge to, as Vagrant sees them: `VBoxManage
/// list bridgedifs` without those whose status is `Down` or `Unknown`.
/// `None` when VirtualBox isn't installed or the list couldn't be read.
pub async fn bridged_interfaces(ctx: &Context<'_>) -> Option<Vec<BridgedInterface>> {
    let exe = find_vboxmanage(ctx)?;
    let output = tools::run(ctx, &exe, &["list", "bridgedifs"])
        .await
        .filter(|o| o.success)?;
    Some(parse_bridged_interfaces(&output.stdout))
}

fn parse_bridged_interfaces(text: &str) -> Vec<BridgedInterface> {
    let mut out = Vec::new();
    for block in text.replace("\r\n", "\n").split("\n\n") {
        let field = |key: &str| {
            block.lines().find_map(|l| {
                l.strip_prefix(key)
                    .and_then(|rest| rest.strip_prefix(':'))
                    .map(str::trim)
            })
        };
        let Some(name) = field("Name").filter(|n| !n.is_empty()) else {
            continue;
        };
        if matches!(field("Status"), Some("Down" | "Unknown")) {
            continue;
        }
        out.push(BridgedInterface {
            name: name.to_owned(),
            wireless: field("Wireless") == Some("Yes"),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridged_interfaces_skip_adapters_that_are_down() {
        let text = include_str!(
            "../../../fixtures/command-output/windows-vagrant-2.4.9/virtualbox-list-bridgedifs.txt"
        );
        assert!(text.contains("\r\n"));
        assert_eq!(
            parse_bridged_interfaces(text),
            vec![BridgedInterface {
                name: "Intel(R) Wi-Fi 6 AX200 160MHz".into(),
                wireless: true,
            }]
        );
        assert!(parse_bridged_interfaces("").is_empty());
    }
}
