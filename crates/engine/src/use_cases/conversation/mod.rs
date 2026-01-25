// Conversation use cases - fields for future conversation features
#![allow(dead_code)]

//! Conversation use cases.
//!
//! Handles player-NPC dialogue interactions and DM conversation management.
//! The conversation flow is:
//! 1. Player initiates conversation (StartConversation)
//! 2. Action is queued for LLM processing
//! 3. LLM generates NPC response
//! 4. Response goes to DM for approval
//! 5. Approved response is sent to player
//! 6. Player can continue conversation (ContinueConversation)
//! 7. Player ends conversation (EndConversation)
//!
//! DM conversation management:
//! - List active conversations for monitoring
//! - End conversations by ID (force-end stuck sessions)
//! - View conversation details with participants

use std::sync::Arc;

mod continue_conversation;
mod end;
mod end_by_id;
mod get_details;
mod list_active;
mod start;

#[cfg(test)]
mod llm_context_tests;

pub use continue_conversation::ContinueConversation;
pub use end::{EndConversation, EndConversationError};
pub use end_by_id::{EndConversationById, EndConversationByIdError};
pub use get_details::{GetConversationDetails, GetConversationDetailsError, GetConversationDetailsInput};
pub use list_active::{ListActiveConversations, ListActiveConversationsError};
pub use start::{ConversationError, StartConversation};

/// Container for conversation use cases.
pub struct ConversationUseCases {
    pub start: Arc<StartConversation>,
    pub continue_conversation: Arc<ContinueConversation>,
    pub end: Arc<EndConversation>,
    pub end_by_id: Arc<EndConversationById>,
    pub list_active: Arc<ListActiveConversations>,
    pub get_details: Arc<GetConversationDetails>,
}

impl ConversationUseCases {
    pub fn new(
        start: Arc<StartConversation>,
        continue_conversation: Arc<ContinueConversation>,
        end: Arc<EndConversation>,
        end_by_id: Arc<EndConversationById>,
        list_active: Arc<ListActiveConversations>,
        get_details: Arc<GetConversationDetails>,
    ) -> Self {
        Self {
            start,
            continue_conversation,
            end,
            end_by_id,
            list_active,
            get_details,
        }
    }
}
