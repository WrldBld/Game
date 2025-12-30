//! Query operations for EventChain entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{EventChain, NarrativeEventId, WorldId};

/// Query operations for finding event chains.
///
/// This trait covers lookup operations that return collections
/// of event chains based on various criteria.
#[async_trait]
pub trait EventChainQueryPort: Send + Sync {
    /// List all event chains for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// List active event chains for a world
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// List favorite event chains for a world
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// Get chains containing a specific narrative event
    async fn get_chains_for_event(&self, event_id: NarrativeEventId) -> Result<Vec<EventChain>>;
}
