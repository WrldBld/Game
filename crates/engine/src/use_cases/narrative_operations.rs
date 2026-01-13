//! Narrative entity operations.

use std::collections::HashMap;
use std::sync::Arc;

use wrldbldr_domain::{
    self as domain, CharacterId, EventChainId, LocationId, NarrativeEventId, NarrativeTriggerType,
    PlayerCharacterId, RegionId, SceneId, StoryEvent, StoryEventId, StoryEventType, TimeContext,
    TriggerContext, WorldId,
};

use crate::infrastructure::ports::{
    ChallengeRepo, CharacterRepo, ClockPort, FlagRepo, LocationRepo, NarrativeRepo,
    ObservationRepo, PlayerCharacterRepo, RepoError, SceneRepo, WorldRepo,
};

/// Narrative entity operations.
///
/// Handles narrative events, event chains, story events, and triggers.
pub struct Narrative {
    repo: Arc<dyn NarrativeRepo>,
    location_repo: Arc<dyn LocationRepo>,
    world_repo: Arc<dyn WorldRepo>,
    player_character_repo: Arc<dyn PlayerCharacterRepo>,
    character_repo: Arc<dyn CharacterRepo>,
    observation_repo: Arc<dyn ObservationRepo>,
    challenge_repo: Arc<dyn ChallengeRepo>,
    flag_repo: Arc<dyn FlagRepo>,
    scene_repo: Arc<dyn SceneRepo>,
    clock: Arc<dyn ClockPort>,
}

#[cfg(test)]
mod trigger_tests {
    use std::sync::Arc;

    use chrono::Utc;
    use wrldbldr_domain::{RegionId, WorldId};

    use crate::infrastructure::ports::{
        ClockPort, MockChallengeRepo, MockCharacterRepo, MockFlagRepo, MockLocationRepo,
        MockNarrativeRepo, MockObservationRepo, MockPlayerCharacterRepo, MockSceneRepo,
        MockWorldRepo,
    };

    struct FixedClock(chrono::DateTime<chrono::Utc>);

    impl ClockPort for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
    }

    #[tokio::test]
    async fn when_get_triggers_for_region_then_world_id_is_passed_to_repo() {
        let world_id = WorldId::new();
        let region_id = RegionId::new();
        let now = Utc::now();

        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_get_triggers_for_region()
            .withf(move |w, r| *w == world_id && *r == region_id)
            .returning(|_, _| Ok(vec![]));

        let location_repo = MockLocationRepo::new();
        let world_repo = MockWorldRepo::new();
        let player_character_repo = MockPlayerCharacterRepo::new();
        let character_repo = MockCharacterRepo::new();
        let observation_repo = MockObservationRepo::new();
        let challenge_repo = MockChallengeRepo::new();
        let flag_repo = MockFlagRepo::new();
        let scene_repo = MockSceneRepo::new();

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));

        let narrative = super::Narrative::new(
            Arc::new(narrative_repo),
            Arc::new(location_repo),
            Arc::new(world_repo),
            Arc::new(player_character_repo),
            Arc::new(character_repo),
            Arc::new(observation_repo),
            Arc::new(challenge_repo),
            Arc::new(flag_repo),
            Arc::new(scene_repo),
            clock,
        );

        narrative
            .get_triggers_for_region(world_id, region_id)
            .await
            .expect("get_triggers_for_region should succeed");
    }
}

impl Narrative {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repo: Arc<dyn NarrativeRepo>,
        location_repo: Arc<dyn LocationRepo>,
        world_repo: Arc<dyn WorldRepo>,
        player_character_repo: Arc<dyn PlayerCharacterRepo>,
        character_repo: Arc<dyn CharacterRepo>,
        observation_repo: Arc<dyn ObservationRepo>,
        challenge_repo: Arc<dyn ChallengeRepo>,
        flag_repo: Arc<dyn FlagRepo>,
        scene_repo: Arc<dyn SceneRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            repo,
            location_repo,
            world_repo,
            player_character_repo,
            character_repo,
            observation_repo,
            challenge_repo,
            flag_repo,
            scene_repo,
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

    /// Delete a narrative event by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete_event(&self, id: NarrativeEventId) -> Result<(), RepoError> {
        self.repo.delete_event(id).await
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

