//! Event Bus Port - Interface for publishing domain events
//!
//! This port abstracts the event bus, allowing the application to publish
//! domain events without knowing the underlying transport (in-process, SQLite, Redis, etc.)

use async_trait::async_trait;
use wrldbldr_domain::DomainEvent;

/// Port for publishing domain events
#[async_trait]
pub trait EventBusPort: Send + Sync {
    /// Publish a domain event to the bus
    ///
    /// This is a best-effort operation; failures should be logged but typically
    /// should not break the main application flow.
    async fn publish(&self, event: DomainEvent) -> Result<(), EventBusError>;
}

/// Errors that can occur when publishing events
#[derive(Debug)]
pub enum EventBusError {
    /// Transport-level error (e.g., database write failure, network issue)
    Transport(String),
}

impl std::fmt::Display for EventBusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventBusError::Transport(msg) => write!(f, "Event bus transport error: {}", msg),
        }
    }
}

impl std::error::Error for EventBusError {}
