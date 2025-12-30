//! State and status operations for EventChain entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{ChainStatus, EventChainId, WorldId};

/// State and status management for event chains.
///
/// This trait covers toggling favorite, setting active status,
/// resetting progress, and querying chain statuses.
#[async_trait]
pub trait EventChainStatePort: Send + Sync {
    /// Toggle favorite status
    async fn toggle_favorite(&self, id: EventChainId) -> Result<bool>;

    /// Set active status
    async fn set_active(&self, id: EventChainId, is_active: bool) -> Result<bool>;

    /// Reset chain progress
    async fn reset(&self, id: EventChainId) -> Result<bool>;

    /// Get chain status summary
    async fn get_status(&self, id: EventChainId) -> Result<Option<ChainStatus>>;

    /// Get all chain statuses for a world
    async fn list_statuses(&self, world_id: WorldId) -> Result<Vec<ChainStatus>>;
}
