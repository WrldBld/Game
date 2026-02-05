//! SQLite-backed prompt template storage.

use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

use crate::infrastructure::ports::{ClockPort, PromptTemplateRepo, RepoError, ResolvedPromptTemplate};
use crate::prompt_templates::{key_to_env_var, is_known_key, PromptDefaultsStore};
use wrldbldr_domain::WorldId;

/// SQLite implementation for prompt template storage.
///
/// Supports global and world-specific template overrides with
/// resolution priority: World DB > Global DB > Environment Variable > Default.
pub struct SqlitePromptTemplateRepo {
    pool: SqlitePool,
    clock: Arc<dyn ClockPort>,
    defaults_store: PromptDefaultsStore,
}

impl SqlitePromptTemplateRepo {
    /// Create a new prompt template repository.
    ///
    /// The `defaults_store` is used to load default prompt templates from files.
    pub async fn new(
        db_path: &str,
        clock: Arc<dyn ClockPort>,
        defaults_store: PromptDefaultsStore,
    ) -> Result<Self, RepoError> {
        let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path))
            .await
            .map_err(|e| RepoError::database("prompt_templates", e))?;

        // Create global overrides table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS prompt_templates (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| RepoError::database("prompt_templates", e))?;

        // Create world-specific overrides table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS world_prompt_templates (
                world_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (world_id, key)
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| RepoError::database("prompt_templates", e))?;

        Ok(Self {
            pool,
            clock,
            defaults_store,
        })
    }

    /// Get a reference to the defaults store.
    #[allow(dead_code)]
    pub fn defaults_store(&self) -> &PromptDefaultsStore {
        &self.defaults_store
    }
}

#[async_trait]
impl PromptTemplateRepo for SqlitePromptTemplateRepo {
    async fn get_global_override(&self, key: &str) -> Result<Option<String>, RepoError> {
        let row = sqlx::query("SELECT value FROM prompt_templates WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RepoError::database("prompt_templates", e))?;

        Ok(row.map(|r| r.get::<String, _>("value")))
    }

    async fn get_world_override(
        &self,
        world_id: WorldId,
        key: &str,
    ) -> Result<Option<String>, RepoError> {
        let row = sqlx::query("SELECT value FROM world_prompt_templates WHERE world_id = ? AND key = ?")
            .bind(world_id.to_string())
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RepoError::database("prompt_templates", e))?;

        Ok(row.map(|r| r.get::<String, _>("value")))
    }

    async fn set_global_override(&self, key: &str, value: &str) -> Result<(), RepoError> {
        let now = self.clock.now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO prompt_templates (key, value, updated_at)
            VALUES (?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepoError::database("prompt_templates", e))?;

        Ok(())
    }

    async fn set_world_override(
        &self,
        world_id: WorldId,
        key: &str,
        value: &str,
    ) -> Result<(), RepoError> {
        let now = self.clock.now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO world_prompt_templates (world_id, key, value, updated_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(world_id, key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(world_id.to_string())
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| RepoError::database("prompt_templates", e))?;

        Ok(())
    }

    async fn delete_global_override(&self, key: &str) -> Result<(), RepoError> {
        sqlx::query("DELETE FROM prompt_templates WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| RepoError::database("prompt_templates", e))?;
        Ok(())
    }

    async fn delete_world_override(&self, world_id: WorldId, key: &str) -> Result<(), RepoError> {
        sqlx::query("DELETE FROM world_prompt_templates WHERE world_id = ? AND key = ?")
            .bind(world_id.to_string())
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| RepoError::database("prompt_templates", e))?;
        Ok(())
    }

    async fn list_global_overrides(&self) -> Result<Vec<(String, String)>, RepoError> {
        let rows = sqlx::query("SELECT key, value FROM prompt_templates ORDER BY key")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RepoError::database("prompt_templates", e))?;

        let overrides = rows
            .iter()
            .map(|row| (row.get::<String, _>("key"), row.get::<String, _>("value")))
            .collect();

        Ok(overrides)
    }

