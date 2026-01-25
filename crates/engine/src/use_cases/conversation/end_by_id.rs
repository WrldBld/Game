// End conversation by ID - methods for DM to force-end conversations
#![allow(dead_code)]

//! End conversation by ID use case.
//!
//! Allows DMs to end a specific conversation by conversation ID.
//! Used for resolving stuck conversations or managing active sessions.

use std::sync::Arc;
use wrldbldr_domain::{CharacterId, ConversationId};

use crate::infrastructure::ports::{CharacterRepo, NarrativeRepo, PlayerCharacterRepo, RepoError};

/// Result of ending a conversation by ID.
#[derive(Debug, Clone)]
pub struct EndedConversation {
    /// The conversation ID that was ended
    pub conversation_id: ConversationId,
    /// Who ended the conversation (if tracked)
    pub ended_by: Option<CharacterId>,
    /// Reason for ending (if provided)
    pub reason: Option<String>,
    /// NPC that was part of this conversation
    pub npc_id: CharacterId,
    pub npc_name: String,
    /// Player character that was part of this conversation
    pub pc_id: wrldbldr_domain::PlayerCharacterId,
    pub pc_name: String,
    /// Optional summary from conversation
    pub summary: Option<String>,
}

/// End conversation by ID use case.
///
/// Ends a specific conversation by conversation_id regardless of who's in it.
/// Used by DMs to force-end stuck conversations or manage active sessions.
pub struct EndConversationById {
    character: Arc<dyn CharacterRepo>,
    player_character: Arc<dyn PlayerCharacterRepo>,
    narrative: Arc<dyn NarrativeRepo>,
}

impl EndConversationById {
    pub fn new(
        character: Arc<dyn CharacterRepo>,
        player_character: Arc<dyn PlayerCharacterRepo>,
        narrative: Arc<dyn NarrativeRepo>,
    ) -> Self {
        Self {
            character,
            player_character,
            narrative,
        }
    }

    /// End a conversation by conversation ID.
    ///
    /// # Arguments
    /// * `conversation_id` - The conversation to end
    /// * `ended_by` - Optional character ID of who ended it (for tracking)
    /// * `reason` - Optional reason for ending
    ///
    /// # Returns
    /// * `Ok(EndedConversation)` - Conversation end data
    /// * `Err(EndConversationByIdError)` - Failed to end conversation
    pub async fn execute(
        &self,
        conversation_id: ConversationId,
        ended_by: Option<CharacterId>,
        reason: Option<String>,
    ) -> Result<EndedConversation, EndConversationByIdError> {
        // 1. Get conversation details first
        let details = self
            .narrative
            .get_conversation_details(conversation_id)
            .await?
            .ok_or(EndConversationByIdError::ConversationNotFound(conversation_id))?;

        // 2. Validate conversation is still active
        if !details.conversation.is_active {
            return Err(EndConversationByIdError::ConversationAlreadyEnded(conversation_id));
        }

        // 3. End the conversation
        let was_ended = self
            .narrative
            .end_conversation_by_id(conversation_id, ended_by, reason.clone())
            .await?;

        if !was_ended {
            // Conversation might have been ended by another process
            return Err(EndConversationByIdError::ConversationAlreadyEnded(conversation_id));
        }

        tracing::info!(
            conversation_id = %conversation_id,
            ended_by = ?ended_by,
            reason = ?reason,
            pc_id = %details.conversation.pc_id,
            npc_id = %details.conversation.npc_id,
            "Conversation ended by ID"
        );

        Ok(EndedConversation {
            conversation_id,
            ended_by,
            reason,
            npc_id: details.conversation.npc_id,
            npc_name: details.conversation.npc_name.clone(),
            pc_id: details.conversation.pc_id,
            pc_name: details.conversation.pc_name.clone(),
            summary: details.conversation.topic_hint,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EndConversationByIdError {
    #[error("Conversation not found: {0}")]
    ConversationNotFound(ConversationId),
    #[error("Conversation already ended: {0}")]
    ConversationAlreadyEnded(ConversationId),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use chrono::Utc;
    use wrldbldr_domain::{CharacterId, ConversationId, PlayerCharacterId, WorldId, LocationId};

    use crate::infrastructure::ports::{
        CharacterRepo, MockCharacterRepo, MockNarrativeRepo, MockPlayerCharacterRepo,
        ParticipantType,
    };

    #[tokio::test]
    async fn when_conversation_not_found_then_returns_error() {
        let conversation_id = ConversationId::new();

        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_get_conversation_details()
            .withf(move |id| *id == conversation_id)
            .returning(|_| Ok(None));

        let use_case = super::EndConversationById::new(
            Arc::new(MockCharacterRepo::new()),
            Arc::new(MockPlayerCharacterRepo::new()),
            Arc::new(narrative_repo),
        );

        let err = use_case
            .execute(conversation_id, None, None)
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            super::EndConversationByIdError::ConversationNotFound(_)
        ));
    }

    #[tokio::test]
    async fn when_conversation_already_ended_then_returns_error() {
        let conversation_id = ConversationId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_get_conversation_details()
            .withf(move |id| *id == conversation_id)
            .returning(move |_| {
                Ok(Some(crate::infrastructure::types::ConversationDetails {
                    conversation: crate::infrastructure::types::ActiveConversationRecord {
                        id: conversation_id,
                        pc_id,
                        npc_id,
                        pc_name: "TestPC".to_string(),
                        npc_name: "TestNPC".to_string(),
                        topic_hint: None,
                        started_at: Utc::now(),
                        last_updated_at: Utc::now(),
                        is_active: false, // Already ended
                        turn_count: 0,
                        pending_approval: false,
                        location: None,
                        scene: None,
                    },
                    participants: vec![],
                    recent_turns: vec![],
                }))
            });

        let use_case = super::EndConversationById::new(
            Arc::new(MockCharacterRepo::new()),
            Arc::new(MockPlayerCharacterRepo::new()),
            Arc::new(narrative_repo),
        );

        let err = use_case
            .execute(conversation_id, None, None)
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            super::EndConversationByIdError::ConversationAlreadyEnded(_)
        ));
    }

