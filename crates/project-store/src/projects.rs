use std::collections::HashMap;

use sqlx::{Sqlite, Transaction};
use time::OffsetDateTime;
use vagrant_studio_domain::{NewProject, Project, ProjectId, ProjectStatus, slugify};

use crate::{Result, Store, StoreError, is_unique_violation, now};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectFilter {
    #[default]
    Active,
    Archived,
    All,
}

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    slug: String,
    description: String,
    root_path: String,
    repository_url: Option<String>,
    status: String,
    favorite: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    archived_at: Option<OffsetDateTime>,
    last_opened_at: Option<OffsetDateTime>,
    environment_count: i64,
}

impl ProjectRow {
    fn into_project(self, tags: Vec<String>) -> Result<Project> {
        Ok(Project {
            id: self
                .id
                .parse()
                .map_err(|_| StoreError::Corrupt(format!("invalid project id {}", self.id)))?,
            status: ProjectStatus::parse(&self.status)
                .ok_or_else(|| StoreError::Corrupt(format!("invalid status {}", self.status)))?,
            name: self.name,
            slug: self.slug,
            description: self.description,
            root_path: self.root_path,
            repository_url: self.repository_url,
            favorite: self.favorite,
            tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            archived_at: self.archived_at,
            last_opened_at: self.last_opened_at,
            environment_count: self.environment_count,
        })
    }
}

/// Builds a `&'static str` project query (SQLx only accepts static SQL).
macro_rules! select_project {
    ($where:literal) => {
        concat!(
            "SELECT p.id, p.name, p.slug, p.description, p.root_path, p.repository_url, p.status, p.favorite, p.created_at, p.updated_at, p.archived_at, p.last_opened_at, ",
            "(SELECT COUNT(*) FROM environments e WHERE e.project_id = p.id) AS environment_count FROM projects p ",
            $where
        )
    };
}

/// Validated field updates (see the service layer for validation).
#[derive(Debug, Clone, Default)]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub repository_url: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
}

