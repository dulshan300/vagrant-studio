use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Runtime, State};
use vagrant_studio_domain::{EnvironmentId, Operation, OperationId, OperationType};
use vagrant_studio_operation_engine::{
    EnvironmentStatus, EventSink, LogLine, OperationEngine, OperationEvent, OperationRequest,
};
use vagrant_studio_platform::SystemPlatform;
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_vagrant_adapter::readiness::{Checker, HostReadiness, TcpProbe};
use vagrant_studio_vagrant_adapter::{ProcessRunner, VagrantCommand, locate};

use super::error::{CommandError, CommandResult};

/// Tauri event carrying every [`OperationEvent`].
pub const OPERATION_EVENT: &str = "vagrant-studio://operation";
const LOG_LINES: usize = 5_000;

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

async fn vagrant_program(projects: &ProjectService) -> CommandResult<PathBuf> {
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
    Ok(Checker {
        runner: &ProcessRunner,
        platform: &SystemPlatform,
        probe: &TcpProbe,
        vagrant_override: override_path.as_deref(),
    }
    .check()
    .await)
}

async fn start_environment_operation(
    projects: &ProjectService,
    engine: &OperationEngine,
    environment_id: EnvironmentId,
    operation_type: OperationType,
    command: VagrantCommand,
) -> CommandResult<Operation> {
    let (project, env) = projects.environment(environment_id).await?;
    if !Path::new(&env.root_path).is_dir() {
        return Err(CommandError::unavailable(format!(
            "The environment folder {} no longer exists.",
            env.root_path
        )));
    }
    let program = vagrant_program(projects).await?;
    Ok(engine
        .start(OperationRequest {
            operation_type,
            command,
            program,
            working_dir: PathBuf::from(&env.root_path),
            project_id: Some(project.id),
            environment_id: Some(env.id),
            machine_name: None,
            target_label: format!("{} · {}", project.name, env.name),
            // Machines that don't exist yet are reported for the default provider.
            env: env
                .default_provider
                .iter()
                .map(|p| ("VAGRANT_DEFAULT_PROVIDER".to_owned(), p.clone()))
                .collect(),
            timeout: Some(Duration::from_secs(180)),
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
        OperationType::Status,
        VagrantCommand::Status { machine: None },
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
        OperationType::Validate,
        VagrantCommand::Validate,
    )
    .await
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

#[tauri::command]
pub async fn list_operations(
    engine: State<'_, OperationEngine>,
    environment_id: Option<EnvironmentId>,
    limit: Option<i64>,
) -> CommandResult<Vec<Operation>> {
    Ok(engine.list(environment_id, limit.unwrap_or(100)).await?)
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

#[tauri::command]
pub async fn cancel_operation(
    engine: State<'_, OperationEngine>,
    id: OperationId,
) -> CommandResult<bool> {
    Ok(engine.cancel(id).await?)
}
