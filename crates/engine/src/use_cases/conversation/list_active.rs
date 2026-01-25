//!
//! Lists all active conversations in a world for DM monitoring.
//! Returns conversation info with participants, location, and status.

use std::sync::Arc;

use wrldbldr_domain::{ConversationId, PlayerCharacterId, WorldId};

use crate::infrastructure::ports::{CharacterRepo, NarrativeRepo, PlayerCharacterRepo, RepoError};
use wrldbldr_shared::messages::{
    ConversationInfo as ProtocolConversationInfo,
    ConversationParticipant as ProtocolConversationParticipant,
    LocationContext as ProtocolLocationContext,
    ParticipantType as ProtocolParticipantType, SceneContext as ProtocolSceneContext,
};

/// Information about an active conversation for DM monitoring.
#[derive(Debug, Clone)]
pub struct ConversationInfo {
    /// The conversation ID
    pub conversation_id: ConversationId,
    /// Optional topic hint (derived from recent dialogue or summary)
    pub topic_hint: Option<String>,
    /// When conversation started
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Last time conversation was updated (new turn, etc.)
    pub last_updated_at: chrono::DateTime<chrono::Utc>,
    /// Whether conversation is still active
    pub is_active: bool,
    /// Participants in this conversation
    pub participants: Vec<ConversationParticipant>,
    /// Location context (if available)
    pub location: Option<LocationContext>,
    /// Scene context (if available)
    pub scene: Option<SceneContext>,
    /// Number of dialogue turns
    pub turn_count: u32,
    /// Whether there's pending DM approval for this conversation
    pub pending_approval: bool,
}

