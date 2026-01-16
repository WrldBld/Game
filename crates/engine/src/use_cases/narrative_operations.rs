//! Narrative operations with complex trigger evaluation.
//!
//! This module handles complex narrative operations that require multiple
//! repositories, including trigger evaluation and dialogue recording.
//! For simple CRUD operations, see `entities::narrative::Narrative`.

use std::collections::HashMap;
use std::sync::Arc;

use wrldbldr_domain::{
    self as domain, CharacterId, LocationId, NarrativeTriggerType, PlayerCharacterId, RegionId,
    SceneId, StoryEvent, StoryEventId, StoryEventType, TimeContext, TriggerContext, WorldId,
};

use crate::infrastructure::ports::RepoError;
use crate::llm_context::ConversationTurn;
use crate::repositories::{
    Challenge, Character, Clock, Flag, Location, Narrative as NarrativeRepository, Observation,
    PlayerCharacter, Scene, World,
};

/// Backward-compatible alias for `NarrativeOps`.
///
/// Consumers that import `narrative_operations::Narrative` will continue to work.
/// New code should prefer importing `NarrativeOps` directly.
pub type Narrative = NarrativeOps;

/// Narrative operations requiring multiple repositories.
///
/// Handles complex operations like trigger evaluation and dialogue recording
/// that need access to multiple data sources. Uses `entities::Narrative` for
/// simple CRUD operations.
pub struct NarrativeOps {
    /// Narrative repository wrapper
    narrative: Arc<NarrativeRepository>,
    location_repo: Arc<Location>,
    world_repo: Arc<World>,
    player_character_repo: Arc<PlayerCharacter>,
    character_repo: Arc<Character>,
    observation_repo: Arc<Observation>,
    challenge_repo: Arc<Challenge>,
    flag_repo: Arc<Flag>,
    scene_repo: Arc<Scene>,
    clock: Arc<Clock>,
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
    use crate::repositories::{
        Challenge, Character, Clock, Flag, Location, Narrative, Observation, PlayerCharacter,
        Scene, World,
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

        let location_repo = Arc::new(MockLocationRepo::new());
        let world_repo = Arc::new(MockWorldRepo::new());
        let player_character_repo = Arc::new(MockPlayerCharacterRepo::new());
        let character_repo = Arc::new(MockCharacterRepo::new());
        let observation_repo = Arc::new(MockObservationRepo::new());
        let challenge_repo = Arc::new(MockChallengeRepo::new());
        let flag_repo = Arc::new(MockFlagRepo::new());
        let scene_repo = Arc::new(MockSceneRepo::new());

        let clock_port: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let clock = Arc::new(Clock::new(clock_port.clone()));

        let narrative_ops = super::NarrativeOps::new(
            Arc::new(Narrative::new(Arc::new(narrative_repo), clock_port.clone())),
            Arc::new(Location::new(location_repo.clone())),
            Arc::new(World::new(world_repo.clone(), clock_port.clone())),
            Arc::new(PlayerCharacter::new(player_character_repo)),
            Arc::new(Character::new(character_repo)),
            Arc::new(Observation::new(
                observation_repo,
                location_repo,
                clock_port.clone(),
            )),
            Arc::new(Challenge::new(challenge_repo)),
            Arc::new(Flag::new(flag_repo)),
            Arc::new(Scene::new(scene_repo)),
            clock,
        );

        narrative_ops
            .get_triggers_for_region(world_id, region_id)
            .await
            .expect("get_triggers_for_region should succeed");
    }
}

