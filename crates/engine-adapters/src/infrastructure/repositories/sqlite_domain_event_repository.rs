//! SQLite Domain Event Repository - Adapts DomainEvent <-> AppEvent for storage
//!
//! This adapter implements DomainEventRepositoryPort, converting domain events
//! to/from the wire format (AppEvent) for persistent storage.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use wrldbldr_domain::DomainEvent;
use wrldbldr_engine_ports::outbound::{ClockPort, DomainEventRepositoryError, DomainEventRepositoryPort};
use wrldbldr_protocol::AppEvent;

use crate::infrastructure::event_bus::domain_event_mapper::{
    app_event_to_domain_event, domain_event_to_app_event,
};

/// SQLite implementation of DomainEventRepositoryPort
///
/// Stores events as AppEvent JSON (wire format) but exposes DomainEvent at the boundary.
pub struct SqliteDomainEventRepository {
    pool: SqlitePool,
    clock: Arc<dyn ClockPort>,
}

impl SqliteDomainEventRepository {
    /// Create a new repository and ensure the table exists
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    /// * `clock` - Clock for time operations
    pub async fn new(pool: SqlitePool, clock: Arc<dyn ClockPort>) -> Result<Self, DomainEventRepositoryError> {
        // Create the app_events table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS app_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at TEXT NOT NULL,
                processed INTEGER DEFAULT 0,
                processed_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainEventRepositoryError::StorageError(e.to_string()))?;

        // Create index on id for efficient fetch_since queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_app_events_id_created 
            ON app_events(id, created_at)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| DomainEventRepositoryError::StorageError(e.to_string()))?;

        Ok(Self { pool, clock })
    }

    /// Expose underlying pool for related repositories that share the same DB.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl DomainEventRepositoryPort for SqliteDomainEventRepository {
    async fn insert(&self, event: &DomainEvent) -> Result<i64, DomainEventRepositoryError> {
        // Convert to wire format for storage
        let app_event = domain_event_to_app_event(event.clone());
        let event_type = app_event.event_type();
        let payload = serde_json::to_string(&app_event)
            .map_err(|e| DomainEventRepositoryError::SerializationError(e.to_string()))?;
        let created_at = self.clock.now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO app_events (event_type, payload, created_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(event_type)
        .bind(&payload)
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainEventRepositoryError::StorageError(e.to_string()))?;

        Ok(result.last_insert_rowid())
    }

    async fn fetch_since(
        &self,
        last_id: i64,
        limit: u32,
    ) -> Result<Vec<(i64, DomainEvent, DateTime<Utc>)>, DomainEventRepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT id, payload, created_at
            FROM app_events
            WHERE id > ?
            ORDER BY id ASC
            LIMIT ?
            "#,
        )
        .bind(last_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainEventRepositoryError::StorageError(e.to_string()))?;

        let mut events = Vec::new();
        for row in rows {
            let id: i64 = row.get("id");
            let payload: String = row.get("payload");
            let created_at_str: String = row.get("created_at");

            // Deserialize from wire format
            let app_event: AppEvent = serde_json::from_str(&payload)
                .map_err(|e| DomainEventRepositoryError::SerializationError(e.to_string()))?;

            // Convert to domain event
            let domain_event = app_event_to_domain_event(app_event)
                .map_err(|e| DomainEventRepositoryError::ConversionError(e.to_string()))?;

            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| {
                    DomainEventRepositoryError::StorageError(format!("Invalid timestamp: {}", e))
                })?
                .with_timezone(&Utc);

            events.push((id, domain_event, created_at));
        }

        Ok(events)
    }
}
