use time::OffsetDateTime;
use vagrant_studio_domain::{
    EnvironmentId, NewOperation, Operation, OperationId, OperationStatus, OperationType,
};

use crate::{Result, Store, StoreError, now};

#[derive(sqlx::FromRow)]
struct OperationRow {
    id: String,
    project_id: Option<String>,
    environment_id: Option<String>,
    machine_name: Option<String>,
    target_label: String,
    operation_type: String,
    status: String,
    command_line: String,
    working_dir: Option<String>,
    started_at: Option<OffsetDateTime>,
    finished_at: Option<OffsetDateTime>,
    exit_code: Option<i32>,
    log_path: Option<String>,
    error_summary: Option<String>,
    created_at: OffsetDateTime,
}

fn opt_id<T: std::str::FromStr>(value: Option<String>, what: &str) -> Result<Option<T>> {
    value
        .map(|s| {
            s.parse()
                .map_err(|_| StoreError::Corrupt(format!("invalid {what} {s}")))
        })
        .transpose()
}

impl TryFrom<OperationRow> for Operation {
    type Error = StoreError;

    fn try_from(r: OperationRow) -> Result<Self> {
        let corrupt = |what: &str, v: &str| StoreError::Corrupt(format!("invalid {what} {v}"));
        Ok(Operation {
            id: r.id.parse().map_err(|_| corrupt("operation id", &r.id))?,
            project_id: opt_id(r.project_id, "project id")?,
            environment_id: opt_id(r.environment_id, "environment id")?,
            status: OperationStatus::parse(&r.status)
                .ok_or_else(|| corrupt("operation status", &r.status))?,
            operation_type: OperationType::parse(&r.operation_type),
            machine_name: r.machine_name,
            target_label: r.target_label,
            command_line: r.command_line,
            working_dir: r.working_dir,
            started_at: r.started_at,
            finished_at: r.finished_at,
            exit_code: r.exit_code,
            log_path: r.log_path,
            error_summary: r.error_summary,
            created_at: r.created_at,
        })
    }
}

/// Last observed `vagrant status` for an environment, stored as JSON.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct EnvironmentStatusRecord {
    pub environment_id: String,
    pub operation_id: Option<String>,
    pub observed_at: OffsetDateTime,
    pub machines_json: String,
    pub error_class: Option<String>,
    pub error_message: Option<String>,
}

