//!
//! Lists all active conversations in a world for DM monitoring.
//! Returns conversation info with participants, location, and status.

use std::sync::Arc;

use wrldbldr_domain::WorldId;

use crate::infrastructure::ports::{NarrativeRepo, RepoError};

/// Result of listing active conversations.
#[derive(Debug, Clone)]
pub struct ListActiveConversationsResult {
    pub conversations: Vec<crate::infrastructure::ports::ActiveConversationRecord>,
}

/// Error types for list active conversations use case.
#[derive(Debug, thiserror::Error)]
pub enum ListActiveConversationsError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

/// List active conversations use case.
///
/// Returns all conversations in a world for DM monitoring.
/// Includes participant info, location context, and turn counts.
/// Note: Protocol conversion is handled in conversation_protocol.rs (API layer).
pub struct ListActiveConversations {
    narrative: Arc<dyn NarrativeRepo>,
}

impl ListActiveConversations {
    pub fn new(
        narrative: Arc<dyn NarrativeRepo>,
    ) -> Self {
        Self {
            narrative,
        }
    }

    /// List all conversations in a world.
    ///
    /// # Arguments
    /// * `world_id` - The world to list conversations for
    /// * `include_ended` - If true, includes ended conversations; if false, only active conversations
    ///
    /// # Returns
    /// * `Ok(ListActiveConversationsResult)` - List of conversations (empty if world has no conversations)
    /// * `Err(ListActiveConversationsError)` - Failed to list conversations
    pub async fn execute(
        &self,
        world_id: WorldId,
        include_ended: bool,
    ) -> Result<ListActiveConversationsResult, ListActiveConversationsError> {
        // Get conversations from narrative repo
        // Note: If world doesn't exist or has no conversations, repo returns empty list
        // This is semantically correct - no conversations exist for that world
        let conversations = self
            .narrative
            .list_active_conversations(world_id, include_ended)
            .await?;

        Ok(ListActiveConversationsResult {
            conversations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::MockNarrativeRepo;
    use wrldbldr_domain::{WorldId};

    #[tokio::test]
    async fn list_conversations_returns_empty_list() {
        let mut mock_narrative = MockNarrativeRepo::new();

        // Setup mocks
        mock_narrative
            .expect_list_active_conversations()
            .returning(move |_, _| Ok(vec![]));

        let use_case = ListActiveConversations::new(
            Arc::new(mock_narrative),
        );

        let world_id = WorldId::new();
        let result = use_case.execute(world_id, false).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().conversations.len(), 0);
    }
}
