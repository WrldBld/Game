//! Featured NPC management for NarrativeEvent entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{CharacterId, EventChainMembership, FeaturedNpc, NarrativeEventId};

/// Featured NPC management for NarrativeEvent entities.
///
/// This trait manages the FEATURES_NPC edges and chain membership queries:
/// - Adding/removing NPCs featured in events
/// - Updating NPC roles in events
/// - Querying chain memberships (via CONTAINS_EVENT edges)
///
/// # Used By
/// - `NarrativeEventServiceImpl` - For managing featured NPCs
/// - `StagingService` - For determining which NPCs should appear
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait NarrativeEventNpcPort: Send + Sync {
    /// Add a featured NPC to the event (creates FEATURES_NPC edge)
    async fn add_featured_npc(
        &self,
        event_id: NarrativeEventId,
        featured_npc: FeaturedNpc,
    ) -> Result<bool>;

    /// Get all featured NPCs for an event
    async fn get_featured_npcs(&self, event_id: NarrativeEventId) -> Result<Vec<FeaturedNpc>>;

    /// Remove a featured NPC from the event (deletes FEATURES_NPC edge)
    async fn remove_featured_npc(
        &self,
        event_id: NarrativeEventId,
        character_id: CharacterId,
    ) -> Result<bool>;

    /// Update featured NPC role
    async fn update_featured_npc_role(
        &self,
        event_id: NarrativeEventId,
        character_id: CharacterId,
        role: Option<String>,
    ) -> Result<bool>;

    /// Get chain membership info for an event (queries CONTAINS_EVENT edges pointing to this event)
    async fn get_chain_memberships(
        &self,
        event_id: NarrativeEventId,
    ) -> Result<Vec<EventChainMembership>>;
}
