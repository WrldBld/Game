//! SQLite-based persistence for DM directorial context.
//!
//! Stores directorial context (scene notes, tone, NPC motivations, forbidden topics)
//! as a JSON blob per world, allowing it to survive server restarts.

use async_trait::async_trait;
use anyhow::Result;
use sqlx::SqlitePool;
use wrldbldr_domain::WorldId;
use wrldbldr_protocol::DirectorialContext;
use wrldbldr_engine_ports::outbound::DirectorialContextRepositoryPort;

/// SQLite implementation of DirectorialContextRepositoryPort
pub struct SqliteDirectorialContextRepository {
    pool: SqlitePool,
}

impl SqliteDirectorialContextRepository {
    /// Create a new repository and ensure the table exists
    pub async fn new(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        // Create table if not exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS directorial_context (
                world_id TEXT PRIMARY KEY,
                context_json TEXT NOT NULL,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&pool)
        .await?;

        tracing::debug!("Initialized directorial_context table");
        Ok(Self { pool })
    }
}

#[async_trait]
impl DirectorialContextRepositoryPort for SqliteDirectorialContextRepository {
    async fn get(&self, world_id: &WorldId) -> Result<Option<DirectorialContext>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT context_json FROM directorial_context WHERE world_id = ?")
                .bind(world_id.to_string())
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some((json,)) => {
                let context: DirectorialContext = serde_json::from_str(&json)?;
                Ok(Some(context))
            }
            None => Ok(None),
        }
    }

    async fn save(&self, world_id: &WorldId, context: &DirectorialContext) -> Result<()> {
        let json = serde_json::to_string(context)?;

        sqlx::query(
            "INSERT OR REPLACE INTO directorial_context (world_id, context_json, updated_at) 
             VALUES (?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(world_id.to_string())
        .bind(json)
        .execute(&self.pool)
        .await?;

        tracing::debug!(world_id = %world_id, "Saved directorial context");
        Ok(())
    }

    async fn delete(&self, world_id: &WorldId) -> Result<()> {
        sqlx::query("DELETE FROM directorial_context WHERE world_id = ?")
            .bind(world_id.to_string())
            .execute(&self.pool)
            .await?;

        tracing::debug!(world_id = %world_id, "Deleted directorial context");
        Ok(())
    }
}
