// Get conversation details use case
#![allow(dead_code)]

//! Get conversation details use case.
//!
//! Allows DMs to view full details of a specific conversation.
//! Returns conversation info, participants, and recent turns.

use std::sync::Arc;
use wrldbldr_domain::{ConversationId, WorldId};

use crate::infrastructure::ports::{NarrativeRepo, RepoError};

// Re-export shared DTOs from helpers module
pub use super::helpers::ConversationDetailResult;

/// Input for getting conversation details.
#[derive(Debug, Clone)]
pub struct GetConversationDetailsInput {
    pub conversation_id: ConversationId,
    pub world_id: WorldId,
}

/// Output for getting conversation details (use-case DTO, not infrastructure type).
pub type GetConversationDetailsOutput = ConversationDetailResult;

/// Errors for get conversation details.
#[derive(Debug, thiserror::Error)]
pub enum GetConversationDetailsError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),

    #[error("Conversation {0} not found")]
    ConversationNotFound(ConversationId),

    #[error("Conversation not in world {0}")]
    WorldMismatch(WorldId),
}

/// Get conversation details use case.
///
/// Retrieves full details for a specific conversation including
/// participants and recent dialogue turns. Used by DMs for monitoring
/// and managing active conversations.
pub struct GetConversationDetails {
    narrative: Arc<dyn NarrativeRepo>,
}

impl GetConversationDetails {
    pub fn new(
        narrative: Arc<dyn NarrativeRepo>,
    ) -> Self {
        Self { narrative }
    }

    /// Get conversation details by ID.
    ///
    /// # Arguments
    /// * `input` - Input containing conversation_id and world_id
    ///
    /// # Returns
    /// * `Ok(ConversationDetailResult)` - Full conversation details (use case DTO)
    /// * `Err(GetConversationDetailsError)` - Failed to get details
    pub async fn execute(
        &self,
        input: GetConversationDetailsInput,
    ) -> Result<GetConversationDetailsOutput, GetConversationDetailsError> {
        let details = self
            .narrative
            .get_conversation_details(input.conversation_id, input.world_id)
            .await?
            .ok_or(GetConversationDetailsError::ConversationNotFound(
                input.conversation_id,
            ))?;

        // Map infrastructure type to use case DTO
        Ok(super::helpers::ConversationDetailResult::from_infrastructure(details))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::MockNarrativeRepo;

    #[tokio::test]
    async fn get_conversation_details_returns_details() {
        let mut mock_narrative = MockNarrativeRepo::new();

        // Setup mock response (infrastructure type)
        let conversation_id = ConversationId::new();
        let expected_details = crate::infrastructure::ports::ConversationDetails {
            conversation: crate::infrastructure::ports::ActiveConversationRecord {
                id: conversation_id,
                pc_id: wrldbldr_domain::PlayerCharacterId::new(),
                npc_id: wrldbldr_domain::CharacterId::new(),
                pc_name: "Test PC".to_string(),
                npc_name: "Test NPC".to_string(),
                topic_hint: Some("Test topic".to_string()),
                started_at: chrono::Utc::now(),
                last_updated_at: chrono::Utc::now(),
                is_active: true,
                turn_count: 5,
                pending_approval: false,
                location: None,
                scene: None,
            },
            participants: vec![],
            recent_turns: vec![],
        };

        mock_narrative
            .expect_get_conversation_details()
            .returning(move |_, _| Ok(Some(expected_details.clone())));

        let use_case = GetConversationDetails::new(
            Arc::new(mock_narrative),
        );

        let input = GetConversationDetailsInput {
            conversation_id,
            world_id: wrldbldr_domain::WorldId::new(),
        };

        let result = use_case.execute(input).await;

        assert!(result.is_ok());
        // Verify we got back a use case DTO, not the infrastructure type
        let details = result.unwrap();
        assert_eq!(details.conversation.id, conversation_id);
        assert_eq!(details.conversation.pc_name, "Test PC");
        assert_eq!(details.conversation.npc_name, "Test NPC");
    }
}
