//! Provider capability model and detection (docs/provider-capabilities.md).
//!
//! Each [`ProviderAdapter`] checks whether its provider is usable on this host
//! and which prerequisites are missing. Capabilities are reported only for
//! providers that detection found, and a provider counts as *verified* only
//! once it has passed the full test plan on this host OS ([`VERIFIED`]).
//! Checks are read-only: nothing is installed, started or reconfigured.

mod compat;
mod docker;
mod libvirt;
mod ports;
mod probe;
mod tools;
mod virtualbox;
mod vmware;

#[cfg(test)]
mod tests;

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use serde::Serialize;
use vagrant_studio_platform::{HostOs, PlatformAdapter};
use vagrant_studio_vagrant_adapter::Runner;
use vagrant_studio_vagrant_adapter::machine_readable::PluginInfo;
use vagrant_studio_vagrant_adapter::readiness::{
    Finding, Level, VagrantCheck, check_vagrant, plugin_list,
};

pub use compat::{VirtualBoxSupport, virtualbox_support};
pub use docker::Docker;
pub use libvirt::Libvirt;
pub use ports::{HostPort, ports_in_use};
pub use probe::{HostProbe, SystemProbe};
pub use virtualbox::{BridgedInterface, VirtualBox, bridged_interfaces};
pub use vmware::VmwareDesktop;

/// What a provider can do through Vagrant, as listed in
/// docs/provider-capabilities.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Capability {
    #[serde(rename = "lifecycle.basic")]
    LifecycleBasic,
    #[serde(rename = "lifecycle.suspend")]
    LifecycleSuspend,
    #[serde(rename = "provision.shell")]
    ProvisionShell,
    #[serde(rename = "network.forwarded_port")]
    NetworkForwardedPort,
    #[serde(rename = "network.private")]
    NetworkPrivate,
    #[serde(rename = "network.public")]
    NetworkPublic,
    #[serde(rename = "synced_folder.default")]
    SyncedFolderDefault,
    #[serde(rename = "config.cpu_memory")]
    ConfigCpuMemory,
    #[serde(rename = "ssh")]
    Ssh,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "export")]
    Export,
}

/// The capabilities Vagrant's VM providers offer.
const VM_CAPABILITIES: &[Capability] = &[
    Capability::LifecycleBasic,
    Capability::LifecycleSuspend,
    Capability::ProvisionShell,
    Capability::NetworkForwardedPort,
    Capability::NetworkPrivate,
    Capability::NetworkPublic,
    Capability::SyncedFolderDefault,
    Capability::ConfigCpuMemory,
    Capability::Ssh,
    Capability::Snapshot,
    Capability::Export,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Vm,
    Container,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderState {
    /// All checks passed.
    Ready,
    /// Installed, but something needs attention before machines can run.
    Attention,
    /// Not installed or not usable.
    Unavailable,
}

/// Support levels from docs/provider-capabilities.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SupportLevel {
    /// Passed the integration test plan on this host OS.
    Verified,
    /// Usable according to detection, but not verified by Vagrant Studio.
    Detected,
    Unavailable,
}

/// Providers that passed the full test plan, per host OS. Add an entry only
/// together with a row in the verification log of docs/provider-capabilities.md.
pub const VERIFIED: &[(&str, HostOs)] = &[("virtualbox", HostOs::Windows)];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderReport {
    /// Vagrant provider name, e.g. `virtualbox`, `vmware_desktop`, `docker`.
    pub id: &'static str,
    pub name: &'static str,
    pub kind: ProviderKind,
    pub state: ProviderState,
    pub support: SupportLevel,
    pub version: Option<String>,
    /// Empty unless detection found the provider.
    pub capabilities: Vec<Capability>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostReadiness {
    pub vagrant: VagrantCheck,
    pub plugins: Vec<PluginInfo>,
    pub providers: Vec<ProviderReport>,
}

/// What adapters can use while detecting.
pub struct Context<'a> {
    pub runner: &'a dyn Runner,
    pub platform: &'a dyn PlatformAdapter,
    pub probe: &'a dyn HostProbe,
    pub os: HostOs,
    /// `None` when Vagrant isn't installed or couldn't be run.
    pub vagrant_version: Option<&'a str>,
    pub plugins: &'a [PluginInfo],
}

