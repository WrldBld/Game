//! Event Infrastructure Container - Port-based abstraction for event system services
//!
//! This module provides `EventInfra`, a grouped structure for event bus, notification,
//! and event storage services using **port traits** from `wrldbldr-engine-ports`.
//!
//! # Architecture
//!
//! This struct groups all infrastructure services related to the domain event system,
//! including event publishing, notification dispatch, and event persistence.
//! All fields use port traits for clean hexagonal architecture boundaries.
//!
//! # Services Included
//!
//! - **Event Bus**: Domain event publishing and subscription
//! - **Event Notifier**: WebSocket notification dispatch to clients
//! - **Domain Event Repository**: Persistent storage for domain events
//! - **Generation Read State**: Read-side projection for generation status
//!
//! # Usage
//!
//! ```ignore
//! use wrldbldr_engine_composition::EventInfra;
//!
//! let event_infra = EventInfra::new(
//!     event_bus,
//!     event_notifier,
//!     domain_event_repository,
//!     generation_read_state_repository,
//! );
//!
//! // Publish a domain event
//! event_infra.event_bus.publish(domain_event).await?;
//! ```

use std::sync::Arc;

use wrldbldr_engine_ports::outbound::{
    DomainEventRepositoryPort, EventBusPort, EventNotifierPort, GenerationReadStatePort,
};

/// Container for event system infrastructure services.
///
/// This struct groups all infrastructure services that support the domain event
/// system, from event publishing through notification to persistence.
///
/// All fields are `Arc<dyn ...Port>` for:
/// - Shared ownership across handlers and workers
/// - Dynamic dispatch enabling mock injection for tests
/// - No generic type parameters for simpler composition
///
/// # Service Categories
///
/// ## Event Publishing
/// - `event_bus`: In-memory event bus for domain event distribution
///
/// ## Client Notification
/// - `event_notifier`: Dispatches events to connected WebSocket clients
///
/// ## Event Persistence
/// - `domain_event_repository`: Stores domain events for audit and replay
/// - `generation_read_state_repository`: Read-side projection for generation status
#[derive(Clone)]
pub struct EventInfra {
    /// Domain event bus for publish-subscribe communication.
    ///
    /// Enables loose coupling between components through domain events.
    /// Components can subscribe to specific event types and react accordingly.
    /// Events are distributed in-memory within the application.
    pub event_bus: Arc<dyn EventBusPort>,

    /// Event notifier for WebSocket client notifications.
    ///
    /// Dispatches relevant domain events to connected clients via WebSocket.
    /// Handles event filtering and transformation for client consumption.
    /// Manages world-scoped subscriptions for targeted notifications.
    pub event_notifier: Arc<dyn EventNotifierPort>,

    /// Repository for persistent domain event storage.
    ///
    /// Stores all domain events for audit trails, debugging, and potential
    /// event replay. Supports querying events by type, aggregate, and time range.
    pub domain_event_repository: Arc<dyn DomainEventRepositoryPort>,

    /// Read-side projection for generation status.
    ///
    /// Maintains materialized views of generation queue and batch status.
    /// Optimized for read queries to support UI status displays and
    /// generation progress tracking.
    pub generation_read_state_repository: Arc<dyn GenerationReadStatePort>,
}

impl EventInfra {
    /// Creates a new `EventInfra` instance with all event infrastructure services.
    ///
    /// # Arguments
    ///
    /// All arguments are `Arc<dyn ...Port>` to allow any implementation:
    ///
    /// * `event_bus` - For domain event publishing and subscription
    /// * `event_notifier` - For WebSocket client notification dispatch
    /// * `domain_event_repository` - For persistent event storage
    /// * `generation_read_state_repository` - For generation status projections
    ///
    /// # Example
    ///
    /// ```ignore
    /// let event_infra = EventInfra::new(
    ///     Arc::new(event_bus_impl) as Arc<dyn EventBusPort>,
    ///     Arc::new(notifier_impl) as Arc<dyn EventNotifierPort>,
    ///     Arc::new(event_repo_impl) as Arc<dyn DomainEventRepositoryPort>,
    ///     Arc::new(read_state_impl) as Arc<dyn GenerationReadStatePort>,
    /// );
    /// ```
    pub fn new(
        event_bus: Arc<dyn EventBusPort>,
        event_notifier: Arc<dyn EventNotifierPort>,
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

impl std::fmt::Debug for EventInfra {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventInfra")
            .field("event_bus", &"Arc<dyn EventBusPort>")
            .field("event_notifier", &"Arc<dyn EventNotifierPort>")
            .field(
                "domain_event_repository",
                &"Arc<dyn DomainEventRepositoryPort>",
            )
            .field(
                "generation_read_state_repository",
                &"Arc<dyn GenerationReadStatePort>",
            )
            .finish()
    }
}
