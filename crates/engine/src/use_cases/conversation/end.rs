//! End conversation use case.
//!
//! Handles ending a conversation between a player character and an NPC.
//! Returns the conversation end result; the caller (websocket handler)
//! is responsible for broadcasting to clients.

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{CharacterId, PlayerCharacterId};

use crate::entities::{Character, Narrative, PlayerCharacter};
use crate::infrastructure::ports::RepoError;

/// Result of ending a conversation.
#[derive(Debug, Clone)]
pub struct ConversationEnded {
    /// The NPC the conversation was with
    pub npc_id: CharacterId,
    pub npc_name: String,
    /// The player character who was conversing
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    /// Optional summary of the conversation
    pub summary: Option<String>,
    /// The conversation ID that was ended (if any)
    pub conversation_id: Option<Uuid>,
}

/// End conversation use case.
///
/// Validates the PC and NPC exist, ends the active conversation tracking,
/// and returns conversation end data.
/// The caller is responsible for broadcasting the result to clients.
///
/// Future enhancements could include:
/// - Optionally save conversation summary to persistent storage
/// - Notify any listeners/subscribers that the conversation has ended
/// - Update NPC disposition based on conversation outcome
pub struct EndConversation {
    character: Arc<Character>,
    player_character: Arc<PlayerCharacter>,
    narrative: Arc<Narrative>,
}

impl EndConversation {
    pub fn new(
        character: Arc<Character>,
        player_character: Arc<PlayerCharacter>,
        narrative: Arc<Narrative>,
    ) -> Self {
        Self {
            character,
            player_character,
            narrative,
        }
    }

    /// End a conversation with an NPC.
    ///
    /// # Arguments
    /// * `pc_id` - The player character ending the conversation
    /// * `npc_id` - The NPC the conversation was with
    /// * `summary` - Optional summary of the conversation
    ///
    /// # Returns
    /// * `Ok(ConversationEnded)` - Conversation end data for broadcasting
    /// * `Err(EndConversationError)` - Failed to end conversation
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        summary: Option<String>,
    ) -> Result<ConversationEnded, EndConversationError> {
        // 1. Validate the player character exists
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(EndConversationError::PlayerCharacterNotFound)?;

        // 2. Get the NPC
        let npc = self
            .character
            .get(npc_id)
            .await?
            .ok_or(EndConversationError::NpcNotFound)?;

        // 3. End the active conversation tracking (clear active conversation state)
        // This atomically finds and ends the active conversation between PC and NPC
        let ended_conversation_id = match self
            .narrative
            .end_active_conversation(pc_id, npc_id)
            .await
        {
            Ok(id) => {
                if let Some(conv_id) = &id {
                    tracing::info!(
                        conversation_id = %conv_id,
                        pc_id = %pc_id,
                        npc_id = %npc_id,
                        "Marked conversation as ended"
                    );
                } else {
                    tracing::debug!(
                        pc_id = %pc_id,
                        npc_id = %npc_id,
                        "No active conversation found to end"
                    );
                }
                id
            }
            Err(e) => {
                // Log but don't fail - the conversation end should still succeed
                // even if we can't update the tracking state
                tracing::warn!(
                    error = %e,
                    pc_id = %pc_id,
                    npc_id = %npc_id,
                    "Failed to end active conversation tracking, proceeding anyway"
                );
                None
            }
        };

        tracing::info!(
            pc_id = %pc_id,
            pc_name = %pc.name,
            npc_id = %npc_id,
            npc_name = %npc.name,
            conversation_id = ?ended_conversation_id,
            "Conversation ended"
        );

        Ok(ConversationEnded {
            npc_id,
            npc_name: npc.name,
            pc_id,
            pc_name: pc.name,
            summary,
            conversation_id: ended_conversation_id,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EndConversationError {
    #[error("Player character not found")]
    PlayerCharacterNotFound,
    #[error("NPC not found")]
    NpcNotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
