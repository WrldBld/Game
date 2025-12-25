//! SQLite Prompt Template Repository
//!
//! Stores prompt template overrides in SQLite. This handles the DB layer only;
//! resolution priority (world DB → global DB → env → default) is handled by
//! the PromptTemplateService in engine-app.

use async_trait::async_trait;
use sqlx::SqlitePool;

use wrldbldr_engine_ports::outbound::{PromptTemplateError, PromptTemplateRepositoryPort};
use wrldbldr_domain::WorldId;

/// SQLite-backed prompt template repository
pub struct SqlitePromptTemplateRepository {
    pool: SqlitePool,
}

impl SqlitePromptTemplateRepository {
    /// Create a new repository with the given SQLite pool
    ///
    /// Creates the required tables if they don't exist.
    pub async fn new(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        // Create global prompt templates table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS prompt_templates (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(&pool).await?;

        // Create per-world prompt templates table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS world_prompt_templates (
                world_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (world_id, key)
            )
        "#).execute(&pool).await?;

        Ok(Self { pool })
    }

    /// Get the underlying pool (for sharing with other repos)
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl PromptTemplateRepositoryPort for SqlitePromptTemplateRepository {
    async fn get_global(&self, key: &str) -> Result<Option<String>, PromptTemplateError> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM prompt_templates WHERE key = ?"
        )
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| PromptTemplateError::Database(e.to_string()))?;

        Ok(result.map(|(v,)| v))
    }

    async fn get_all_global(&self) -> Result<Vec<(String, String)>, PromptTemplateError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, value FROM prompt_templates ORDER BY key"
        )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PromptTemplateError::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn set_global(&self, key: &str, value: &str) -> Result<(), PromptTemplateError> {
        sqlx::query(
            "INSERT OR REPLACE INTO prompt_templates (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)"
        )
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| PromptTemplateError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_global(&self, key: &str) -> Result<(), PromptTemplateError> {
        sqlx::query("DELETE FROM prompt_templates WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| PromptTemplateError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_all_global(&self) -> Result<(), PromptTemplateError> {
        sqlx::query("DELETE FROM prompt_templates")
            .execute(&self.pool)
            .await
            .map_err(|e| PromptTemplateError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_for_world(&self, world_id: WorldId, key: &str) -> Result<Option<String>, PromptTemplateError> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM world_prompt_templates WHERE world_id = ? AND key = ?"
        )
            .bind(world_id.to_string())
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| PromptTemplateError::Database(e.to_string()))?;

        Ok(result.map(|(v,)| v))
    }

    async fn get_all_for_world(&self, world_id: WorldId) -> Result<Vec<(String, String)>, PromptTemplateError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, value FROM world_prompt_templates WHERE world_id = ? ORDER BY key"
        )
            .bind(world_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PromptTemplateError::Database(e.to_string()))?;

        Ok(rows)
    }

    async fn set_for_world(&self, world_id: WorldId, key: &str, value: &str) -> Result<(), PromptTemplateError> {
        sqlx::query(
            "INSERT OR REPLACE INTO world_prompt_templates (world_id, key, value, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP)"
        )
            .bind(world_id.to_string())
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| PromptTemplateError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_for_world(&self, world_id: WorldId, key: &str) -> Result<(), PromptTemplateError> {
        sqlx::query("DELETE FROM world_prompt_templates WHERE world_id = ? AND key = ?")
            .bind(world_id.to_string())
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| PromptTemplateError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_all_for_world(&self, world_id: WorldId) -> Result<(), PromptTemplateError> {
        sqlx::query("DELETE FROM world_prompt_templates WHERE world_id = ?")
            .bind(world_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| PromptTemplateError::Database(e.to_string()))?;

        Ok(())
    }
}
