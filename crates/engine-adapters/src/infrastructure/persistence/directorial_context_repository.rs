//! SQLite-based persistence for DM directorial context.
//!
//! Stores directorial context (scene notes, tone, NPC motivations, forbidden topics)
//! as a JSON blob per world, allowing it to survive server restarts.
//!
//! The port uses domain `DirectorialNotes`, but we store as protocol `DirectorialContext`
//! for backward compatibility with existing data. Converters handle the transformation.

use async_trait::async_trait;
use anyhow::Result;
use sqlx::SqlitePool;
use wrldbldr_domain::WorldId;
use wrldbldr_domain::value_objects::DirectorialNotes;
use wrldbldr_protocol::DirectorialContext;
use wrldbldr_engine_ports::outbound::DirectorialContextRepositoryPort;

use crate::infrastructure::websocket::directorial_converters::{
    directorial_context_to_notes, directorial_notes_to_context,
};

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
    async fn get(&self, world_id: &WorldId) -> Result<Option<DirectorialNotes>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT context_json FROM directorial_context WHERE world_id = ?")
                .bind(world_id.to_string())
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some((json,)) => {
                // Parse as protocol type, then convert to domain type
                let context: DirectorialContext = serde_json::from_str(&json)?;
                Ok(Some(directorial_context_to_notes(context)))
            }
            None => Ok(None),
        }
    }

    async fn save(&self, world_id: &WorldId, notes: &DirectorialNotes) -> Result<()> {
        // Convert domain type to protocol type for storage
        let context = directorial_notes_to_context(notes.clone());
        let json = serde_json::to_string(&context)?;

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
