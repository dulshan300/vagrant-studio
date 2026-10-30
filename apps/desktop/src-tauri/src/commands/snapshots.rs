//! Machine snapshots and export to a `.box` file. Listing is a quick read-only
//! query; everything else is a recorded operation. Restoring and deleting
//! lose state, so they need a confirmation like `destroy`. The UI offers
//! these only for providers with the matching capability; Vagrant has the
//! last word and its refusal is shown as is.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::State;
use vagrant_studio_domain::{EnvironmentId, Operation, OperationType};
use vagrant_studio_operation_engine::{Confirmation, ConfirmationProof, OperationEngine};
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_vagrant_adapter::snapshot::{SnapshotName, parse_snapshots};
use vagrant_studio_vagrant_adapter::{
    MachineName, ProcessRunner, StateClass, VagrantCommand, query_in,
};

use super::error::{CommandError, CommandResult};
use super::operations::{
    EnvironmentRun, lifecycle_targets, start_environment_operation, vagrant_program,
};

const LIST_TIMEOUT: Duration = Duration::from_secs(120);
/// A safety net only: the user can cancel at any time.
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(60 * 60);
/// Exporting compresses the whole disk.
const PACKAGE_TIMEOUT: Duration = Duration::from_secs(4 * 60 * 60);

/// Which snapshot change needs a confirmation.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotChange {
    Restore,
    Delete,
}

impl SnapshotChange {
    fn command(self, machine: MachineName, name: SnapshotName) -> VagrantCommand {
        match self {
            Self::Restore => VagrantCommand::SnapshotRestore { machine, name },
            Self::Delete => VagrantCommand::SnapshotDelete { machine, name },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineSnapshots {
    pub machine: String,
    /// In the order the provider lists them.
    pub names: Vec<String>,
}

async fn one_machine(
    engine: &OperationEngine,
    environment_id: EnvironmentId,
    machine: &str,
) -> CommandResult<MachineName> {
    let targets = lifecycle_targets(engine, environment_id, &[machine.to_owned()]).await?;
    targets
        .into_iter()
        .next()
        .ok_or_else(|| CommandError::validation("Choose a machine."))
}

fn snapshot_name(name: String) -> CommandResult<SnapshotName> {
    SnapshotName::try_from(name).map_err(CommandError::validation)
}

#[tauri::command]
pub async fn list_snapshots(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
    machine: String,
) -> CommandResult<MachineSnapshots> {
    let target = one_machine(&engine, environment_id, &machine).await?;
    let (_, env) = projects.environment(environment_id).await?;
    let program = vagrant_program(&projects).await?;
    let lines = query_in(
        &ProcessRunner,
        &program,
        &VagrantCommand::SnapshotList {
            machine: target.clone(),
        },
        PathBuf::from(&env.root_path),
        env.default_provider
            .iter()
            .map(|p| ("VAGRANT_DEFAULT_PROVIDER".to_owned(), p.clone()))
            .collect(),
        LIST_TIMEOUT,
    )
    .await?;
    Ok(MachineSnapshots {
        names: parse_snapshots(&lines, target.as_str()),
        machine,
    })
}

#[tauri::command]
pub async fn save_snapshot(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
    machine: String,
    name: String,
) -> CommandResult<Operation> {
    let target = one_machine(&engine, environment_id, &machine).await?;
    start_environment_operation(
        &projects,
        &engine,
        environment_id,
        EnvironmentRun {
            operation_type: OperationType::SnapshotSave,
            command: VagrantCommand::SnapshotSave {
                machine: target.clone(),
                name: snapshot_name(name)?,
            },
            machines: vec![target],
            timeout: SNAPSHOT_TIMEOUT,
            confirmation: None,
        },
    )
    .await
}

/// Issues the one-time token and the phrase to type: the snapshot's name.
#[tauri::command]
pub async fn request_snapshot_confirmation(
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
    machine: String,
    change: SnapshotChange,
    name: String,
) -> CommandResult<Confirmation> {
    let target = one_machine(&engine, environment_id, &machine).await?;
    let snapshot = snapshot_name(name)?;
    let phrase = snapshot.as_str().to_owned();
    Ok(engine.confirm(
        Some(environment_id),
        change.command(target, snapshot),
        phrase,
    ))
}

#[tauri::command]
pub async fn change_snapshot(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
    machine: String,
    change: SnapshotChange,
    name: String,
    confirmation: ConfirmationProof,
) -> CommandResult<Operation> {
    let target = one_machine(&engine, environment_id, &machine).await?;
    start_environment_operation(
        &projects,
        &engine,
        environment_id,
        EnvironmentRun {
            operation_type: match change {
                SnapshotChange::Restore => OperationType::SnapshotRestore,
                SnapshotChange::Delete => OperationType::SnapshotDelete,
            },
            command: change.command(target.clone(), snapshot_name(name)?),
            machines: vec![target],
            timeout: SNAPSHOT_TIMEOUT,
            confirmation: Some(confirmation),
        },
    )
    .await
}

/// The chosen file must be new: an existing file is never overwritten.
fn check_export_path(output: &str) -> CommandResult<PathBuf> {
    let path = Path::new(output);
    let is_box = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("box"));
    if !path.is_absolute() || !is_box {
        return Err(CommandError::validation(
            "Choose a full path ending in .box.",
        ));
    }
    if !path.parent().is_some_and(Path::is_dir) {
        return Err(CommandError::validation(
            "The folder for the .box file doesn't exist.",
        ));
    }
    if std::fs::symlink_metadata(path).is_ok() {
        return Err(CommandError::validation(
            "That file already exists. Choose a new name; Vagrant Studio doesn't overwrite files.",
        ));
    }
    Ok(path.to_path_buf())
}

/// Runs `vagrant package` for a stopped machine (providers stop a running
/// machine to export it, which the user should do knowingly).
#[tauri::command]
pub async fn export_machine(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
    machine: String,
    output: String,
) -> CommandResult<Operation> {
    let target = one_machine(&engine, environment_id, &machine).await?;
    let stopped = engine
        .environment_status(environment_id)
        .await?
        .iter()
        .flat_map(|s| &s.machines)
        .find(|m| m.name == machine)
        .and_then(|m| m.state.as_deref())
        .is_some_and(|s| StateClass::of(s) == StateClass::Stopped);
    if !stopped {
        return Err(CommandError::validation(
            "Stop the machine before exporting it.",
        ));
    }
    let output = check_export_path(&output)?;
    start_environment_operation(
        &projects,
        &engine,
        environment_id,
        EnvironmentRun {
            operation_type: OperationType::Package,
            command: VagrantCommand::Package {
                machine: target.clone(),
                output,
            },
            machines: vec![target],
            timeout: PACKAGE_TIMEOUT,
            confirmation: None,
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_path_must_be_a_new_box_file_in_an_existing_folder() {
        let dir = tempfile::tempdir().unwrap();
        let free = dir.path().join("web.box");
        assert_eq!(
            check_export_path(free.to_str().unwrap()).unwrap(),
            free.clone()
        );
        assert!(check_export_path(dir.path().join("WEB.BOX").to_str().unwrap()).is_ok());

        std::fs::write(&free, "keep").unwrap();
        for bad in [
            free.to_str().unwrap().to_owned(),
            "web.box".to_owned(),
            dir.path().join("web.zip").display().to_string(),
            dir.path().join("none/web.box").display().to_string(),
        ] {
            assert!(check_export_path(&bad).is_err(), "{bad}");
        }
    }
}
