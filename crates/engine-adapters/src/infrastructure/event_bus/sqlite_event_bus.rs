//! SQLite Event Bus - Persistent event bus backed by SQLite
//!
//! Publishes domain events to SQLite storage and triggers in-process notifications.
//! The adapter converts DomainEvent to AppEvent (wire format) for storage.

use async_trait::async_trait;
use std::sync::Arc;

use wrldbldr_domain::DomainEvent;
use wrldbldr_engine_ports::outbound::{
    AppEventRepositoryPort, EventBusError, EventBusPort,
};

use super::domain_event_mapper::domain_event_to_app_event;
use super::in_process_notifier::InProcessEventNotifier;

/// SQLite-backed event bus implementation
pub struct SqliteEventBus {
    repository: Arc<dyn AppEventRepositoryPort>,
    notifier: InProcessEventNotifier,
}

impl SqliteEventBus {
    /// Create a new SQLite event bus
    pub fn new(
        repository: Arc<dyn AppEventRepositoryPort>,
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
        // Convert DomainEvent to AppEvent for storage
        let app_event = domain_event_to_app_event(event);
        
        // Insert into storage
        self.repository
            .insert(&app_event)
            .await
            .map_err(|e| EventBusError::Transport(e.to_string()))?;

        // Best-effort notification (don't fail the whole operation if this fails)
        self.notifier.notify();

        Ok(())
    }
}

