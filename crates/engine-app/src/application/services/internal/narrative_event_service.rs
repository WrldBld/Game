//! Narrative event service port - Interface for narrative event operations
//!
//! This port abstracts narrative event business logic from infrastructure adapters.
//! It provides a subset of the full NarrativeEventService interface, exposing only
//! the methods that adapters actually need for their operations.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::{FeaturedNpc, NarrativeEvent};
use wrldbldr_domain::{NarrativeEventId, WorldId};

/// Port for narrative event service operations
///
/// This port exposes narrative event operations needed by infrastructure adapters:
/// - Fetching events by ID for DM actions (TriggerEvent)
/// - Listing pending events for prompt context building
/// - Getting featured NPCs for narrative context
/// - Marking events as triggered after DM approval
#[async_trait]
pub trait NarrativeEventServicePort: Send + Sync {
    /// Get a narrative event by ID
    ///
    /// Used by DM action processing to load event details before triggering.
    async fn get(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>>;

    /// List pending (not yet triggered) narrative events for a world
    ///
    /// Used by prompt building to include active narrative events in LLM context.
    async fn list_pending(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// Mark a narrative event as triggered
    ///
    /// Used by DM action processing when a TriggerEvent action is approved.
    /// Returns true if the event was successfully marked as triggered.
    async fn mark_triggered(
        &self,
        id: NarrativeEventId,
        outcome_name: Option<String>,
    ) -> Result<bool>;

    /// Get featured NPCs for a narrative event
    ///
    /// Used by prompt building to include featured NPC names in context.
    async fn get_featured_npcs(&self, event_id: NarrativeEventId) -> Result<Vec<FeaturedNpc>>;
}
