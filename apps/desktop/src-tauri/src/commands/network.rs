//! Host facts for the network settings of the Vagrantfile editor. Both are
//! read-only and change nothing on the host.

use vagrant_studio_platform::{PlatformAdapter, SystemPlatform};
use vagrant_studio_provider_registry::{
    BridgedInterface, Context, HostPort, SystemProbe, bridged_interfaces, ports_in_use,
};
use vagrant_studio_vagrant_adapter::ProcessRunner;

/// For each port, whether something on this computer already accepts TCP
/// connections on it (what makes `vagrant up` fail with a port collision).
#[tauri::command]
pub async fn check_host_ports(ports: Vec<HostPort>) -> Vec<bool> {
    ports_in_use(&ports).await
}

/// Host adapters a VirtualBox public network can bridge to; empty when
/// VirtualBox isn't installed or didn't answer.
#[tauri::command]
pub async fn list_bridged_interfaces() -> Vec<BridgedInterface> {
    let ctx = Context {
        runner: &ProcessRunner,
        platform: &SystemPlatform,
        probe: &SystemProbe,
        os: SystemPlatform.host_info().os,
        vagrant_version: None,
        plugins: &[],
    };
    bridged_interfaces(&ctx).await.unwrap_or_default()
}
