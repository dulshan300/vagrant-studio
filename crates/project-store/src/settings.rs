use crate::{Result, Store, now};

impl Store {
    /// Raw JSON value of an application setting, if set.
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT value_json FROM app_settings WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|(v,)| v))
    }

    pub async fn set_setting(&self, key: &str, value_json: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO app_settings (key, value_json, updated_at) VALUES (?, ?, ?)
             ON CONFLICT (key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value_json)
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::store;

    #[tokio::test]
    async fn settings_upsert() {
        let s = store().await;
        assert_eq!(s.get_setting("k").await.unwrap(), None);
        s.set_setting("k", "1").await.unwrap();
        s.set_setting("k", "\"two\"").await.unwrap();
        assert_eq!(
            s.get_setting("k").await.unwrap().as_deref(),
            Some("\"two\"")
        );
    }
}
