//! Core CRUD operations for EventChain entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{EventChain, EventChainId};

/// Core CRUD operations for event chains.
///
/// This trait covers basic create, read, update, delete operations.
#[async_trait]
pub trait EventChainCrudPort: Send + Sync {
    /// Create a new event chain
    async fn create(&self, chain: &EventChain) -> Result<()>;

    /// Get an event chain by ID
    async fn get(&self, id: EventChainId) -> Result<Option<EventChain>>;

    /// Update an event chain
    async fn update(&self, chain: &EventChain) -> Result<bool>;

    /// Delete an event chain
    async fn delete(&self, id: EventChainId) -> Result<bool>;
}
