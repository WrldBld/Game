//! Dialogue context service port - Interface for dialogue context operations
//!
//! This port provides dialogue-specific operations for LLM prompt context building
//! and the staging system. It handles recording dialogue exchanges to the timeline,
//! retrieving recent dialogue history with NPCs, and maintaining PC-NPC relationship
//! edges that track conversational interactions.
//!
//! Note: `record_dialogue_exchange` intentionally appears in both this port and
//! `StoryEventRecordingServicePort` as dialogue recording serves both concerns:
//! - Timeline recording (story events)
//! - Dialogue context for LLM/staging

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::StoryEvent;
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, SceneId, StoryEventId, WorldId};

/// Port for dialogue context service operations
///
/// This trait defines operations for managing dialogue context, including
/// recording dialogue exchanges, retrieving dialogue history, and maintaining
/// PC-NPC relationship edges for the staging system.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait DialogueContextServicePort: Send + Sync {
    /// Record a dialogue exchange between player and NPC
    ///
    /// Creates a story event capturing the dialogue exchange and updates
    /// relationship tracking. This is a dual-concern method that serves both
    /// timeline recording and dialogue context building.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world this dialogue occurs in
    /// * `scene_id` - Optional scene context
    /// * `location_id` - Optional location context
    /// * `npc_id` - The NPC being spoken to
    /// * `npc_name` - Display name of the NPC
    /// * `player_dialogue` - What the player said
    /// * `npc_response` - The NPC's response
    /// * `topics` - Topics discussed in this exchange
    /// * `tone` - Optional tone/mood of the conversation
    /// * `involved_characters` - Other characters involved/mentioned
    /// * `game_time` - Optional in-game timestamp
    ///
    /// # Returns
    ///
    /// The ID of the created story event
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

    /// Get recent dialogue exchanges with a specific NPC
    ///
    /// Returns the raw DialogueExchange events for further processing.
    /// Useful for building detailed dialogue context or analysis.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to query
    /// * `npc_id` - The NPC to get dialogue history for
    /// * `limit` - Maximum number of exchanges to return
    ///
    /// # Returns
    ///
    /// A vector of story events representing dialogue exchanges, ordered
    /// by most recent first
    async fn get_dialogues_with_npc(
        &self,
        world_id: WorldId,
        npc_id: CharacterId,
        limit: u32,
    ) -> Result<Vec<StoryEvent>>;

    /// Get a summarized view of recent dialogues with an NPC for LLM context
    ///
    /// Returns a string summary suitable for including in LLM prompts.
    /// The summary includes the last `limit` conversations with topics discussed.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to query
    /// * `npc_id` - The NPC to get dialogue summary for
    /// * `limit` - Maximum number of exchanges to summarize
    ///
    /// # Returns
    ///
    /// An optional string containing the formatted dialogue summary,
    /// or None if no dialogue history exists
    async fn get_dialogue_summary_for_npc(
        &self,
        world_id: WorldId,
        npc_id: CharacterId,
        limit: u32,
    ) -> Result<Option<String>>;

    /// Update or create a SPOKE_TO edge between a PlayerCharacter and an NPC
    ///
    /// This should be called after a dialogue exchange is recorded to maintain
    /// the relationship metadata used by the Staging System. The edge tracks
    /// the most recent conversation topic and timestamp.
    ///
    /// # Arguments
    ///
    /// * `pc_id` - The player character who spoke
    /// * `npc_id` - The NPC they spoke to
    /// * `topic` - Optional topic of conversation to record on the edge
    async fn update_spoke_to_edge(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        topic: Option<String>,
    ) -> Result<()>;
}
