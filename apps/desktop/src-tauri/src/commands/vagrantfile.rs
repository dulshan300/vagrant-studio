use serde::Serialize;
use tauri::State;
use vagrant_studio_config_model::{
    Analysis, FieldError, LineEnding, VagrantConfig, analyze, generate,
};
use vagrant_studio_domain::EnvironmentId;
use vagrant_studio_operation_engine::OperationEngine;
use vagrant_studio_project_service::{
    ProjectService, SaveVagrantfileOutcome, SaveVagrantfileRequest, VagrantfileBackup,
    VagrantfileDocument,
};

use super::error::{CommandError, CommandResult};

type Service<'a> = State<'a, ProjectService>;

/// Generated text, or the problems that prevent generating it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedVagrantfile {
    content: Option<String>,
    errors: Vec<FieldError>,
}

#[tauri::command]
pub async fn get_vagrantfile(
    service: Service<'_>,
    environment_id: EnvironmentId,
) -> CommandResult<VagrantfileDocument> {
    Ok(service.vagrantfile(environment_id).await?)
}

/// Pure: what the visual editor makes of unsaved text.
#[tauri::command]
pub fn analyze_vagrantfile(content: String) -> Analysis {
    analyze(&content)
}

/// Pure: the Vagrantfile for a model edited in the forms.
#[tauri::command]
pub fn generate_vagrantfile(model: VagrantConfig, line_ending: LineEnding) -> GeneratedVagrantfile {
    match generate(&model, line_ending) {
        Ok(content) => GeneratedVagrantfile {
            content: Some(content),
            errors: Vec::new(),
        },
        Err(errors) => GeneratedVagrantfile {
            content: None,
            errors,
        },
    }
}

/// Saves while holding the environment's lock, so no Vagrant command reads
/// the file half-way through and none is running against the old version.
#[tauri::command]
pub async fn save_vagrantfile(
    service: Service<'_>,
    engine: State<'_, OperationEngine>,
    environment_id: EnvironmentId,
    request: SaveVagrantfileRequest,
) -> CommandResult<SaveVagrantfileOutcome> {
    let _lock = engine.try_lock_environment(environment_id).ok_or_else(|| {
        CommandError::conflict(
            "A Vagrant operation is queued or running for this environment. Save when it has finished.",
        )
    })?;
    Ok(service.save_vagrantfile(environment_id, request).await?)
}

#[tauri::command]
pub async fn list_vagrantfile_backups(
    service: Service<'_>,
    environment_id: EnvironmentId,
) -> CommandResult<Vec<VagrantfileBackup>> {
    Ok(service.vagrantfile_backups(environment_id).await?)
}

#[tauri::command]
pub async fn read_vagrantfile_backup(
    service: Service<'_>,
    environment_id: EnvironmentId,
    name: String,
) -> CommandResult<String> {
    Ok(service.vagrantfile_backup(environment_id, &name).await?)
}
