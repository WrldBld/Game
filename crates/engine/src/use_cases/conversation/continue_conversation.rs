//! Continue conversation use case.
//!
//! Handles continuing an existing conversation between a player character and an NPC.
//! This use case validates the interaction context and enqueues the player's response
//! for LLM processing.

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{CharacterId, PlayerActionData, PlayerCharacterId, WorldId};

use crate::entities::{Character, Narrative, PlayerCharacter, Staging, World};
use crate::infrastructure::ports::{ClockPort, QueuePort};

// Re-use the shared ConversationError from start.rs
use super::start::ConversationError;

/// Response from continuing a conversation.
#[derive(Debug)]
pub struct ConversationContinued {
    /// ID of the queued player action
    pub action_queue_id: Uuid,
    /// The conversation is still active
    pub conversation_active: bool,
    /// The conversation ID for tracking
    pub conversation_id: Option<Uuid>,
}

/// Continue conversation use case.
///
/// Orchestrates: Context validation, player action queuing.
pub struct ContinueConversation {
    character: Arc<Character>,
    player_character: Arc<PlayerCharacter>,
    staging: Arc<Staging>,
    world: Arc<World>,
    narrative: Arc<Narrative>,
    queue: Arc<dyn QueuePort>,
    clock: Arc<dyn ClockPort>,
}

impl ContinueConversation {
    pub fn new(
        character: Arc<Character>,
        player_character: Arc<PlayerCharacter>,
        staging: Arc<Staging>,
        world: Arc<World>,
        narrative: Arc<Narrative>,
        queue: Arc<dyn QueuePort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            character,
            player_character,
            staging,
            world,
            narrative,
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
    /// * `conversation_id` - Optional conversation ID (if not provided, looks up active conversation)
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
        conversation_id: Option<Uuid>,
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
        let pc_region_id = pc
            .current_region_id
            .ok_or(ConversationError::PlayerNotInRegion)?;

        // Get current game time for staging TTL check
        let world_data = self
            .world
            .get(world_id)
            .await?
            .ok_or(ConversationError::WorldNotFound)?;
        let current_game_time = world_data.game_time.current();

        let staged_npcs = self
            .staging
            .resolve_for_region(pc_region_id, current_game_time)
            .await?;
        let npc_in_region = staged_npcs
            .iter()
            .any(|staged| staged.character_id == npc_id);

        if !npc_in_region {
            // NPC left the region - conversation is over
            return Err(ConversationError::NpcLeftRegion);
        }

        // 4. Resolve conversation_id: use provided one or look up active conversation
        let resolved_conversation_id = if let Some(id) = conversation_id {
            Some(id)
        } else {
            // Look up active conversation between PC and NPC
            match self
                .narrative
                .get_active_conversation_id(pc_id, npc_id)
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        pc_id = %pc_id,
                        npc_id = %npc_id,
                        "Failed to look up active conversation, proceeding without ID"
                    );
                    None
                }
            }
        };

        // 5. Enqueue the player action for processing
        let action_data = PlayerActionData {
            world_id,
            player_id,
            pc_id: Some(pc_id),
            action_type: "talk".to_string(),
            target: Some(npc.name.clone()),
            dialogue: Some(player_message),
            timestamp: self.clock.now(),
            conversation_id: resolved_conversation_id,
        };

        let action_queue_id = self
            .queue
            .enqueue_player_action(&action_data)
            .await
            .map_err(|e| ConversationError::QueueError(e.to_string()))?;

        Ok(ConversationContinued {
            action_queue_id,
            conversation_active: true,
            conversation_id: resolved_conversation_id,
        })
    }
}
