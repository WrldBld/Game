//! End conversation use case.
//!
//! Handles ending a conversation between a player character and an NPC.
//! Returns the conversation end result; the caller (websocket handler)
//! is responsible for broadcasting to clients.

use std::sync::Arc;
use wrldbldr_domain::{CharacterId, PlayerCharacterId};

use crate::entities::{Character, PlayerCharacter};
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
}

/// End conversation use case.
///
/// Validates the PC and NPC exist and returns conversation end data.
/// The caller is responsible for broadcasting the result to clients.
pub struct EndConversation {
    character: Arc<Character>,
    player_character: Arc<PlayerCharacter>,
}

impl EndConversation {
    pub fn new(character: Arc<Character>, player_character: Arc<PlayerCharacter>) -> Self {
        Self {
            character,
            player_character,
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

        tracing::info!(
            pc_id = %pc_id,
            pc_name = %pc.name,
            npc_id = %npc_id,
            npc_name = %npc.name,
            "Conversation ended"
        );

        Ok(ConversationEnded {
            npc_id,
            npc_name: npc.name,
            pc_id,
            pc_name: pc.name,
            summary,
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
