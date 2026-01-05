//! Start conversation use case.
//!
//! Handles initiating a conversation between a player character and an NPC.
//! This use case validates the interaction is possible and enqueues the
//! player action for LLM processing.

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{CharacterId, PlayerActionData, PlayerCharacterId, WorldId};

use crate::entities::{Character, PlayerCharacter, Scene, Staging};
use crate::infrastructure::ports::{ClockPort, QueuePort, RepoError};

/// Result of starting a conversation.
#[derive(Debug)]
pub struct ConversationStarted {
    /// Unique ID for this conversation session
    pub conversation_id: Uuid,
    /// ID of the queued player action
    pub action_queue_id: Uuid,
    /// NPC name for display
    pub npc_name: String,
    /// NPC's current disposition toward the PC (if available)
    pub npc_disposition: Option<String>,
}

/// Start conversation use case.
///
/// Orchestrates: NPC validation, staging check, player action queuing.
pub struct StartConversation {
    character: Arc<Character>,
    player_character: Arc<PlayerCharacter>,
    staging: Arc<Staging>,
    scene: Arc<Scene>,
    queue: Arc<dyn QueuePort>,
    clock: Arc<dyn ClockPort>,
}

impl StartConversation {
    pub fn new(
        character: Arc<Character>,
        player_character: Arc<PlayerCharacter>,
        staging: Arc<Staging>,
        scene: Arc<Scene>,
        queue: Arc<dyn QueuePort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            character,
            player_character,
            staging,
            scene,
            queue,
            clock,
        }
    }

    /// Start a conversation with an NPC.
    ///
    /// # Arguments
    /// * `world_id` - The world context
    /// * `pc_id` - The player character initiating the conversation
    /// * `npc_id` - The NPC to converse with
    /// * `player_id` - The player's user ID
    /// * `initial_dialogue` - The player's opening message
    ///
    /// # Returns
    /// * `Ok(ConversationStarted)` - Conversation initiated, action queued
    /// * `Err(ConversationError)` - Failed to start conversation
    pub async fn execute(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        player_id: String,
        initial_dialogue: String,
    ) -> Result<ConversationStarted, ConversationError> {
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

        // 3. Verify the NPC is in the same region as the PC
        let pc_region_id = pc.current_region_id
            .ok_or(ConversationError::PlayerNotInRegion)?;

        // Check if NPC is staged in this region
        let staged_npcs = self.staging.resolve_for_region(pc_region_id).await?;
        let npc_in_region = staged_npcs.iter().any(|staged| staged.character_id == npc_id);

        if !npc_in_region {
            return Err(ConversationError::NpcNotInRegion);
        }

        // 4. Generate conversation ID
        let conversation_id = Uuid::new_v4();

        // 5. Get NPC's current disposition toward the PC if available
        let npc_disposition = self
            .character
            .get_disposition(npc_id, pc_id)
            .await
            .ok()
            .flatten()
            .map(|d| format!("{:?}", d.disposition));

        // 6. Enqueue the player action for processing
        let action_data = PlayerActionData {
            world_id,
            player_id,
            pc_id: Some(pc_id),
            action_type: "speak".to_string(),
            target: Some(npc.name.clone()),
            dialogue: Some(initial_dialogue),
            timestamp: self.clock.now(),
        };

        let action_queue_id = self
            .queue
            .enqueue_player_action(&action_data)
            .await
            .map_err(|e| ConversationError::QueueError(e.to_string()))?;

        Ok(ConversationStarted {
            conversation_id,
            action_queue_id,
            npc_name: npc.name,
            npc_disposition,
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
    #[error("NPC is not in the player's region")]
    NpcNotInRegion,
    #[error("NPC has left the region")]
    NpcLeftRegion,
    #[error("Queue error: {0}")]
    QueueError(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