/// What an adapter found on this host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    pub state: ProviderState,
    pub version: Option<String>,
    pub findings: Vec<Finding>,
}

impl Detection {
    /// Unavailable when the provider wasn't found; otherwise ready unless a
    /// finding needs attention.
    fn new(found: bool, version: Option<String>, findings: Vec<Finding>) -> Self {
        let state = if !found {
            ProviderState::Unavailable
        } else if findings.iter().any(|f| f.level != Level::Ok) {
            ProviderState::Attention
        } else {
            ProviderState::Ready
        };
        Self {
            state,
            version,
            findings,
        }
    }
}

pub type DetectFuture<'a> = Pin<Box<dyn Future<Output = Detection> + Send + 'a>>;

pub trait ProviderAdapter: Send + Sync {
    /// Vagrant's provider name.
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn kind(&self) -> ProviderKind;
    /// What the provider supports once it is usable.
    fn capabilities(&self) -> &'static [Capability];
    /// Whether the provider exists on this OS; others aren't reported at all.
    fn runs_on(&self, _os: HostOs) -> bool {
        true
    }
    fn detect<'a>(&'a self, ctx: &'a Context<'a>) -> DetectFuture<'a>;
}

pub struct Registry {
    adapters: Vec<Box<dyn ProviderAdapter>>,
}

impl Default for Registry {
    /// The providers Vagrant Studio has integration for, in display order.
    fn default() -> Self {
        Self {
            adapters: vec![
                Box::new(VirtualBox),
                Box::new(VmwareDesktop),
                Box::new(Libvirt),
                Box::new(Docker),
            ],
        }
    }
}

impl Registry {
    pub fn adapter(&self, id: &str) -> Option<&dyn ProviderAdapter> {
        self.adapters
            .iter()
            .find(|a| a.id() == id)
            .map(|a| a.as_ref())
    }

    /// Checks Vagrant, its plugins and every provider.
    pub async fn check_host(
        &self,
        runner: &dyn Runner,
        platform: &dyn PlatformAdapter,
        probe: &dyn HostProbe,
        vagrant_override: Option<&Path>,
    ) -> HostReadiness {
        let (vagrant, program) = check_vagrant(runner, platform, vagrant_override).await;
        let plugins = match &program {
            Some(p) => plugin_list(runner, p).await,
            None => Vec::new(),
        };
        let ctx = Context {
            runner,
            platform,
            probe,
            os: platform.host_info().os,
            vagrant_version: vagrant.version.as_deref(),
            plugins: &plugins,
        };
        let mut providers = Vec::with_capacity(self.adapters.len());
        for adapter in self.adapters.iter().filter(|a| a.runs_on(ctx.os)) {
            let detection = adapter.detect(&ctx).await;
            providers.push(report(adapter.as_ref(), detection, ctx.os));
        }
        HostReadiness {
            vagrant,
            plugins,
            providers,
        }
    }
}

fn report(adapter: &dyn ProviderAdapter, detection: Detection, os: HostOs) -> ProviderReport {
    let found = detection.state != ProviderState::Unavailable;
    let support = if !found {
        SupportLevel::Unavailable
    } else if VERIFIED.contains(&(adapter.id(), os)) {
        SupportLevel::Verified
    } else {
        SupportLevel::Detected
    };
    ProviderReport {
        id: adapter.id(),
        name: adapter.name(),
        kind: adapter.kind(),
        state: detection.state,
        support,
        version: detection.version,
        capabilities: if found {
            adapter.capabilities().to_vec()
        } else {
            Vec::new()
        },
        findings: detection.findings,
    }
}
