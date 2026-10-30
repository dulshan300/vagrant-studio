use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Runtime, State};
use tauri_plugin_opener::OpenerExt;
use vagrant_studio_domain::{EnvironmentId, Operation, OperationId, OperationType, ProjectId};
use vagrant_studio_operation_engine::lifecycle::{
    check_provisioning, confirmation_phrase, operation_type, order_targets,
};
use vagrant_studio_operation_engine::{
    Confirmation, ConfirmationProof, EnvironmentStatus, EventSink, LogLine, OperationEngine,
    OperationEvent, OperationFilter, OperationRequest,
};
use vagrant_studio_platform::SystemPlatform;
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_provider_registry::{HostReadiness, Registry, SystemProbe};
use vagrant_studio_vagrant_adapter::{
    LifecycleAction, MachineName, ProcessRunner, Provisioning, VagrantCommand, locate,
};

use super::error::{CommandError, CommandResult};

/// Tauri event carrying every [`OperationEvent`].
pub const OPERATION_EVENT: &str = "vagrant-studio://operation";
const LOG_LINES: usize = 5_000;
const STATUS_TIMEOUT: Duration = Duration::from_secs(180);
/// A safety net only: `up` may download a multi-gigabyte box first, and the
/// user can cancel at any time.
const LIFECYCLE_TIMEOUT: Duration = Duration::from_secs(4 * 60 * 60);

/// Forwards engine events to the WebView.
pub struct TauriSink<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> EventSink for TauriSink<R> {
    fn emit(&self, event: OperationEvent) {
        if let Err(e) = self.0.emit(OPERATION_EVENT, &event) {
            tracing::warn!(error = %e, "failed to emit operation event");
        }
    }
}

async fn vagrant_override(projects: &ProjectService) -> CommandResult<Option<PathBuf>> {
    Ok(projects.settings().await?.vagrant_path.map(PathBuf::from))
}

pub(crate) async fn vagrant_program(projects: &ProjectService) -> CommandResult<PathBuf> {
    let override_path = vagrant_override(projects).await?;
    locate(override_path.as_deref(), &SystemPlatform).ok_or_else(|| {
        CommandError::unavailable(
            "Vagrant was not found. Install it, or set its location in Settings.",
        )
    })
}

#[tauri::command]
pub async fn get_host_readiness(
    projects: State<'_, ProjectService>,
) -> CommandResult<HostReadiness> {
    let override_path = vagrant_override(&projects).await?;
    Ok(Registry::default()
        .check_host(
            &ProcessRunner,
            &SystemPlatform,
            &SystemProbe,
            override_path.as_deref(),
        )
        .await)
}

/// What to run in an environment, beyond the command itself.
struct EnvironmentRun {
    operation_type: OperationType,
    command: VagrantCommand,
    machines: Vec<MachineName>,
    timeout: Duration,
    confirmation: Option<ConfirmationProof>,
}

async fn start_environment_operation(
    projects: &ProjectService,
    engine: &OperationEngine,
    environment_id: EnvironmentId,
    run: EnvironmentRun,
) -> CommandResult<Operation> {
    let (project, env) = projects.environment(environment_id).await?;
    if !Path::new(&env.root_path).is_dir() {
        return Err(CommandError::unavailable(format!(
            "The environment folder {} no longer exists.",
            env.root_path
        )));
    }
    let program = vagrant_program(projects).await?;
    let names: Vec<&str> = run.machines.iter().map(MachineName::as_str).collect();
    let mut target_label = format!("{} · {}", project.name, env.name);
    if !names.is_empty() {
        target_label = format!("{target_label} · {}", names.join(", "));
    }
    Ok(engine
        .start(OperationRequest {
            operation_type: run.operation_type,
            command: run.command,
            program,
            working_dir: PathBuf::from(&env.root_path),
            project_id: Some(project.id),
            environment_id: Some(env.id),
            machine_name: match names.as_slice() {
                [one] => Some((*one).to_owned()),
                _ => None,
            },
            target_label,
            // Machines that don't exist yet are reported for the default provider.
            env: env
                .default_provider
                .iter()
                .map(|p| ("VAGRANT_DEFAULT_PROVIDER".to_owned(), p.clone()))
                .collect(),
            timeout: Some(run.timeout),
            confirmation: run.confirmation,
        })
        .await?)
}

#[tauri::command]
pub async fn refresh_environment_status(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
) -> CommandResult<Operation> {
    start_environment_operation(
        &projects,
        &engine,
        environment_id,
        EnvironmentRun {
            operation_type: OperationType::Status,
            command: VagrantCommand::Status { machine: None },
            machines: vec![],
            timeout: STATUS_TIMEOUT,
            confirmation: None,
        },
    )
    .await
}

#[tauri::command]
pub async fn validate_environment(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
) -> CommandResult<Operation> {
    start_environment_operation(
        &projects,
        &engine,
        environment_id,
        EnvironmentRun {
            operation_type: OperationType::Validate,
            command: VagrantCommand::Validate,
            machines: vec![],
            timeout: STATUS_TIMEOUT,
            confirmation: None,
        },
    )
    .await
}

/// Machines checked against the last `vagrant status`, in Vagrantfile order.
async fn lifecycle_targets(
    engine: &OperationEngine,
    environment_id: EnvironmentId,
    machines: &[String],
) -> CommandResult<Vec<MachineName>> {
    let status = engine.environment_status(environment_id).await?;
    Ok(order_targets(status.as_ref(), machines)?)
}

