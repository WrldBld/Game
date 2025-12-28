//! Domain Event Repository Port - Interface for persisting domain events
//!
//! This port abstracts event storage, allowing the infrastructure to provide
//! different implementations (SQLite, PostgreSQL, Redis, etc.)
//!
//! Note: This port works with DomainEvent at the boundary. Adapters are responsible
//! for converting to/from the wire format (AppEvent) for actual storage.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use wrldbldr_domain::DomainEvent;

/// Port for storing and retrieving domain events
#[async_trait]
pub trait DomainEventRepositoryPort: Send + Sync {
    /// Insert a new domain event into storage
    ///
    /// Returns the unique ID assigned to the event by the storage backend
    async fn insert(&self, event: &DomainEvent) -> Result<i64, DomainEventRepositoryError>;

    /// Fetch events since a given ID
    ///
    /// Returns events with ID > last_id, up to `limit` events, ordered by ID ascending
    async fn fetch_since(
        &self,
        last_id: i64,
        limit: u32,
    ) -> Result<Vec<(i64, DomainEvent, DateTime<Utc>)>, DomainEventRepositoryError>;
}

/// Errors that can occur when accessing the event repository
#[derive(Debug)]
pub enum DomainEventRepositoryError {
    /// Database or storage-level error
    StorageError(String),
    /// Serialization/deserialization error
    SerializationError(String),
    /// Conversion error (e.g., failed to parse IDs from stored format)
    ConversionError(String),
}

impl std::fmt::Display for DomainEventRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainEventRepositoryError::StorageError(msg) => {
                write!(f, "Event repository storage error: {}", msg)
            }
            DomainEventRepositoryError::SerializationError(msg) => {
                write!(f, "Event serialization error: {}", msg)
            }
            DomainEventRepositoryError::ConversionError(msg) => {
                write!(f, "Event conversion error: {}", msg)
            }
        }
    }
}

impl std::error::Error for DomainEventRepositoryError {}