impl Store {
    pub async fn insert_operation(&self, new: &NewOperation, log_path: &str) -> Result<Operation> {
        let id = OperationId::new();
        sqlx::query(
            "INSERT INTO operations (id, project_id, environment_id, machine_name, target_label, operation_type, status, command_line, working_dir, log_path, created_at)
             VALUES (?, ?, ?, ?, ?, ?, 'queued', ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(new.project_id.map(|i| i.to_string()))
        .bind(new.environment_id.map(|i| i.to_string()))
        .bind(&new.machine_name)
        .bind(&new.target_label)
        .bind(new.operation_type.as_str())
        .bind(&new.command_line)
        .bind(&new.working_dir)
        .bind(log_path.replace("{id}", &id.to_string()))
        .bind(now())
        .execute(&self.pool)
        .await?;
        self.get_operation(id)
            .await?
            .ok_or_else(|| StoreError::Corrupt("inserted operation not found".into()))
    }

    pub async fn get_operation(&self, id: OperationId) -> Result<Option<Operation>> {
        let row: Option<OperationRow> = sqlx::query_as("SELECT * FROM operations WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(Operation::try_from).transpose()
    }

    /// Most recent first (UUIDv7 ids sort by creation time).
    pub async fn list_operations(
        &self,
        environment_id: Option<EnvironmentId>,
        limit: i64,
    ) -> Result<Vec<Operation>> {
        let rows: Vec<OperationRow> = match environment_id {
            Some(env) => sqlx::query_as(
                "SELECT * FROM operations WHERE environment_id = ? ORDER BY id DESC LIMIT ?",
            )
            .bind(env.to_string())
            .bind(limit),
            None => sqlx::query_as("SELECT * FROM operations ORDER BY id DESC LIMIT ?").bind(limit),
        }
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Operation::try_from).collect()
    }

    pub async fn mark_operation_running(&self, id: OperationId) -> Result<Operation> {
        sqlx::query(
            "UPDATE operations SET status = 'running', started_at = ? WHERE id = ? AND status = 'queued'",
        )
        .bind(now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        self.require_operation(id).await
    }

    pub async fn mark_operation_cancel_requested(&self, id: OperationId) -> Result<()> {
        sqlx::query(
            "UPDATE operations SET status = 'cancel_requested' WHERE id = ? AND status IN ('queued', 'running')",
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Records the final state. Only unfinished operations are updated.
    pub async fn finish_operation(
        &self,
        id: OperationId,
        status: OperationStatus,
        exit_code: Option<i32>,
        error_summary: Option<&str>,
    ) -> Result<Operation> {
        debug_assert!(status.is_finished());
        sqlx::query(
            "UPDATE operations SET status = ?, exit_code = ?, error_summary = ?, finished_at = ?
             WHERE id = ? AND status IN ('queued', 'running', 'cancel_requested')",
        )
        .bind(status.as_str())
        .bind(exit_code)
        .bind(error_summary)
        .bind(now())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        self.require_operation(id).await
    }

    /// Crash recovery: anything left unfinished by a previous session can no
    /// longer be tracked. Returns how many operations were marked interrupted.
    pub async fn interrupt_unfinished_operations(&self) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE operations SET status = 'interrupted', finished_at = ?,
                 error_summary = COALESCE(error_summary, 'Vagrant Studio closed while this operation was running. Its outcome is unknown; check the machine state.')
             WHERE status IN ('queued', 'running', 'cancel_requested')",
        )
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn save_environment_status(&self, record: &EnvironmentStatusRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO environment_status (environment_id, operation_id, observed_at, machines_json, error_class, error_message)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (environment_id) DO UPDATE SET operation_id = excluded.operation_id, observed_at = excluded.observed_at,
                 machines_json = excluded.machines_json, error_class = excluded.error_class, error_message = excluded.error_message",
        )
        .bind(&record.environment_id)
        .bind(&record.operation_id)
        .bind(record.observed_at)
        .bind(&record.machines_json)
        .bind(&record.error_class)
        .bind(&record.error_message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_environment_status(
        &self,
        environment_id: EnvironmentId,
    ) -> Result<Option<EnvironmentStatusRecord>> {
        Ok(
            sqlx::query_as("SELECT * FROM environment_status WHERE environment_id = ?")
                .bind(environment_id.to_string())
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub async fn list_environment_statuses(&self) -> Result<Vec<EnvironmentStatusRecord>> {
        Ok(sqlx::query_as("SELECT * FROM environment_status")
            .fetch_all(&self.pool)
            .await?)
    }

    async fn require_operation(&self, id: OperationId) -> Result<Operation> {
        self.get_operation(id)
            .await?
            .ok_or_else(|| StoreError::Conflict("Operation no longer exists.".into()))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::test_support::store;
    use vagrant_studio_domain::{EnvironmentType, NewEnvironment, NewProject};

    pub(crate) fn new_op(env: Option<EnvironmentId>) -> NewOperation {
        NewOperation {
            project_id: None,
            environment_id: env,
            machine_name: None,
            target_label: "Web · default".into(),
            operation_type: OperationType::Status,
            command_line: "vagrant status --machine-readable".into(),
            working_dir: Some("/p".into()),
        }
    }

    pub(crate) async fn environment(s: &Store) -> EnvironmentId {
        let p = s
            .insert_project(&NewProject {
                name: "Web".into(),
                description: String::new(),
                root_path: "/p".into(),
                repository_url: None,
                tags: vec![],
            })
            .await
            .unwrap();
        s.insert_environment(
            p.id,
            &NewEnvironment {
                name: "default".into(),
                root_path: "/p".into(),
                vagrantfile_path: None,
                default_provider: None,
                environment_type: EnvironmentType::Development,
            },
        )
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn lifecycle_of_an_operation() {
        let s = store().await;
        let op = s
            .insert_operation(&new_op(None), "/logs/{id}.log")
            .await
            .unwrap();
        assert_eq!(op.status, OperationStatus::Queued);
        assert_eq!(op.log_path, Some(format!("/logs/{}.log", op.id)));
        assert_eq!(op.environment_id, None);

        let running = s.mark_operation_running(op.id).await.unwrap();
        assert_eq!(running.status, OperationStatus::Running);
        assert!(running.started_at.is_some());

        let done = s
            .finish_operation(op.id, OperationStatus::Failed, Some(1), Some("boom"))
            .await
            .unwrap();
        assert_eq!(done.status, OperationStatus::Failed);
        assert_eq!(done.exit_code, Some(1));
        assert_eq!(done.error_summary.as_deref(), Some("boom"));

        // A finished operation can't be changed again.
        let again = s
            .finish_operation(op.id, OperationStatus::Succeeded, Some(0), None)
            .await
            .unwrap();
        assert_eq!(again.status, OperationStatus::Failed);
    }

    #[tokio::test]
    async fn unfinished_operations_are_interrupted_on_recovery() {
        let s = store().await;
        let a = s.insert_operation(&new_op(None), "").await.unwrap();
        let b = s.insert_operation(&new_op(None), "").await.unwrap();
        s.mark_operation_running(b.id).await.unwrap();
        let c = s.insert_operation(&new_op(None), "").await.unwrap();
        s.finish_operation(c.id, OperationStatus::Succeeded, Some(0), None)
            .await
            .unwrap();

        assert_eq!(s.interrupt_unfinished_operations().await.unwrap(), 2);
        for id in [a.id, b.id] {
            let op = s.get_operation(id).await.unwrap().unwrap();
            assert_eq!(op.status, OperationStatus::Interrupted);
            assert!(op.error_summary.unwrap().contains("outcome is unknown"));
        }
        assert_eq!(
            s.get_operation(c.id).await.unwrap().unwrap().status,
            OperationStatus::Succeeded
        );
    }

    #[tokio::test]
    async fn history_is_newest_first_filterable_and_survives_environment_removal() {
        let s = store().await;
        let env = environment(&s).await;
        let first = s.insert_operation(&new_op(Some(env)), "").await.unwrap();
        let host = s.insert_operation(&new_op(None), "").await.unwrap();
        let ids = |ops: Vec<Operation>| ops.into_iter().map(|o| o.id).collect::<Vec<_>>();

        assert_eq!(
            ids(s.list_operations(None, 10).await.unwrap()),
            vec![host.id, first.id]
        );
        assert_eq!(
            ids(s.list_operations(Some(env), 10).await.unwrap()),
            vec![first.id]
        );
        assert_eq!(s.list_operations(None, 1).await.unwrap().len(), 1);

        s.delete_environment(env).await.unwrap();
        let kept = s.get_operation(first.id).await.unwrap().unwrap();
        assert_eq!(kept.environment_id, None);
        assert_eq!(kept.target_label, "Web · default");
    }

    #[tokio::test]
    async fn environment_status_is_upserted_and_cascades() {
        let s = store().await;
        let env = environment(&s).await;
        let mut record = EnvironmentStatusRecord {
            environment_id: env.to_string(),
            operation_id: None,
            observed_at: now(),
            machines_json: "[]".into(),
            error_class: None,
            error_message: None,
        };
        s.save_environment_status(&record).await.unwrap();
        record.machines_json = r#"[{"name":"web"}]"#.into();
        s.save_environment_status(&record).await.unwrap();
        assert_eq!(
            s.get_environment_status(env).await.unwrap().unwrap(),
            record
        );
        s.delete_environment(env).await.unwrap();
        assert_eq!(s.get_environment_status(env).await.unwrap(), None);
    }
}