/// A participant in a conversation.
#[derive(Debug, Clone)]
pub struct ConversationParticipant {
    /// Character ID
    pub id: wrldbldr_domain::CharacterId,
    /// Display name
    pub name: String,
    /// PC or NPC
    pub participant_type: ParticipantType,
    /// Number of turns this character has spoken
    pub turn_count: u32,
    /// Last time this character spoke
    pub last_spoke_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Type of participant (PC or NPC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantType {
    Pc,
    Npc,
}

/// Location context for a conversation.
#[derive(Debug, Clone)]
pub struct LocationContext {
    pub location_id: wrldbldr_domain::LocationId,
    pub location_name: String,
    pub region_name: String,
}

/// Scene context for a conversation.
#[derive(Debug, Clone)]
pub struct SceneContext {
    pub scene_id: wrldbldr_domain::SceneId,
    pub scene_name: String,
}

/// Result of listing active conversations.
#[derive(Debug, Clone)]
pub struct ListActiveConversationsResult {
    pub conversations: Vec<ConversationInfo>,
}

/// Error types for list active conversations use case.
#[derive(Debug, thiserror::Error)]
pub enum ListActiveConversationsError {
    #[error("World not found: {0}")]
    WorldNotFound(WorldId),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

/// List active conversations use case.
///
/// Returns all active conversations in a world for DM monitoring.
/// Includes participant info, location context, and turn counts.
pub struct ListActiveConversations {
    character: Arc<dyn CharacterRepo>,
    player_character: Arc<dyn PlayerCharacterRepo>,
    narrative: Arc<dyn NarrativeRepo>,
}

impl ListActiveConversations {
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

    /// List all active conversations in a world.
    ///
    /// # Arguments
    /// * `world_id` - The world to list conversations for
    ///
    /// # Returns
    /// * `Ok(ListActiveConversationsResult)` - List of active conversations
    /// * `Err(ListActiveConversationsError)` - Failed to list conversations
    pub async fn execute(
        &self,
        world_id: WorldId,
    ) -> Result<ListActiveConversationsResult, ListActiveConversationsError> {
        // Get active conversations from narrative repo
        let conversations = self
            .narrative
            .list_active_conversations(world_id)
            .await
            .map_err(|e| ListActiveConversationsError::Repo(e))?;

        // Build conversation info with participant details
        let mut conversation_infos = Vec::new();
        for conv in conversations {
            // Build participant list
            let participants = self
                .build_participants(&conv.pc_id, &conv.npc_id, &conv.pc_name, &conv.npc_name)
                .await?;

            conversation_infos.push(ConversationInfo {
                conversation_id: conv.id,
                topic_hint: conv.topic_hint,
                started_at: conv.started_at,
                last_updated_at: conv.last_updated_at,
                is_active: conv.is_active,
                participants,
                location: conv.location.map(|l| LocationContext {
                    location_id: l.location_id,
                    location_name: l.location_name,
                    region_name: l.region_name,
                }),
                scene: conv.scene.map(|s| SceneContext {
                    scene_id: s.scene_id,
                    scene_name: s.scene_name,
                }),
                turn_count: conv.turn_count,
                pending_approval: conv.pending_approval,
            });
        }

        Ok(ListActiveConversationsResult {
            conversations: conversation_infos,
        })
    }

    /// Build participant list for a conversation.
    async fn build_participants(
        &self,
        pc_id: &PlayerCharacterId,
        npc_id: &wrldbldr_domain::CharacterId,
        pc_name: &str,
        npc_name: &str,
    ) -> Result<Vec<ConversationParticipant>, RepoError> {
        let mut participants = Vec::new();

        // For now, use placeholders - turn tracking not implemented
        participants.push(ConversationParticipant {
            id: wrldbldr_domain::CharacterId::from(pc_id.to_uuid()),
            name: pc_name.to_string(),
            participant_type: ParticipantType::Pc,
            turn_count: 0,
            last_spoke_at: None,
        });

        participants.push(ConversationParticipant {
            id: *npc_id,
            name: npc_name.to_string(),
            participant_type: ParticipantType::Npc,
            turn_count: 0,
            last_spoke_at: None,
        });

        Ok(participants)
    }
}

impl ConversationInfo {
    /// Convert to protocol message type.
    pub fn to_protocol(&self) -> ProtocolConversationInfo {
        ProtocolConversationInfo {
            conversation_id: self.conversation_id.to_string(),
            topic_hint: self.topic_hint.clone(),
            started_at: self.started_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            last_updated_at: self
                .last_updated_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            is_active: self.is_active,
            participants: self
                .participants
                .iter()
                .map(|p| p.to_protocol())
                .collect(),
            location: self.location.as_ref().map(|l| l.to_protocol()),
            scene: self.scene.as_ref().map(|s| s.to_protocol()),
            turn_count: self.turn_count,
            pending_approval: self.pending_approval,
        }
    }
}

impl ConversationParticipant {
    /// Convert to protocol message type.
    pub fn to_protocol(&self) -> ProtocolConversationParticipant {
        ProtocolConversationParticipant {
            id: self.id.to_string(),
            name: self.name.clone(),
            participant_type: match self.participant_type {
                ParticipantType::Pc => ProtocolParticipantType::Pc,
                ParticipantType::Npc => ProtocolParticipantType::Npc,
            },
            turn_count: self.turn_count,
            last_spoke_at: self.last_spoke_at.as_ref().map(|dt| {
                dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            }),
            want: None,
            relationship: None,
        }
    }
}

impl LocationContext {
    /// Convert to protocol message type.
    pub fn to_protocol(&self) -> ProtocolLocationContext {
        ProtocolLocationContext {
            location_id: self.location_id.to_string(),
            location_name: self.location_name.clone(),
            region_name: self.region_name.clone(),
        }
    }
}

impl SceneContext {
    /// Convert to protocol message type.
    pub fn to_protocol(&self) -> ProtocolSceneContext {
        ProtocolSceneContext {
            scene_id: self.scene_id.to_string(),
            scene_name: self.scene_name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::neo4j::narrative_repo::tests::MockNarrativeRepo;
    use crate::infrastructure::neo4j::test_helpers::MockCharacterRepo;
    use crate::infrastructure::neo4j::test_helpers::MockPlayerCharacterRepo;
    use wrldbldr_domain::{CharacterId, PlayerCharacterId, WorldId};

    #[tokio::test]
    async fn list_conversations_returns_empty_list() {
        let mut mock_narrative = MockNarrativeRepo::new();
        let mut mock_character = MockCharacterRepo::new();
        let mut mock_pc = MockPlayerCharacterRepo::new();

        // Setup mocks
        mock_narrative
            .expect_list_active_conversations()
            .returning(move |_| Ok(vec![]));

        let use_case = ListActiveConversations::new(
            Arc::new(mock_character.clone()),
            Arc::new(mock_pc.clone()),
            Arc::new(mock_narrative),
        );

        let world_id = WorldId::new();
        let result = use_case.execute(world_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().conversations.len(), 0);
    }
}