    async fn list_world_overrides(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<(String, String)>, RepoError> {
        let rows = sqlx::query("SELECT key, value FROM world_prompt_templates WHERE world_id = ? ORDER BY key")
            .bind(world_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RepoError::database("prompt_templates", e))?;

        let overrides = rows
            .iter()
            .map(|row| (row.get::<String, _>("key"), row.get::<String, _>("value")))
            .collect();

        Ok(overrides)
    }

    async fn resolve_template(
        &self,
        world_id: Option<WorldId>,
        key: &str,
    ) -> Result<Option<String>, RepoError> {
        // Check if this is a recognized template key
        if !is_known_key(key) {
            return Ok(None); // Unknown key
        }

        // Resolution priority:
        // 1. World-specific override (if world_id provided)
        // 2. Global override
        // 3. Environment variable
        // 4. Default value (from file-backed store)

        if let Some(world_id) = world_id {
            if let Some(override_value) = self.get_world_override(world_id, key).await? {
                return Ok(Some(override_value));
            }
        }

        if let Some(override_value) = self.get_global_override(key).await? {
            return Ok(Some(override_value));
        }

        // Check environment variable
        let env_var = key_to_env_var(key);
        if let Ok(env_value) = std::env::var(&env_var) {
            if !env_value.trim().is_empty() {
                return Ok(Some(env_value));
            }
        }

        // Return default value from file-backed store
        match self.defaults_store.get(key).await {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => {
                // File not found - this is a configuration error
                tracing::error!(
                    key = key,
                    path = %self.defaults_store.path_for_key(key).display(),
                    "Default prompt file not found for known key"
                );
                Ok(None)
            }
            Err(e) => {
                // Error reading file
                tracing::error!(
                    key = key,
                    error = %e,
                    "Failed to read default prompt file"
                );
                Err(RepoError::Config(format!(
                    "Failed to read default prompt for '{}': {}",
                    key, e
                )))
            }
        }
    }

    async fn resolve_template_with_metadata(
        &self,
        world_id: Option<WorldId>,
        key: &str,
    ) -> Result<Option<ResolvedPromptTemplate>, RepoError> {
        // Check if this is a recognized template key
        if !is_known_key(key) {
            return Ok(None); // Unknown key
        }

        // First, get the default template metadata (includes temperature from front matter)
        let default_template = self.defaults_store.get_with_metadata(key).await.ok().flatten();

        // Resolution priority for template text:
        // 1. World-specific override (if world_id provided)
        // 2. Global override
        // 3. Environment variable
        // 4. Default value (from file-backed store)
        //
        // Temperature precedence:
        // 1. Template front matter temperature (if using default template)
        // 2. None (when using override - caller should use AppSettings temperature)

        // Check for overrides
        if let Some(world_id) = world_id {
            if let Some(override_value) = self.get_world_override(world_id, key).await? {
                // Override text with default temperature (if default exists)
                return Ok(Some(ResolvedPromptTemplate {
                    text: override_value,
                    temperature: default_template.as_ref().and_then(|t| t.temperature),
                }));
            }
        }

        if let Some(override_value) = self.get_global_override(key).await? {
            // Override text with default temperature (if default exists)
            return Ok(Some(ResolvedPromptTemplate {
                text: override_value,
                temperature: default_template.as_ref().and_then(|t| t.temperature),
            }));
        }

        // Check environment variable
        let env_var = key_to_env_var(key);
        if let Ok(env_value) = std::env::var(&env_var) {
            if !env_value.trim().is_empty() {
                // Env var text with default temperature (if default exists)
                return Ok(Some(ResolvedPromptTemplate {
                    text: env_value,
                    temperature: default_template.as_ref().and_then(|t| t.temperature),
                }));
            }
        }

        // Return default template with its metadata (including front matter temperature)
        match default_template {
            Some(template) => Ok(Some(template)),
            None => {
                // File not found - this is a configuration error
                tracing::error!(
                    key = key,
                    path = %self.defaults_store.path_for_key(key).display(),
                    "Default prompt file not found for known key"
                );
                Ok(None)
            }
        }
    }
}
