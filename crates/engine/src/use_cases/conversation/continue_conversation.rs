//! Continue conversation use case.

use std::sync::Arc;

use crate::entities::Character;
use crate::infrastructure::ports::LlmPort;

/// Continue conversation use case.
pub struct ContinueConversation {
    #[allow(dead_code)]
    character: Arc<Character>,
    #[allow(dead_code)]
    llm: Arc<dyn LlmPort>,
}

impl ContinueConversation {
    pub fn new(character: Arc<Character>, llm: Arc<dyn LlmPort>) -> Self {
        Self { character, llm }
    }

    /// Continue an existing conversation.
    pub async fn execute(
        &self,
        _conversation_id: uuid::Uuid,
        _player_message: &str,
    ) -> Result<ConversationResponse, super::start::ConversationError> {
        // TODO: Implement
        todo!("Continue conversation use case")
    }
}

#[derive(Debug)]
pub struct ConversationResponse {
    pub npc_response: String,
    pub suggested_actions: Vec<String>,
}
