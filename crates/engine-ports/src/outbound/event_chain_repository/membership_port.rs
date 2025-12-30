//! Event membership operations for EventChain entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{EventChainId, NarrativeEventId};

/// Event membership management for chains.
///
/// This trait covers adding, removing, and completing events
/// within an event chain.
#[async_trait]
pub trait EventChainMembershipPort: Send + Sync {
    /// Add an event to a chain
    async fn add_event_to_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<bool>;

    /// Remove an event from a chain
    async fn remove_event_from_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<bool>;

    /// Mark an event as completed in a chain
    async fn complete_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<bool>;
}
