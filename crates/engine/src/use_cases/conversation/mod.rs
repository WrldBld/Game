//! Conversation use cases.
//!
//! Handles player-NPC dialogue interactions. The conversation flow is:
//! 1. Player initiates conversation (StartConversation)
//! 2. Action is queued for LLM processing
//! 3. LLM generates NPC response
//! 4. Response goes to DM for approval
//! 5. Approved response is sent to player
//! 6. Player can continue conversation (ContinueConversation)

use std::sync::Arc;

mod start;
mod continue_conversation;

pub use start::{StartConversation, ConversationStarted, ConversationError};
pub use continue_conversation::{ContinueConversation, ConversationContinued};

/// Container for conversation use cases.
pub struct ConversationUseCases {
    pub start: Arc<StartConversation>,
    pub continue_conversation: Arc<ContinueConversation>,
}

impl ConversationUseCases {
    pub fn new(
        start: Arc<StartConversation>,
        continue_conversation: Arc<ContinueConversation>,
    ) -> Self {
        Self {
            start,
            continue_conversation,
        }
    }
}
