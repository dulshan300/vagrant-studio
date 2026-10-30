//! Notes and tasks per project. They live in the app database only.

use serde::Deserialize;
use vagrant_studio_domain::{
    NoteId, ProjectId, ProjectNote, ProjectTask, TaskId, validate_item_title, validate_note_body,
};
use vagrant_studio_project_store::TaskUpdate;

use crate::{ProjectService, Result, ServiceError};

/// Changes to a task from the UI; absent fields stay as they are.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskChanges {
    pub title: Option<String>,
    pub done: Option<bool>,
}

impl ProjectService {
    pub async fn notes(&self, project_id: ProjectId) -> Result<Vec<ProjectNote>> {
        self.require(project_id).await?;
        Ok(self.store.list_notes(project_id).await?)
    }

    pub async fn create_note(
        &self,
        project_id: ProjectId,
        title: &str,
        body: &str,
    ) -> Result<ProjectNote> {
        let title = validate_item_title(title)?;
        let body = validate_note_body(body)?;
        self.require(project_id).await?;
        Ok(self.store.insert_note(project_id, &title, &body).await?)
    }

    pub async fn update_note(
        &self,
        project_id: ProjectId,
        id: NoteId,
        title: &str,
        body: &str,
    ) -> Result<ProjectNote> {
        let title = validate_item_title(title)?;
        let body = validate_note_body(body)?;
        self.note_of(project_id, id).await?;
        Ok(self.store.update_note(id, &title, &body).await?)
    }

    pub async fn delete_note(&self, project_id: ProjectId, id: NoteId) -> Result<()> {
        self.note_of(project_id, id).await?;
        self.store.delete_note(id).await?;
        Ok(())
    }

    pub async fn tasks(&self, project_id: ProjectId) -> Result<Vec<ProjectTask>> {
        self.require(project_id).await?;
        Ok(self.store.list_tasks(project_id).await?)
    }

    pub async fn create_task(&self, project_id: ProjectId, title: &str) -> Result<ProjectTask> {
        let title = validate_item_title(title)?;
        self.require(project_id).await?;
        Ok(self.store.insert_task(project_id, &title).await?)
    }

    pub async fn update_task(
        &self,
        project_id: ProjectId,
        id: TaskId,
        changes: TaskChanges,
    ) -> Result<ProjectTask> {
        let title = changes
            .title
            .as_deref()
            .map(validate_item_title)
            .transpose()?;
        self.task_of(project_id, id).await?;
        Ok(self
            .store
            .update_task(
                id,
                &TaskUpdate {
                    title,
                    done: changes.done,
                },
            )
            .await?)
    }

    pub async fn delete_task(&self, project_id: ProjectId, id: TaskId) -> Result<()> {
        self.task_of(project_id, id).await?;
        self.store.delete_task(id).await?;
        Ok(())
    }

    /// Puts the tasks in the given order and returns the list.
    pub async fn reorder_tasks(
        &self,
        project_id: ProjectId,
        ids: &[TaskId],
    ) -> Result<Vec<ProjectTask>> {
        self.require(project_id).await?;
        self.store.reorder_tasks(project_id, ids).await?;
        Ok(self.store.list_tasks(project_id).await?)
    }

    async fn note_of(&self, project_id: ProjectId, id: NoteId) -> Result<ProjectNote> {
        self.store
            .get_note(id)
            .await?
            .filter(|n| n.project_id == project_id)
            .ok_or_else(|| ServiceError::NotFound("Note not found.".into()))
    }

    async fn task_of(&self, project_id: ProjectId, id: TaskId) -> Result<ProjectTask> {
        self.store
            .get_task(id)
            .await?
            .filter(|t| t.project_id == project_id)
            .ok_or_else(|| ServiceError::NotFound("Task not found.".into()))
    }
}
