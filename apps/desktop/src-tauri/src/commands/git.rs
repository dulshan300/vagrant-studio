use std::path::Path;

use tauri::State;
use vagrant_studio_domain::ProjectId;
use vagrant_studio_git::{GitStatus, locate, status};
use vagrant_studio_platform::SystemPlatform;
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_vagrant_adapter::ProcessRunner;

use super::error::{CommandError, CommandResult};

/// Read-only `git status` of the project folder (path from the database).
#[tauri::command]
pub async fn get_git_status(
    projects: State<'_, ProjectService>,
    project_id: ProjectId,
) -> CommandResult<GitStatus> {
    let root = projects.details(project_id).await?.project.root_path;
    if !Path::new(&root).is_dir() {
        return Err(CommandError::unavailable(format!(
            "The folder {root} no longer exists."
        )));
    }
    let git = locate(&SystemPlatform);
    Ok(status(&ProcessRunner, git.as_deref(), Path::new(&root)).await)
}
