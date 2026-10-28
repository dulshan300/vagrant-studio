use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_opener::OpenerExt;
use vagrant_studio_domain::{Environment, EnvironmentId, Project, ProjectChanges, ProjectId};
use vagrant_studio_project_service::{
    AppSettings, CreateProjectRequest, DeleteOutcome, DeleteProjectRequest, EnvironmentRequest,
    FolderInspection, ImportProjectRequest, PortablePreview, ProjectDetails, ProjectService,
    Template,
};
use vagrant_studio_project_store::ProjectFilter;

use super::error::{CommandError, CommandResult};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectListFilter {
    Active,
    Archived,
    All,
}

impl From<ProjectListFilter> for ProjectFilter {
    fn from(f: ProjectListFilter) -> Self {
        match f {
            ProjectListFilter::Active => Self::Active,
            ProjectListFilter::Archived => Self::Archived,
            ProjectListFilter::All => Self::All,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TagCount {
    name: String,
    count: i64,
}

type Service<'a> = State<'a, ProjectService>;

#[tauri::command]
pub async fn list_projects(
    service: Service<'_>,
    filter: ProjectListFilter,
) -> CommandResult<Vec<Project>> {
    Ok(service.list(filter.into()).await?)
}

#[tauri::command]
pub async fn list_project_tags(service: Service<'_>) -> CommandResult<Vec<TagCount>> {
    Ok(service
        .list_tags()
        .await?
        .into_iter()
        .map(|(name, count)| TagCount { name, count })
        .collect())
}

#[tauri::command]
pub async fn get_project(service: Service<'_>, id: ProjectId) -> CommandResult<ProjectDetails> {
    Ok(service.details(id).await?)
}

#[tauri::command]
pub async fn create_project(
    service: Service<'_>,
    request: CreateProjectRequest,
) -> CommandResult<ProjectDetails> {
    Ok(service.create(request).await?)
}

#[tauri::command]
pub async fn inspect_project_folder(
    service: Service<'_>,
    path: String,
) -> CommandResult<FolderInspection> {
    Ok(service.inspect_folder(&path).await?)
}

#[tauri::command]
pub async fn import_project(
    service: Service<'_>,
    request: ImportProjectRequest,
) -> CommandResult<ProjectDetails> {
    Ok(service.import(request).await?)
}

#[tauri::command]
pub async fn update_project(
    service: Service<'_>,
    id: ProjectId,
    changes: ProjectChanges,
) -> CommandResult<Project> {
    Ok(service.update(id, changes).await?)
}

#[tauri::command]
pub async fn set_project_favorite(
    service: Service<'_>,
    id: ProjectId,
    favorite: bool,
) -> CommandResult<Project> {
    Ok(service.set_favorite(id, favorite).await?)
}

#[tauri::command]
pub async fn set_project_archived(
    service: Service<'_>,
    id: ProjectId,
    archived: bool,
) -> CommandResult<Project> {
    Ok(service.set_archived(id, archived).await?)
}

#[tauri::command]
pub async fn mark_project_opened(service: Service<'_>, id: ProjectId) -> CommandResult<()> {
    Ok(service.mark_opened(id).await?)
}

#[tauri::command]
pub async fn delete_project(
    service: Service<'_>,
    id: ProjectId,
    request: DeleteProjectRequest,
) -> CommandResult<DeleteOutcome> {
    Ok(service.delete(id, request).await?)
}

#[tauri::command]
pub async fn add_environment(
    service: Service<'_>,
    project_id: ProjectId,
    request: EnvironmentRequest,
) -> CommandResult<Environment> {
    Ok(service.add_environment(project_id, request).await?)
}

#[tauri::command]
pub async fn set_environment_provider(
    service: Service<'_>,
    id: EnvironmentId,
    provider: Option<String>,
) -> CommandResult<Environment> {
    Ok(service.set_environment_provider(id, provider).await?)
}

#[tauri::command]
pub async fn remove_environment(service: Service<'_>, id: EnvironmentId) -> CommandResult<()> {
    Ok(service.remove_environment(id).await?)
}

#[tauri::command]
pub async fn preview_project_metadata(
    service: Service<'_>,
    id: ProjectId,
) -> CommandResult<PortablePreview> {
    Ok(service.preview_portable(id).await?)
}

#[tauri::command]
pub async fn write_project_metadata(
    service: Service<'_>,
    id: ProjectId,
    overwrite: bool,
) -> CommandResult<PortablePreview> {
    Ok(service.write_portable(id, overwrite).await?)
}

#[tauri::command]
pub fn list_templates(service: Service<'_>) -> Vec<Template> {
    service.templates()
}

#[tauri::command]
pub async fn get_settings(service: Service<'_>) -> CommandResult<AppSettings> {
    Ok(service.settings().await?)
}

#[tauri::command]
pub async fn save_settings(
    service: Service<'_>,
    settings: AppSettings,
) -> CommandResult<AppSettings> {
    Ok(service.save_settings(settings).await?)
}

/// Opens the project's registered folder in the system file manager. The path
/// comes from the database, never from the frontend.
#[tauri::command]
pub async fn open_project_folder<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    service: Service<'_>,
    id: ProjectId,
) -> CommandResult<()> {
    let details = service.details(id).await?;
    app.opener()
        .open_path(&details.project.root_path, None::<&str>)
        .map_err(|e| CommandError::internal(format!("Could not open the folder: {e}")))
}
