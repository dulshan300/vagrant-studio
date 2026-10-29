//! Shell provisioners: what each machine runs (from the Vagrantfile) and
//! what the last runs did (from the output of `up`, `reload` and `provision`).

use serde::Serialize;
use tauri::State;
use vagrant_studio_config_model::MachineProvisioners;
use vagrant_studio_domain::{EnvironmentId, OperationId};
use vagrant_studio_operation_engine::{MachineProvisioning, OperationEngine};
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_vagrant_adapter::{MachineName, Provisioning};

use super::error::{CommandError, CommandResult};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionerList {
    pub machines: Vec<MachineProvisioners>,
    /// Why the provisioners can't be listed (no Vagrantfile, or one the
    /// visual editor can't read); `machines` is then empty.
    pub unavailable: Option<String>,
}

async fn provisioner_list(
    service: &ProjectService,
    environment_id: EnvironmentId,
) -> CommandResult<ProvisionerList> {
    let doc = service.vagrantfile(environment_id).await?;
    let unavailable = |reason: String| ProvisionerList {
        machines: Vec::new(),
        unavailable: Some(reason),
    };
    if !doc.exists {
        return Ok(unavailable("This environment has no Vagrantfile.".into()));
    }
    Ok(match (doc.analysis.model, doc.analysis.unsupported) {
        (Some(model), _) => ProvisionerList {
            machines: model.provisioners_by_machine(),
            unavailable: None,
        },
        (None, Some(u)) => unavailable(format!(
            "The Vagrantfile can't be read here (line {}: {}). Provision runs all provisioners.",
            u.line, u.message
        )),
        (None, None) => unavailable("The Vagrantfile can't be read here.".into()),
    })
}

#[tauri::command]
pub async fn list_provisioners(
    service: State<'_, ProjectService>,
    environment_id: EnvironmentId,
) -> CommandResult<ProvisionerList> {
    provisioner_list(&service, environment_id).await
}

/// Vagrant ignores `--provision-with` names it doesn't know and reports
/// success, so chosen names must exist for every targeted machine.
pub(crate) async fn check_provisioner_names(
    service: &ProjectService,
    environment_id: EnvironmentId,
    targets: &[MachineName],
    provisioning: &Provisioning,
) -> CommandResult<()> {
    let Provisioning::Only { names } = provisioning else {
        return Ok(());
    };
    let list = provisioner_list(service, environment_id).await?;
    if let Some(reason) = list.unavailable {
        return Err(CommandError::validation(reason));
    }
    for target in targets {
        let Some(machine) = list.machines.iter().find(|m| m.machine == target.as_str()) else {
            return Err(CommandError::validation(format!(
                "The Vagrantfile doesn't define a machine named “{}”.",
                target.as_str()
            )));
        };
        if let Some(missing) = names.iter().find(|n| {
            !machine
                .provisioners
                .iter()
                .any(|p| p.provisioner.name.as_deref() == Some(n.as_str()))
        }) {
            return Err(CommandError::validation(format!(
                "{} has no provisioner named “{}”.",
                target.as_str(),
                missing.as_str()
            )));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_operation_provisioning(
    engine: State<'_, OperationEngine>,
    operation_id: OperationId,
) -> CommandResult<Vec<MachineProvisioning>> {
    Ok(engine.operation_provisioning(operation_id).await?)
}

/// The most recent provisioner results of each machine.
#[tauri::command]
pub async fn get_latest_provisioning(
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
) -> CommandResult<Vec<MachineProvisioning>> {
    Ok(engine.latest_provisioning(environment_id).await?)
}
