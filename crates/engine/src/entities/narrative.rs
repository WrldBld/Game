//! Narrative entity operations.

use std::sync::Arc;
use wrldbldr_domain::{
    self as domain, EventChainId, LocationId, NarrativeEventId, RegionId, StoryEventId,
    TriggerContext, WorldId,
};

use crate::infrastructure::ports::{
    LocationRepo, NarrativeRepo, ObservationRepo, PlayerCharacterRepo, RepoError,
};

/// Narrative entity operations.
///
/// Handles narrative events, event chains, story events, and triggers.
pub struct Narrative {
    repo: Arc<dyn NarrativeRepo>,
    location_repo: Arc<dyn LocationRepo>,
    player_character_repo: Arc<dyn PlayerCharacterRepo>,
    observation_repo: Arc<dyn ObservationRepo>,
}

impl Narrative {
    pub fn new(
        repo: Arc<dyn NarrativeRepo>,
        location_repo: Arc<dyn LocationRepo>,
        player_character_repo: Arc<dyn PlayerCharacterRepo>,
        observation_repo: Arc<dyn ObservationRepo>,
    ) -> Self {
        Self {
            repo,
            location_repo,
            player_character_repo,
            observation_repo,
        }
    }

    // =========================================================================
    // Narrative Events
    // =========================================================================

    pub async fn get_event(
        &self,
        id: NarrativeEventId,
    ) -> Result<Option<domain::NarrativeEvent>, RepoError> {
        self.repo.get_event(id).await
    }

    pub async fn save_event(&self, event: &domain::NarrativeEvent) -> Result<(), RepoError> {
        self.repo.save_event(event).await
    }

    pub async fn list_events(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        self.repo.list_events_for_world(world_id).await
    }

    // =========================================================================
    // Event Chains
    // =========================================================================

    pub async fn get_chain(
        &self,
        id: EventChainId,
    ) -> Result<Option<domain::EventChain>, RepoError> {
        self.repo.get_chain(id).await
    }

    pub async fn save_chain(&self, chain: &domain::EventChain) -> Result<(), RepoError> {
        self.repo.save_chain(chain).await
    }

    // =========================================================================
    // Story Events
    // =========================================================================

    pub async fn get_story_event(
        &self,
        id: StoryEventId,
    ) -> Result<Option<domain::StoryEvent>, RepoError> {
        self.repo.get_story_event(id).await
    }

    pub async fn save_story_event(&self, event: &domain::StoryEvent) -> Result<(), RepoError> {
        self.repo.save_story_event(event).await
    }

    pub async fn list_story_events(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<domain::StoryEvent>, RepoError> {
        self.repo.list_story_events(world_id, limit).await
    }

    // =========================================================================
    // Triggers
    // =========================================================================

    pub async fn get_triggers_for_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        self.repo.get_triggers_for_region(region_id).await
    }

    /// Check for triggered events when entering a region.
    ///
    /// Builds a TriggerContext with current location info and evaluates all
    /// candidate events using the domain's trigger evaluation logic.
    /// Returns events that evaluate as triggered, sorted by priority.
    pub async fn check_triggers(
        &self,
        region_id: RegionId,
        pc_id: wrldbldr_domain::PlayerCharacterId,
    ) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        // Get candidate events for this region
        let candidates = self.get_triggers_for_region(region_id).await?;

        if candidates.is_empty() {
            return Ok(vec![]);
        }

        // Get the region's location for context
        let location_id: Option<LocationId> = self
            .location_repo
            .get_region(region_id)
            .await?
            .map(|region| region.location_id);

        // Get PC's inventory (item names for trigger matching)
        let inventory: Vec<String> = self
            .player_character_repo
            .get_inventory(pc_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|item| item.name)
            .collect();

        // Get PC's observations - these represent what the PC has "witnessed"
        // While not direct event completions, observations can proxy for story progress
        let _observations = self
            .observation_repo
            .get_observations(pc_id)
            .await
            .unwrap_or_default();

        // Build trigger context with enriched PC state
        // NOTE: completed_events would ideally come from a PC-specific event tracking system.
        // For now, we populate inventory and location. Future enhancements could add:
        // - flags: from a PC flags/state system
        // - completed_events: from event chain progress tracking
        // - completed_challenges: from challenge history
        let context = TriggerContext {
            current_location: location_id,
            current_scene: None, // Would need SceneRepo to get current scene
            time_context: None,
            flags: std::collections::HashMap::new(),
            inventory,
            completed_events: Vec::new(), // TODO: Add PC event completion tracking
            event_outcomes: std::collections::HashMap::new(),
            turns_since_event: std::collections::HashMap::new(),
            completed_challenges: Vec::new(), // TODO: Add challenge history tracking
            challenge_successes: std::collections::HashMap::new(),
            turn_count: 0,
            recent_dialogue_topics: Vec::new(),
            recent_player_action: None,
        };

        // Evaluate each candidate and collect triggered events
        let mut triggered: Vec<domain::NarrativeEvent> = candidates
            .into_iter()
            .filter(|event| {
                let eval = event.evaluate_triggers(&context);
                eval.is_triggered
            })
            .collect();

        // Sort by priority (higher priority first)
        triggered.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(triggered)
    }
}
