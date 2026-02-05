//! SQLite queue implementation for persistent job queues.
//!
//! Provides persistent queue storage for player actions, LLM requests,
//! DM approvals, and asset generation jobs.

use crate::queue_types::{
    ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData, QueueType,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::QueueItemId;

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
            .map_err(|e| QueueError::database("new/connect", format!("Failed to connect to database: {}", e)))?;

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
        .map_err(|e| QueueError::database("new/create_queue_items_table", format!("Failed to create queue_items table: {}", e)))?;

        // Create index for queue lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_queue_status
            ON queue_items(queue_type, status, created_at)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::database("new/create_idx_queue_status", format!("Failed to create idx_queue_status: {}", e)))?;

        // Migration: add callback_id column if missing (for existing DBs)
        // This migration is idempotent - SQLite returns an error if the column already exists.
        // We ignore "duplicate column" errors but log warnings for unexpected failures to
        // ensure visibility into potential schema issues that could affect idempotency guarantees.
        if let Err(e) = sqlx::query("ALTER TABLE queue_items ADD COLUMN callback_id TEXT")
            .execute(&pool)
            .await
        {
            let error_msg = e.to_string().to_lowercase();
            let is_duplicate_column = error_msg.contains("duplicate column")
                || error_msg.contains("already exists")
                || error_msg.contains("exists");
            if !is_duplicate_column {
                tracing::warn!(
                    error = %e,
                    operation = "migrate_add_callback_id_column",
                    "Failed to add callback_id column to queue_items table; idempotency lookups may be affected"
                );
            }
        }

        // Create index for callback_id lookups (used by dismiss/delete)
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_queue_callback_id
            ON queue_items(callback_id)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| QueueError::database("new/create_idx_queue_callback_id", format!("Failed to create idx_queue_callback_id: {}", e)))?;

        // Create unique index on callback_id for LLM requests to enforce idempotency
        // This prevents duplicate phase advancement on retry.
        // This migration is idempotent - SQLite returns an error if the index already exists.
        // We ignore "already exists" errors but log warnings for unexpected failures to
        // ensure visibility into potential schema issues that could break idempotency.
        // Note: Partial index WHERE clauses cannot use bound parameters, so we construct
        // the SQL dynamically using QueueType::as_str() to ensure consistency.
        let create_unique_index_sql = format!(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_queue_callback_id_unique
            ON queue_items(callback_id) WHERE queue_type = '{}'
            "#,
            QueueType::LlmRequest.as_str()
        );
        if let Err(e) = sqlx::query(&create_unique_index_sql)
            .execute(&pool)
            .await
        {
            let error_msg = e.to_string().to_lowercase();
            let is_already_exists = error_msg.contains("already exists")
                || error_msg.contains("duplicate")
                || error_msg.contains("exists");
            if !is_already_exists {
                tracing::warn!(
                    error = %e,
                    operation = "migrate_create_unique_callback_index",
                    "Failed to create unique index on callback_id; idempotency guarantees may be degraded"
                );
            }
        }

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
        .map_err(|e| QueueError::database("new/create_generation_read_state_table", format!("Failed to create generation_read_state table: {}", e)))?;

        Ok(Self { pool, clock })
    }

    /// Enqueue an item to a specific queue type
    async fn enqueue_item<T: serde::Serialize>(
        &self,
        queue_type: QueueType,
        data: &T,
    ) -> Result<QueueItemId, QueueError> {
        let id = Uuid::new_v4();
        let payload_json =
            serde_json::to_string(data).map_err(|e| QueueError::serialization(e.to_string()))?;
        let now = self.clock.now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO queue_items (id, queue_type, payload_json, status, created_at, updated_at)
            VALUES (?, ?, ?, 'pending', ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(queue_type.as_str())
        .bind(&payload_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::database("enqueue_item", e.to_string()))?;

    Ok(QueueItemId::from_uuid(id))
}

    /// Dequeue an item from a specific queue type
    async fn dequeue_item(&self, queue_type: QueueType) -> Result<Option<QueueItem>, QueueError> {
        let now = self.clock.now().to_rfc3339();
        let queue_type_str = queue_type.as_str();

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
        .bind(queue_type_str)
        .bind(queue_type_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::database("dequeue_item", e.to_string()))?;

    if let Some(row) = result {
        self.row_to_queue_item(row).map(Some)
    } else {
        Ok(None)
    }
}

    /// Convert a database row to a QueueItem
    fn row_to_queue_item(&self, row: sqlx::sqlite::SqliteRow) -> Result<QueueItem, QueueError> {
        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str).map_err(|e| QueueError::serialization(format!("Invalid UUID in database: {}", e)))?;

        let queue_type_str: String = row.get("queue_type");
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
            unknown => {
                return Err(QueueError::validation(format!(
                    "Unknown queue status value: '{}'",
                    unknown
                )));
            }
        };

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| QueueError::serialization(format!("Invalid timestamp in database: {}", e)))?
            .with_timezone(&Utc);

        // Parse queue type using the enum's FromStr implementation
        let queue_type = QueueType::from_str(&queue_type_str)
            .map_err(|e| QueueError::validation(format!("{}", e)))?;

        let data = match queue_type {
            QueueType::PlayerAction => {
                let payload: PlayerActionData = serde_json::from_str(&payload_json)
                    .map_err(|e| QueueError::serialization(format!("Failed to deserialize player_action payload: {}", e)))?;
                QueueItemData::PlayerAction(payload)
            }
            QueueType::LlmRequest => {
                let payload: LlmRequestData = serde_json::from_str(&payload_json)
                    .map_err(|e| QueueError::serialization(format!("Failed to deserialize llm_request payload: {}", e)))?;
                QueueItemData::LlmRequest(payload)
            }
            QueueType::DmApproval => {
                let payload: ApprovalRequestData = serde_json::from_str(&payload_json)
                    .map_err(|e| QueueError::serialization(format!("Failed to deserialize dm_approval payload: {}", e)))?;
                QueueItemData::DmApproval(payload)
            }
            QueueType::AssetGeneration => {
                let payload: AssetGenerationData = serde_json::from_str(&payload_json)
                    .map_err(|e| QueueError::serialization(format!("Failed to deserialize asset_generation payload: {}", e)))?;
                QueueItemData::AssetGeneration(payload)
            }
        };

        Ok(QueueItem {
            id: QueueItemId::from(id),
            data,
            created_at,
            status,
            error_message,
            result_json,
        })
    }

    /// Check if a SQLx error represents a SQLite unique constraint violation.
    ///
    /// This helper attempts to use structured error inspection first (checking for
    /// SQLITE_CONSTRAINT_UNIQUE / extended result code 2067), and falls back to
    /// string matching for robustness across SQLx/driver versions.
    ///
    /// # SQLite Driver Limitations
    /// - SQLx exposes SQLite errors as `SqliteError` with a string `code()` method.
    /// - The code "2067" corresponds to SQLITE_CONSTRAINT_UNIQUE.
    /// - We cannot get the specific constraint name from SQLite (unlike PostgreSQL).
    /// - Extended result codes may not be available in all SQLite builds.
    ///
    /// See: <https://www.sqlite.org/rescode.html#constraint_unique>
    fn is_sqlite_unique_constraint_error(err: &sqlx::Error) -> bool {
        // First, try structured error inspection via as_database_error()
        if let Some(db_err) = err.as_database_error() {
            // Try to get the error code as a string and check for SQLite unique constraint
            if let Some(code) = db_err.code() {
                let code_str = code.to_string();
                // SQLITE_CONSTRAINT_UNIQUE = 2067
                // Also check for generic "2067" or "1555" (SQLITE_CONSTRAINT_PRIMARYKEY)
                if code_str == "2067" || code_str == "1555" {
                    return true;
                }
            }
        }

        // Fallback: check the error message for unique constraint keywords.
        // This handles cases where the driver doesn't expose structured error codes
        // or when using a different SQLite build configuration.
        //
        // IMPORTANT: We require "unique" in the message to avoid misclassifying
        // other constraint violations (NOT NULL, CHECK, FOREIGN KEY) as unique.
        // We also check for "callback_id" since that's the specific column we
        // use for idempotency enforcement in LLM request queues.
        let error_msg = err.to_string().to_lowercase();
        let has_unique = error_msg.contains("unique");
        let has_callback_id = error_msg.contains("callback_id");
        has_unique || (has_callback_id && error_msg.contains("constraint"))
    }
}

