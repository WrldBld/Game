//! Story event recording service port - Interface for recording gameplay events.
//!
//! This port is part of an Interface Segregation Principle (ISP) split of the
//! `StoryEventService` trait. It contains all the `record_*` methods for recording
//! gameplay events to the story timeline.
//!
//! # ISP Split
//!
//! The original `StoryEventService` trait has been split into focused ports:
//! - `StoryEventServicePort`: Core query operations (get, list)
//! - `StoryEventRecordingServicePort`: Recording operations (this port)
//!
//! Services should depend only on the specific traits they need, following the
//! Interface Segregation Principle.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::{
    ChallengeEventOutcome, DmMarkerType, InfoType, ItemSource, MarkerImportance,
    StoryEventInfoImportance,
};
use wrldbldr_domain::{
    ChallengeId, CharacterId, LocationId, NarrativeEventId, SceneId, StoryEventId, WorldId,
};

/// Port for recording gameplay events to the story timeline.
///
/// This trait provides convenient methods for creating StoryEvents when
/// gameplay actions occur, such as dialogue exchanges, challenge results,
/// scene transitions, and more.
///
/// # Graph-First Architecture
///
/// StoryEvent relationships (session, location, scene, involved characters, triggered_by,
/// recorded_challenge) are stored as graph edges, not embedded fields. Implementations
/// handle creating the event node and all related edges in a single operation.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait StoryEventRecordingServicePort: Send + Sync {
    /// Record a dialogue exchange between player and NPC
    async fn record_dialogue_exchange(
        &self,
        world_id: WorldId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        npc_id: CharacterId,
        npc_name: String,
        player_dialogue: String,
        npc_response: String,
        topics: Vec<String>,
        tone: Option<String>,
        involved_characters: Vec<CharacterId>,
        game_time: Option<String>,
    ) -> Result<StoryEventId>;

    /// Record a challenge attempt and its result
    async fn record_challenge_attempted(
        &self,
        world_id: WorldId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        challenge_id: Option<ChallengeId>,
        challenge_name: String,
        character_id: CharacterId,
        skill_used: Option<String>,
        difficulty: Option<String>,
        roll_result: Option<i32>,
        modifier: Option<i32>,
        outcome: ChallengeEventOutcome,
        game_time: Option<String>,
    ) -> Result<StoryEventId>;

    /// Record a scene transition
    async fn record_scene_transition(
        &self,
        world_id: WorldId,
        from_scene: Option<SceneId>,
        to_scene: SceneId,
        from_scene_name: Option<String>,
        to_scene_name: String,
        trigger_reason: String,
        location_id: Option<LocationId>,
        game_time: Option<String>,
    ) -> Result<StoryEventId>;

    /// Record a DM marker (note, plot point, etc.)
    async fn record_dm_marker(
        &self,
        world_id: WorldId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        title: String,
        note: String,
        importance: MarkerImportance,
        marker_type: DmMarkerType,
        is_hidden: bool,
        tags: Vec<String>,
        game_time: Option<String>,
    ) -> Result<StoryEventId>;

    /// Record information revealed to players
    async fn record_information_revealed(
        &self,
        world_id: WorldId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        info_type: InfoType,
        title: String,
        content: String,
        source: Option<CharacterId>,
        importance: StoryEventInfoImportance,
        persist_to_journal: bool,
        involved_characters: Vec<CharacterId>,
        game_time: Option<String>,
    ) -> Result<StoryEventId>;

    /// Record a relationship change
    async fn record_relationship_changed(
        &self,
        world_id: WorldId,
        scene_id: Option<SceneId>,
        from_character: CharacterId,
        to_character: CharacterId,
        previous_sentiment: Option<f32>,
        new_sentiment: f32,
        reason: String,
        game_time: Option<String>,
    ) -> Result<StoryEventId>;

    /// Record an item being acquired
    async fn record_item_acquired(
        &self,
        world_id: WorldId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        item_name: String,
        item_description: Option<String>,
        character_id: CharacterId,
        source: ItemSource,
        quantity: u32,
        game_time: Option<String>,
    ) -> Result<StoryEventId>;

    /// Record when a narrative event is triggered
    async fn record_narrative_event_triggered(
        &self,
        world_id: WorldId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        narrative_event_id: NarrativeEventId,
        narrative_event_name: String,
        outcome_branch: Option<String>,
        effects_applied: Vec<String>,
        involved_characters: Vec<CharacterId>,
        game_time: Option<String>,
    ) -> Result<StoryEventId>;

    /// Record session start
    async fn record_session_started(
        &self,
        world_id: WorldId,
        session_number: u32,
        session_name: Option<String>,
        players_present: Vec<String>,
    ) -> Result<StoryEventId>;

    /// Record session end
    async fn record_session_ended(
        &self,
        world_id: WorldId,
        duration_minutes: u32,
        summary: String,
    ) -> Result<StoryEventId>;
}
