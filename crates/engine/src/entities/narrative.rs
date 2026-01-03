//! Narrative entity operations.

use std::sync::Arc;
use wrldbldr_domain::{
    self as domain, EventChainId, NarrativeEventId, RegionId, StoryEventId, WorldId,
};

use crate::infrastructure::ports::{NarrativeRepo, RepoError};

/// Narrative entity operations.
///
/// Handles narrative events, event chains, story events, and triggers.
pub struct Narrative {
    repo: Arc<dyn NarrativeRepo>,
}

impl Narrative {
    pub fn new(repo: Arc<dyn NarrativeRepo>) -> Self {
        Self { repo }
    }

    // =========================================================================
    // Narrative Events
    // =========================================================================

    pub async fn get_event(&self, id: NarrativeEventId) -> Result<Option<domain::NarrativeEvent>, RepoError> {
        self.repo.get_event(id).await
    }

    pub async fn save_event(&self, event: &domain::NarrativeEvent) -> Result<(), RepoError> {
        self.repo.save_event(event).await
    }

    pub async fn list_events(&self, world_id: WorldId) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        self.repo.list_events_for_world(world_id).await
    }

    // =========================================================================
    // Event Chains
    // =========================================================================

    pub async fn get_chain(&self, id: EventChainId) -> Result<Option<domain::EventChain>, RepoError> {
        self.repo.get_chain(id).await
    }

    pub async fn save_chain(&self, chain: &domain::EventChain) -> Result<(), RepoError> {
        self.repo.save_chain(chain).await
    }

    // =========================================================================
    // Story Events
    // =========================================================================

    pub async fn get_story_event(&self, id: StoryEventId) -> Result<Option<domain::StoryEvent>, RepoError> {
        self.repo.get_story_event(id).await
    }

    pub async fn save_story_event(&self, event: &domain::StoryEvent) -> Result<(), RepoError> {
        self.repo.save_story_event(event).await
    }

    pub async fn list_story_events(&self, world_id: WorldId, limit: usize) -> Result<Vec<domain::StoryEvent>, RepoError> {
        self.repo.list_story_events(world_id, limit).await
    }

    // =========================================================================
    // Triggers
    // =========================================================================

    pub async fn get_triggers_for_region(&self, region_id: RegionId) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        self.repo.get_triggers_for_region(region_id).await
    }

    /// Check for triggered events when entering a region.
    pub async fn check_triggers(
        &self,
        region_id: RegionId,
        _pc_id: wrldbldr_domain::PlayerCharacterId,
    ) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        // TODO: Evaluate trigger conditions
        self.get_triggers_for_region(region_id).await
    }
}
