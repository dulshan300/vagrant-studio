//! Project notes and tasks.

use std::collections::HashSet;

use time::OffsetDateTime;
use vagrant_studio_domain::{NoteId, ProjectId, ProjectNote, ProjectTask, TaskId};

use crate::{Result, Store, StoreError, now};

#[derive(sqlx::FromRow)]
struct NoteRow {
    id: String,
    project_id: String,
    title: String,
    body: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl NoteRow {
    fn into_note(self) -> Result<ProjectNote> {
        Ok(ProjectNote {
            id: parse_id(&self.id, "note")?,
            project_id: parse_id(&self.project_id, "project")?,
            title: self.title,
            body: self.body,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: String,
    project_id: String,
    title: String,
    done: bool,
    sort_order: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    completed_at: Option<OffsetDateTime>,
}

impl TaskRow {
    fn into_task(self) -> Result<ProjectTask> {
        Ok(ProjectTask {
            id: parse_id(&self.id, "task")?,
            project_id: parse_id(&self.project_id, "project")?,
            title: self.title,
            done: self.done,
            sort_order: self.sort_order,
            created_at: self.created_at,
            updated_at: self.updated_at,
            completed_at: self.completed_at,
        })
    }
}

fn parse_id<T: std::str::FromStr>(id: &str, what: &str) -> Result<T> {
    id.parse()
        .map_err(|_| StoreError::Corrupt(format!("invalid {what} id {id}")))
}

macro_rules! select_task {
    ($where:literal) => {
        concat!(
            "SELECT id, project_id, title, done, sort_order, created_at, updated_at, completed_at FROM project_tasks ",
            $where
        )
    };
}

/// Validated changes to a task (see the service layer for validation).
#[derive(Debug, Clone, Default)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub done: Option<bool>,
}

impl Store {
    /// The project's notes, most recently changed first.
    pub async fn list_notes(&self, project_id: ProjectId) -> Result<Vec<ProjectNote>> {
        let rows: Vec<NoteRow> = sqlx::query_as(
            "SELECT id, project_id, title, body, created_at, updated_at FROM project_notes WHERE project_id = ?",
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        let mut notes = rows
            .into_iter()
            .map(NoteRow::into_note)
            .collect::<Result<Vec<_>>>()?;
        // Timestamps are compared parsed, never as strings.
        notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(b.id.cmp(&a.id)));
        Ok(notes)
    }

    pub async fn insert_note(
        &self,
        project_id: ProjectId,
        title: &str,
        body: &str,
    ) -> Result<ProjectNote> {
        let id = NoteId::new();
        let ts = now();
        sqlx::query(
            "INSERT INTO project_notes (id, project_id, title, body, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(project_id.to_string())
        .bind(title)
        .bind(body)
        .bind(ts)
        .bind(ts)
        .execute(&self.pool)
        .await?;
        self.require_note(id).await
    }

    pub async fn update_note(&self, id: NoteId, title: &str, body: &str) -> Result<ProjectNote> {
        let result = sqlx::query(
            "UPDATE project_notes SET title = ?, body = ?, updated_at = ? WHERE id = ?",
        )
        .bind(title)
        .bind(body)
        .bind(now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::Conflict("The note no longer exists.".into()));
        }
        self.require_note(id).await
    }

    /// Returns false if the note did not exist.
    pub async fn delete_note(&self, id: NoteId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM project_notes WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_note(&self, id: NoteId) -> Result<Option<ProjectNote>> {
        let row: Option<NoteRow> = sqlx::query_as(
            "SELECT id, project_id, title, body, created_at, updated_at FROM project_notes WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(NoteRow::into_note).transpose()
    }

    async fn require_note(&self, id: NoteId) -> Result<ProjectNote> {
        self.get_note(id)
            .await?
            .ok_or_else(|| StoreError::Conflict("The note no longer exists.".into()))
    }

    /// The project's tasks in list order.
    pub async fn list_tasks(&self, project_id: ProjectId) -> Result<Vec<ProjectTask>> {
        let rows: Vec<TaskRow> =
            sqlx::query_as(select_task!("WHERE project_id = ? ORDER BY sort_order, id"))
                .bind(project_id.to_string())
                .fetch_all(&self.pool)
                .await?;
        rows.into_iter().map(TaskRow::into_task).collect()
    }

    /// Adds a task at the end of the project's list.
    pub async fn insert_task(&self, project_id: ProjectId, title: &str) -> Result<ProjectTask> {
        let id = TaskId::new();
        let ts = now();
        sqlx::query(
            "INSERT INTO project_tasks (id, project_id, title, done, sort_order, created_at, updated_at)
             VALUES (?, ?, ?, 0, (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM project_tasks WHERE project_id = ?), ?, ?)",
        )
        .bind(id.to_string())
        .bind(project_id.to_string())
        .bind(title)
        .bind(project_id.to_string())
        .bind(ts)
        .bind(ts)
        .execute(&self.pool)
        .await?;
        self.require_task(id).await
    }

    /// Changes the title and/or done state. Marking a task done records when;
    /// reopening it clears that.
    pub async fn update_task(&self, id: TaskId, update: &TaskUpdate) -> Result<ProjectTask> {
        let ts = now();
        let result = sqlx::query(
            "UPDATE project_tasks SET
                title = COALESCE(?, title),
                completed_at = CASE
                    WHEN ? IS NULL THEN completed_at
                    WHEN ? THEN CASE WHEN done THEN completed_at ELSE ? END
                    ELSE NULL END,
                done = COALESCE(?, done),
                updated_at = ?
             WHERE id = ?",
        )
        .bind(&update.title)
        .bind(update.done)
        .bind(update.done)
        .bind(ts)
        .bind(update.done)
        .bind(ts)
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::Conflict("The task no longer exists.".into()));
        }
        self.require_task(id).await
    }

    /// Returns false if the task did not exist.
    pub async fn delete_task(&self, id: TaskId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM project_tasks WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Puts the project's tasks in the given order. `ids` must name every task
    /// of the project exactly once, so a stale list is refused, not merged.
    pub async fn reorder_tasks(&self, project_id: ProjectId, ids: &[TaskId]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let current: Vec<(String,)> =
            sqlx::query_as("SELECT id FROM project_tasks WHERE project_id = ?")
                .bind(project_id.to_string())
                .fetch_all(&mut *tx)
                .await?;
        let current: HashSet<String> = current.into_iter().map(|(id,)| id).collect();
        let wanted: HashSet<String> = ids.iter().map(TaskId::to_string).collect();
        if wanted.len() != ids.len() || wanted != current {
            return Err(StoreError::Conflict(
                "The task list changed; reload and try again.".into(),
            ));
        }
        for (i, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE project_tasks SET sort_order = ? WHERE id = ?")
                .bind(i64::try_from(i).unwrap_or(i64::MAX))
                .bind(id.to_string())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_task(&self, id: TaskId) -> Result<Option<ProjectTask>> {
        let row: Option<TaskRow> = sqlx::query_as(select_task!("WHERE id = ?"))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(TaskRow::into_task).transpose()
    }

    async fn require_task(&self, id: TaskId) -> Result<ProjectTask> {
        self.get_task(id)
            .await?
            .ok_or_else(|| StoreError::Conflict("The task no longer exists.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::store;
    use vagrant_studio_domain::NewProject;

    async fn project(s: &Store, root: &str) -> ProjectId {
        s.insert_project(&NewProject {
            name: "A".into(),
            description: String::new(),
            root_path: root.into(),
            repository_url: None,
            tags: vec![],
        })
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn notes_are_listed_most_recently_changed_first() {
        let s = store().await;
        let p = project(&s, "/a").await;
        let other = project(&s, "/b").await;
        let first = s.insert_note(p, "Ports", "web: 8080").await.unwrap();
        let second = s.insert_note(p, "Logins", "").await.unwrap();
        s.insert_note(other, "Elsewhere", "").await.unwrap();
        let titles =
            |notes: Vec<ProjectNote>| notes.into_iter().map(|n| n.title).collect::<Vec<_>>();
        assert_eq!(titles(s.list_notes(p).await.unwrap()), ["Logins", "Ports"]);

        let edited = s.update_note(first.id, "Ports", "web: 8081").await.unwrap();
        assert_eq!(edited.body, "web: 8081");
        assert!(edited.updated_at >= edited.created_at);
        assert_eq!(titles(s.list_notes(p).await.unwrap()), ["Ports", "Logins"]);

        assert!(s.delete_note(second.id).await.unwrap());
        assert!(!s.delete_note(second.id).await.unwrap());
        assert!(matches!(
            s.update_note(second.id, "x", "").await,
            Err(StoreError::Conflict(_))
        ));
    }

    #[tokio::test]
    async fn tasks_keep_their_order_and_completion_time() {
        let s = store().await;
        let p = project(&s, "/a").await;
        let a = s.insert_task(p, "Install box").await.unwrap();
        let b = s.insert_task(p, "Open port").await.unwrap();
        let c = s.insert_task(p, "Seed data").await.unwrap();
        assert_eq!((a.sort_order, b.sort_order, c.sort_order), (0, 1, 2));
        assert!(!a.done && a.completed_at.is_none());

        let done = s
            .update_task(
                a.id,
                &TaskUpdate {
                    done: Some(true),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(done.done);
        let completed = done.completed_at.unwrap();
        // Renaming a done task keeps when it was completed.
        let renamed = s
            .update_task(
                a.id,
                &TaskUpdate {
                    title: Some("Install the box".into()),
                    done: Some(true),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            (renamed.title.as_str(), renamed.completed_at),
            ("Install the box", Some(completed))
        );
        let reopened = s
            .update_task(
                a.id,
                &TaskUpdate {
                    done: Some(false),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(!reopened.done && reopened.completed_at.is_none());

        s.reorder_tasks(p, &[c.id, a.id, b.id]).await.unwrap();
        let order: Vec<_> = s
            .list_tasks(p)
            .await
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(order, [c.id, a.id, b.id]);

        // A stale or partial list is refused.
        assert!(s.reorder_tasks(p, &[c.id, a.id]).await.is_err());
        assert!(s.reorder_tasks(p, &[c.id, a.id, a.id]).await.is_err());

        assert!(s.delete_task(b.id).await.unwrap());
        assert_eq!(s.insert_task(p, "Last").await.unwrap().sort_order, 2);
    }

    #[tokio::test]
    async fn removing_a_project_removes_its_notes_and_tasks() {
        let s = store().await;
        let p = project(&s, "/a").await;
        let note = s.insert_note(p, "n", "").await.unwrap();
        let task = s.insert_task(p, "t").await.unwrap();
        s.delete_project(p).await.unwrap();
        assert!(s.get_note(note.id).await.unwrap().is_none());
        assert!(s.get_task(task.id).await.unwrap().is_none());
    }
}