    #[tokio::test]
    async fn when_valid_active_conversation_then_ends_and_returns_data() {
        let conversation_id = ConversationId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let ended_by = CharacterId::new();
        let reason = "Forced end by DM".to_string();

        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_get_conversation_details()
            .withf(move |id| *id == conversation_id)
            .returning(move |_| {
                Ok(Some(crate::infrastructure::types::ConversationDetails {
                    conversation: crate::infrastructure::types::ActiveConversationRecord {
                        id: conversation_id,
                        pc_id,
                        npc_id,
                        pc_name: "TestPC".to_string(),
                        npc_name: "TestNPC".to_string(),
                        topic_hint: Some("Test topic".to_string()),
                        started_at: Utc::now(),
                        last_updated_at: Utc::now(),
                        is_active: true,
                        turn_count: 5,
                        pending_approval: false,
                        location: None,
                        scene: None,
                    },
                    participants: vec![],
                    recent_turns: vec![],
                }))
            });

        narrative_repo
            .expect_end_conversation_by_id()
            .withf(move |id, eb, r| {
                *id == conversation_id && *eb == Some(ended_by) && *r == Some(reason.clone())
            })
            .returning(|_, _, _| Ok(true));

        let use_case = super::EndConversationById::new(
            Arc::new(MockCharacterRepo::new()),
            Arc::new(MockPlayerCharacterRepo::new()),
            Arc::new(narrative_repo),
        );

        let result = use_case
            .execute(conversation_id, Some(ended_by), Some(reason.clone()))
            .await
            .expect("EndConversationById should succeed");

        assert_eq!(result.conversation_id, conversation_id);
        assert_eq!(result.ended_by, Some(ended_by));
        assert_eq!(result.reason, Some(reason));
        assert_eq!(result.npc_id, npc_id);
        assert_eq!(result.npc_name, "TestNPC");
        assert_eq!(result.pc_id, pc_id);
        assert_eq!(result.pc_name, "TestPC");
        assert_eq!(result.summary, Some("Test topic".to_string()));
    }
}
