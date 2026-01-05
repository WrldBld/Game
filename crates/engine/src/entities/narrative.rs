//! Narrative entity operations.

use std::sync::Arc;

use wrldbldr_domain::{
    self as domain, CharacterId, EventChainId, LocationId, NarrativeEventId, PlayerCharacterId,
    RegionId, SceneId, StoryEvent, StoryEventId, StoryEventType, TriggerContext, WorldId,
};

use crate::infrastructure::ports::{
    ChallengeRepo, ClockPort, LocationRepo, NarrativeRepo, ObservationRepo, PlayerCharacterRepo, RepoError,
};

/// Narrative entity operations.
///
/// Handles narrative events, event chains, story events, and triggers.
pub struct Narrative {
    repo: Arc<dyn NarrativeRepo>,
    location_repo: Arc<dyn LocationRepo>,
    player_character_repo: Arc<dyn PlayerCharacterRepo>,
    observation_repo: Arc<dyn ObservationRepo>,
    challenge_repo: Arc<dyn ChallengeRepo>,
    clock: Arc<dyn ClockPort>,
}

impl Narrative {
    pub fn new(
        repo: Arc<dyn NarrativeRepo>,
        location_repo: Arc<dyn LocationRepo>,
        player_character_repo: Arc<dyn PlayerCharacterRepo>,
        observation_repo: Arc<dyn ObservationRepo>,
        challenge_repo: Arc<dyn ChallengeRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            repo,
            location_repo,
            player_character_repo,
            observation_repo,
            challenge_repo,
            clock,
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

    /// Record a dialogue exchange between a PC and NPC.
    ///
    /// Creates a StoryEvent of type DialogueExchange and updates the SPOKE_TO
    /// relationship between the PC and NPC for dialogue history tracking.
    ///
    /// # Arguments
    /// * `world_id` - The world where the dialogue occurred
    /// * `pc_id` - The player character who initiated the dialogue
    /// * `npc_id` - The NPC who responded
    /// * `npc_name` - Display name of the NPC
    /// * `player_dialogue` - What the player said
    /// * `npc_response` - The NPC's response (after DM approval)
    /// * `topics` - Topics discussed in this exchange
    /// * `scene_id` - Optional scene where dialogue occurred (for future use)
    /// * `location_id` - Optional location where dialogue occurred (for future use)
    /// * `game_time` - Optional in-game time context
    #[allow(clippy::too_many_arguments)]
    pub async fn record_dialogue_exchange(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        npc_name: String,
        player_dialogue: String,
        npc_response: String,
        topics: Vec<String>,
        _scene_id: Option<SceneId>,
        _location_id: Option<LocationId>,
        game_time: Option<String>,
    ) -> Result<StoryEventId, RepoError> {
        let event_id = StoryEventId::new();
        let timestamp = self.clock.now();

        // Get PC name from repo
        let pc_name = self
            .player_character_repo
            .get(pc_id)
            .await?
            .map(|pc| pc.name)
            .unwrap_or_else(|| "Player".to_string());

        // Build summary from dialogue
        let summary = format!(
            "{} spoke with {}: \"{}\" - \"{}\"",
            pc_name,
            npc_name,
            truncate_dialogue(&player_dialogue, 50),
            truncate_dialogue(&npc_response, 50),
        );

        let event = StoryEvent {
            id: event_id,
            world_id,
            event_type: StoryEventType::DialogueExchange {
                npc_id,
                npc_name: npc_name.clone(),
                player_dialogue,
                npc_response,
                topics_discussed: topics.clone(),
                tone: None,
            },
            timestamp,
            game_time,
            summary,
            is_hidden: false,
            tags: vec!["dialogue".to_string()],
        };

        // Save the story event
        self.repo.save_story_event(&event).await?;

        // Update SPOKE_TO relationship for dialogue history tracking
        let last_topic = topics.first().cloned();
        self.repo
            .update_spoke_to(pc_id, npc_id, timestamp, last_topic)
            .await?;

        Ok(event_id)
    }

    /// Get dialogue history between a PC and NPC.
    ///
    /// Returns DialogueExchange story events in reverse chronological order.
    pub async fn get_dialogues_with_npc(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<domain::StoryEvent>, RepoError> {
        self.repo.get_dialogues_with_npc(pc_id, npc_id, limit).await
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

    /// Set a narrative event's active status.
    ///
    /// Used by EnableEvent/DisableEvent effects.
    pub async fn set_event_active(
        &self,
        id: NarrativeEventId,
        active: bool,
    ) -> Result<(), RepoError> {
        self.repo.set_event_active(id, active).await
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

        // Get completed events from event chains in the world
        // Note: This is world-wide rather than PC-specific. A future enhancement
        // would be to track per-PC event completion via PC->Event edges.
        let world_id = self
            .player_character_repo
            .get(pc_id)
            .await?
            .map(|pc| pc.world_id);
        
        let (completed_events, completed_challenges) = if let Some(world_id) = world_id {
            let events = self.repo.get_completed_events(world_id).await.unwrap_or_default();
            let challenges = self.challenge_repo.get_resolved_challenges(world_id).await.unwrap_or_default();
            (events, challenges)
        } else {
            (Vec::new(), Vec::new())
        };

        // Build trigger context with enriched PC state
        // NOTE: flags, event_outcomes, challenge_successes are still TODO for future enhancements
        let context = TriggerContext {
            current_location: location_id,
            current_scene: None, // Would need SceneRepo to get current scene
            time_context: None,
            flags: std::collections::HashMap::new(),
            inventory,
            completed_events,
            event_outcomes: std::collections::HashMap::new(),
            turns_since_event: std::collections::HashMap::new(),
            completed_challenges,
            challenge_successes: std::collections::HashMap::new(), // TODO: Track success/failure
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

/// Truncate dialogue for summary display.
/// Uses character-based truncation to avoid panics on multi-byte UTF-8.
fn truncate_dialogue(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_dialogue_ascii() {
        assert_eq!(truncate_dialogue("hello", 10), "hello");
        assert_eq!(truncate_dialogue("hello world", 5), "hello...");
    }

    #[test]
    fn test_truncate_dialogue_multibyte_utf8() {
        // Japanese text (3 bytes per char)
        let japanese = "こんにちは"; // 5 characters, 15 bytes
        assert_eq!(truncate_dialogue(japanese, 10), "こんにちは");
        assert_eq!(truncate_dialogue(japanese, 3), "こんに...");

        // Emoji (4 bytes per char)
        let emoji = "👋🌍🎉"; // 3 characters, 12 bytes
        assert_eq!(truncate_dialogue(emoji, 2), "👋🌍...");
    }
}
