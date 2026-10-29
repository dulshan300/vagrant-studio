use time::OffsetDateTime;
use vagrant_studio_domain::{EnvironmentId, OperationId};

use crate::{Result, Store};

/// Provisioner runs of one machine in one operation, stored as JSON.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct ProvisioningRecord {
    pub operation_id: String,
    pub environment_id: Option<String>,
    pub machine_name: String,
    pub finished_at: OffsetDateTime,
    pub runs_json: String,
}

impl Store {
    pub async fn save_provisioning(&self, records: &[ProvisioningRecord]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for r in records {
            sqlx::query(
                "INSERT INTO provisioning_results (operation_id, environment_id, machine_name, finished_at, runs_json)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&r.operation_id)
            .bind(&r.environment_id)
            .bind(&r.machine_name)
            .bind(r.finished_at)
            .bind(&r.runs_json)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// The machines of one operation, in the order they were saved.
    pub async fn operation_provisioning(
        &self,
        operation_id: OperationId,
    ) -> Result<Vec<ProvisioningRecord>> {
        Ok(sqlx::query_as(concat!(
            "SELECT operation_id, environment_id, machine_name, finished_at, runs_json ",
            "FROM provisioning_results WHERE operation_id = ? ORDER BY seq"
        ))
        .bind(operation_id.to_string())
        .fetch_all(&self.pool)
        .await?)
    }

    /// The most recent record of each machine of an environment.
    pub async fn latest_provisioning(
        &self,
        environment_id: EnvironmentId,
    ) -> Result<Vec<ProvisioningRecord>> {
        Ok(sqlx::query_as(concat!(
            "SELECT operation_id, environment_id, machine_name, finished_at, runs_json ",
            "FROM provisioning_results WHERE seq IN (",
            "SELECT MAX(seq) FROM provisioning_results WHERE environment_id = ? GROUP BY machine_name",
            ") ORDER BY seq"
        ))
        .bind(environment_id.to_string())
        .fetch_all(&self.pool)
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::tests::{environment, new_op};
    use crate::test_support::store;

    fn record(
        op: OperationId,
        env: EnvironmentId,
        machine: &str,
        runs: &str,
    ) -> ProvisioningRecord {
        ProvisioningRecord {
            operation_id: op.to_string(),
            environment_id: Some(env.to_string()),
            machine_name: machine.into(),
            finished_at: crate::now(),
            runs_json: runs.into(),
        }
    }

    #[tokio::test]
    async fn keeps_results_per_operation_and_finds_the_latest_per_machine() {
        let s = store().await;
        let env = environment(&s).await;
        let first = s.insert_operation(&new_op(Some(env)), "/l").await.unwrap();
        let second = s.insert_operation(&new_op(Some(env)), "/l").await.unwrap();
        s.save_provisioning(&[
            record(first.id, env, "web", "[1]"),
            record(first.id, env, "db", "[2]"),
        ])
        .await
        .unwrap();
        s.save_provisioning(&[record(second.id, env, "web", "[3]")])
            .await
            .unwrap();

        let of_first = s.operation_provisioning(first.id).await.unwrap();
        let machines: Vec<_> = of_first.iter().map(|r| r.machine_name.as_str()).collect();
        assert_eq!(machines, ["web", "db"]);

        let latest = s.latest_provisioning(env).await.unwrap();
        let runs: Vec<_> = latest
            .iter()
            .map(|r| (r.machine_name.as_str(), r.runs_json.as_str()))
            .collect();
        assert_eq!(runs, [("db", "[2]"), ("web", "[3]")]);

        // A machine is recorded once per operation.
        assert!(
            s.save_provisioning(&[record(second.id, env, "web", "[]")])
                .await
                .is_err()
        );
    }
}