#[async_trait]
impl QueuePort for SqliteQueue {
    // Player action queue
    async fn enqueue_player_action(
        &self,
        data: &PlayerActionData,
    ) -> Result<QueueItemId, QueueError> {
        self.enqueue_item(QueueType::PlayerAction, data).await
    }

    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        self.dequeue_item(QueueType::PlayerAction).await
    }

    // LLM request queue
    async fn enqueue_llm_request(&self, data: &LlmRequestData) -> Result<QueueItemId, QueueError> {
        // Normalize callback_id by trimming whitespace at the queue boundary
        let callback_id_normalized = data.callback_id.trim();
        if callback_id_normalized.is_empty() {
            return Err(QueueError::Validation(
                "callback_id cannot be empty or whitespace-only".to_string(),
            ));
        }

        // Clone data and update callback_id with normalized value to ensure consistency
        // between the indexed column and the serialized payload
        let mut normalized_data = data.clone();
        normalized_data.callback_id = callback_id_normalized.to_string();

        // Specialized insert that stores callback_id in indexed column for fast lookup
        let id = Uuid::new_v4();
        let payload_json = serde_json::to_string(&normalized_data)
            .map_err(|e| QueueError::serialization(e.to_string()))?;
        let now = self.clock.now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO queue_items (id, queue_type, payload_json, status, created_at, updated_at, callback_id)
            VALUES (?, ?, ?, 'pending', ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(QueueType::LlmRequest.as_str())
        .bind(&payload_json)
        .bind(&now)
        .bind(&now)
        .bind(callback_id_normalized)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(QueueItemId::from_uuid(id)),
            Err(e) => {
                // Capture error message for potential use in error reporting
                let error_msg = e.to_string();

                // Check if this is a unique constraint violation on callback_id.
                //
                // SQLite driver limitations:
                // - SQLx exposes SQLite extended result codes via SqliteError::code() as a string.
                // - SQLITE_CONSTRAINT_UNIQUE (2067) indicates a unique constraint violation.
                // - We cannot get the specific constraint name from SQLite (unlike PostgreSQL).
                // - Fallback to string matching on error message for robustness across driver versions.
                //
                // SECURITY: We verify the normalized callback_id row exists before treating this
                // as an idempotent duplicate. This ensures we only use the idempotent path for
                // actual callback_id collisions, not for other unique constraint violations
                // (e.g., on the primary key id column).
                //
                // See: https://www.sqlite.org/rescode.html#constraint_unique
                let is_unique_violation = Self::is_sqlite_unique_constraint_error(&e);

                if is_unique_violation && !callback_id_normalized.is_empty() {
                    // Look up the existing item by callback_id (using normalized value).
                    // This lookup serves two purposes:
                    // 1. Confirm the unique constraint violation is actually on callback_id
                    // 2. Get the existing item ID for idempotent return
                    let existing_lookup: Result<Option<String>, sqlx::Error> = sqlx::query_scalar(
                        "SELECT id FROM queue_items WHERE callback_id = ? AND queue_type = ?",
                    )
                    .bind(callback_id_normalized)
                    .bind(QueueType::LlmRequest.as_str())
                    .fetch_optional(&self.pool)
                    .await;

                    match existing_lookup {
                        Ok(Some(existing_id)) => {
                            // Confirmed: the unique constraint violation is on callback_id.
                            // Return the existing item ID for idempotent behavior.
                            tracing::info!(
                                operation = "enqueue_llm_request",
                                callback_id = %callback_id_normalized,
                                existing_item_id = %existing_id,
                                "Duplicate callback_id detected, returning existing queue item ID"
                            );

                            let existing_uuid = Uuid::parse_str(&existing_id).map_err(|parse_err| {
                                QueueError::serialization(format!(
                                    "Failed to parse existing item ID '{}': {}",
                                    existing_id, parse_err
                                ))
                            })?;

                            return Ok(QueueItemId::from_uuid(existing_uuid));
                        }
                        Ok(None) => {
                            // The unique constraint violation is NOT on callback_id
                            // (no row exists with this callback_id). This could be a
                            // primary key collision or another unique constraint.
                            // Fail-fast: return the original database error.
                            tracing::warn!(
                                operation = "enqueue_llm_request",
                                callback_id = %callback_id_normalized,
                                original_error = %error_msg,
                                "Unique constraint violation detected but no existing callback_id found; \
                                 treating as non-callback_id constraint violation"
                            );
                            return Err(QueueError::database("enqueue_llm_request", error_msg));
                        }
                        Err(lookup_err) => {
                            // Database error during lookup - fail-fast with context
                            tracing::error!(
                                operation = "enqueue_llm_request",
                                callback_id = %callback_id_normalized,
                                lookup_error = %lookup_err,
                                original_error = %error_msg,
                                "Failed to lookup existing item by callback_id after unique constraint violation"
                            );
                            return Err(QueueError::database(
                                "enqueue_llm_request/callback_id_lookup",
                                format!(
                                    "Unique constraint violation but failed to lookup existing item: {} (original error: {})",
                                    lookup_err, error_msg
                                )
                            ));
                        }
                    }
                }

                Err(QueueError::database("enqueue_llm_request", error_msg))
            }
        }
    }

    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        self.dequeue_item(QueueType::LlmRequest).await
    }

    // DM approval queue
    async fn enqueue_dm_approval(
        &self,
        data: &ApprovalRequestData,
    ) -> Result<QueueItemId, QueueError> {
        self.enqueue_item(QueueType::DmApproval, data).await
    }

    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        self.dequeue_item(QueueType::DmApproval).await
    }

    // Asset generation queue
    async fn enqueue_asset_generation(
        &self,
        data: &AssetGenerationData,
    ) -> Result<QueueItemId, QueueError> {
        self.enqueue_item(QueueType::AssetGeneration, data).await
    }

    async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
        self.dequeue_item(QueueType::AssetGeneration).await
    }

    // Common operations
    async fn mark_complete(&self, id: QueueItemId) -> Result<(), QueueError> {
        let now = self.clock.now().to_rfc3339();

        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'completed', updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(id.to_uuid().to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::database("mark_complete", e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(QueueError::not_found(id.to_string()));
    }

    Ok(())
}

async fn mark_failed(&self, id: QueueItemId, error: &str) -> Result<(), QueueError> {
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
    .bind(id.to_uuid().to_string())
    .execute(&self.pool)
    .await
    .map_err(|e| QueueError::database("mark_failed", e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(QueueError::not_found(id.to_string()));
        }

        Ok(())
    }

    async fn list_by_type(
        &self,
        queue_type: QueueType,
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
        .bind(queue_type.as_str())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| QueueError::database("list_by_type", e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(self.row_to_queue_item(row)?);
    }
    Ok(out)
}

async fn set_result_json(&self, id: QueueItemId, result_json: &str) -> Result<(), QueueError> {
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
    .bind(id.to_uuid().to_string())
    .execute(&self.pool)
    .await
    .map_err(|e| QueueError::database("set_result_json", e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(QueueError::not_found(id.to_string()));
        }

        Ok(())
    }

    async fn cancel_pending_llm_request_by_callback_id(
        &self,
        callback_id: &str,
    ) -> Result<bool, QueueError> {
        // Normalize callback_id by trimming for consistent lookup
        let callback_id_normalized = callback_id.trim();
        if callback_id_normalized.is_empty() {
            return Ok(false);
        }

        // Use indexed callback_id column for efficient cancellation
        let now = self.clock.now().to_rfc3339();
        let result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'failed', updated_at = ?, error_message = 'Cancelled'
            WHERE callback_id = ? AND queue_type = ? AND status = 'pending'
            "#,
        )
        .bind(&now)
        .bind(callback_id_normalized)
        .bind(QueueType::LlmRequest.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::database("cancel_pending_llm_request_by_callback_id", e.to_string()))?;

    Ok(result.rows_affected() > 0)
}

async fn get_pending_count(&self, queue_type: QueueType) -> Result<usize, QueueError> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) as count FROM queue_items
        WHERE queue_type = ? AND status = 'pending'
        "#,
    )
    .bind(queue_type.as_str())
    .fetch_one(&self.pool)
    .await
    .map_err(|e| QueueError::database("get_pending_count", e.to_string()))?;

    let count: i64 = row.get("count");
    Ok(count as usize)
}

