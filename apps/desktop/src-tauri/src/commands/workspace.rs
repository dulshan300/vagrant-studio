use tauri::State;
use vagrant_studio_domain::{NoteId, ProjectId, ProjectNote, ProjectTask, TaskId};
use vagrant_studio_project_service::{ProjectService, TaskChanges};

use super::error::CommandResult;

type Service<'a> = State<'a, ProjectService>;

#[tauri::command]
pub async fn list_project_notes(
    service: Service<'_>,
    project_id: ProjectId,
) -> CommandResult<Vec<ProjectNote>> {
    Ok(service.notes(project_id).await?)
}

#[tauri::command]
pub async fn create_project_note(
    service: Service<'_>,
    project_id: ProjectId,
    title: String,
    body: String,
) -> CommandResult<ProjectNote> {
    Ok(service.create_note(project_id, &title, &body).await?)
}

#[tauri::command]
pub async fn update_project_note(
    service: Service<'_>,
    project_id: ProjectId,
    id: NoteId,
    title: String,
    body: String,
) -> CommandResult<ProjectNote> {
    Ok(service.update_note(project_id, id, &title, &body).await?)
}

#[tauri::command]
pub async fn delete_project_note(
    service: Service<'_>,
    project_id: ProjectId,
    id: NoteId,
) -> CommandResult<()> {
    Ok(service.delete_note(project_id, id).await?)
}

#[tauri::command]
pub async fn list_project_tasks(
    service: Service<'_>,
    project_id: ProjectId,
) -> CommandResult<Vec<ProjectTask>> {
    Ok(service.tasks(project_id).await?)
}

#[tauri::command]
pub async fn create_project_task(
    service: Service<'_>,
    project_id: ProjectId,
    title: String,
) -> CommandResult<ProjectTask> {
    Ok(service.create_task(project_id, &title).await?)
}

#[tauri::command]
pub async fn update_project_task(
    service: Service<'_>,
    project_id: ProjectId,
    id: TaskId,
    changes: TaskChanges,
) -> CommandResult<ProjectTask> {
    Ok(service.update_task(project_id, id, changes).await?)
}

#[tauri::command]
pub async fn delete_project_task(
    service: Service<'_>,
    project_id: ProjectId,
    id: TaskId,
) -> CommandResult<()> {
    Ok(service.delete_task(project_id, id).await?)
}

/// Puts the tasks in the given order; `ids` must list every task once.
#[tauri::command]
pub async fn reorder_project_tasks(
    service: Service<'_>,
    project_id: ProjectId,
    ids: Vec<TaskId>,
) -> CommandResult<Vec<ProjectTask>> {
    Ok(service.reorder_tasks(project_id, &ids).await?)
}
