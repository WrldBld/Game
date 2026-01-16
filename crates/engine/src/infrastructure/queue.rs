//! SQLite queue implementation for persistent job queues.
//!
//! Provides persistent queue storage for player actions, LLM requests,
//! DM approvals, and asset generation jobs.

use crate::queue_types::{
    ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use uuid::Uuid;

use crate::infrastructure::ports::{
    ClockPort, QueueError, QueueItem, QueueItemData, QueueItemStatus, QueuePort,
};

/// SQLite-backed queue implementation
pub struct SqliteQueue {
    pool: SqlitePool,
    clock: Arc<dyn ClockPort>,
}

impl SqliteQueue {
    /// Create a new SQLite queue with the given database path
    pub async fn new(db_path: &str, clock: Arc<dyn ClockPort>) -> Result<Self, QueueError> {
        // Create connection pool
        let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path))
            .await
            .map_err(|e| QueueError::Error(e.to_string()))?;

        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS queue_items (
                id TEXT PRIMARY KEY,
                queue_type TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                error_message TEXT,
                result_json TEXT,
                callback_id TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        // Create index for queue lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_queue_status
            ON queue_items(queue_type, status, created_at)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        // Migration: add callback_id column if missing (for existing DBs)
        let _ = sqlx::query("ALTER TABLE queue_items ADD COLUMN callback_id TEXT")
            .execute(&pool)
            .await;

        // Create index for callback_id lookups (used by dismiss/delete)
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_queue_callback_id
            ON queue_items(callback_id)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        // Durable read-state for Creator generation queue hydration.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS generation_read_state (
                user_id TEXT NOT NULL,
                world_id TEXT NOT NULL,
                read_batches_json TEXT NOT NULL,
                read_suggestions_json TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (user_id, world_id)
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        Ok(Self { pool, clock })
    }

    /// Enqueue an item to a specific queue type
    async fn enqueue_item<T: serde::Serialize>(
        &self,
        queue_type: &str,
        data: &T,
    ) -> Result<Uuid, QueueError> {
        let id = Uuid::new_v4();
        let payload_json =
            serde_json::to_string(data).map_err(|e| QueueError::Error(e.to_string()))?;
        let now = self.clock.now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO queue_items (id, queue_type, payload_json, status, created_at, updated_at)
            VALUES (?, ?, ?, 'pending', ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(queue_type)
        .bind(&payload_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        Ok(id)
    }

    /// Dequeue an item from a specific queue type
    async fn dequeue_item(&self, queue_type: &str) -> Result<Option<QueueItem>, QueueError> {
        let now = self.clock.now().to_rfc3339();

        // Atomically select and update the next pending item
        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'processing', updated_at = ?
            WHERE id = (
                SELECT id FROM queue_items
                WHERE queue_type = ? AND status = 'pending'
                ORDER BY created_at ASC
                LIMIT 1
            )
            AND queue_type = ? AND status = 'pending'
            RETURNING id, queue_type, payload_json, status, created_at, updated_at, error_message, result_json
            "#,
        )
        .bind(&now)
        .bind(queue_type)
        .bind(queue_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        if let Some(row) = result {
            self.row_to_queue_item(row).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Convert a database row to a QueueItem
    fn row_to_queue_item(&self, row: sqlx::sqlite::SqliteRow) -> Result<QueueItem, QueueError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str).map_err(|e| QueueError::Error(e.to_string()))?;

        let queue_type: String = row.get("queue_type");
        let payload_json: String = row.get("payload_json");
        let status_str: String = row.get("status");
        let created_at_str: String = row.get("created_at");
        let error_message: Option<String> = row.get("error_message");
        let result_json: Option<String> = row.try_get("result_json").ok();

        let status = match status_str.as_str() {
            "pending" => QueueItemStatus::Pending,
            "processing" => QueueItemStatus::Processing,
            "completed" => QueueItemStatus::Completed,
            "failed" => QueueItemStatus::Failed,
            _ => QueueItemStatus::Pending,
        };

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| QueueError::Error(e.to_string()))?
            .with_timezone(&Utc);

        let data = match queue_type.as_str() {
            "player_action" => {
                let payload: PlayerActionData = serde_json::from_str(&payload_json)
                    .map_err(|e| QueueError::Error(e.to_string()))?;
                QueueItemData::PlayerAction(payload)
            }
            "llm_request" => {
                let payload: LlmRequestData = serde_json::from_str(&payload_json)
                    .map_err(|e| QueueError::Error(e.to_string()))?;
                QueueItemData::LlmRequest(payload)
            }
            "dm_approval" => {
                let payload: ApprovalRequestData = serde_json::from_str(&payload_json)
                    .map_err(|e| QueueError::Error(e.to_string()))?;
                QueueItemData::DmApproval(payload)
            }
            "asset_generation" => {
                let payload: AssetGenerationData = serde_json::from_str(&payload_json)
                    .map_err(|e| QueueError::Error(e.to_string()))?;
                QueueItemData::AssetGeneration(payload)
            }
            _ => {
                return Err(QueueError::Error(format!(
                    "Unknown queue type: {}",
                    queue_type
                )))
            }
        };

        Ok(QueueItem {
            id,
            data,
            created_at,
            status,
            error_message,
            result_json,
        })
    }
}

#[async_trait]
impl QueuePort for SqliteQueue {
    // Player action queue
    async fn enqueue_player_action(&self, data: &PlayerActionData) -> Result<Uuid, QueueError> {
        self.enqueue_item("player_action", data).await
    }

    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        self.dequeue_item("player_action").await
    }

    // LLM request queue
    async fn enqueue_llm_request(&self, data: &LlmRequestData) -> Result<Uuid, QueueError> {
        // Specialized insert that stores callback_id in indexed column for fast lookup
        let id = Uuid::new_v4();
        let payload_json =
            serde_json::to_string(data).map_err(|e| QueueError::Error(e.to_string()))?;
        let now = self.clock.now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO queue_items (id, queue_type, payload_json, status, created_at, updated_at, callback_id)
            VALUES (?, ?, ?, 'pending', ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind("llm_request")
        .bind(&payload_json)
        .bind(&now)
        .bind(&now)
        .bind(&data.callback_id)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        Ok(id)
    }

    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        self.dequeue_item("llm_request").await
    }

    // DM approval queue
    async fn enqueue_dm_approval(&self, data: &ApprovalRequestData) -> Result<Uuid, QueueError> {
        self.enqueue_item("dm_approval", data).await
    }

    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        self.dequeue_item("dm_approval").await
    }

    // Asset generation queue
    async fn enqueue_asset_generation(
        &self,
        data: &AssetGenerationData,
    ) -> Result<Uuid, QueueError> {
        self.enqueue_item("asset_generation", data).await
    }

    async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
        self.dequeue_item("asset_generation").await
    }

    // Common operations
    async fn mark_complete(&self, id: Uuid) -> Result<(), QueueError> {
        let now = self.clock.now().to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'completed', updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(QueueError::Error(format!("Queue item not found: {}", id)));
        }

        Ok(())
    }

    async fn mark_failed(&self, id: Uuid, error: &str) -> Result<(), QueueError> {
        let now = self.clock.now().to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'failed', updated_at = ?, error_message = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(error)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(QueueError::Error(format!("Queue item not found: {}", id)));
        }

        Ok(())
    }

    async fn list_by_type(
        &self,
        queue_type: &str,
        limit: usize,
    ) -> Result<Vec<QueueItem>, QueueError> {
        let rows = sqlx::query(
            r#"
            SELECT id, queue_type, payload_json, status, created_at, updated_at, error_message, result_json
            FROM queue_items
            WHERE queue_type = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(queue_type)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.row_to_queue_item(row)?);
        }
        Ok(out)
    }

    async fn set_result_json(&self, id: Uuid, result_json: &str) -> Result<(), QueueError> {
        let now = self.clock.now().to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET result_json = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(result_json)
        .bind(&now)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(QueueError::Error(format!("Queue item not found: {}", id)));
        }

        Ok(())
    }

    async fn cancel_pending_llm_request_by_callback_id(
        &self,
        callback_id: &str,
    ) -> Result<bool, QueueError> {
        // Use indexed callback_id column for efficient cancellation
        let now = self.clock.now().to_rfc3339();
        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'failed', updated_at = ?, error_message = 'Cancelled'
            WHERE callback_id = ? AND queue_type = 'llm_request' AND status = 'pending'
            "#,
        )
        .bind(&now)
        .bind(callback_id)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_pending_count(&self, queue_type: &str) -> Result<usize, QueueError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count FROM queue_items
            WHERE queue_type = ? AND status = 'pending'
            "#,
        )
        .bind(queue_type)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        let count: i64 = row.get("count");
        Ok(count as usize)
    }

    async fn get_approval_request(
        &self,
        id: Uuid,
    ) -> Result<Option<ApprovalRequestData>, QueueError> {
        let result = sqlx::query(
            r#"
            SELECT payload_json FROM queue_items
            WHERE id = ? AND queue_type = 'dm_approval'
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        match result {
            Some(row) => {
                let payload_json: String = row.get("payload_json");
                let data: ApprovalRequestData = serde_json::from_str(&payload_json)
                    .map_err(|e| QueueError::Error(e.to_string()))?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    async fn get_generation_read_state(
        &self,
        user_id: &str,
        world_id: wrldbldr_domain::WorldId,
    ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
        let row = sqlx::query(
            r#"
            SELECT read_batches_json, read_suggestions_json
            FROM generation_read_state
            WHERE user_id = ? AND world_id = ?
            "#,
        )
        .bind(user_id)
        .bind(world_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let read_batches_json: String = row
            .try_get("read_batches_json")
            .map_err(|e| QueueError::Error(e.to_string()))?;
        let read_suggestions_json: String = row
            .try_get("read_suggestions_json")
            .map_err(|e| QueueError::Error(e.to_string()))?;

        let read_batches: Vec<String> = serde_json::from_str(&read_batches_json)
            .map_err(|e| QueueError::Error(e.to_string()))?;
        let read_suggestions: Vec<String> = serde_json::from_str(&read_suggestions_json)
            .map_err(|e| QueueError::Error(e.to_string()))?;

        Ok(Some((read_batches, read_suggestions)))
    }

    async fn upsert_generation_read_state(
        &self,
        user_id: &str,
        world_id: wrldbldr_domain::WorldId,
        read_batches: &[String],
        read_suggestions: &[String],
    ) -> Result<(), QueueError> {
        let now = self.clock.now().to_rfc3339();
        let read_batches_json =
            serde_json::to_string(read_batches).map_err(|e| QueueError::Error(e.to_string()))?;
        let read_suggestions_json = serde_json::to_string(read_suggestions)
            .map_err(|e| QueueError::Error(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO generation_read_state (
                user_id,
                world_id,
                read_batches_json,
                read_suggestions_json,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(user_id, world_id)
            DO UPDATE SET
                read_batches_json = excluded.read_batches_json,
                read_suggestions_json = excluded.read_suggestions_json,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(user_id)
        .bind(world_id.to_string())
        .bind(read_batches_json)
        .bind(read_suggestions_json)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Error(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_callback_id(&self, callback_id: &str) -> Result<bool, QueueError> {
        tracing::debug!(callback_id = %callback_id, "Attempting to delete queue item by callback_id");

        let result = sqlx::query("DELETE FROM queue_items WHERE callback_id = ?")
            .bind(callback_id)
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::Error(e.to_string()))?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            tracing::info!(callback_id = %callback_id, "Deleted queue item by callback_id");
        } else {
            // Log existing callback_ids for debugging
            let existing: Vec<String> = sqlx::query_scalar(
                "SELECT callback_id FROM queue_items WHERE callback_id IS NOT NULL LIMIT 10",
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            tracing::warn!(
                callback_id = %callback_id,
                existing_callback_ids = ?existing,
                "No queue item found with callback_id"
            );
        }

        Ok(deleted)
    }
}
