use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;
use vagrant_studio_domain::ProjectId;
use vagrant_studio_git::{
    CommitSummary, FileChange, GitStatus, RepoStatus, commit, init, locate, log, stage, status,
    unstage,
};
use vagrant_studio_platform::SystemPlatform;
use vagrant_studio_project_service::ProjectService;
use vagrant_studio_vagrant_adapter::ProcessRunner;

use super::error::{CommandError, CommandResult};

async fn project_root(projects: &ProjectService, project_id: ProjectId) -> CommandResult<String> {
    let root = projects.details(project_id).await?.project.root_path;
    if !Path::new(&root).is_dir() {
        return Err(CommandError::unavailable(format!(
            "The folder {root} no longer exists."
        )));
    }
    Ok(root)
}

/// Whether the host has Git, for defaults such as "initialize a repository".
#[tauri::command]
pub fn is_git_installed() -> bool {
    locate(&SystemPlatform).is_some()
}

/// Read-only `git status` of the project folder (path from the database).
#[tauri::command]
pub async fn get_git_status(
    projects: State<'_, ProjectService>,
    project_id: ProjectId,
) -> CommandResult<GitStatus> {
    let root = project_root(&projects, project_id).await?;
    let git = locate(&SystemPlatform);
    Ok(status(&ProcessRunner, git.as_deref(), Path::new(&root)).await)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitInit {
    pub status: GitStatus,
    /// False when the folder already had a `.gitignore`, which is kept.
    pub gitignore_created: bool,
}

/// Creates a Git repository in the project folder and adds Vagrant Studio's
/// `.gitignore` unless one exists. Refused when the folder is already inside
/// a repository, including one in a parent folder. Nothing is committed.
#[tauri::command]
pub async fn init_git_repository(
    projects: State<'_, ProjectService>,
    project_id: ProjectId,
) -> CommandResult<GitInit> {
    let root = project_root(&projects, project_id).await?;
    let folder = Path::new(&root);
    let git = locate(&SystemPlatform).ok_or_else(|| {
        CommandError::unavailable("Git isn't installed, or wasn't found on PATH.")
    })?;
    match status(&ProcessRunner, Some(&git), folder).await {
        GitStatus::NotRepository => {}
        GitStatus::Repository(_) => {
            return Err(CommandError::conflict(
                "This folder is already inside a Git repository.",
            ));
        }
        GitStatus::Error { message } => return Err(CommandError::unavailable(message)),
        GitStatus::Unavailable => {
            return Err(CommandError::unavailable("Git isn't installed."));
        }
    }
    init(&ProcessRunner, &git, folder)
        .await
        .map_err(CommandError::unavailable)?;
    let gitignore_created = projects.add_gitignore(project_id).await?;
    tracing::info!(folder = %root, gitignore_created, "initialized git repository");
    Ok(GitInit {
        status: status(&ProcessRunner, Some(&git), folder).await,
        gitignore_created,
    })
}

const MAX_COMMIT_MESSAGE: usize = 10_000;

/// Git, the project folder and its current status; fails unless the folder is
/// inside a repository.
async fn repository(
    projects: &ProjectService,
    project_id: ProjectId,
) -> CommandResult<(PathBuf, PathBuf, RepoStatus)> {
    let root = PathBuf::from(project_root(projects, project_id).await?);
    let git = locate(&SystemPlatform).ok_or_else(|| {
        CommandError::unavailable("Git isn't installed, or wasn't found on PATH.")
    })?;
    match status(&ProcessRunner, Some(&git), &root).await {
        GitStatus::Repository(s) => Ok((git, root, s)),
        GitStatus::NotRepository => Err(CommandError::conflict(
            "This folder isn't a Git repository.",
        )),
        GitStatus::Error { message } => Err(CommandError::unavailable(message)),
        GitStatus::Unavailable => Err(CommandError::unavailable("Git isn't installed.")),
    }
}

/// The changes named by `paths`, which must all be in Git's current status
/// for the folder; the frontend can't pass other paths to Git.
fn reported<'a>(s: &'a RepoStatus, paths: &[String]) -> CommandResult<Vec<&'a FileChange>> {
    if paths.is_empty() {
        return Err(CommandError::validation("Choose at least one file."));
    }
    paths
        .iter()
        .map(|p| {
            s.files.iter().find(|f| &f.path == p).ok_or_else(|| {
                CommandError::conflict(format!(
                    "{p} has no changes any more. Refresh and try again."
                ))
            })
        })
        .collect()
}

async fn status_after(git: &Path, root: &Path) -> GitStatus {
    status(&ProcessRunner, Some(git), root).await
}

/// The latest commits that touched the project folder.
#[tauri::command]
pub async fn get_git_log(
    projects: State<'_, ProjectService>,
    project_id: ProjectId,
) -> CommandResult<Vec<CommitSummary>> {
    let (git, root, _) = repository(&projects, project_id).await?;
    log(&ProcessRunner, &git, &root)
        .await
        .map_err(CommandError::unavailable)
}

/// Stages the changes to `paths` (as listed by the status) and returns the new status.
#[tauri::command]
pub async fn git_stage(
    projects: State<'_, ProjectService>,
    project_id: ProjectId,
    paths: Vec<String>,
) -> CommandResult<GitStatus> {
    let (git, root, s) = repository(&projects, project_id).await?;
    stage(&ProcessRunner, &git, &root, &reported(&s, &paths)?)
        .await
        .map_err(CommandError::unavailable)?;
    Ok(status_after(&git, &root).await)
}

/// Unstages `paths`, keeping the files as they are, and returns the new status.
#[tauri::command]
pub async fn git_unstage(
    projects: State<'_, ProjectService>,
    project_id: ProjectId,
    paths: Vec<String>,
) -> CommandResult<GitStatus> {
    let (git, root, s) = repository(&projects, project_id).await?;
    let files = reported(&s, &paths)?;
    unstage(&ProcessRunner, &git, &root, &files, s.commit.is_some())
        .await
        .map_err(CommandError::unavailable)?;
    Ok(status_after(&git, &root).await)
}

/// Commits what is staged with the user's Git identity and returns the new status.
#[tauri::command]
pub async fn git_commit(
    projects: State<'_, ProjectService>,
    project_id: ProjectId,
    message: String,
) -> CommandResult<GitStatus> {
    let message = message.trim();
    if message.is_empty() {
        return Err(CommandError::validation("Write a commit message."));
    }
    if message.chars().count() > MAX_COMMIT_MESSAGE {
        return Err(CommandError::validation(format!(
            "Use at most {MAX_COMMIT_MESSAGE} characters."
        )));
    }
    let (git, root, s) = repository(&projects, project_id).await?;
    if s.staged == 0 {
        return Err(CommandError::validation(
            "Nothing is staged in this folder. Tick the files to commit first.",
        ));
    }
    commit(&ProcessRunner, &git, &root, message)
        .await
        .map_err(CommandError::unavailable)?;
    tracing::info!(folder = %root.display(), "committed");
    Ok(status_after(&git, &root).await)
}
