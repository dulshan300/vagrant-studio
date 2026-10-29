//! Provisioner results of `up`, `reload` and `provision`, read from their
//! output when the command ends and stored per machine.

use vagrant_studio_domain::{EnvironmentId, OperationId, OperationStatus};
use vagrant_studio_project_store::ProvisioningRecord;
use vagrant_studio_vagrant_adapter::provision::{self, CommandEnd, ProvisionerRun};
use vagrant_studio_vagrant_adapter::{RunResult, Termination, VagrantCommand};

/// The provisioners one machine ran in one operation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineProvisioning {
    pub operation_id: OperationId,
    pub machine: String,
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: time::OffsetDateTime,
    pub runs: Vec<ProvisionerRun>,
}

/// Provisioner runs grouped by machine (in the order machines started
/// provisioning), or nothing for commands that don't provision.
pub(crate) fn collect(
    operation_id: OperationId,
    command: &VagrantCommand,
    outcome: &Result<RunResult, std::io::Error>,
    status: OperationStatus,
    stdout: &[String],
) -> Vec<MachineProvisioning> {
    let VagrantCommand::Lifecycle { action, .. } = command else {
        return Vec::new();
    };
    if !action.runs_provisioners() || outcome.is_err() {
        return Vec::new();
    }
    let end = match (status, outcome) {
        (OperationStatus::Succeeded, _) => CommandEnd::Succeeded,
        (_, Ok(r)) if r.termination == Termination::Exited => CommandEnd::Failed,
        _ => CommandEnd::Stopped,
    };
    let finished_at = vagrant_studio_project_store::now();
    let mut machines: Vec<MachineProvisioning> = Vec::new();
    for run in provision::parse(stdout, end) {
        match machines.iter_mut().find(|m| m.machine == run.machine) {
            Some(m) => m.runs.push(run),
            None => machines.push(MachineProvisioning {
                operation_id,
                machine: run.machine.clone(),
                finished_at,
                runs: vec![run],
            }),
        }
    }
    machines
}

pub(crate) fn to_record(
    environment_id: Option<EnvironmentId>,
    m: &MachineProvisioning,
) -> ProvisioningRecord {
    ProvisioningRecord {
        operation_id: m.operation_id.to_string(),
        environment_id: environment_id.map(|e| e.to_string()),
        machine_name: m.machine.clone(),
        finished_at: m.finished_at,
        runs_json: serde_json::to_string(&m.runs).expect("serializable"),
    }
}

pub(crate) fn from_record(r: ProvisioningRecord) -> Option<MachineProvisioning> {
    Some(MachineProvisioning {
        operation_id: r.operation_id.parse().ok()?,
        machine: r.machine_name,
        finished_at: r.finished_at,
        runs: serde_json::from_str(&r.runs_json).unwrap_or_default(),
    })
}