/// Runs `up`, `halt`, `reload`, `suspend`, `resume`, `provision` or `destroy`
/// for the named machines. `destroy` needs a confirmation from
/// [`request_lifecycle_confirmation`]. `provisioning` chooses which
/// provisioners `up`, `reload` and `provision` run.
#[tauri::command]
pub async fn run_lifecycle(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
    action: LifecycleAction,
    machines: Vec<String>,
    provisioning: Option<Provisioning>,
    confirmation: Option<ConfirmationProof>,
) -> CommandResult<Operation> {
    let provisioning = check_provisioning(action, provisioning.unwrap_or_default())?;
    let targets = lifecycle_targets(&engine, environment_id, &machines).await?;
    super::provisioning::check_provisioner_names(
        &projects,
        environment_id,
        &targets,
        &provisioning,
    )
    .await?;
    start_environment_operation(
        &projects,
        &engine,
        environment_id,
        EnvironmentRun {
            operation_type: operation_type(action),
            command: VagrantCommand::Lifecycle {
                action,
                machines: targets.clone(),
                provisioning,
            },
            machines: targets,
            timeout: LIFECYCLE_TIMEOUT,
            confirmation,
        },
    )
    .await
}

/// Issues the one-time token and the phrase to type for a destructive action.
#[tauri::command]
pub async fn request_lifecycle_confirmation(
    projects: State<'_, ProjectService>,
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
    action: LifecycleAction,
    machines: Vec<String>,
) -> CommandResult<Confirmation> {
    if !action.is_destructive() {
        return Err(CommandError::validation(format!(
            "`vagrant {}` doesn't need confirmation.",
            action.verb()
        )));
    }
    let (_, env) = projects.environment(environment_id).await?;
    let targets = lifecycle_targets(&engine, environment_id, &machines).await?;
    let phrase = confirmation_phrase(&env.name, &targets);
    Ok(engine.confirm(
        Some(environment_id),
        VagrantCommand::Lifecycle {
            action,
            machines: targets,
            provisioning: Provisioning::Default,
        },
        phrase,
    ))
}

#[tauri::command]
pub async fn get_environment_status(
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
) -> CommandResult<Option<EnvironmentStatus>> {
    Ok(engine.environment_status(environment_id).await?)
}

#[tauri::command]
pub async fn list_environment_statuses(
    engine: State<'_, OperationEngine>,
) -> CommandResult<Vec<EnvironmentStatus>> {
    Ok(engine.all_environment_statuses().await?)
}

/// Newest first. Every filter is optional; `machine` matches operations that
/// named that machine.
#[tauri::command]
pub async fn list_operations(
    engine: State<'_, OperationEngine>,
    project_id: Option<ProjectId>,
    environment_id: Option<EnvironmentId>,
    machine: Option<String>,
    limit: Option<i64>,
) -> CommandResult<Vec<Operation>> {
    let filter = OperationFilter {
        project_id,
        environment_id,
        machine,
    };
    Ok(engine.list(&filter, limit.unwrap_or(100)).await?)
}

#[tauri::command]
pub async fn get_operation(
    engine: State<'_, OperationEngine>,
    id: OperationId,
) -> CommandResult<Operation> {
    engine.get(id).await?.ok_or_else(|| {
        CommandError::from(vagrant_studio_project_service::ServiceError::NotFound(
            "Operation not found.".into(),
        ))
    })
}

#[tauri::command]
pub async fn read_operation_log(
    engine: State<'_, OperationEngine>,
    id: OperationId,
) -> CommandResult<Vec<LogLine>> {
    Ok(engine.log(id, LOG_LINES).await?)
}

/// Shows the operation's log file in the system file manager. The path comes
/// from the database, never from the frontend.
#[tauri::command]
pub async fn reveal_operation_log<R: Runtime>(
    app: AppHandle<R>,
    engine: State<'_, OperationEngine>,
    id: OperationId,
) -> CommandResult<()> {
    let path = engine
        .get(id)
        .await?
        .and_then(|op| op.log_path)
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .ok_or_else(|| CommandError::unavailable("This operation has no log file."))?;
    // Revealing waits on the file manager (D-Bus on Linux), so keep it off
    // the async workers.
    tauri::async_runtime::spawn_blocking(move || app.opener().reveal_item_in_dir(path))
        .await
        .map_err(|e| CommandError::internal(e.to_string()))?
        .map_err(|e| CommandError::internal(format!("Could not show the log file: {e}")))
}

/// Opens the folder holding every operation log in the system file manager.
#[tauri::command]
pub async fn open_operation_logs_folder<R: Runtime>(
    app: AppHandle<R>,
    engine: State<'_, OperationEngine>,
) -> CommandResult<()> {
    let folder = engine.logs_folder();
    std::fs::create_dir_all(folder)
        .map_err(|e| CommandError::internal(format!("Could not create the logs folder: {e}")))?;
    app.opener()
        .open_path(folder.display().to_string(), None::<&str>)
        .map_err(|e| CommandError::internal(format!("Could not open the logs folder: {e}")))
}

#[tauri::command]
pub async fn cancel_operation(
    engine: State<'_, OperationEngine>,
    id: OperationId,
) -> CommandResult<bool> {
    Ok(engine.cancel(id).await?)
}
