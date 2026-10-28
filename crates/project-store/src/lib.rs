//! SQLite persistence for Vagrant Studio metadata.
//!
//! Every repository works against a real SQLite database; tests use a fresh
//! temporary database instead of mocks so SQL, constraints and migrations are
//! exercised for real (see ADR-0007).

mod environments;
mod projects;
mod settings;

use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use time::OffsetDateTime;

pub use projects::{ProjectFilter, ProjectUpdate};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("stored data is corrupt: {0}")]
    Corrupt(String),
    #[error("{0}")]
    Conflict(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// Handle to the application database. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// Opens (creating if needed) the database file and applies pending migrations.
    pub async fn open(path: &Path) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;
        Self::migrate(pool).await
    }

    /// Private in-memory database (single connection), mainly for tests.
    pub async fn open_in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        Self::migrate(pool).await
    }

    async fn migrate(pool: SqlitePool) -> Result<Self> {
        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    /// Number of migrations known to this build (the schema version).
    pub fn schema_version() -> i64 {
        MIGRATOR.iter().map(|m| m.version).max().unwrap_or(0)
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// Current UTC time truncated to milliseconds so stored values have a
/// consistent precision.
pub(crate) fn now() -> OffsetDateTime {
    let now = OffsetDateTime::now_utc();
    now.replace_nanosecond(now.millisecond() as u32 * 1_000_000)
        .unwrap_or(now)
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(db) if db.is_unique_violation())
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::Store;

    pub async fn store() -> Store {
        Store::open_in_memory().await.expect("in-memory store")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_create_all_tables() {
        let store = test_support::store().await;
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations' ORDER BY name",
        )
        .fetch_all(&store.pool)
        .await
        .unwrap();
        let tables: Vec<_> = tables.into_iter().map(|(n,)| n).collect();
        assert_eq!(
            tables,
            [
                "app_settings",
                "environments",
                "machine_configs",
                "operations",
                "port_mappings",
                "project_notes",
                "project_tags",
                "project_tasks",
                "projects",
                "provisioners",
                "synced_folders",
                "tags",
            ]
        );
        assert_eq!(Store::schema_version(), 1);
    }

    #[tokio::test]
    async fn foreign_keys_are_enforced() {
        let store = test_support::store().await;
        let err = sqlx::query(
            "INSERT INTO environments (id, project_id, name, root_path, created_at, updated_at) VALUES ('e', 'missing', 'dev', '/x', 't', 't')",
        )
        .execute(&store.pool)
        .await
        .unwrap_err();
        assert!(err.to_string().contains("FOREIGN KEY"), "{err}");
    }

    #[tokio::test]
    async fn reopening_a_file_database_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Vägrant Studio data").join("studio.db");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let store = Store::open(&path).await.unwrap();
        store.close().await;
        let store = Store::open(&path).await.unwrap();
        let applied: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(applied.0, 1);
    }
}