    /// Delete an event chain by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete_chain(&self, id: EventChainId) -> Result<(), RepoError> {
        self.repo.delete_chain(id).await
    }

    pub async fn list_chains_for_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::EventChain>, RepoError> {
        self.repo.list_chains_for_world(world_id).await
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

    /// Delete a story event by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete_story_event(&self, id: StoryEventId) -> Result<(), RepoError> {
        self.repo.delete_story_event(id).await
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
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        game_time: Option<String>,
    ) -> Result<StoryEventId, RepoError> {
        let event_id = StoryEventId::new();
        let timestamp = self.clock.now();

        // Get PC name from repo
        let pc = self.player_character_repo.get(pc_id).await?;
        let pc_name = pc
            .as_ref()
            .map(|pc| pc.name.clone())
            .unwrap_or_else(|| "Player".to_string());

        let (pc_location_id, pc_region_id) = pc
            .as_ref()
            .map(|pc| (Some(pc.current_location_id), pc.current_region_id))
            .unwrap_or((None, None));

        let world_game_time = self
            .world_repo
            .get(world_id)
            .await?
            .map(|world| world.game_time);

        // Build summary from dialogue
        let summary = format!(
            "{} spoke with {}: \"{}\" - \"{}\"",
            pc_name,
            npc_name,
            truncate_dialogue(&player_dialogue, 50),
            truncate_dialogue(&npc_response, 50),
        );

        let player_text = player_dialogue.clone();
        let npc_text = npc_response.clone();
        let topics_for_context = topics.clone();

        let fallback_game_time = world_game_time.as_ref().map(|gt| gt.display_date());

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
            game_time: game_time.or(fallback_game_time),
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

        let resolved_scene_id = if scene_id.is_some() {
            scene_id
        } else {
            self.scene_repo
                .get_current(world_id)
                .await?
                .map(|scene| scene.id)
        };

        let resolved_location_id = location_id.or(pc_location_id);
        let resolved_region_id = pc_region_id;

        if let Err(e) = self
            .repo
            .record_dialogue_context(
                world_id,
                event_id,
                pc_id,
                npc_id,
                player_text,
                npc_text,
                topics_for_context,
                resolved_scene_id,
                resolved_location_id,
                resolved_region_id,
                world_game_time.clone(),
                timestamp,
            )
            .await
        {
            tracing::error!(error = %e, "Failed to record dialogue conversation context");
        }

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

    /// Get conversation turns for LLM context.
    ///
    /// Returns ConversationTurn records from the active conversation between
    /// PC and NPC, in chronological order (oldest first). These are formatted
    /// for use in LLM prompts.
    ///
    /// # Arguments
    /// * `pc_id` - The player character ID
    /// * `npc_id` - The NPC character ID
    /// * `limit` - Maximum number of turns to return
    pub async fn get_conversation_turns(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<domain::ConversationTurn>, RepoError> {
        let records = self.repo.get_conversation_turns(pc_id, npc_id, limit).await?;

        // Convert ConversationTurnRecord to ConversationTurn
        let turns = records
            .into_iter()
            .map(|r| domain::ConversationTurn {
                speaker: r.speaker,
                text: r.text,
            })
            .collect();

        Ok(turns)
    }

    /// Get the active conversation ID between PC and NPC (if one exists).
    pub async fn get_active_conversation_id(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<uuid::Uuid>, RepoError> {
        self.repo.get_active_conversation_id(pc_id, npc_id).await
    }

    /// Check if a specific conversation is still active (not ended).
    ///
    /// Returns true if the conversation exists and has is_active = true.
    /// Returns false if the conversation doesn't exist or has been ended.
    pub async fn is_conversation_active(
        &self,
        conversation_id: uuid::Uuid,
    ) -> Result<bool, RepoError> {
        self.repo.is_conversation_active(conversation_id).await
    }

    /// End a conversation by setting is_active = false.
    ///
    /// This marks the conversation as ended so it cannot be resumed.
    /// Returns Ok(true) if the conversation was found and ended,
    /// Ok(false) if the conversation was not found or already ended.
    pub async fn end_conversation(&self, conversation_id: uuid::Uuid) -> Result<bool, RepoError> {
        self.repo.end_conversation(conversation_id).await
    }

    /// End the active conversation between PC and NPC (if one exists).
    ///
    /// Finds the active conversation and marks it as ended.
    /// Returns the conversation ID if one was ended, None if no active conversation.
    pub async fn end_active_conversation(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<uuid::Uuid>, RepoError> {
        self.repo.end_active_conversation(pc_id, npc_id).await
    }

    // =========================================================================
    // Triggers
    // =========================================================================

    pub async fn get_triggers_for_region(
        &self,
        world_id: WorldId,
        region_id: RegionId,
    ) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        self.repo.get_triggers_for_region(world_id, region_id).await
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

    /// Get all unique custom trigger descriptions from events in a region.
    ///
    /// This allows callers to pre-evaluate custom triggers via LLM before
    /// calling `check_triggers_with_custom_results`. Returns unique descriptions
    /// only for triggers that have `llm_evaluation: true`.
    pub async fn get_custom_triggers_for_region(
        &self,
        world_id: WorldId,
        region_id: RegionId,
    ) -> Result<Vec<String>, RepoError> {
        let events = self.get_triggers_for_region(world_id, region_id).await?;

        let mut triggers = std::collections::HashSet::new();
        for event in events {
            for trigger in &event.trigger_conditions {
                if let NarrativeTriggerType::Custom {
                    description,
                    llm_evaluation: true,
                } = &trigger.trigger_type
                {
                    triggers.insert(description.clone());
                }
            }
        }

        Ok(triggers.into_iter().collect())
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
        self.check_triggers_with_custom_results(region_id, pc_id, HashMap::new())
            .await
    }

    /// Check for triggered events with pre-evaluated custom trigger results.
    ///
    /// Same as `check_triggers` but accepts a map of pre-evaluated custom trigger
    /// results. For triggers with `llm_evaluation: true`, these results are used
    /// instead of treating the triggers as not met.
    ///
    /// # Arguments
    /// * `region_id` - The region being entered
    /// * `pc_id` - The player character triggering events
    /// * `custom_trigger_results` - Map of trigger description to whether it's met
    pub async fn check_triggers_with_custom_results(
        &self,
        region_id: RegionId,
        pc_id: wrldbldr_domain::PlayerCharacterId,
        custom_trigger_results: HashMap<String, bool>,
    ) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        // Resolve world context from PC (required for safe trigger queries)
        let pc = self.player_character_repo.get(pc_id).await?;
        let world_id = pc.as_ref().map(|pc| pc.world_id);

        let Some(world_id) = world_id else {
            tracing::warn!(pc_id = %pc_id, "Missing world_id for trigger evaluation");
            return Ok(vec![]);
        };

        // Extract compendium-related data from PC sheet_data
        let (origin_id, class_levels, known_spells, character_feats) =
            extract_compendium_context(&pc);

        // Get candidate events for this region (world-bounded)
        let candidates = self.get_triggers_for_region(world_id, region_id).await?;

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
        let inventory: Vec<String> = match self.player_character_repo.get_inventory(pc_id).await {
            Ok(items) => items.into_iter().map(|item| item.name).collect(),
            Err(e) => {
                tracing::warn!(
                    pc_id = %pc_id,
                    error = %e,
                    "Failed to fetch inventory for trigger evaluation, using empty inventory"
                );
                Vec::new()
            }
        };

        // Get PC's observations - these represent what the PC has "witnessed"
        // While not direct event completions, observations can proxy for story progress
        let _observations = match self.observation_repo.get_observations(pc_id).await {
            Ok(obs) => obs,
            Err(e) => {
                tracing::warn!(
                    pc_id = %pc_id,
                    error = %e,
                    "Failed to fetch observations for trigger evaluation"
                );
                Vec::new()
            }
        };

        // Get completed events from event chains in the world
        // Note: This is world-wide rather than PC-specific. A future enhancement
        // would be to track per-PC event completion via PC->Event edges.
        let (completed_events, completed_challenges) = {
            let events = match self.repo.get_completed_events(world_id).await {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(
                        world_id = %world_id,
                        error = %e,
                        "Failed to fetch completed events for trigger evaluation"
                    );
                    Vec::new()
                }
            };
            let challenges = match self.challenge_repo.get_resolved_challenges(world_id).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        world_id = %world_id,
                        error = %e,
                        "Failed to fetch resolved challenges for trigger evaluation"
                    );
                    Vec::new()
                }
            };
            (events, challenges)
        };

        // Get flags for this PC (both world and PC-scoped)
        let flags: HashMap<String, bool> = match self.flag_repo.get_world_flags(world_id).await {
            Ok(world_flags) => {
                let pc_flags = self.flag_repo.get_pc_flags(pc_id).await.unwrap_or_default();
                // Combine world and PC flags into a HashMap<String, bool>
                let mut flag_map = HashMap::new();
                for flag in world_flags {
                    flag_map.insert(flag, true);
                }
                for flag in pc_flags {
                    flag_map.insert(flag, true);
                }
                flag_map
            }
            Err(e) => {
                tracing::warn!(
                    world_id = %world_id,
                    error = %e,
                    "Failed to fetch flags for trigger evaluation"
                );
                HashMap::new()
            }
        };

        // Get current scene for the world (including time context)
        let (current_scene, time_context_string): (Option<SceneId>, Option<String>) =
            match self.scene_repo.get_current(world_id).await {
                Ok(Some(scene)) => {
                    let time_str = match &scene.time_context {
                        TimeContext::Unspecified => None,
                        TimeContext::TimeOfDay(tod) => Some(tod.display_name().to_string()),
                        TimeContext::During(event) => Some(event.clone()),
                        TimeContext::Custom(desc) => Some(desc.clone()),
                    };
                    (Some(scene.id), time_str)
                }
                Ok(None) => (None, None),
                Err(e) => {
                    tracing::warn!(
                        world_id = %world_id,
                        error = %e,
                        "Failed to fetch current scene for trigger evaluation"
                    );
                    (None, None)
                }
            };

        // Collect NPC IDs from RelationshipThreshold triggers and fetch their dispositions.
        //
        // SCOPE LIMITATION: Only NPC→PC relationships are supported because:
        // - NpcDispositionState tracks NPC feelings toward PCs
        // - PC→NPC feelings are not tracked in the disposition system
        // - NPC→NPC relationships are not tracked
        //
        // For triggers using NPC→NPC or PC→NPC, the trigger will not fire
        // (relationship data won't be found in context).
        //
        // NOTE: We iterate over candidates twice (here and in evaluate loop below).
        // This is intentional for readability - the relationship fetch is logically
        // separate from trigger evaluation. Can be combined if performance is an issue.
        let relationships = {
            // Extract unique NPC IDs that need relationship data (character_id in triggers)
            let mut npc_ids: std::collections::HashSet<CharacterId> =
                std::collections::HashSet::new();
            for event in &candidates {
                for trigger in &event.trigger_conditions {
                    if let NarrativeTriggerType::RelationshipThreshold { character_id, .. } =
                        &trigger.trigger_type
                    {
                        npc_ids.insert(*character_id);
                    }
                }
            }

            // Fetch dispositions for each NPC toward this PC
            let mut relationships: HashMap<CharacterId, HashMap<CharacterId, f32>> = HashMap::new();
            let pc_as_char_id = CharacterId::from(*pc_id.as_uuid());

            for npc_id in npc_ids {
                match self.character_repo.get_disposition(npc_id, pc_id).await {
                    Ok(Some(disposition)) => {
                        relationships
                            .entry(npc_id)
                            .or_default()
                            .insert(pc_as_char_id, disposition.sentiment);
                    }
                    Ok(None) => {
                        // Default to neutral (0.0) for NPCs who haven't interacted with this PC.
                        // This allows "stranger/neutral zone" triggers (e.g., min: -0.1, max: 0.1)
                        // to fire for NPCs the player hasn't met yet.
                        relationships
                            .entry(npc_id)
                            .or_default()
                            .insert(pc_as_char_id, 0.0);
                    }
                    Err(e) => {
                        tracing::warn!(
                            npc_id = %npc_id,
                            pc_id = %pc_id,
                            error = %e,
                            "Failed to fetch disposition for relationship trigger"
                        );
                    }
                }
            }
            relationships
        };

        // Collect character stats for StatThreshold triggers
        let character_stats = {
            // Extract unique character IDs that need stat data
            let mut char_ids: std::collections::HashSet<CharacterId> =
                std::collections::HashSet::new();
            for event in &candidates {
                for trigger in &event.trigger_conditions {
                    if let NarrativeTriggerType::StatThreshold { character_id, .. } =
                        &trigger.trigger_type
                    {
                        char_ids.insert(*character_id);
                    }
                }
            }

            // Fetch stats for each character
            let mut stats_map: HashMap<CharacterId, HashMap<String, i32>> = HashMap::new();
            for char_id in char_ids {
                match self.character_repo.get(char_id).await {
                    Ok(Some(character)) => {
                        // Extract effective stat values (base + modifiers)
                        let mut char_stats = HashMap::new();
                        for (stat_name, stat_value) in character.stats.get_all_stats() {
                            char_stats.insert(stat_name, stat_value.effective);
                        }
                        // Also include HP if present
                        if let Some(hp) = character.stats.get_current_hp() {
                            char_stats.insert("current_hp".to_string(), hp);
                        }
                        if let Some(max_hp) = character.stats.get_max_hp() {
                            char_stats.insert("max_hp".to_string(), max_hp);
                        }
                        stats_map.insert(char_id, char_stats);
                    }
                    Ok(None) => {
                        tracing::warn!(
                            character_id = %char_id,
                            "Character not found for StatThreshold trigger"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            character_id = %char_id,
                            error = %e,
                            "Failed to fetch character for StatThreshold trigger"
                        );
                    }
                }
            }
            stats_map
        };

        // Build trigger context with enriched PC state
        // NOTE: event_outcomes, challenge_successes, turns_since_event, turn_count
        // are caller-specific context that cannot be determined here.
        // These should be passed in by callers that have this information.
        let context = TriggerContext {
            current_location: location_id,
            current_scene,
            time_context: time_context_string,
            flags,
            inventory,
            completed_events,
            event_outcomes: HashMap::new(), // Caller responsibility - not stored in DB
            turns_since_event: HashMap::new(), // Caller responsibility - session state
            completed_challenges,
            challenge_successes: HashMap::new(), // TODO: Could query from ChallengeRepo if needed
            turn_count: 0,                       // Caller responsibility - session state
            recent_dialogue_topics: Vec::new(),  // Caller responsibility - session state
            recent_player_action: None,          // Caller responsibility - session state
            custom_trigger_results,              // Pre-evaluated LLM results for Custom triggers
            relationships,                       // NPC disposition sentiments toward this PC
            character_stats,                     // Character stat values for StatThreshold triggers
            // Compendium-based trigger context (populated from PC sheet data)
            known_spells,
            character_feats,
            class_levels,
            origin_id,
            known_creatures: Vec::new(),    // TODO: Populate from LoreKnowledge when implemented
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

/// Extract compendium-related context from a PlayerCharacter's sheet_data.
///
/// Returns (origin_id, class_levels, known_spells, character_feats).
/// Extracts from:
/// - Individual field values (RACE, CLASS, LEVEL) for basic identity
/// - Structured CharacterSpells, CharacterFeats, CharacterIdentity if stored in sheet_data
fn extract_compendium_context(
    pc: &Option<wrldbldr_domain::PlayerCharacter>,
) -> (
    Option<String>,
    HashMap<String, u8>,
    Vec<String>,
    Vec<String>,
) {
    let Some(pc) = pc else {
        return (None, HashMap::new(), Vec::new(), Vec::new());
    };

    let Some(sheet_data) = &pc.sheet_data else {
        return (None, HashMap::new(), Vec::new(), Vec::new());
    };

    let mut origin_id = None;
    let mut class_levels = HashMap::new();
    let mut known_spells = Vec::new();
    let mut character_feats = Vec::new();

    // Try to extract from structured CharacterIdentity first
    if let Some(identity_json) = sheet_data.get("character_identity") {
        if let Ok(identity) =
            serde_json::from_value::<wrldbldr_domain::CharacterIdentity>(identity_json.clone())
        {
            origin_id = identity.race.clone();
            for class_entry in &identity.classes {
                class_levels.insert(class_entry.class_id.to_lowercase(), class_entry.level);
            }
        }
    }

    // Fallback: extract from individual fields if CharacterIdentity not found
    if origin_id.is_none() {
        if let Some(race) = sheet_data.get_string("RACE") {
            origin_id = Some(race.to_lowercase());
        }
    }

    if class_levels.is_empty() {
        if let Some(class_name) = sheet_data.get_string("CLASS") {
            let level = sheet_data.get_number("LEVEL").unwrap_or(1) as u8;
            class_levels.insert(class_name.to_lowercase(), level);
        }
    }

    // Extract from structured CharacterSpells if present
    if let Some(spells_json) = sheet_data.get("character_spells") {
        if let Ok(spells) =
            serde_json::from_value::<wrldbldr_domain::CharacterSpells>(spells_json.clone())
        {
            // Add cantrips
            known_spells.extend(spells.cantrips.iter().map(|s| s.to_lowercase()));
            // Add known spells
            known_spells.extend(spells.known.iter().map(|s| s.spell_id.to_lowercase()));
        }
    }

    // Extract from structured CharacterFeats if present
    if let Some(feats_json) = sheet_data.get("character_feats") {
        if let Ok(feats) =
            serde_json::from_value::<wrldbldr_domain::CharacterFeats>(feats_json.clone())
        {
            character_feats.extend(feats.feats.iter().map(|f| f.feat_id.to_lowercase()));
        }
    }

    (origin_id, class_levels, known_spells, character_feats)
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
