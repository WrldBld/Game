//! DM Conversation Management Service
//!
//! Provides methods for DM to manage conversations:
//! - List active conversations
//! - Get conversation details
//! - End conversation by ID
//!
use crate::application::ServiceError;
use crate::infrastructure::messaging::CommandBus;
use crate::infrastructure::websocket::ClientMessageBuilder;
use wrldbldr_shared::RequestError;

/// Service for DM conversation management operations
#[derive(Clone)]
pub struct ConversationService {
    /// Command bus for sending WebSocket messages
    command_bus: CommandBus,
}

impl ConversationService {
    /// Create a new ConversationService
    pub fn new(command_bus: CommandBus) -> Self {
        Self { command_bus }
    }

    /// List all active conversations for a world
    pub fn list_active_conversations(
        &self,
        world_id: uuid::Uuid,
        include_ended: bool,
    ) -> Result<(), ServiceError> {
        let msg = ClientMessageBuilder::list_active_conversations(world_id, include_ended);
        self.command_bus
            .send(msg)
            .map_err(|e| ServiceError::Request(RequestError::SendFailed(e.to_string())))?;
        Ok(())
    }

    /// Get details for a specific conversation
    pub fn get_conversation_details(
        &self,
        conversation_id: uuid::Uuid,
    ) -> Result<(), ServiceError> {
        let msg = ClientMessageBuilder::get_conversation_details(conversation_id);
        self.command_bus
            .send(msg)
            .map_err(|e| ServiceError::Request(RequestError::SendFailed(e.to_string())))?;
        Ok(())
    }

    /// End a specific conversation by ID
    pub fn end_conversation_by_id(
        &self,
        conversation_id: uuid::Uuid,
        reason: Option<&str>,
    ) -> Result<(), ServiceError> {
        let msg = ClientMessageBuilder::end_conversation_by_id(conversation_id, reason);
        self.command_bus
            .send(msg)
            .map_err(|e| ServiceError::Request(RequestError::SendFailed(e.to_string())))?;
        Ok(())
    }
}
