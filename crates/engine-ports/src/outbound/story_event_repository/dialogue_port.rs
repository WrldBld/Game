//! Dialogue-specific operations for StoryEvent entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{CharacterId, PlayerCharacterId, StoryEvent, WorldId};

/// Dialogue-specific operations for StoryEvent entities.
///
/// This trait handles specialized dialogue history tracking:
/// - Get recent dialogues with specific NPCs
/// - Track conversation metadata (SPOKE_TO edges)
///
/// # Used By
/// - `StagingContextProvider` - For building NPC context
/// - `ActantialContextService` - For relationship tracking
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait StoryEventDialoguePort: Send + Sync {
    /// Get recent dialogue exchanges with a specific NPC
    ///
    /// Returns DialogueExchange events involving the specified NPC,
    /// ordered by timestamp descending (most recent first).
    ///
    /// Used by the Staging System to provide LLM context about
    /// recent conversations with NPCs who might be present.
    async fn get_dialogues_with_npc(
        &self,
        world_id: WorldId,
        npc_id: CharacterId,
        limit: u32,
    ) -> Result<Vec<StoryEvent>>;

    /// Update or create a SPOKE_TO edge between a PlayerCharacter and an NPC
    ///
    /// This edge tracks conversation history metadata:
    /// - `last_dialogue_at`: When the most recent dialogue occurred
    /// - `last_topic`: Primary topic of the last conversation (optional)
    /// - `conversation_count`: Total number of conversations
    ///
    /// Used by the Staging System to understand PC-NPC relationship history.
    async fn update_spoke_to_edge(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        topic: Option<String>,
    ) -> Result<()>;
}
