//! Conversation use cases.
//!
//! Handles player-NPC dialogue interactions. The conversation flow is:
//! 1. Player initiates conversation (StartConversation)
//! 2. Action is queued for LLM processing
//! 3. LLM generates NPC response
//! 4. Response goes to DM for approval
//! 5. Approved response is sent to player
//! 6. Player can continue conversation (ContinueConversation)
//! 7. Player ends conversation (EndConversation)

use std::sync::Arc;

mod continue_conversation;
mod end;
mod start;

#[cfg(test)]
mod llm_context_tests;

pub use continue_conversation::{ContinueConversation, ConversationContinued};
pub use end::{ConversationEnded, EndConversation, EndConversationError};
pub use start::{ConversationError, ConversationStarted, StartConversation};

/// Container for conversation use cases.
pub struct ConversationUseCases {
    pub start: Arc<StartConversation>,
    pub continue_conversation: Arc<ContinueConversation>,
    pub end: Arc<EndConversation>,
}

impl ConversationUseCases {
    pub fn new(
        start: Arc<StartConversation>,
        continue_conversation: Arc<ContinueConversation>,
        end: Arc<EndConversation>,
    ) -> Self {
        Self {
            start,
            continue_conversation,
            end,
        }
    }
}
