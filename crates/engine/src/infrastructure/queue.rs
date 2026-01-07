//! SQLite queue implementation for persistent job queues.
//!
//! Provides persistent queue storage for player actions, LLM requests,
//! DM approvals, and asset generation jobs.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData};

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
                result_json TEXT
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
        self.enqueue_item("llm_request", data).await
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
        // We don't have a dedicated callback_id column; scan a bounded set of pending items.
        let items = self.list_by_type("llm_request", 500).await?;
        let mut cancelled_any = false;

        for item in items {
            if item.status != QueueItemStatus::Pending {
                continue;
            }

            let QueueItemData::LlmRequest(req) = item.data else {
                continue;
            };

            if req.callback_id == callback_id {
                self.mark_failed(item.id, "Cancelled").await?;
                cancelled_any = true;
            }
        }

        Ok(cancelled_any)
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
}
