use std::path::{Path, PathBuf};

use tauri::State;
use vagrant_studio_domain::{EnvironmentId, ProjectId};
use vagrant_studio_platform::{DetectedEditor, spawn_editor, system_editors};
use vagrant_studio_project_service::ProjectService;

use super::error::{CommandError, CommandResult};

/// Known code editors installed on this computer, in order of preference.
#[tauri::command]
pub fn list_editors() -> Vec<DetectedEditor> {
    system_editors()
}

/// The editor from Settings, else the first detected one.
async fn chosen_editor(projects: &ProjectService) -> CommandResult<DetectedEditor> {
    if let Some(path) = projects.settings().await?.editor_path {
        let path = PathBuf::from(path);
        if !path.is_file() {
            return Err(CommandError::unavailable(format!(
                "The editor set in Settings ({}) no longer exists.",
                path.display()
            )));
        }
        let name = system_editors()
            .into_iter()
            .find(|e| e.path == path)
            .map(|e| e.name)
            .or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "the editor".to_owned());
        return Ok(DetectedEditor { name, path });
    }
    system_editors().into_iter().next().ok_or_else(|| {
        CommandError::unavailable(
            "No code editor was found. Choose one in Settings (for example VS Code's Code.exe).",
        )
    })
}

/// Opens the project's folder, or one of its environments' folders, in the
/// code editor. Paths come from the database, never from the frontend.
/// Returns the editor's name.
#[tauri::command]
pub async fn open_in_editor(
    projects: State<'_, ProjectService>,
    project_id: ProjectId,
    environment_id: Option<EnvironmentId>,
) -> CommandResult<String> {
    tracing::debug!(%project_id, ?environment_id, "open in editor requested");
    let folder = match environment_id {
        Some(id) => {
            let (project, env) = projects.environment(id).await?;
            if project.id != project_id {
                return Err(CommandError::validation(
                    "That environment belongs to another project.",
                ));
            }
            env.root_path
        }
        None => projects.details(project_id).await?.project.root_path,
    };
    if !Path::new(&folder).is_dir() {
        return Err(CommandError::unavailable(format!(
            "The folder {folder} no longer exists."
        )));
    }
    let editor = chosen_editor(&projects).await?;
    spawn_editor(&editor.path, Path::new(&folder)).map_err(|e| {
        tracing::warn!(editor = %editor.path.display(), %folder, error = %e, "editor failed to start");
        CommandError::unavailable(format!("{} couldn't be started: {e}", editor.name))
    })?;
    tracing::info!(editor = %editor.path.display(), %folder, "opened folder in editor");
    Ok(editor.name)
}
