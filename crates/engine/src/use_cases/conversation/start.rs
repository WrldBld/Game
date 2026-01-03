//! Start conversation use case.

use std::sync::Arc;

use crate::entities::Character;
use crate::infrastructure::ports::LlmPort;

/// Start conversation use case.
pub struct StartConversation {
    #[allow(dead_code)]
    character: Arc<Character>,
    #[allow(dead_code)]
    llm: Arc<dyn LlmPort>,
}

impl StartConversation {
    pub fn new(character: Arc<Character>, llm: Arc<dyn LlmPort>) -> Self {
        Self { character, llm }
    }

    /// Start a conversation with an NPC.
    pub async fn execute(
        &self,
        _pc_id: wrldbldr_domain::PlayerCharacterId,
        _npc_id: wrldbldr_domain::CharacterId,
    ) -> Result<ConversationStarted, ConversationError> {
        // TODO: Implement
        todo!("Start conversation use case")
    }
}

#[derive(Debug)]
pub struct ConversationStarted {
    pub conversation_id: uuid::Uuid,
    pub greeting: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("NPC not found")]
    NpcNotFound,
    #[error("LLM error: {0}")]
    Llm(String),
}