impl NarrativeOps {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        narrative: Arc<NarrativeRepository>,
        location_repo: Arc<Location>,
        world_repo: Arc<World>,
        player_character_repo: Arc<PlayerCharacter>,
        character_repo: Arc<Character>,
        observation_repo: Arc<Observation>,
        challenge_repo: Arc<Challenge>,
        flag_repo: Arc<Flag>,
        scene_repo: Arc<Scene>,
        clock: Arc<Clock>,
    ) -> Self {
        Self {
            narrative,
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

    /// Get the underlying Narrative entity for CRUD operations.
    pub fn entity(&self) -> &Arc<NarrativeRepository> {
        &self.narrative
    }

    /// Access current time via injected clock.
    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.clock.now()
    }

    // =========================================================================
    // Delegated CRUD operations (pass-through to entity)
    // =========================================================================

    pub async fn get_event(
        &self,
        id: wrldbldr_domain::NarrativeEventId,
    ) -> Result<Option<domain::NarrativeEvent>, RepoError> {
        self.narrative.get_event(id).await
    }

    pub async fn save_event(&self, event: &domain::NarrativeEvent) -> Result<(), RepoError> {
        self.narrative.save_event(event).await
    }

    pub async fn list_events(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        self.narrative.list_events(world_id).await
    }

    pub async fn delete_event(
        &self,
        id: wrldbldr_domain::NarrativeEventId,
    ) -> Result<(), RepoError> {
        self.narrative.delete_event(id).await
    }

    pub async fn get_chain(
        &self,
        id: wrldbldr_domain::EventChainId,
    ) -> Result<Option<domain::EventChain>, RepoError> {
        self.narrative.get_chain(id).await
    }

    pub async fn save_chain(&self, chain: &domain::EventChain) -> Result<(), RepoError> {
        self.narrative.save_chain(chain).await
    }

    pub async fn delete_chain(&self, id: wrldbldr_domain::EventChainId) -> Result<(), RepoError> {
        self.narrative.delete_chain(id).await
    }

    pub async fn list_chains_for_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::EventChain>, RepoError> {
        self.narrative.list_chains_for_world(world_id).await
    }

    pub async fn get_story_event(
        &self,
        id: wrldbldr_domain::StoryEventId,
    ) -> Result<Option<domain::StoryEvent>, RepoError> {
        self.narrative.get_story_event(id).await
    }

    pub async fn save_story_event(&self, event: &domain::StoryEvent) -> Result<(), RepoError> {
        self.narrative.save_story_event(event).await
    }

    pub async fn delete_story_event(
        &self,
        id: wrldbldr_domain::StoryEventId,
    ) -> Result<(), RepoError> {
        self.narrative.delete_story_event(id).await
    }

    pub async fn list_story_events(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<domain::StoryEvent>, RepoError> {
        self.narrative.list_story_events(world_id, limit).await
    }

    pub async fn get_dialogues_with_npc(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<domain::StoryEvent>, RepoError> {
        self.narrative
            .get_dialogues_with_npc(pc_id, npc_id, limit)
            .await
    }

    pub async fn get_conversation_turns(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<ConversationTurn>, RepoError> {
        self.narrative
            .get_conversation_turns(pc_id, npc_id, limit)
            .await
    }

    pub async fn get_active_conversation_id(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<uuid::Uuid>, RepoError> {
        self.narrative
            .get_active_conversation_id(pc_id, npc_id)
            .await
    }

    pub async fn is_conversation_active(
        &self,
        conversation_id: uuid::Uuid,
    ) -> Result<bool, RepoError> {
        self.narrative.is_conversation_active(conversation_id).await
    }

    pub async fn end_conversation(&self, conversation_id: uuid::Uuid) -> Result<bool, RepoError> {
        self.narrative.end_conversation(conversation_id).await
    }

    pub async fn end_active_conversation(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<uuid::Uuid>, RepoError> {
        self.narrative.end_active_conversation(pc_id, npc_id).await
    }

    pub async fn get_triggers_for_region(
        &self,
        world_id: WorldId,
        region_id: RegionId,
    ) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        self.narrative
            .get_triggers_for_region(world_id, region_id)
            .await
    }

    pub async fn set_event_active(
        &self,
        id: wrldbldr_domain::NarrativeEventId,
        active: bool,
    ) -> Result<(), RepoError> {
        self.narrative.set_event_active(id, active).await
    }

    // =========================================================================
    // Complex operations (require multiple repos)
    // =========================================================================

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
            .map(|pc| pc.name().to_string())
            .unwrap_or_else(|| "Player".to_string());

        let (pc_location_id, pc_region_id) = pc
            .as_ref()
            .map(|pc| (Some(pc.current_location_id()), pc.current_region_id()))
            .unwrap_or((None, None));

        let world_game_time = self
            .world_repo
            .get(world_id)
            .await?
            .map(|world| world.game_time().clone());

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

        let event = StoryEvent::from_parts(
            event_id,
            world_id,
            StoryEventType::DialogueExchange {
                npc_id,
                npc_name: npc_name.clone(),
                player_dialogue,
                npc_response,
                topics_discussed: topics.clone(),
                tone: None,
            },
            timestamp,
            game_time.or(fallback_game_time),
            summary,
            false, // is_hidden
            vec![wrldbldr_domain::Tag::new("dialogue").expect("valid tag")],
        );

        // Save the story event
        self.narrative.save_story_event(&event).await?;

        // Update SPOKE_TO relationship for dialogue history tracking
        let last_topic = topics.first().cloned();
        self.narrative
            .update_spoke_to(pc_id, npc_id, timestamp, last_topic)
            .await?;

        let resolved_scene_id = if scene_id.is_some() {
            scene_id
        } else {
            self.scene_repo
                .get_current(world_id)
                .await?
                .map(|scene| scene.id())
        };

        let resolved_location_id = location_id.or(pc_location_id);
        let resolved_region_id = pc_region_id;

        if let Err(e) = self
            .narrative
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
            for trigger in event.trigger_conditions() {
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
        let world_id = pc.as_ref().map(|pc| pc.world_id());

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
            .map(|region| region.location_id());

        // Get PC's inventory (item names for trigger matching)
        let inventory: Vec<String> = match self.player_character_repo.get_inventory(pc_id).await {
            Ok(items) => items
                .into_iter()
                .map(|item| item.name().to_string())
                .collect(),
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
            let events = match self.narrative.get_completed_events(world_id).await {
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
            let challenges = match self.challenge_repo.get_resolved(world_id).await {
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
                let pc_flags = match self.flag_repo.get_pc_flags(pc_id).await {
                    Ok(flags) => flags,
                    Err(e) => {
                        tracing::warn!(
                            pc_id = %pc_id,
                            error = %e,
                            "Failed to fetch PC flags for trigger evaluation"
                        );
                        vec![]
                    }
                };
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
                    let time_str = match scene.time_context() {
                        TimeContext::Unspecified => None,
                        TimeContext::TimeOfDay(tod) => Some(tod.display_name().to_string()),
                        TimeContext::During(event) => Some(event.clone()),
                        TimeContext::Custom(desc) => Some(desc.clone()),
                    };
                    (Some(scene.id()), time_str)
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
                for trigger in event.trigger_conditions() {
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
                            .insert(pc_as_char_id, disposition.sentiment());
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
                for trigger in event.trigger_conditions() {
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
                        for (stat_name, stat_value) in character.stats().get_all_stats() {
                            char_stats.insert(stat_name, stat_value.effective());
                        }
                        // Also include HP if present
                        if let Some(hp) = character.stats().get_current_hp() {
                            char_stats.insert("current_hp".to_string(), hp);
                        }
                        if let Some(max_hp) = character.stats().get_max_hp() {
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

        // Build trigger context with enriched PC state using builder pattern
        // NOTE: event_outcomes, challenge_successes, turns_since_event, turn_count
        // are caller-specific context that cannot be determined here.
        // These should be passed in by callers that have this information.
        let mut context = TriggerContext::new()
            .with_flags(flags)
            .with_inventory(inventory)
            .with_completed_events(completed_events)
            .with_completed_challenges(completed_challenges)
            .with_custom_trigger_results(custom_trigger_results)
            .with_known_spells(known_spells)
            .with_character_feats(character_feats)
            .with_class_levels(class_levels);

        if let Some(loc_id) = location_id {
            context = context.with_current_location(loc_id);
        }
        if let Some(scene_id) = current_scene {
            context = context.with_current_scene(scene_id);
        }
        if let Some(time_ctx) = time_context_string {
            context = context.with_time_context(time_ctx);
        }
        if let Some(origin) = origin_id {
            context = context.with_origin_id(origin);
        }

        // Add relationships to context
        for (from_char, sentiments) in relationships {
            for (to_char, sentiment) in sentiments {
                context.add_relationship(from_char, to_char, sentiment);
            }
        }

        // Add character stats to context
        for (char_id, stats) in character_stats {
            for (stat_name, stat_value) in stats {
                context.add_character_stat(char_id, stat_name, stat_value);
            }
        }

        // Evaluate each candidate and collect triggered events
        let mut triggered: Vec<domain::NarrativeEvent> = candidates
            .into_iter()
            .filter(|event| {
                let eval = event.evaluate_triggers(&context);
                eval.is_triggered
            })
            .collect();

        // Sort by priority (higher priority first)
        triggered.sort_by(|a, b| b.priority().cmp(&a.priority()));

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

    let Some(sheet_data) = pc.sheet_data() else {
        return (None, HashMap::new(), Vec::new(), Vec::new());
    };

    let mut origin_id = None;
    let mut class_levels = HashMap::new();
    let mut known_spells = Vec::new();
    let mut character_feats = Vec::new();

    if let Some(race) = sheet_data.get_string("RACE") {
        origin_id = Some(race.to_lowercase());
    }

    if let Some(class_name) = sheet_data.get_string("CLASS") {
        let level = sheet_data.get_number("LEVEL").unwrap_or(1) as u8;
        class_levels.insert(class_name.to_lowercase(), level);
    }

    let known_spells_key = sheet_data
        .get("KNOWN_SPELLS")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if !known_spells_key.is_empty() {
        known_spells.extend(
            known_spells_key
                .split(',')
                .map(|spell| spell.trim().to_lowercase())
                .filter(|spell| !spell.is_empty()),
        );
    }

    let feats_key = sheet_data
        .get("FEATS")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if !feats_key.is_empty() {
        character_feats.extend(
            feats_key
                .split(',')
                .map(|feat| feat.trim().to_lowercase())
                .filter(|feat| !feat.is_empty()),
        );
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