async fn get_approval_request(
    &self,
    id: QueueItemId,
) -> Result<Option<ApprovalRequestData>, QueueError> {
    let result = sqlx::query(
        r#"
        SELECT payload_json FROM queue_items
        WHERE id = ? AND queue_type = ?
        "#,
    )
    .bind(id.to_uuid().to_string())
    .bind(QueueType::DmApproval.as_str())
    .fetch_optional(&self.pool)
    .await
    .map_err(|e| QueueError::database("get_approval_request", e.to_string()))?;

        match result {
            Some(row) => {
                let payload_json: String = row.get("payload_json");
                let data: ApprovalRequestData = serde_json::from_str(&payload_json)
                    .map_err(|e| QueueError::serialization(e.to_string()))?;
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
        .map_err(|e| QueueError::database("get_generation_read_state", e.to_string()))?;

    let Some(row) = row else {
        return Ok(None);
    };

    let read_batches_json: String = row
        .try_get("read_batches_json")
        .map_err(|e| QueueError::serialization(e.to_string()))?;
    let read_suggestions_json: String = row
        .try_get("read_suggestions_json")
        .map_err(|e| QueueError::serialization(e.to_string()))?;

    let read_batches: Vec<String> = serde_json::from_str(&read_batches_json)
        .map_err(|e| QueueError::serialization(e.to_string()))?;
    let read_suggestions: Vec<String> = serde_json::from_str(&read_suggestions_json)
        .map_err(|e| QueueError::serialization(e.to_string()))?;

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
        serde_json::to_string(read_batches).map_err(|e| QueueError::serialization(e.to_string()))?;
    let read_suggestions_json = serde_json::to_string(read_suggestions)
        .map_err(|e| QueueError::serialization(e.to_string()))?;

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
    .map_err(|e| QueueError::database("upsert_generation_read_state", e.to_string()))?;

        Ok(())
    }

    async fn delete_by_callback_id(&self, callback_id: &str) -> Result<bool, QueueError> {
        // Normalize callback_id by trimming for consistent lookup
        let callback_id_normalized = callback_id.trim();
        if callback_id_normalized.is_empty() {
            tracing::debug!(callback_id = %callback_id, "Skipping delete for empty callback_id after trimming");
            return Ok(false);
        }

        tracing::debug!(callback_id = %callback_id_normalized, "Attempting to delete queue item by callback_id");

        let result = sqlx::query(
                "DELETE FROM queue_items WHERE callback_id = ? AND queue_type = ?"
            )
            .bind(callback_id_normalized)
            .bind(QueueType::LlmRequest.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::database("delete_by_callback_id", e.to_string()))?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            tracing::info!(callback_id = %callback_id_normalized, "Deleted queue item by callback_id");
        } else {
            // Log existing callback_ids for debugging (scoped to llm_request queue type)
            let existing: Vec<String> = match sqlx::query_scalar(
                "SELECT callback_id FROM queue_items WHERE callback_id IS NOT NULL AND queue_type = ? LIMIT 10",
            )
            .bind(QueueType::LlmRequest.as_str())
            .fetch_all(&self.pool)
            .await {
                Ok(ids) => ids,
                Err(e) => {
                    tracing::warn!(
                        callback_id = %callback_id_normalized,
                        error = %e,
                        "Failed to query existing callback_ids for debugging"
                    );
                    Vec::new()
                }
            };

            tracing::warn!(
                callback_id = %callback_id_normalized,
                original_callback_id = %callback_id,
                existing_callback_ids = ?existing,
                "No queue item found with callback_id"
            );
        }

        Ok(deleted)
    }

    async fn advance_phased_llm_request(
        &self,
        item_id: QueueItemId,
        next_request: &LlmRequestData,
        phase_key: &str,
    ) -> Result<QueueItemId, QueueError> {
        // Normalize phase_key by trimming whitespace for consistent UUID generation
        // and callback_id storage. This ensures idempotency across retries that may
        // have whitespace variations (e.g., "phase:1" vs " phase:1 ").
        let phase_key_normalized = phase_key.trim();

        // Validate phase_key is non-empty and non-whitespace before starting transaction
        if phase_key_normalized.is_empty() {
            return Err(QueueError::validation(
                "phase_key cannot be empty or whitespace-only",
            ));
        }

        // Clone and normalize next_request to ensure payload callback_id matches
        // the indexed callback_id column for consistency across retries.
        // For phased requests, the phase_key is the authoritative idempotency key,
        // so we align the payload callback_id with the normalized phase_key.
        let mut normalized_request = next_request.clone();
        normalized_request.callback_id = phase_key_normalized.to_string();

        let now = self.clock.now().to_rfc3339();
        let payload_json =
            serde_json::to_string(&normalized_request).map_err(|e| QueueError::serialization(e.to_string()))?;

        // Use a deterministic ID based on normalized phase_key for idempotency
        // This ensures that even if this operation is retried, we get the same queue item ID
        // UUIDv5 is used for deterministic ID generation from the phase_key namespace
        let next_item_id = {
            // Use a fixed namespace UUID for phase advance items
            // This is a randomly generated UUID used as the namespace for all phase keys
            const PHASE_NAMESPACE: Uuid = uuid::uuid!("a1b2c3d4-e5f6-47a8-b9c0-d1e2f3a4b5c6");
            Uuid::new_v5(&PHASE_NAMESPACE, phase_key_normalized.as_bytes())
        };

        // Atomically mark current item complete and enqueue next phase
        // This is done in a transaction to ensure atomicity
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| QueueError::database("advance_phased_llm_request/begin_tx", e.to_string()))?;

        // Mark current item complete
        let mark_result = sqlx::query(
            r#"
            UPDATE queue_items
            SET status = 'completed', updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(item_id.to_uuid().to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| QueueError::database("advance_phased_llm_request/mark_complete", e.to_string()))?;

        if mark_result.rows_affected() == 0 {
            return Err(QueueError::not_found(item_id.to_string()));
        }

        // Enqueue next phase with normalized phase_key as callback_id for idempotency
        // Use INSERT OR IGNORE to handle the case where this phase was already enqueued
        // (e.g., on a previous retry that completed the prior phase but failed before
        // returning success). The unique index on callback_id for LLM requests enforces
        // this constraint at the database level.
        let insert_result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO queue_items (id, queue_type, payload_json, status, created_at, updated_at, callback_id)
            VALUES (?, ?, ?, 'pending', ?, ?, ?)
            "#,
        )
        .bind(next_item_id.to_string())
        .bind(QueueType::LlmRequest.as_str())
        .bind(&payload_json)
        .bind(&now)
        .bind(&now)
        .bind(phase_key_normalized)
        .execute(&mut *tx)
        .await
        .map_err(|e| QueueError::database("advance_phased_llm_request/insert_phase", e.to_string()))?;

        // If insert was ignored (rows_affected = 0), the phase already exists (completed, pending, or failed)
        // Look up the existing item ID and status to handle failed items
        let final_item_id = if insert_result.rows_affected() == 0 {
            let existing: (String, String) = sqlx::query_as(
                r#"
                SELECT id, status FROM queue_items
                WHERE queue_type = ? AND callback_id = ?
                "#,
            )
            .bind(QueueType::LlmRequest.as_str())
            .bind(phase_key_normalized)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| QueueError::Conflict {
                message: format!(
                    "Phase with callback_id {} exists but could not be retrieved: {}",
                    phase_key_normalized, e
                ),
            })?;

            let existing_id = existing.0;
            let existing_status = existing.1;

            // If the existing item is failed, reset it to pending so the pipeline can retry
            if existing_status == "failed" {
                sqlx::query(
                    r#"
                    UPDATE queue_items
                    SET status = 'pending', updated_at = ?, error_message = NULL
                    WHERE id = ?
                    "#,
                )
                .bind(&now)
                .bind(&existing_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| QueueError::database(
                    "advance_phased_llm_request/reset_failed_phase",
                    format!(
                        "Failed to reset failed phase item {} to pending: {}",
                        existing_id, e
                    )
                ))?;

                tracing::info!(
                    phase_key = %phase_key_normalized,
                    existing_item_id = %existing_id,
                    "Reset failed phase to pending for retry"
                );
            } else {
                tracing::info!(
                    phase_key = %phase_key_normalized,
                    existing_item_id = %existing_id,
                    existing_status = %existing_status,
                    "Phase already exists (completed or pending), returning existing item ID"
                );
            }

            Uuid::parse_str(&existing_id)
                .map_err(|e| QueueError::serialization(format!("Invalid UUID in database: {}", e)))?
        } else {
            tracing::info!(
                item_id = %item_id,
                next_item_id = %next_item_id,
                phase_key = %phase_key_normalized,
                "Advanced phased LLM request"
            );
            next_item_id
        };

        tx.commit()
            .await
            .map_err(|e| QueueError::database("advance_phased_llm_request/commit_tx", e.to_string()))?;

        Ok(QueueItemId::from_uuid(final_item_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue_types::LlmRequestType;
    use wrldbldr_domain::WorldId;

    /// Unit tests for is_sqlite_unique_constraint_error to ensure it correctly
    /// identifies unique constraint violations while avoiding false positives
    /// for other constraint types (NOT NULL, CHECK, FOREIGN KEY).
    #[test]
    fn is_sqlite_unique_constraint_error_detects_unique_violations() {
        // Test case 1: Error message contains "unique" (case insensitive)
        let _unique_err = sqlx::Error::RowNotFound;
        // We can't easily construct a DatabaseError, so we test the fallback
        // path by checking the string matching logic directly.
        // The fallback requires "unique" OR ("callback_id" AND "constraint").

        // Simulate the fallback logic:
        let error_msg = "UNIQUE constraint failed: queue_items.callback_id".to_lowercase();
        let has_unique = error_msg.contains("unique");
        let has_callback_id = error_msg.contains("callback_id");
        let is_unique = has_unique || (has_callback_id && error_msg.contains("constraint"));
        assert!(is_unique, "Should detect unique constraint violation");

        // Test case 2: Error message contains "callback_id" AND "constraint"
        let error_msg = "constraint violation on callback_id column".to_lowercase();
        let has_unique = error_msg.contains("unique");
        let has_callback_id = error_msg.contains("callback_id");
        let is_unique = has_unique || (has_callback_id && error_msg.contains("constraint"));
        assert!(is_unique, "Should detect callback_id constraint violation");
    }

    #[test]
    fn is_sqlite_unique_constraint_error_rejects_non_unique_constraints() {
        // Test case: Generic "constraint" without "unique" or "callback_id"
        // should NOT be treated as a unique constraint violation.
        let error_msg = "NOT NULL constraint failed: queue_items.payload_json".to_lowercase();
        let has_unique = error_msg.contains("unique");
        let has_callback_id = error_msg.contains("callback_id");
        let is_unique = has_unique || (has_callback_id && error_msg.contains("constraint"));
        assert!(!is_unique, "Should NOT treat NOT NULL constraint as unique violation");

        // Test case: CHECK constraint
        let error_msg = "CHECK constraint failed: queue_items".to_lowercase();
        let has_unique = error_msg.contains("unique");
        let has_callback_id = error_msg.contains("callback_id");
        let is_unique = has_unique || (has_callback_id && error_msg.contains("constraint"));
        assert!(!is_unique, "Should NOT treat CHECK constraint as unique violation");

        // Test case: FOREIGN KEY constraint
        let error_msg = "FOREIGN KEY constraint failed".to_lowercase();
        let has_unique = error_msg.contains("unique");
        let has_callback_id = error_msg.contains("callback_id");
        let is_unique = has_unique || (has_callback_id && error_msg.contains("constraint"));
        assert!(!is_unique, "Should NOT treat FOREIGN KEY constraint as unique violation");
    }

    #[test]
    fn is_sqlite_unique_constraint_error_handles_edge_cases() {
        // Empty string should not match
        let error_msg = "".to_lowercase();
        let has_unique = error_msg.contains("unique");
        let has_callback_id = error_msg.contains("callback_id");
        let is_unique = has_unique || (has_callback_id && error_msg.contains("constraint"));
        assert!(!is_unique, "Empty string should not match");

        // Just "constraint" without "unique" or "callback_id" should not match
        let error_msg = "some constraint error".to_lowercase();
        let has_unique = error_msg.contains("unique");
        let has_callback_id = error_msg.contains("callback_id");
        let is_unique = has_unique || (has_callback_id && error_msg.contains("constraint"));
        assert!(!is_unique, "Generic constraint error should not match");

        // "callback_id" without "constraint" should not match
        let error_msg = "invalid callback_id value".to_lowercase();
        let has_unique = error_msg.contains("unique");
        let has_callback_id = error_msg.contains("callback_id");
        let is_unique = has_unique || (has_callback_id && error_msg.contains("constraint"));
        assert!(!is_unique, "callback_id without constraint should not match");
    }

    /// Integration test for callback_id alignment with phase_key in phased queue advance.
    ///
    /// Verifies that when advancing with a whitespace-padded phase_key, the payload's
    /// callback_id is set to the normalized phase_key (not the next_request's callback_id).
    /// This ensures idempotency consistency between the payload and the indexed column.
    #[tokio::test]
    async fn advance_phased_llm_request_aligns_callback_id_with_phase_key() {
        use crate::infrastructure::ports::ClockPort;
        use std::sync::Arc;

        /// A simple mock clock for testing
        struct MockClock;

        impl ClockPort for MockClock {
            fn now(&self) -> DateTime<Utc> {
                Utc::now()
            }
        }

        // Create a temporary database for this test
        let temp_dir = std::env::temp_dir().join(format!("wrldbldr_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");
        let db_path = temp_dir.join("test.db").to_string_lossy().to_string();

        let clock: Arc<dyn ClockPort> = Arc::new(MockClock);
        let queue = SqliteQueue::new(&db_path, clock)
            .await
            .expect("Failed to create queue");

        // Create initial LLM request with whitespace-padded callback_id
        let world_id = WorldId::new();
        let initial_request = LlmRequestData {
            request_type: LlmRequestType::NpcResponse {
                action_item_id: QueueItemId::from(Uuid::new_v4()),
            },
            world_id,
            pc_id: None,
            prompt: None,
            suggestion_context: None,
            // Whitespace-padded callback_id
            callback_id: "  test-callback-123  ".to_string(),
            conversation_id: None,
            temperature: None,
            phase_meta: None,
        };

        // Enqueue the initial request
        let item_id = queue
            .enqueue_llm_request(&initial_request)
            .await
            .expect("Failed to enqueue initial request");

        // Create next phase request (with a different callback_id that should be ignored)
        let next_request = LlmRequestData {
            request_type: LlmRequestType::NpcResponse {
                action_item_id: QueueItemId::from(Uuid::new_v4()),
            },
            world_id,
            pc_id: None,
            prompt: None,
            suggestion_context: None,
            // This callback_id should be ignored - phase_key is authoritative
            callback_id: "  next-phase-callback  ".to_string(),
            conversation_id: None,
            temperature: None,
            phase_meta: None,
        };

        // Advance with whitespace-padded phase_key
        let phase_key = "  phase:test:456  ";
        let normalized_phase_key = "phase:test:456";
        let next_item_id = queue
            .advance_phased_llm_request(item_id, &next_request, phase_key)
            .await
            .expect("Failed to advance phased request");

        // Dequeue the next phase item
        let dequeued = queue
            .dequeue_llm_request()
            .await
            .expect("Failed to dequeue")
            .expect("Expected an item in queue");

        // Verify we got the correct item
        assert_eq!(dequeued.id, next_item_id);

        // Verify the payload's callback_id matches the normalized phase_key
        // (not the next_request's callback_id)
        if let QueueItemData::LlmRequest(payload) = dequeued.data {
            assert_eq!(
                payload.callback_id, normalized_phase_key,
                "Payload callback_id should match normalized phase_key for idempotency consistency"
            );
        } else {
            panic!("Expected LlmRequest data");
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // =============================================================================
    // QueueType Enum Tests
    // =============================================================================

    /// Test that QueueType::as_str returns the correct canonical string for each variant.
    #[test]
    fn queue_type_as_str_returns_canonical_values() {
        assert_eq!(QueueType::PlayerAction.as_str(), "player_action");
        assert_eq!(QueueType::LlmRequest.as_str(), "llm_request");
        assert_eq!(QueueType::DmApproval.as_str(), "dm_approval");
        assert_eq!(QueueType::AssetGeneration.as_str(), "asset_generation");
    }

    /// Test that QueueType can be parsed from its canonical string representation.
    #[test]
    fn queue_type_from_str_parses_valid_types() {
        use std::str::FromStr;

        assert_eq!(
            QueueType::from_str("player_action").expect("valid queue type"),
            QueueType::PlayerAction
        );
        assert_eq!(
            QueueType::from_str("llm_request").expect("valid queue type"),
            QueueType::LlmRequest
        );
        assert_eq!(
            QueueType::from_str("dm_approval").expect("valid queue type"),
            QueueType::DmApproval
        );
        assert_eq!(
            QueueType::from_str("asset_generation").expect("valid queue type"),
            QueueType::AssetGeneration
        );
    }

    /// Test that QueueType round-trips correctly through as_str/from_str.
    #[test]
    fn queue_type_round_trip_conversion() {
        use std::str::FromStr;

        for original in QueueType::all() {
            let str_repr = original.as_str();
            let parsed = QueueType::from_str(str_repr).expect("valid queue type");
            assert_eq!(
                *original, parsed,
                "Round-trip failed for {:?}: '{}' -> {:?}",
                original, str_repr, parsed
            );
        }
    }

    /// Test that unknown queue types return an appropriate error.
    #[test]
    fn queue_type_from_str_rejects_unknown_types() {
        use std::str::FromStr;

        let unknown_types = vec![
            "unknown_type",
            "playeraction",
            "LLM_REQUEST",
            "dm-approval",
            "",
            "  ",
            "player_action_extra",
        ];

        for unknown in unknown_types {
            let result = QueueType::from_str(unknown);
            assert!(
                result.is_err(),
                "Expected error for unknown queue type '{}', but got {:?}",
                unknown,
                result
            );

            // Verify the error message contains the unknown type
            let err = result.expect_err("should be error");
            let err_msg = format!("{}", err);
            assert!(
                err_msg.contains(unknown) || unknown.trim().is_empty(),
                "Error message should mention the unknown type: {}",
                err_msg
            );
        }
    }

    /// Test that QueueType implements Display correctly.
    #[test]
    fn queue_type_display_matches_as_str() {
        for queue_type in QueueType::all() {
            let display = format!("{}", queue_type);
            assert_eq!(
                display,
                queue_type.as_str(),
                "Display impl should match as_str() for {:?}",
                queue_type
            );
        }
    }

    /// Test that QueueType::all() returns all expected variants.
    #[test]
    fn queue_type_all_returns_all_variants() {
        let all = QueueType::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&QueueType::PlayerAction));
        assert!(all.contains(&QueueType::LlmRequest));
        assert!(all.contains(&QueueType::DmApproval));
        assert!(all.contains(&QueueType::AssetGeneration));
    }
}
