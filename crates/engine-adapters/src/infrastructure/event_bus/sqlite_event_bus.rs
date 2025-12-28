//! SQLite Event Bus - Persistent event bus backed by SQLite
//!
//! Publishes domain events to SQLite storage and triggers in-process notifications.
//! Uses DomainEventRepositoryPort which handles domain event storage directly.

use async_trait::async_trait;
use std::sync::Arc;

use wrldbldr_domain::DomainEvent;
use wrldbldr_engine_ports::outbound::{DomainEventRepositoryPort, EventBusError, EventBusPort};

use super::in_process_notifier::InProcessEventNotifier;

/// SQLite-backed event bus implementation
pub struct SqliteEventBus {
    repository: Arc<dyn DomainEventRepositoryPort>,
    notifier: InProcessEventNotifier,
}

impl SqliteEventBus {
    /// Create a new SQLite event bus
    pub fn new(
        repository: Arc<dyn DomainEventRepositoryPort>,
        notifier: InProcessEventNotifier,
    ) -> Self {
        Self {
            repository,
            notifier,
        }
    }
}

#[async_trait]
impl EventBusPort for SqliteEventBus {
    async fn publish(&self, event: DomainEvent) -> Result<(), EventBusError> {
        // Insert domain event directly - repository handles conversion to wire format
        self.repository
            .insert(&event)
            .await
            .map_err(|e| EventBusError::Transport(e.to_string()))?;

        // Best-effort notification (don't fail the whole operation if this fails)
        self.notifier.notify();

        Ok(())
    }
}

