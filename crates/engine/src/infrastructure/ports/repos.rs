// Port traits define the full contract - many methods are for future use
#![allow(dead_code)]

//! Repository port traits for database access.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use wrldbldr_domain::*;

use super::error::RepoError;
use super::types::{
    ActantialViewRecord, ConversationTurnRecord, GoalDetails, NpcRegionRelationType,
    NpcWithRegionInfo, WantDetails, WantTargetRef,
};
use crate::infrastructure::app_settings::AppSettings;

// =============================================================================
// Settings Storage
// =============================================================================

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SettingsRepo: Send + Sync {
    async fn get_global(&self) -> Result<Option<AppSettings>, RepoError>;
    async fn save_global(&self, settings: &AppSettings) -> Result<(), RepoError>;
    async fn get_for_world(&self, world_id: WorldId) -> Result<Option<AppSettings>, RepoError>;
    async fn save_for_world(
        &self,
        world_id: WorldId,
        settings: &AppSettings,
    ) -> Result<(), RepoError>;
    async fn delete_for_world(&self, world_id: WorldId) -> Result<(), RepoError>;
}

// =============================================================================
// Database Ports (one per entity type)
// =============================================================================

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait CharacterRepo: Send + Sync {
    // CRUD
    async fn get(&self, id: CharacterId) -> Result<Option<Character>, RepoError>;
    async fn save(&self, character: &Character) -> Result<(), RepoError>;
    async fn delete(&self, id: CharacterId) -> Result<(), RepoError>;

    // Queries
    async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<Character>, RepoError>;
    async fn list_in_world(
        &self,
        world_id: WorldId,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Character>, RepoError>;
    async fn list_npcs_in_world(&self, world_id: WorldId) -> Result<Vec<Character>, RepoError>;

    // Position
    async fn update_position(&self, id: CharacterId, region_id: RegionId) -> Result<(), RepoError>;

    // Relationships
    async fn get_relationships(&self, id: CharacterId) -> Result<Vec<Relationship>, RepoError>;
    async fn save_relationship(&self, relationship: &Relationship) -> Result<(), RepoError>;
    async fn delete_relationship(&self, id: RelationshipId) -> Result<(), RepoError>;

    // Inventory
    async fn get_inventory(&self, id: CharacterId) -> Result<Vec<Item>, RepoError>;
    async fn add_to_inventory(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError>;
    async fn remove_from_inventory(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError>;

    // Wants/Goals
    async fn get_wants(&self, id: CharacterId) -> Result<Vec<WantDetails>, RepoError>;
    async fn get_want(&self, id: WantId) -> Result<Option<WantDetails>, RepoError>;
    async fn save_want(
        &self,
        character_id: CharacterId,
        want: &Want,
        priority: u32,
    ) -> Result<(), RepoError>;
    async fn delete_want(&self, id: WantId) -> Result<(), RepoError>;
    async fn set_want_target(
        &self,
        want_id: WantId,
        target: WantTargetRef,
    ) -> Result<WantTarget, RepoError>;
    async fn remove_want_target(&self, want_id: WantId) -> Result<(), RepoError>;

    // Disposition (NPC's view of a specific PC)
    async fn get_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<NpcDispositionState>, RepoError>;
    async fn save_disposition(&self, disposition: &NpcDispositionState) -> Result<(), RepoError>;
    async fn list_dispositions_for_pc(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>, RepoError>;

    // Actantial
    async fn get_actantial_context(
        &self,
        id: CharacterId,
    ) -> Result<Option<ActantialContext>, RepoError>;
    async fn save_actantial_context(
        &self,
        id: CharacterId,
        context: &ActantialContext,
    ) -> Result<(), RepoError>;
    async fn list_actantial_views(
        &self,
        id: CharacterId,
    ) -> Result<Vec<ActantialViewRecord>, RepoError>;
    async fn add_actantial_view(
        &self,
        character_id: CharacterId,
        want_id: WantId,
        target: ActantialTarget,
        role: ActantialRole,
        reason: String,
    ) -> Result<ActantialViewRecord, RepoError>;
    async fn remove_actantial_view(
        &self,
        character_id: CharacterId,
        want_id: WantId,
        target: ActantialTarget,
        role: ActantialRole,
    ) -> Result<(), RepoError>;

    // NPC-Region relationships (for staging suggestions)
    /// Get all region relationships for a character (home, work, frequents, avoids)
    async fn get_region_relationships(
        &self,
        id: CharacterId,
    ) -> Result<Vec<super::types::NpcRegionRelationship>, RepoError>;
    /// Set an NPC's home region
    async fn set_home_region(&self, id: CharacterId, region_id: RegionId) -> Result<(), RepoError>;
    /// Set an NPC's work region with optional shift (Day/Night/Always)
    async fn set_work_region(
        &self,
        id: CharacterId,
        region_id: RegionId,
        shift: Option<wrldbldr_domain::RegionShift>,
    ) -> Result<(), RepoError>;
    /// Add a region the NPC frequents with frequency (Always/Often/Sometimes/Rarely)
    async fn add_frequents_region(
        &self,
        id: CharacterId,
        region_id: RegionId,
        frequency: wrldbldr_domain::RegionFrequency,
        time_of_day: Option<wrldbldr_domain::TimeOfDay>,
    ) -> Result<(), RepoError>;
    /// Add a region the NPC avoids
    async fn add_avoids_region(
        &self,
        id: CharacterId,
        region_id: RegionId,
        reason: Option<String>,
    ) -> Result<(), RepoError>;
    /// Remove a region relationship
    async fn remove_region_relationship(
        &self,
        id: CharacterId,
        region_id: RegionId,
        relationship_type: NpcRegionRelationType,
    ) -> Result<(), RepoError>;
    /// Get NPCs that have any relationship to a region (for staging suggestions)
    async fn get_npcs_for_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<NpcWithRegionInfo>, RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait GoalRepo: Send + Sync {
    async fn get(&self, id: GoalId) -> Result<Option<GoalDetails>, RepoError>;
    async fn save(&self, goal: &Goal) -> Result<(), RepoError>;
    async fn delete(&self, id: GoalId) -> Result<(), RepoError>;
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<GoalDetails>, RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PlayerCharacterRepo: Send + Sync {
    async fn get(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>, RepoError>;
    async fn save(&self, pc: &PlayerCharacter) -> Result<(), RepoError>;
    async fn delete(&self, id: PlayerCharacterId) -> Result<(), RepoError>;
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<PlayerCharacter>, RepoError>;
    async fn get_by_user(
        &self,
        world_id: WorldId,
        user_id: &UserId,
    ) -> Result<Option<PlayerCharacter>, RepoError>;
    async fn update_position(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
        region_id: RegionId,
    ) -> Result<(), RepoError>;
    async fn get_inventory(&self, id: PlayerCharacterId) -> Result<Vec<Item>, RepoError>;

    // Inventory management
    async fn add_to_inventory(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError>;
    async fn remove_from_inventory(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError>;

    /// Modify a stat on a player character (for ModifyCharacterStat trigger)
    async fn modify_stat(
        &self,
        id: PlayerCharacterId,
        stat: &str,
        modifier: i32,
    ) -> Result<(), RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LocationRepo: Send + Sync {
    // Location CRUD
    async fn get_location(&self, id: LocationId) -> Result<Option<Location>, RepoError>;
    async fn save_location(&self, location: &Location) -> Result<(), RepoError>;
    async fn delete_location(&self, id: LocationId) -> Result<(), RepoError>;
    async fn list_locations_in_world(
        &self,
        world_id: WorldId,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Location>, RepoError>;

    // Region CRUD
    async fn get_region(&self, id: RegionId) -> Result<Option<Region>, RepoError>;
    async fn save_region(&self, region: &Region) -> Result<(), RepoError>;
    async fn delete_region(&self, id: RegionId) -> Result<(), RepoError>;
    async fn list_regions_in_location(
        &self,
        location_id: LocationId,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Region>, RepoError>;

    // Connections
    async fn get_connections(
        &self,
        region_id: RegionId,
        limit: Option<u32>,
    ) -> Result<Vec<RegionConnection>, RepoError>;
    async fn save_connection(&self, connection: &RegionConnection) -> Result<(), RepoError>;
    async fn delete_connection(
        &self,
        from_region: RegionId,
        to_region: RegionId,
    ) -> Result<(), RepoError>;

    // Location connections (exits)
    async fn get_location_exits(
        &self,
        location_id: LocationId,
        limit: Option<u32>,
    ) -> Result<Vec<LocationConnection>, RepoError>;
    async fn save_location_connection(
        &self,
        connection: &LocationConnection,
    ) -> Result<(), RepoError>;
    async fn delete_location_connection(
        &self,
        from_location: LocationId,
        to_location: LocationId,
    ) -> Result<(), RepoError>;

    // Region exits (to locations)
    async fn get_region_exits(
        &self,
        region_id: RegionId,
        limit: Option<u32>,
    ) -> Result<Vec<RegionExit>, RepoError>;
    async fn save_region_exit(&self, exit: &RegionExit) -> Result<(), RepoError>;
    async fn delete_region_exit(
        &self,
        region_id: RegionId,
        location_id: LocationId,
    ) -> Result<(), RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SceneRepo: Send + Sync {
    async fn get(&self, id: SceneId) -> Result<Option<Scene>, RepoError>;
    async fn save(&self, scene: &Scene) -> Result<(), RepoError>;
    async fn delete(&self, id: SceneId) -> Result<(), RepoError>;
    async fn get_current(&self, world_id: WorldId) -> Result<Option<Scene>, RepoError>;
    async fn set_current(&self, world_id: WorldId, scene_id: SceneId) -> Result<(), RepoError>;
    async fn list_for_region(&self, region_id: RegionId) -> Result<Vec<Scene>, RepoError>;
    async fn list_for_act(
        &self,
        act_id: ActId,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Scene>, RepoError>;

    // Location edge management (graph-first design)
    /// Get the location for a scene via AT_LOCATION edge.
    async fn get_location(&self, scene_id: SceneId) -> Result<Option<LocationId>, RepoError>;
    /// Set the location for a scene via AT_LOCATION edge.
    async fn set_location(
        &self,
        scene_id: SceneId,
        location_id: LocationId,
    ) -> Result<(), RepoError>;

    // Featured character edge management (graph-first design)
    async fn get_featured_characters(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<SceneCharacter>, RepoError>;
    async fn set_featured_characters(
        &self,
        scene_id: SceneId,
        characters: &[SceneCharacter],
    ) -> Result<(), RepoError>;

    // Completed scene tracking for scene resolution
    /// Check if a PC has completed a specific scene.
    async fn has_completed_scene(
        &self,
        pc_id: PlayerCharacterId,
        scene_id: SceneId,
    ) -> Result<bool, RepoError>;
    /// Mark a scene as completed for a PC.
    async fn mark_scene_completed(
        &self,
        pc_id: PlayerCharacterId,
        scene_id: SceneId,
    ) -> Result<(), RepoError>;
    /// Get all completed scene IDs for a PC.
    async fn get_completed_scenes(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<SceneId>, RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ActRepo: Send + Sync {
    async fn get(&self, id: ActId) -> Result<Option<Act>, RepoError>;
    async fn save(&self, act: &Act) -> Result<(), RepoError>;
    async fn delete(&self, id: ActId) -> Result<(), RepoError>;
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<Act>, RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ContentRepo: Send + Sync {
    // Skills
    async fn get_skill(&self, id: SkillId) -> Result<Option<Skill>, RepoError>;
    async fn save_skill(&self, skill: &Skill) -> Result<(), RepoError>;
    async fn delete_skill(&self, id: SkillId) -> Result<(), RepoError>;
    async fn list_skills_in_world(&self, world_id: WorldId) -> Result<Vec<Skill>, RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait InteractionRepo: Send + Sync {
    async fn get(&self, id: InteractionId) -> Result<Option<InteractionTemplate>, RepoError>;
    async fn save(&self, interaction: &InteractionTemplate) -> Result<(), RepoError>;
    async fn delete(&self, id: InteractionId) -> Result<(), RepoError>;
    async fn list_for_scene(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<InteractionTemplate>, RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ChallengeRepo: Send + Sync {
    async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>, RepoError>;
    async fn save(&self, challenge: &Challenge) -> Result<(), RepoError>;
    async fn delete(&self, id: ChallengeId) -> Result<(), RepoError>;
    async fn list_for_world(&self, world_id: WorldId) -> Result<Vec<Challenge>, RepoError>;
    async fn list_for_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>, RepoError>;
    async fn list_pending_for_world(&self, world_id: WorldId) -> Result<Vec<Challenge>, RepoError>;
    async fn mark_resolved(&self, id: ChallengeId) -> Result<(), RepoError>;
    /// Enable or disable a challenge (for EnableChallenge/DisableChallenge triggers)
    async fn set_enabled(&self, id: ChallengeId, enabled: bool) -> Result<(), RepoError>;
    /// Get all resolved (inactive) challenge IDs in a world.
    /// Used for trigger context building.
    async fn get_resolved_challenges(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<ChallengeId>, RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait NarrativeRepo: Send + Sync {
    // Events
    async fn get_event(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>, RepoError>;
    async fn save_event(&self, event: &NarrativeEvent) -> Result<(), RepoError>;
    async fn delete_event(&self, id: NarrativeEventId) -> Result<(), RepoError>;
    async fn list_events_for_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<NarrativeEvent>, RepoError>;

    // Event chains
    async fn get_chain(&self, id: EventChainId) -> Result<Option<EventChain>, RepoError>;
    async fn save_chain(&self, chain: &EventChain) -> Result<(), RepoError>;
    async fn delete_chain(&self, id: EventChainId) -> Result<(), RepoError>;
    async fn list_chains_for_world(&self, world_id: WorldId) -> Result<Vec<EventChain>, RepoError>;

    // Story events
    async fn get_story_event(&self, id: StoryEventId) -> Result<Option<StoryEvent>, RepoError>;
    async fn save_story_event(&self, event: &StoryEvent) -> Result<(), RepoError>;
    async fn delete_story_event(&self, id: StoryEventId) -> Result<(), RepoError>;
    async fn list_story_events(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<StoryEvent>, RepoError>;

    // Dialogue history
    /// Get dialogue exchanges between a PC and NPC (reverse chronological order).
    async fn get_dialogues_with_npc(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<StoryEvent>, RepoError>;

    /// Update or create SPOKE_TO relationship between PC and NPC.
    /// Tracks last dialogue timestamp, topic, and increments conversation count.
    async fn update_spoke_to(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        timestamp: chrono::DateTime<chrono::Utc>,
        last_topic: Option<String>,
    ) -> Result<(), RepoError>;

    async fn record_dialogue_context(
        &self,
        world_id: WorldId,
        story_event_id: StoryEventId,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        player_dialogue: String,
        npc_dialogue: String,
        topics: Vec<String>,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
        game_time: Option<GameTime>,
        timestamp: DateTime<Utc>,
    ) -> Result<(), RepoError>;

    // =========================================================================
    // Conversation History (for LLM context)
    // =========================================================================

    /// Get dialogue turns from an active conversation between PC and NPC.
    ///
    /// Returns turns in chronological order (oldest first) for LLM context.
    /// Each turn includes the speaker name and text.
    ///
    /// # Arguments
    /// * `pc_id` - The player character ID
    /// * `npc_id` - The NPC character ID
    /// * `limit` - Maximum number of turns to return
    async fn get_conversation_turns(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<ConversationTurnRecord>, RepoError>;

    /// Get the active conversation ID between PC and NPC (if one exists).
    async fn get_active_conversation_id(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<ConversationId>, RepoError>;

    /// Check if a specific conversation is still active (not ended).
    ///
    /// Returns true if the conversation exists and has is_active = true.
    /// Returns false if the conversation doesn't exist or has been ended.
    async fn is_conversation_active(
        &self,
        conversation_id: ConversationId,
    ) -> Result<bool, RepoError>;

    /// End a conversation by setting is_active = false.
    ///
    /// This marks the conversation as ended so it cannot be resumed.
    /// Returns Ok(true) if the conversation was found and ended,
    /// Ok(false) if the conversation was not found or already ended.
    async fn end_conversation(&self, conversation_id: ConversationId) -> Result<bool, RepoError>;

    /// End the active conversation between PC and NPC (if one exists).
    ///
    /// Finds the active conversation and marks it as ended.
    /// Returns the conversation ID if one was ended, None if no active conversation.
    async fn end_active_conversation(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<ConversationId>, RepoError>;

    // Triggers
    async fn get_triggers_for_region(
        &self,
        world_id: WorldId,
        region_id: RegionId,
    ) -> Result<Vec<NarrativeEvent>, RepoError>;

    // Event management for effect execution
    /// Set a narrative event's active status (for EnableEvent/DisableEvent effects)
    async fn set_event_active(&self, id: NarrativeEventId, active: bool) -> Result<(), RepoError>;

    /// Get all completed event IDs from all event chains in a world.
    /// Used for trigger context building.
    async fn get_completed_events(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<NarrativeEventId>, RepoError>;

    /// Get outcomes for all triggered events in a world.
    /// Returns a map of event_id -> selected_outcome for events that have an outcome.
    /// Used for EventCompleted triggers that check for specific outcomes.
    async fn get_event_outcomes(
        &self,
        world_id: WorldId,
    ) -> Result<std::collections::HashMap<NarrativeEventId, String>, RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait StagingRepo: Send + Sync {
    async fn get_staged_npcs(&self, region_id: RegionId) -> Result<Vec<StagedNpc>, RepoError>;
    async fn stage_npc(
        &self,
        region_id: RegionId,
        character_id: CharacterId,
    ) -> Result<(), RepoError>;
    async fn unstage_npc(
        &self,
        region_id: RegionId,
        character_id: CharacterId,
    ) -> Result<(), RepoError>;
    async fn get_pending_staging(&self, world_id: WorldId) -> Result<Vec<Staging>, RepoError>;
    async fn save_pending_staging(&self, staging: &Staging) -> Result<(), RepoError>;
    async fn save_and_activate_pending_staging(
        &self,
        staging: &Staging,
        region_id: RegionId,
    ) -> Result<(), RepoError>;
    async fn save_and_activate_pending_staging_with_states(
        &self,
        staging: &Staging,
        region_id: RegionId,
        location_state_id: Option<LocationStateId>,
        region_state_id: Option<RegionStateId>,
    ) -> Result<(), RepoError>;
    async fn delete_pending_staging(&self, id: StagingId) -> Result<(), RepoError>;

    /// Get active staging for a region, checking TTL expiry.
    /// Returns None if no staging exists or if the current staging is expired.
    ///
    /// # Arguments
    /// * `region_id` - The region to get staging for
    /// * `current_game_time_seconds` - Current game time in total seconds since epoch
    async fn get_active_staging(
        &self,
        region_id: RegionId,
        current_game_time_seconds: i64,
    ) -> Result<Option<Staging>, RepoError>;

    /// Activate a staging (after DM approval), replacing any existing current staging.
    async fn activate_staging(
        &self,
        staging_id: StagingId,
        region_id: RegionId,
    ) -> Result<(), RepoError>;

    /// Get staging history for a region (most recent first, limited).
    /// Returns past stagings that are no longer active.
    async fn get_staging_history(
        &self,
        region_id: RegionId,
        limit: usize,
    ) -> Result<Vec<Staging>, RepoError>;

    // =========================================================================
    // Mood Operations (Tier 2 of three-tier emotional model)
    // =========================================================================

    /// Get an NPC's current mood in a region's active staging.
    /// Returns the NPC's default_mood if not staged or no mood override set.
    async fn get_npc_mood(
        &self,
        region_id: RegionId,
        npc_id: CharacterId,
    ) -> Result<MoodState, RepoError>;

    /// Set an NPC's mood in a region's active staging.
    /// Creates or updates the mood property on the INCLUDES_NPC edge.
    async fn set_npc_mood(
        &self,
        region_id: RegionId,
        npc_id: CharacterId,
        mood: MoodState,
    ) -> Result<(), RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ObservationRepo: Send + Sync {
    async fn get_observations(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcObservation>, RepoError>;
    async fn save_observation(&self, observation: &NpcObservation) -> Result<(), RepoError>;
    async fn delete_observation(
        &self,
        pc_id: PlayerCharacterId,
        target_id: CharacterId,
    ) -> Result<(), RepoError>;
    async fn has_observed(
        &self,
        pc_id: PlayerCharacterId,
        target_id: CharacterId,
    ) -> Result<bool, RepoError>;
    /// Save deduced information from a challenge (for RevealInformation trigger)
    async fn save_deduced_info(
        &self,
        pc_id: PlayerCharacterId,
        info: String,
    ) -> Result<(), RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ItemRepo: Send + Sync {
    async fn get(&self, id: ItemId) -> Result<Option<Item>, RepoError>;
    async fn save(&self, item: &Item) -> Result<(), RepoError>;
    async fn delete(&self, id: ItemId) -> Result<(), RepoError>;
    async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<Item>, RepoError>;
    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<Item>, RepoError>;

    // Equipment management (EQUIPPED_BY edge)
    async fn set_equipped(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError>;
    async fn set_unequipped(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<(), RepoError>;

    // Region placement (IN_REGION edge for dropped items)
    async fn place_in_region(&self, item_id: ItemId, region_id: RegionId) -> Result<(), RepoError>;
    async fn remove_from_region(&self, item_id: ItemId) -> Result<(), RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WorldRepo: Send + Sync {
    async fn get(&self, id: WorldId) -> Result<Option<World>, RepoError>;
    async fn save(&self, world: &World) -> Result<(), RepoError>;
    async fn list_all(&self) -> Result<Vec<World>, RepoError>;
    async fn delete(&self, id: WorldId) -> Result<(), RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LoreRepo: Send + Sync {
    // CRUD
    async fn get(&self, id: LoreId) -> Result<Option<Lore>, RepoError>;
    async fn save(&self, lore: &Lore) -> Result<(), RepoError>;
    async fn delete(&self, id: LoreId) -> Result<(), RepoError>;

    // Queries
    async fn list_for_world(&self, world_id: WorldId) -> Result<Vec<Lore>, RepoError>;
    async fn list_by_category(
        &self,
        world_id: WorldId,
        category: LoreCategory,
    ) -> Result<Vec<Lore>, RepoError>;
    async fn list_common_knowledge(&self, world_id: WorldId) -> Result<Vec<Lore>, RepoError>;
    async fn search_by_tags(
        &self,
        world_id: WorldId,
        tags: &[String],
    ) -> Result<Vec<Lore>, RepoError>;

    // Knowledge management
    async fn grant_knowledge(&self, knowledge: &LoreKnowledge) -> Result<(), RepoError>;
    async fn revoke_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
    ) -> Result<(), RepoError>;
    async fn get_character_knowledge(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<LoreKnowledge>, RepoError>;
    async fn get_knowledge_for_lore(
        &self,
        lore_id: LoreId,
    ) -> Result<Vec<LoreKnowledge>, RepoError>;
    async fn character_knows_lore(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
    ) -> Result<Option<LoreKnowledge>, RepoError>;

    // Add chunks to existing knowledge
    async fn add_chunks_to_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
        chunk_ids: &[LoreChunkId],
    ) -> Result<(), RepoError>;

    // Remove chunks from existing knowledge (partial revocation)
    // Returns true if the knowledge relationship was completely removed (no chunks left)
    async fn remove_chunks_from_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
        chunk_ids: &[LoreChunkId],
    ) -> Result<bool, RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LocationStateRepo: Send + Sync {
    // CRUD
    async fn get(&self, id: LocationStateId) -> Result<Option<LocationState>, RepoError>;
    async fn save(&self, state: &LocationState) -> Result<(), RepoError>;
    async fn delete(&self, id: LocationStateId) -> Result<(), RepoError>;

    // Queries
    async fn list_for_location(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<LocationState>, RepoError>;
    async fn get_default(
        &self,
        location_id: LocationId,
    ) -> Result<Option<LocationState>, RepoError>;

    // Active state management
    async fn set_active(
        &self,
        location_id: LocationId,
        state_id: LocationStateId,
    ) -> Result<(), RepoError>;
    async fn get_active(&self, location_id: LocationId)
        -> Result<Option<LocationState>, RepoError>;
    async fn clear_active(&self, location_id: LocationId) -> Result<(), RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RegionStateRepo: Send + Sync {
    // CRUD
    async fn get(&self, id: RegionStateId) -> Result<Option<RegionState>, RepoError>;
    async fn save(&self, state: &RegionState) -> Result<(), RepoError>;
    async fn delete(&self, id: RegionStateId) -> Result<(), RepoError>;

    // Queries
    async fn list_for_region(&self, region_id: RegionId) -> Result<Vec<RegionState>, RepoError>;
    async fn get_default(&self, region_id: RegionId) -> Result<Option<RegionState>, RepoError>;

    // Active state management
    async fn set_active(
        &self,
        region_id: RegionId,
        state_id: RegionStateId,
    ) -> Result<(), RepoError>;
    async fn get_active(&self, region_id: RegionId) -> Result<Option<RegionState>, RepoError>;
    async fn clear_active(&self, region_id: RegionId) -> Result<(), RepoError>;
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AssetRepo: Send + Sync {
    async fn get(&self, id: AssetId) -> Result<Option<GalleryAsset>, RepoError>;
    async fn save(&self, asset: &GalleryAsset) -> Result<(), RepoError>;
    async fn delete(&self, id: AssetId) -> Result<(), RepoError>;
    async fn list_for_entity(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
    ) -> Result<Vec<GalleryAsset>, RepoError>;
    async fn set_active(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
        asset_id: AssetId,
    ) -> Result<(), RepoError>;
}

// =============================================================================
// Flag Storage Port
// =============================================================================

/// Repository for game flags (used in scene conditions and narrative triggers).
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait FlagRepo: Send + Sync {
    /// Get all set flags for a world (world-scoped flags).
    async fn get_world_flags(&self, world_id: WorldId) -> Result<Vec<String>, RepoError>;

    /// Get all set flags for a player character (PC-scoped flags).
    async fn get_pc_flags(&self, pc_id: PlayerCharacterId) -> Result<Vec<String>, RepoError>;

    /// Set a world-scoped flag.
    async fn set_world_flag(&self, world_id: WorldId, flag_name: &str) -> Result<(), RepoError>;

    /// Unset a world-scoped flag.
    async fn unset_world_flag(&self, world_id: WorldId, flag_name: &str) -> Result<(), RepoError>;

    /// Set a PC-scoped flag.
    async fn set_pc_flag(&self, pc_id: PlayerCharacterId, flag_name: &str)
        -> Result<(), RepoError>;

    /// Unset a PC-scoped flag.
    async fn unset_pc_flag(
        &self,
        pc_id: PlayerCharacterId,
        flag_name: &str,
    ) -> Result<(), RepoError>;

    /// Check if a world-scoped flag is set.
    async fn is_world_flag_set(
        &self,
        world_id: WorldId,
        flag_name: &str,
    ) -> Result<bool, RepoError>;

    /// Check if a PC-scoped flag is set.
    async fn is_pc_flag_set(
        &self,
        pc_id: PlayerCharacterId,
        flag_name: &str,
    ) -> Result<bool, RepoError>;

    /// Get all flags relevant to a PC (combines world and PC-scoped flags).
    ///
    /// Uses HashSet internally to deduplicate flags that may exist at both scopes.
    /// Default implementation calls get_world_flags and get_pc_flags.
    async fn get_all_flags_for_pc(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<String>, RepoError> {
        use std::collections::HashSet;
        let world_flags = self.get_world_flags(world_id).await?;
        let pc_flags = self.get_pc_flags(pc_id).await?;

        let mut unique_flags: HashSet<String> =
            HashSet::with_capacity(world_flags.len() + pc_flags.len());
        unique_flags.extend(world_flags);
        unique_flags.extend(pc_flags);

        Ok(unique_flags.into_iter().collect())
    }
}

// =============================================================================
// Prompt Template Storage
// =============================================================================

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PromptTemplateRepo: Send + Sync {
    /// Get a global template override by key.
    async fn get_global_override(&self, key: &str) -> Result<Option<String>, RepoError>;

    /// Get a world-specific template override by key.
    async fn get_world_override(
        &self,
        world_id: WorldId,
        key: &str,
    ) -> Result<Option<String>, RepoError>;

    /// Set a global template override.
    async fn set_global_override(&self, key: &str, value: &str) -> Result<(), RepoError>;

    /// Set a world-specific template override.
    async fn set_world_override(
        &self,
        world_id: WorldId,
        key: &str,
        value: &str,
    ) -> Result<(), RepoError>;

    /// Delete a global template override.
    async fn delete_global_override(&self, key: &str) -> Result<(), RepoError>;

    /// Delete a world-specific template override.
    async fn delete_world_override(&self, world_id: WorldId, key: &str) -> Result<(), RepoError>;

    /// List all global template overrides.
    async fn list_global_overrides(&self) -> Result<Vec<(String, String)>, RepoError>;

    /// List all world-specific template overrides.
    async fn list_world_overrides(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<(String, String)>, RepoError>;

    /// Resolve a template value for a specific world.
    ///
    /// Resolution priority:
    /// 1. World-specific override
    /// 2. Global override
    /// 3. Environment variable
    /// 4. Default value (from domain)
    ///
    /// Returns None if the key is not a recognized template.
    async fn resolve_template(
        &self,
        world_id: Option<WorldId>,
        key: &str,
    ) -> Result<Option<String>, RepoError>;
}