impl Store {
    /// Inserts a project. Fails with `Conflict` if the folder is already registered.
    pub async fn insert_project(&self, new: &NewProject) -> Result<Project> {
        let id = ProjectId::new();
        let ts = now();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO projects (id, name, slug, description, root_path, repository_url, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(&new.name)
        .bind(slugify(&new.name))
        .bind(&new.description)
        .bind(&new.root_path)
        .bind(&new.repository_url)
        .bind(ts)
        .bind(ts)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if is_unique_violation(&e) {
                StoreError::Conflict("This folder is already registered as a project.".into())
            } else {
                e.into()
            }
        })?;
        replace_tags(&mut tx, id, &new.tags).await?;
        tx.commit().await?;
        self.get_project(id)
            .await?
            .ok_or_else(|| StoreError::Corrupt("inserted project not found".into()))
    }

    pub async fn get_project(&self, id: ProjectId) -> Result<Option<Project>> {
        let row: Option<ProjectRow> = sqlx::query_as(select_project!("WHERE p.id = ?"))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(row) => {
                let tags = self
                    .tags_for(Some(id))
                    .await?
                    .remove(&id)
                    .unwrap_or_default();
                row.into_project(tags).map(Some)
            }
            None => Ok(None),
        }
    }

    pub async fn find_project_by_root(&self, root_path: &str) -> Result<Option<ProjectId>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT id FROM projects WHERE root_path = ?")
            .bind(root_path)
            .fetch_optional(&self.pool)
            .await?;
        row.map(|(id,)| {
            id.parse()
                .map_err(|_| StoreError::Corrupt(format!("invalid project id {id}")))
        })
        .transpose()
    }

    /// Lists projects (unsorted; callers sort for display).
    pub async fn list_projects(&self, filter: ProjectFilter) -> Result<Vec<Project>> {
        let rows: Vec<ProjectRow> = match filter {
            ProjectFilter::Active => sqlx::query_as(select_project!("WHERE p.status = 'active'")),
            ProjectFilter::Archived => {
                sqlx::query_as(select_project!("WHERE p.status = 'archived'"))
            }
            ProjectFilter::All => sqlx::query_as(select_project!("")),
        }
        .fetch_all(&self.pool)
        .await?;
        let mut tags = self.tags_for(None).await?;
        rows.into_iter()
            .map(|row| {
                let id = row.id.parse().ok();
                let t = id.and_then(|id| tags.remove(&id)).unwrap_or_default();
                row.into_project(t)
            })
            .collect()
    }

    pub async fn update_project(&self, id: ProjectId, update: &ProjectUpdate) -> Result<Project> {
        let mut tx = self.pool.begin().await?;
        let ts = now();
        let result = sqlx::query(
            "UPDATE projects SET
                name = COALESCE(?, name),
                slug = COALESCE(?, slug),
                description = COALESCE(?, description),
                repository_url = CASE WHEN ? THEN ? ELSE repository_url END,
                updated_at = ?
             WHERE id = ?",
        )
        .bind(&update.name)
        .bind(update.name.as_deref().map(slugify))
        .bind(&update.description)
        .bind(update.repository_url.is_some())
        .bind(update.repository_url.clone().flatten())
        .bind(ts)
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::Conflict("Project no longer exists.".into()));
        }
        if let Some(tags) = &update.tags {
            replace_tags(&mut tx, id, tags).await?;
        }
        tx.commit().await?;
        self.require(id).await
    }

    pub async fn set_favorite(&self, id: ProjectId, favorite: bool) -> Result<Project> {
        sqlx::query("UPDATE projects SET favorite = ?, updated_at = ? WHERE id = ?")
            .bind(favorite)
            .bind(now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        self.require(id).await
    }

    pub async fn set_archived(&self, id: ProjectId, archived: bool) -> Result<Project> {
        let ts = now();
        sqlx::query("UPDATE projects SET status = ?, archived_at = ?, updated_at = ? WHERE id = ?")
            .bind(if archived { "archived" } else { "active" })
            .bind(archived.then_some(ts))
            .bind(ts)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        self.require(id).await
    }

    /// Records that the user opened the project (drives "recent projects").
    pub async fn mark_opened(&self, id: ProjectId) -> Result<()> {
        sqlx::query("UPDATE projects SET last_opened_at = ? WHERE id = ?")
            .bind(now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Deletes the project's metadata (and its environments, notes, tasks via
    /// cascade). Never touches files. Returns false if it did not exist.
    pub async fn delete_project(&self, id: ProjectId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM project_tags)")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// All tags in use with how many projects carry each.
    pub async fn list_tags(&self) -> Result<Vec<(String, i64)>> {
        Ok(sqlx::query_as(
            "SELECT t.name, COUNT(pt.project_id) FROM tags t JOIN project_tags pt ON pt.tag_id = t.id GROUP BY t.id ORDER BY t.name",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    async fn require(&self, id: ProjectId) -> Result<Project> {
        self.get_project(id)
            .await?
            .ok_or_else(|| StoreError::Conflict("Project no longer exists.".into()))
    }

    async fn tags_for(&self, id: Option<ProjectId>) -> Result<HashMap<ProjectId, Vec<String>>> {
        let rows: Vec<(String, String)> = match id {
            Some(id) => sqlx::query_as(
                "SELECT pt.project_id, t.name FROM project_tags pt JOIN tags t ON t.id = pt.tag_id WHERE pt.project_id = ? ORDER BY t.name",
            )
            .bind(id.to_string()),
            None => sqlx::query_as(
                "SELECT pt.project_id, t.name FROM project_tags pt JOIN tags t ON t.id = pt.tag_id ORDER BY t.name",
            ),
        }
        .fetch_all(&self.pool)
        .await?;
        let mut map: HashMap<ProjectId, Vec<String>> = HashMap::new();
        for (project_id, tag) in rows {
            if let Ok(pid) = project_id.parse() {
                map.entry(pid).or_default().push(tag);
            }
        }
        Ok(map)
    }
}

async fn replace_tags(
    tx: &mut Transaction<'_, Sqlite>,
    id: ProjectId,
    tags: &[String],
) -> Result<()> {
    sqlx::query("DELETE FROM project_tags WHERE project_id = ?")
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    for tag in tags {
        sqlx::query("INSERT INTO tags (name) VALUES (?) ON CONFLICT (name) DO NOTHING")
            .bind(tag)
            .execute(&mut **tx)
            .await?;
        sqlx::query(
            "INSERT INTO project_tags (project_id, tag_id) SELECT ?, id FROM tags WHERE name = ?",
        )
        .bind(id.to_string())
        .bind(tag)
        .execute(&mut **tx)
        .await?;
    }
    sqlx::query("DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM project_tags)")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::store;

    fn new_project(name: &str, root: &str) -> NewProject {
        NewProject {
            name: name.into(),
            description: "desc".into(),
            root_path: root.into(),
            repository_url: None,
            tags: vec!["db".into(), "web".into()],
        }
    }

    #[tokio::test]
    async fn insert_get_and_list() {
        let s = store().await;
        let p = s
            .insert_project(&new_project("Web App", "/p/web"))
            .await
            .unwrap();
        assert_eq!(p.slug, "web-app");
        assert_eq!(p.status, ProjectStatus::Active);
        assert_eq!(p.tags, vec!["db", "web"]);
        assert_eq!(p.environment_count, 0);
        assert_eq!(s.get_project(p.id).await.unwrap().unwrap(), p);
        assert_eq!(
            s.list_projects(ProjectFilter::Active).await.unwrap(),
            vec![p.clone()]
        );
        assert_eq!(s.find_project_by_root("/p/web").await.unwrap(), Some(p.id));
    }

    #[tokio::test]
    async fn same_folder_cannot_be_registered_twice() {
        let s = store().await;
        s.insert_project(&new_project("A", "/p/a")).await.unwrap();
        let err = s
            .insert_project(&new_project("B", "/p/a"))
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
    }

    #[tokio::test]
    async fn update_fields_and_tags() {
        let s = store().await;
        let p = s.insert_project(&new_project("A", "/p/a")).await.unwrap();
        let updated = s
            .update_project(
                p.id,
                &ProjectUpdate {
                    name: Some("Renamed Project".into()),
                    repository_url: Some(Some("https://h/o/r.git".into())),
                    tags: Some(vec!["api".into()]),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "Renamed Project");
        assert_eq!(updated.slug, "renamed-project");
        assert_eq!(updated.description, "desc");
        assert_eq!(updated.repository_url.as_deref(), Some("https://h/o/r.git"));
        assert_eq!(updated.tags, vec!["api"]);
        assert_eq!(s.list_tags().await.unwrap(), vec![("api".into(), 1)]);

        let cleared = s
            .update_project(
                p.id,
                &ProjectUpdate {
                    repository_url: Some(None),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(cleared.repository_url, None);
        assert_eq!(cleared.tags, vec!["api"]);
    }

    #[tokio::test]
    async fn archive_restore_and_filters() {
        let s = store().await;
        let a = s.insert_project(&new_project("A", "/p/a")).await.unwrap();
        let b = s.insert_project(&new_project("B", "/p/b")).await.unwrap();
        let archived = s.set_archived(a.id, true).await.unwrap();
        assert_eq!(archived.status, ProjectStatus::Archived);
        assert!(archived.archived_at.is_some());

        let ids = |v: Vec<Project>| v.into_iter().map(|p| p.id).collect::<Vec<_>>();
        assert_eq!(
            ids(s.list_projects(ProjectFilter::Active).await.unwrap()),
            vec![b.id]
        );
        assert_eq!(
            ids(s.list_projects(ProjectFilter::Archived).await.unwrap()),
            vec![a.id]
        );
        assert_eq!(s.list_projects(ProjectFilter::All).await.unwrap().len(), 2);

        let restored = s.set_archived(a.id, false).await.unwrap();
        assert_eq!(restored.status, ProjectStatus::Active);
        assert_eq!(restored.archived_at, None);
    }

    #[tokio::test]
    async fn favorite_and_opened() {
        let s = store().await;
        let p = s.insert_project(&new_project("A", "/p/a")).await.unwrap();
        assert!(s.set_favorite(p.id, true).await.unwrap().favorite);
        assert_eq!(p.last_opened_at, None);
        s.mark_opened(p.id).await.unwrap();
        assert!(
            s.get_project(p.id)
                .await
                .unwrap()
                .unwrap()
                .last_opened_at
                .is_some()
        );
    }

    #[tokio::test]
    async fn delete_removes_metadata_and_orphan_tags() {
        let s = store().await;
        let p = s.insert_project(&new_project("A", "/p/a")).await.unwrap();
        assert!(s.delete_project(p.id).await.unwrap());
        assert!(!s.delete_project(p.id).await.unwrap());
        assert_eq!(s.get_project(p.id).await.unwrap(), None);
        assert!(s.list_tags().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn timestamps_round_trip_with_millisecond_precision() {
        let s = store().await;
        let p = s.insert_project(&new_project("A", "/p/a")).await.unwrap();
        assert_eq!(p.created_at.nanosecond() % 1_000_000, 0);
        assert_eq!(p.created_at.offset(), time::UtcOffset::UTC);
    }
}
