//! SQLite-backed settings storage.

use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use wrldbldr_domain::{AppSettings, WorldId};

use crate::infrastructure::ports::{ClockPort, RepoError, SettingsRepo};

/// SQLite implementation for application settings storage.
pub struct SqliteSettingsRepo {
    pool: SqlitePool,
    clock: Arc<dyn ClockPort>,
}

impl SqliteSettingsRepo {
    pub async fn new(db_path: &str, clock: Arc<dyn ClockPort>) -> Result<Self, RepoError> {
        let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path))
            .await
            .map_err(|e| RepoError::database("settings", e))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS app_settings (
                scope TEXT NOT NULL,
                world_id TEXT,
                settings_json TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (scope, world_id)
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| RepoError::database("settings", e))?;

        Ok(Self { pool, clock })
    }

    async fn get_by_scope(
        &self,
        scope: &str,
        world_id: Option<&str>,
    ) -> Result<Option<AppSettings>, RepoError> {
        let mut query =
            String::from("SELECT settings_json FROM app_settings WHERE scope = ? AND world_id ");

        if world_id.is_some() {
            query.push_str("= ?");
        } else {
            query.push_str("IS NULL");
        }

        let mut q = sqlx::query(&query).bind(scope);
        if let Some(world_id) = world_id {
            q = q.bind(world_id);
        }

        let row = q
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RepoError::database("settings", e))?;

        match row {
            Some(row) => {
                let json: String = row.get("settings_json");
                let settings = serde_json::from_str(&json)
                    .map_err(|e| RepoError::Serialization(e.to_string()))?;
                Ok(Some(settings))
            }
            None => Ok(None),
        }
    }

    async fn save_by_scope(
        &self,
        scope: &str,
        world_id: Option<&str>,
        settings: &AppSettings,
    ) -> Result<(), RepoError> {
        let json =
            serde_json::to_string(settings).map_err(|e| RepoError::Serialization(e.to_string()))?;
        let now = self.clock.now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO app_settings (scope, world_id, settings_json, updated_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(scope, world_id) DO UPDATE SET
                settings_json = excluded.settings_json,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(scope)
        .bind(world_id)
        .bind(json)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepoError::database("settings", e))?;

        Ok(())
    }
}

#[async_trait]
impl SettingsRepo for SqliteSettingsRepo {
    async fn get_global(&self) -> Result<Option<AppSettings>, RepoError> {
        self.get_by_scope("global", None).await
    }

    async fn save_global(&self, settings: &AppSettings) -> Result<(), RepoError> {
        self.save_by_scope("global", None, settings).await
    }

    async fn get_for_world(&self, world_id: WorldId) -> Result<Option<AppSettings>, RepoError> {
        self.get_by_scope("world", Some(&world_id.to_string()))
            .await
    }

    async fn save_for_world(
        &self,
        world_id: WorldId,
        settings: &AppSettings,
    ) -> Result<(), RepoError> {
        self.save_by_scope("world", Some(&world_id.to_string()), settings)
            .await
    }

    async fn delete_for_world(&self, world_id: WorldId) -> Result<(), RepoError> {
        sqlx::query(
            "DELETE FROM app_settings WHERE scope = ? AND world_id = ?",
        )
        .bind("world")
        .bind(world_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| RepoError::database("settings", e))?;
        Ok(())
    }
}
