//! Continue conversation use case.
//!
//! Handles continuing an existing conversation between a player character and an NPC.
//! This use case validates the interaction context and enqueues the player's response
//! for LLM processing.

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{CharacterId, PlayerActionData, PlayerCharacterId, WorldId};

use crate::entities::{Character, PlayerCharacter, Staging};
use crate::infrastructure::ports::{ClockPort, QueuePort, RepoError};

/// Response from continuing a conversation.
#[derive(Debug)]
pub struct ConversationContinued {
    /// ID of the queued player action
    pub action_queue_id: Uuid,
    /// The conversation is still active
    pub conversation_active: bool,
}

/// Continue conversation use case.
///
/// Orchestrates: Context validation, player action queuing.
pub struct ContinueConversation {
    character: Arc<Character>,
    player_character: Arc<PlayerCharacter>,
    staging: Arc<Staging>,
    queue: Arc<dyn QueuePort>,
    clock: Arc<dyn ClockPort>,
}

impl ContinueConversation {
    pub fn new(
        character: Arc<Character>,
        player_character: Arc<PlayerCharacter>,
        staging: Arc<Staging>,
        queue: Arc<dyn QueuePort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            character,
            player_character,
            staging,
            queue,
            clock,
        }
    }

    /// Continue an existing conversation with an NPC.
    ///
    /// # Arguments
    /// * `world_id` - The world context
    /// * `pc_id` - The player character in the conversation
    /// * `npc_id` - The NPC being spoken to
    /// * `player_id` - The player's user ID
    /// * `player_message` - The player's response message
    ///
    /// # Returns
    /// * `Ok(ConversationContinued)` - Response queued for processing
    /// * `Err(ConversationError)` - Failed to continue conversation
    pub async fn execute(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        player_id: String,
        player_message: String,
    ) -> Result<ConversationContinued, ConversationError> {
        // 1. Validate the player character exists
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ConversationError::PlayerCharacterNotFound)?;

        // 2. Get the NPC
        let npc = self
            .character
            .get(npc_id)
            .await?
            .ok_or(ConversationError::NpcNotFound)?;

        // 3. Verify the NPC is still in the same region as the PC
        let pc_region_id = pc.current_region_id
            .ok_or(ConversationError::PlayerNotInRegion)?;

        let staged_npcs = self.staging.resolve_for_region(pc_region_id).await?;
        let npc_in_region = staged_npcs.iter().any(|staged| staged.character_id == npc_id);

        if !npc_in_region {
            // NPC left the region - conversation is over
            return Err(ConversationError::NpcLeftRegion);
        }

        // 4. Enqueue the player action for processing
        let action_data = PlayerActionData {
            world_id,
            player_id,
            pc_id: Some(pc_id),
            action_type: "speak".to_string(),
            target: Some(npc.name.clone()),
            dialogue: Some(player_message),
            timestamp: self.clock.now(),
        };

        let action_queue_id = self
            .queue
            .enqueue_player_action(&action_data)
            .await
            .map_err(|e| ConversationError::QueueError(e.to_string()))?;

        Ok(ConversationContinued {
            action_queue_id,
            conversation_active: true,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("Player character not found")]
    PlayerCharacterNotFound,
    #[error("NPC not found")]
    NpcNotFound,
    #[error("Player is not in a region")]
    PlayerNotInRegion,
    #[error("NPC has left the region")]
    NpcLeftRegion,
    #[error("Queue error: {0}")]
    QueueError(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
