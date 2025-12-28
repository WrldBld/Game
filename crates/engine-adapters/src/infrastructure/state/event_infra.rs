//! Event infrastructure and repositories

use std::sync::Arc;

use wrldbldr_engine_ports::outbound::{DomainEventRepositoryPort, EventBusPort, GenerationReadStatePort};
use crate::infrastructure::event_bus::InProcessEventNotifier;

/// Event infrastructure for application-level events
///
/// This struct groups the event bus, event notification system, and
/// repositories for tracking application events and generation state.
pub struct EventInfrastructure {
    pub event_bus: Arc<dyn EventBusPort>,
    pub event_notifier: InProcessEventNotifier,
    pub domain_event_repository: Arc<dyn DomainEventRepositoryPort>,
    pub generation_read_state_repository: Arc<dyn GenerationReadStatePort>,
}

impl EventInfrastructure {
    /// Creates a new EventInfrastructure instance
    pub fn new(
        event_bus: Arc<dyn EventBusPort>,
        event_notifier: InProcessEventNotifier,
        domain_event_repository: Arc<dyn DomainEventRepositoryPort>,
        generation_read_state_repository: Arc<dyn GenerationReadStatePort>,
    ) -> Self {
        Self {
            event_bus,
            event_notifier,
            domain_event_repository,
            generation_read_state_repository,
        }
    }
}
