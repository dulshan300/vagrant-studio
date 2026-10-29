//! VMware Desktop: Workstation (`vmrun`), the `vagrant-vmware-desktop` plugin
//! and the Vagrant VMware Utility service (ADR-0003 amendment).

use vagrant_studio_platform::HostOs;
use vagrant_studio_vagrant_adapter::readiness::{Finding, Level, install_hint};

use crate::{
    Capability, Context, DetectFuture, Detection, ProviderAdapter, ProviderKind, VM_CAPABILITIES,
    tools,
};

const UTILITY_ADDR: &str = "127.0.0.1:9922";
const PLUGIN: &str = "vagrant-vmware-desktop";

pub struct VmwareDesktop;

impl ProviderAdapter for VmwareDesktop {
    fn id(&self) -> &'static str {
        "vmware_desktop"
    }
    fn name(&self) -> &'static str {
        "VMware Desktop"
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

async fn detect(ctx: &Context<'_>) -> Detection {
    let mut findings = Vec::new();
    let vmrun = tools::find(
        ctx,
        "vmrun",
        &[
            (
                HostOs::Windows,
                r"%ProgramFiles(x86)%\VMware\VMware Workstation\vmrun.exe",
            ),
            (
                HostOs::Windows,
                r"%ProgramFiles%\VMware\VMware Workstation\vmrun.exe",
            ),
            (HostOs::Linux, "/usr/bin/vmrun"),
        ],
    );
    match &vmrun {
        Some(p) => findings.push(Finding::ok(format!("vmrun found at {}.", p.display()))),
        None => findings.push(Finding::problem(
            Level::Error,
            "VMware Workstation is not installed (vmrun not found).",
            install_hint(
                ctx.os,
                "VMware Workstation",
                "https://www.vmware.com/products/desktop-hypervisor",
            ),
        )),
    }

    let plugin = ctx.plugins.iter().find(|p| p.name == PLUGIN);
    match (plugin, ctx.vagrant_version.is_some()) {
        (Some(p), _) => findings.push(Finding::ok(format!(
            "Vagrant plugin {PLUGIN} {} is installed.",
            p.version.as_deref().unwrap_or("")
        ))),
        (None, true) => findings.push(Finding::problem(
            Level::Error,
            format!("The Vagrant plugin {PLUGIN} is not installed."),
            format!("Run: vagrant plugin install {PLUGIN}"),
        )),
        (None, false) => {}
    }

    if ctx.probe.is_open(UTILITY_ADDR).await {
        findings.push(Finding::ok("The Vagrant VMware Utility is running."));
    } else {
        let fix = match ctx.os {
            HostOs::Windows => {
                "Install the Vagrant VMware Utility if needed, then start its service: open Services and start “Vagrant VMware Utility”, or run `net start VagrantVMware` as Administrator."
            }
            _ => {
                "Install the Vagrant VMware Utility if needed, then start it: `sudo systemctl start vagrant-vmware-utility`."
            }
        };
        findings.push(Finding::problem(
            Level::Error,
            format!("The Vagrant VMware Utility is not reachable on {UTILITY_ADDR}."),
            format!("{fix} Download: https://developer.hashicorp.com/vagrant/install/vmware"),
        ));
    }

    Detection::new(vmrun.is_some(), None, findings)
}
