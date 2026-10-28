use time::OffsetDateTime;
use vagrant_studio_domain::{
    Environment, EnvironmentId, EnvironmentType, NewEnvironment, ProjectId,
};

use crate::{Result, Store, StoreError, is_unique_violation, now};

#[derive(sqlx::FromRow)]
struct EnvironmentRow {
    id: String,
    project_id: String,
    name: String,
    root_path: String,
    vagrantfile_path: Option<String>,
    default_provider: Option<String>,
    environment_type: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<EnvironmentRow> for Environment {
    type Error = StoreError;

    fn try_from(row: EnvironmentRow) -> Result<Self> {
        let corrupt = |what: &str, v: &str| StoreError::Corrupt(format!("invalid {what} {v}"));
        Ok(Environment {
            id: row
                .id
                .parse()
                .map_err(|_| corrupt("environment id", &row.id))?,
            project_id: row
                .project_id
                .parse()
                .map_err(|_| corrupt("project id", &row.project_id))?,
            environment_type: EnvironmentType::parse(&row.environment_type)
                .ok_or_else(|| corrupt("environment type", &row.environment_type))?,
            name: row.name,
            root_path: row.root_path,
            vagrantfile_path: row.vagrantfile_path,
            default_provider: row.default_provider,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl Store {
    pub async fn insert_environment(
        &self,
        project_id: ProjectId,
        new: &NewEnvironment,
    ) -> Result<Environment> {
        let id = EnvironmentId::new();
        let ts = now();
        sqlx::query(
            "INSERT INTO environments (id, project_id, name, root_path, vagrantfile_path, default_provider, environment_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(project_id.to_string())
        .bind(&new.name)
        .bind(&new.root_path)
        .bind(&new.vagrantfile_path)
        .bind(&new.default_provider)
        .bind(new.environment_type.as_str())
        .bind(ts)
        .bind(ts)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if is_unique_violation(&e) {
                StoreError::Conflict(format!(
                    "The project already has an environment named \"{}\".",
                    new.name
                ))
            } else {
                e.into()
            }
        })?;
        self.get_environment(id)
            .await?
            .ok_or_else(|| StoreError::Corrupt("inserted environment not found".into()))
    }

    pub async fn get_environment(&self, id: EnvironmentId) -> Result<Option<Environment>> {
        let row: Option<EnvironmentRow> = sqlx::query_as("SELECT * FROM environments WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(Environment::try_from).transpose()
    }

    /// Environments of a project, ordered by name.
    pub async fn list_environments(&self, project_id: ProjectId) -> Result<Vec<Environment>> {
        let rows: Vec<EnvironmentRow> = sqlx::query_as(
            "SELECT * FROM environments WHERE project_id = ? ORDER BY name COLLATE NOCASE",
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Environment::try_from).collect()
    }

    /// Changes the provider Vagrant should use for this environment's machines.
    pub async fn set_environment_provider(
        &self,
        id: EnvironmentId,
        provider: Option<&str>,
    ) -> Result<Environment> {
        sqlx::query("UPDATE environments SET default_provider = ?, updated_at = ? WHERE id = ?")
            .bind(provider)
            .bind(now())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        self.get_environment(id)
            .await?
            .ok_or_else(|| StoreError::Conflict("Environment no longer exists.".into()))
    }

    /// Removes the environment's metadata only. Returns false if it did not exist.
    pub async fn delete_environment(&self, id: EnvironmentId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM environments WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::store;
    use vagrant_studio_domain::NewProject;

    fn env(name: &str) -> NewEnvironment {
        NewEnvironment {
            name: name.into(),
            root_path: format!("/p/a/{name}"),
            vagrantfile_path: Some("Vagrantfile".into()),
            default_provider: Some("virtualbox".into()),
            environment_type: EnvironmentType::Testing,
        }
    }

    async fn project(s: &Store) -> ProjectId {
        s.insert_project(&NewProject {
            name: "A".into(),
            description: String::new(),
            root_path: "/p/a".into(),
            repository_url: None,
            tags: vec![],
        })
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn environments_belong_to_projects() {
        let s = store().await;
        let pid = project(&s).await;
        let test = s.insert_environment(pid, &env("testing")).await.unwrap();
        let dev = s
            .insert_environment(pid, &env("Development"))
            .await
            .unwrap();
        assert_eq!(test.environment_type, EnvironmentType::Testing);
        assert_eq!(
            s.list_environments(pid).await.unwrap(),
            vec![dev.clone(), test.clone()]
        );
        assert_eq!(
            s.get_project(pid).await.unwrap().unwrap().environment_count,
            2
        );

        let dup = s
            .insert_environment(pid, &env("TESTING"))
            .await
            .unwrap_err();
        assert!(matches!(dup, StoreError::Conflict(_)));

        let changed = s
            .set_environment_provider(test.id, Some("vmware_desktop"))
            .await
            .unwrap();
        assert_eq!(changed.default_provider.as_deref(), Some("vmware_desktop"));
        let cleared = s.set_environment_provider(test.id, None).await.unwrap();
        assert_eq!(cleared.default_provider, None);
        let test = cleared;

        assert!(s.delete_environment(dev.id).await.unwrap());
        assert_eq!(s.list_environments(pid).await.unwrap(), vec![test]);
    }

    #[tokio::test]
    async fn deleting_a_project_cascades_to_environments() {
        let s = store().await;
        let pid = project(&s).await;
        let e = s.insert_environment(pid, &env("dev")).await.unwrap();
        s.delete_project(pid).await.unwrap();
        assert_eq!(s.get_environment(e.id).await.unwrap(), None);
    }
}
