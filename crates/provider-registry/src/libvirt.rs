//! libvirt through the `vagrant-libvirt` plugin, the preferred path on Fedora
//! (docs/provider-capabilities.md). Linux only. The plugin's default
//! connection is `qemu:///system`, so that is the one checked.

use vagrant_studio_platform::HostOs;
use vagrant_studio_vagrant_adapter::readiness::{Finding, Level};

use crate::{
    Capability, Context, DetectFuture, Detection, ProviderAdapter, ProviderKind, VM_CAPABILITIES,
    tools,
};

const PLUGIN: &str = "vagrant-libvirt";
const KVM_DEVICE: &str = "/dev/kvm";

pub struct Libvirt;

impl ProviderAdapter for Libvirt {
    fn id(&self) -> &'static str {
        "libvirt"
    }
    fn name(&self) -> &'static str {
        "libvirt"
    }
    fn kind(&self) -> ProviderKind {
        ProviderKind::Vm
    }
    fn capabilities(&self) -> &'static [Capability] {
        VM_CAPABILITIES
    }
    fn runs_on(&self, os: HostOs) -> bool {
        os == HostOs::Linux
    }
    fn detect<'a>(&'a self, ctx: &'a Context<'a>) -> DetectFuture<'a> {
        Box::pin(detect(ctx))
    }
}

async fn detect(ctx: &Context<'_>) -> Detection {
    let mut findings = Vec::new();
    let virsh = tools::find(ctx, "virsh", &[(HostOs::Linux, "/usr/bin/virsh")]);
    let version = match &virsh {
        Some(exe) => {
            findings.push(Finding::ok(format!("virsh found at {}.", exe.display())));
            tools::run(ctx, exe, &["--version"])
                .await
                .filter(|o| o.success)
                .and_then(|o| o.first_line().map(str::to_owned))
        }
        None => {
            findings.push(Finding::problem(
                Level::Error,
                "libvirt is not installed (virsh not found).",
                "Install libvirt with your package manager, for example `sudo dnf install @virtualization` on Fedora or `sudo apt install libvirt-daemon-system libvirt-clients` on Ubuntu.",
            ));
            None
        }
    };

    let plugin = ctx.plugins.iter().find(|p| p.name == PLUGIN);
    match (plugin, ctx.vagrant_version.is_some()) {
        (Some(p), _) => findings.push(Finding::ok(format!(
            "Vagrant plugin {PLUGIN} {} is installed.",
            p.version.as_deref().unwrap_or("")
        ))),
        (None, true) => findings.push(Finding::problem(
            Level::Error,
            format!("The Vagrant plugin {PLUGIN} is not installed."),
            format!(
                "Install its build dependencies, then run: vagrant plugin install {PLUGIN}. See https://vagrant-libvirt.github.io/vagrant-libvirt/installation.html"
            ),
        )),
        (None, false) => {}
    }

    if ctx.probe.path_exists(std::path::Path::new(KVM_DEVICE)) {
        findings.push(Finding::ok(format!("{KVM_DEVICE} is present.")));
    } else {
        findings.push(Finding::problem(
            Level::Error,
            format!("{KVM_DEVICE} is missing, so machines can't use hardware virtualization."),
            "Enable virtualization (VT-x or AMD-V) in the firmware settings and load the kvm module (`sudo modprobe kvm_intel` or `kvm_amd`).",
        ));
    }

    if let Some(exe) = &virsh {
        // Asks the daemon, so it fails when it is stopped or not accessible.
        let output = tools::run(ctx, exe, &["-c", "qemu:///system", "version"]).await;
        match output {
            Some(o) if o.success => {
                findings.push(Finding::ok("The libvirt daemon is reachable."));
            }
            other => {
                let detail = other
                    .as_ref()
                    .and_then(|o| o.stderr.lines().map(str::trim).find(|l| !l.is_empty()))
                    .map(|l| format!(" virsh said: {l}"))
                    .unwrap_or_default();
                findings.push(Finding::problem(
                    Level::Error,
                    format!("The libvirt daemon isn't reachable.{detail}"),
                    "Start it (`sudo systemctl enable --now libvirtd`, or `virtqemud.socket` on Fedora) and make sure your user may use it (member of the `libvirt` group; log in again after adding it).",
                ));
            }
        }
    }

    Detection::new(virsh.is_some(), version, findings)
}
