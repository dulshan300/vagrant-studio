//! The host's box store. Listing and the update check are quick read-only
//! queries; adding, updating and removing boxes are recorded host-level
//! operations that take the global box lock for writing.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use tauri::State;
use vagrant_studio_domain::{EnvironmentId, Operation, OperationType, ProjectId};
use vagrant_studio_operation_engine::{
    Confirmation, ConfirmationProof, OperationEngine, OperationRequest,
};
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_vagrant_adapter::boxes::{
    self, BoxAdd, BoxAddRequest, BoxRef, BoxSource, IndexedMachine, OutdatedBox,
};
use vagrant_studio_vagrant_adapter::machine_readable::{self, BoxInfo};
use vagrant_studio_vagrant_adapter::{ProcessRunner, QueryError, VagrantCommand, query};

use super::error::{CommandError, CommandResult};
use super::operations::vagrant_program;

const LIST_TIMEOUT: Duration = Duration::from_secs(120);
/// Contacts every box's catalog, a few seconds per box.
const OUTDATED_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// A safety net only: boxes can be several gigabytes, and the user can cancel.
const CHANGE_TIMEOUT: Duration = Duration::from_secs(4 * 60 * 60);

impl From<QueryError> for CommandError {
    fn from(err: QueryError) -> Self {
        CommandError::unavailable(err.to_string())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxStore {
    pub boxes: Vec<InstalledBox>,
    /// Why machine usage couldn't be read, if it couldn't.
    pub usage_error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledBox {
    #[serde(flatten)]
    pub info: BoxInfo,
    /// Machines Vagrant's index records as created from this box.
    pub users: Vec<BoxUser>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxUser {
    pub machine: String,
    pub provider: String,
    /// The state Vagrant last recorded, which may be out of date.
    pub state: Option<String>,
    pub folder: String,
    /// Vagrant ignores index entries whose folder no longer exists.
    pub folder_exists: bool,
    pub project_id: Option<ProjectId>,
    pub project_name: Option<String>,
    pub environment_id: Option<EnvironmentId>,
    pub environment_name: Option<String>,
}

fn box_ref(info: &BoxInfo) -> Option<BoxRef> {
    Some(BoxRef {
        name: info.name.clone(),
        provider: info.provider.clone()?,
        version: info.version.clone()?,
        architecture: info
            .architecture
            .clone()
            .filter(|a| a != "n/a" && !a.is_empty()),
    })
}

/// Index entries joined with the environments registered in the app.
async fn box_users(
    projects: &ProjectService,
    machines: Vec<IndexedMachine>,
) -> CommandResult<Vec<(IndexedMachine, BoxUser)>> {
    let folders: Vec<&str> = machines
        .iter()
        .map(|m| m.vagrantfile_path.as_str())
        .collect();
    let environments = projects.environments_at(&folders).await?;
    Ok(machines
        .into_iter()
        .zip(environments)
        .map(|(m, env)| {
            let user = BoxUser {
                machine: m.name.clone(),
                provider: m.provider.clone(),
                state: m.state.clone(),
                folder: m.vagrantfile_path.clone(),
                folder_exists: Path::new(&m.vagrantfile_path).is_dir(),
                project_id: env.as_ref().map(|(e, _)| e.project_id),
                project_name: env.as_ref().map(|(_, p)| p.clone()),
                environment_id: env.as_ref().map(|(e, _)| e.id),
                environment_name: env.map(|(e, _)| e.name),
            };
            (m, user)
        })
        .collect())
}

/// Installed boxes (`vagrant box list`) and the machines that use each one.
#[tauri::command]
pub async fn list_boxes(projects: State<'_, ProjectService>) -> CommandResult<BoxStore> {
    let program = vagrant_program(&projects).await?;
    let lines = query(
        &ProcessRunner,
        &program,
        &VagrantCommand::BoxList,
        LIST_TIMEOUT,
    )
    .await?;
    let infos = machine_readable::parse_box_list(&lines);

    let (machines, usage_error) = match boxes::vagrant_home() {
        None => (
            Vec::new(),
            Some("Vagrant's data folder couldn't be determined.".to_owned()),
        ),
        Some(home) => match boxes::read_machine_index(&home) {
            Ok(m) => (m, None),
            Err(e) => {
                tracing::warn!(error = %e, "could not read Vagrant's machine index");
                (
                    Vec::new(),
                    Some(format!("Vagrant's machine index couldn't be read: {e}")),
                )
            }
        },
    };
    let users = box_users(&projects, machines).await?;
    Ok(BoxStore {
        boxes: infos
            .into_iter()
            .map(|info| {
                let users = match box_ref(&info) {
                    Some(b) => users
                        .iter()
                        .filter(|(m, _)| m.uses(&b))
                        .map(|(_, u)| u.clone())
                        .collect(),
                    None => Vec::new(),
                };
                InstalledBox { info, users }
            })
            .collect(),
        usage_error,
    })
}

/// Asks each box's catalog for newer versions (`vagrant box outdated --global`).
#[tauri::command]
pub async fn check_box_updates(
    projects: State<'_, ProjectService>,
) -> CommandResult<Vec<OutdatedBox>> {
    let program = vagrant_program(&projects).await?;
    let lines = query(
        &ProcessRunner,
        &program,
        &VagrantCommand::BoxOutdated,
        OUTDATED_TIMEOUT,
    )
    .await?;
    Ok(boxes::parse_outdated(&lines))
}

async fn start_box_operation(
    projects: &ProjectService,
    engine: &OperationEngine,
    operation_type: OperationType,
    command: VagrantCommand,
    target: String,
    confirmation: Option<ConfirmationProof>,
) -> CommandResult<Operation> {
    let program = vagrant_program(projects).await?;
    Ok(engine
        .start(OperationRequest {
            operation_type,
            command,
            program,
            working_dir: std::env::temp_dir(),
            project_id: None,
            environment_id: None,
            machine_name: None,
            target_label: format!("Box · {target}"),
            env: vec![],
            timeout: Some(CHANGE_TIMEOUT),
            confirmation,
        })
        .await?)
}

#[tauri::command]
pub async fn add_box(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    request: BoxAddRequest,
) -> CommandResult<Operation> {
    let add = BoxAdd::try_from(request).map_err(CommandError::validation)?;
    let target = match (&add.name, &add.source) {
        (Some(name), _) => name.clone(),
        (None, BoxSource::Catalog(s) | BoxSource::Url(s)) => s.clone(),
        (None, BoxSource::File(p)) => p.display().to_string(),
    };
    start_box_operation(
        &projects,
        &engine,
        OperationType::BoxAdd,
        VagrantCommand::BoxAdd(add),
        target,
        None,
    )
    .await
}

/// Downloads the newest version of a box from its catalog. Older versions
/// stay installed until removed; machines keep the version they were created with.
#[tauri::command]
pub async fn update_box(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    target: BoxRef,
) -> CommandResult<Operation> {
    let label = format!("{} ({})", target.name, target.provider);
    start_box_operation(
        &projects,
        &engine,
        OperationType::BoxUpdate,
        VagrantCommand::BoxUpdate {
            name: target.name,
            provider: target.provider,
            architecture: target.architecture,
        },
        label,
        None,
    )
    .await
}

/// Issues the one-time token for removing exactly this box; the phrase is its name.
#[tauri::command]
pub fn request_box_removal(
    engine: State<'_, OperationEngine>,
    target: BoxRef,
) -> CommandResult<Confirmation> {
    let phrase = target.name.clone();
    Ok(engine.confirm(None, VagrantCommand::BoxRemove(target), phrase))
}

#[tauri::command]
pub async fn remove_box(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    target: BoxRef,
    confirmation: ConfirmationProof,
) -> CommandResult<Operation> {
    let label = target.label();
    start_box_operation(
        &projects,
        &engine,
        OperationType::BoxRemove,
        VagrantCommand::BoxRemove(target),
        label,
        Some(confirmation),
    )
    .await
}
