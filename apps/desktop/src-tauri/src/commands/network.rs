//! Host facts for the network settings of the Vagrantfile editor, and the
//! forwarded ports of every registered environment. All are read-only and
//! change nothing on the host.

use serde::Serialize;
use tauri::State;
use vagrant_studio_config_model::Protocol;
use vagrant_studio_domain::{EnvironmentId, ProjectId};
use vagrant_studio_platform::{PlatformAdapter, SystemPlatform};
use vagrant_studio_project_service::ProjectService;
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

/// One host port a machine forwards, as its environment's Vagrantfile says.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortUse {
    pub project_id: ProjectId,
    pub project_name: String,
    pub environment_id: EnvironmentId,
    pub environment_name: String,
    pub machine: String,
    /// `None` means every address, as Vagrant binds it.
    pub host_ip: Option<String>,
    pub host: u16,
    pub guest: u16,
    pub protocol: Protocol,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardedPorts {
    pub ports: Vec<PortUse>,
    /// Environments whose Vagrantfile is missing or can't be read here, as
    /// "project · environment"; their ports are unknown.
    pub unchecked: Vec<String>,
}

/// Forwarded ports of every registered environment, from their Vagrantfiles,
/// so the UI can warn when two environments want the same host port.
#[tauri::command]
pub async fn list_forwarded_ports(
    projects: State<'_, ProjectService>,
) -> super::error::CommandResult<ForwardedPorts> {
    let mut out = ForwardedPorts {
        ports: Vec::new(),
        unchecked: Vec::new(),
    };
    for (env, project_name) in projects.all_environments().await? {
        let label = format!("{project_name} · {}", env.name);
        let model = match projects.vagrantfile(env.id).await {
            Ok(doc) if doc.exists => doc.analysis.model,
            _ => None,
        };
        let Some(model) = model else {
            if env.vagrantfile_path.is_some() {
                out.unchecked.push(label);
            }
            continue;
        };
        for (machine, ports) in model.forwarded_ports_by_machine() {
            for p in ports {
                out.ports.push(PortUse {
                    project_id: env.project_id,
                    project_name: project_name.clone(),
                    environment_id: env.id,
                    environment_name: env.name.clone(),
                    machine: machine.clone(),
                    host_ip: p.host_ip.filter(|ip| !ip.trim().is_empty()),
                    host: p.host,
                    guest: p.guest,
                    protocol: p.protocol.unwrap_or(Protocol::Tcp),
                });
            }
        }
    }
    Ok(out)
}
