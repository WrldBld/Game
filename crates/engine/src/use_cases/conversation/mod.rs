//! Conversation use cases.

use std::sync::Arc;

mod start;
mod continue_conversation;

pub use start::StartConversation;
pub use continue_conversation::ContinueConversation;

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
