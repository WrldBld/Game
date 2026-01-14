//! End conversation use case.
//!
//! Handles ending a conversation between a player character and an NPC.
//! Returns the conversation end result; the caller (websocket handler)
//! is responsible for broadcasting to clients.

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{CharacterId, PlayerCharacterId};

use crate::repositories::PlayerCharacter;
use crate::repositories::character::Character;
use crate::use_cases::narrative_operations::Narrative;
use crate::infrastructure::ports::RepoError;

/// Result of ending a conversation.
#[derive(Debug, Clone)]
pub struct ConversationEnded {
    /// The NPC the conversation was with
    pub npc_id: CharacterId,
    pub npc_name: String,
    /// The player character who was conversing
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    /// Optional summary of the conversation
    pub summary: Option<String>,
    /// The conversation ID that was ended (if any)
    pub conversation_id: Option<Uuid>,
}

/// End conversation use case.
///
/// Validates the PC and NPC exist, ends the active conversation tracking,
/// and returns conversation end data.
/// The caller is responsible for broadcasting the result to clients.
///
/// Future enhancements could include:
/// - Optionally save conversation summary to persistent storage
/// - Notify any listeners/subscribers that the conversation has ended
/// - Update NPC disposition based on conversation outcome
pub struct EndConversation {
    character: Arc<Character>,
    player_character: Arc<PlayerCharacter>,
    narrative: Arc<Narrative>,
}

impl EndConversation {
    pub fn new(
        character: Arc<Character>,
        player_character: Arc<PlayerCharacter>,
        narrative: Arc<Narrative>,
    ) -> Self {
        Self {
            character,
            player_character,
            narrative,
        }
    }

    /// End a conversation with an NPC.
    ///
    /// # Arguments
    /// * `pc_id` - The player character ending the conversation
    /// * `npc_id` - The NPC the conversation was with
    /// * `summary` - Optional summary of the conversation
    ///
    /// # Returns
    /// * `Ok(ConversationEnded)` - Conversation end data for broadcasting
    /// * `Err(EndConversationError)` - Failed to end conversation
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        summary: Option<String>,
    ) -> Result<ConversationEnded, EndConversationError> {
        // 1. Validate the player character exists
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(EndConversationError::PlayerCharacterNotFound)?;

        // 2. Get the NPC
        let npc = self
            .character
            .get(npc_id)
            .await?
            .ok_or(EndConversationError::NpcNotFound)?;

        // 3. End the active conversation tracking (clear active conversation state)
        // This atomically finds and ends the active conversation between PC and NPC
        let ended_conversation_id = match self
            .narrative
            .end_active_conversation(pc_id, npc_id)
            .await
        {
            Ok(id) => {
                if let Some(conv_id) = &id {
                    tracing::info!(
                        conversation_id = %conv_id,
                        pc_id = %pc_id,
                        npc_id = %npc_id,
                        "Marked conversation as ended"
                    );
                } else {
                    tracing::debug!(
                        pc_id = %pc_id,
                        npc_id = %npc_id,
                        "No active conversation found to end"
                    );
                }
                id
            }
            Err(e) => {
                // Log but don't fail - the conversation end should still succeed
                // even if we can't update the tracking state
                tracing::warn!(
                    error = %e,
                    pc_id = %pc_id,
                    npc_id = %npc_id,
                    "Failed to end active conversation tracking, proceeding anyway"
                );
                None
            }
        };

        tracing::info!(
            pc_id = %pc_id,
            pc_name = %pc.name().as_str(),
            npc_id = %npc_id,
            npc_name = %npc.name(),
            conversation_id = ?ended_conversation_id,
            "Conversation ended"
        );

        Ok(ConversationEnded {
            npc_id,
            npc_name: npc.name().to_string(),
            pc_id,
            pc_name: pc.name().to_string(),
            summary,
            conversation_id: ended_conversation_id,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EndConversationError {
    #[error("Player character not found")]
    PlayerCharacterNotFound,
    #[error("NPC not found")]
    NpcNotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;
    use wrldbldr_domain::{CampbellArchetype, Character, CharacterId, CharacterName, LocationId, PlayerCharacterId, WorldId};

    use crate::repositories;
    use crate::infrastructure::ports::{
        ClockPort, MockChallengeRepo, MockCharacterRepo, MockFlagRepo, MockLocationRepo,
        MockNarrativeRepo, MockObservationRepo, MockPlayerCharacterRepo, MockSceneRepo,
        MockWorldRepo,
    };
    use crate::repositories::Character as CharacterOp;
    use crate::use_cases::Narrative;

    struct FixedClock(chrono::DateTime<chrono::Utc>);

    impl ClockPort for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
    }

    fn create_narrative_entity(narrative_repo: MockNarrativeRepo) -> Arc<Narrative> {
        let now = Utc::now();
        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        Arc::new(Narrative::new(
            Arc::new(narrative_repo),
            Arc::new(MockLocationRepo::new()),
            Arc::new(MockWorldRepo::new()),
            Arc::new(MockPlayerCharacterRepo::new()),
            Arc::new(MockCharacterRepo::new()),
            Arc::new(MockObservationRepo::new()),
            Arc::new(MockChallengeRepo::new()),
            Arc::new(MockFlagRepo::new()),
            Arc::new(MockSceneRepo::new()),
            clock,
        ))
    }

    #[tokio::test]
    async fn when_pc_not_found_then_returns_player_character_not_found() {
        let now = Utc::now();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(|_| Ok(None));

        let use_case = super::EndConversation::new(
            Arc::new(CharacterOp::new(Arc::new(MockCharacterRepo::new()))),
            Arc::new(repositories::PlayerCharacter::new(Arc::new(pc_repo))),
            create_narrative_entity(MockNarrativeRepo::new()),
        );

        let err = use_case
            .execute(pc_id, npc_id, None)
            .await
            .unwrap_err();

        assert!(matches!(err, super::EndConversationError::PlayerCharacterNotFound));
    }

    #[tokio::test]
    async fn when_npc_not_found_then_returns_npc_not_found() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let pc = wrldbldr_domain::PlayerCharacter::new("user", world_id, "PC", location_id, now)
            .with_id(pc_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut character_repo = MockCharacterRepo::new();
        character_repo
            .expect_get()
            .withf(move |id| *id == npc_id)
            .returning(|_| Ok(None));

        let use_case = super::EndConversation::new(
            Arc::new(CharacterOp::new(Arc::new(character_repo))),
            Arc::new(repositories::PlayerCharacter::new(Arc::new(pc_repo))),
            create_narrative_entity(MockNarrativeRepo::new()),
        );

        let err = use_case
            .execute(pc_id, npc_id, None)
            .await
            .unwrap_err();

        assert!(matches!(err, super::EndConversationError::NpcNotFound));
    }

    #[tokio::test]
    async fn when_valid_with_active_conversation_then_ends_and_returns_data() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let conversation_id = Uuid::new_v4();

        let pc = wrldbldr_domain::PlayerCharacter::new("user", world_id, "TestPC", location_id, now)
            .with_id(pc_id);

        let npc = Character::new(world_id, CharacterName::new("TestNPC").unwrap(), CampbellArchetype::Mentor)
            .with_id(npc_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut character_repo = MockCharacterRepo::new();
        let npc_for_get = npc.clone();
        character_repo
            .expect_get()
            .withf(move |id| *id == npc_id)
            .returning(move |_| Ok(Some(npc_for_get.clone())));

        // Narrative repo returns a conversation ID when ended
        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_end_active_conversation()
            .withf(move |p, n| *p == pc_id && *n == npc_id)
            .returning(move |_, _| Ok(Some(conversation_id)));

        let use_case = super::EndConversation::new(
            Arc::new(CharacterOp::new(Arc::new(character_repo))),
            Arc::new(repositories::PlayerCharacter::new(Arc::new(pc_repo))),
            create_narrative_entity(narrative_repo),
        );

        let summary = Some("Great conversation!".to_string());
        let result = use_case
            .execute(pc_id, npc_id, summary.clone())
            .await
            .expect("EndConversation should succeed");

        assert_eq!(result.pc_id, pc_id);
        assert_eq!(result.pc_name, "TestPC");
        assert_eq!(result.npc_id, npc_id);
        assert_eq!(result.npc_name, "TestNPC");
        assert_eq!(result.summary, summary);
        assert_eq!(result.conversation_id, Some(conversation_id));
    }

    #[tokio::test]
    async fn when_valid_but_no_active_conversation_then_succeeds_with_none_conversation_id() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let pc = wrldbldr_domain::PlayerCharacter::new("user", world_id, "TestPC", location_id, now)
            .with_id(pc_id);

        let npc = Character::new(world_id, CharacterName::new("TestNPC").unwrap(), CampbellArchetype::Mentor)
            .with_id(npc_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut character_repo = MockCharacterRepo::new();
        let npc_for_get = npc.clone();
        character_repo
            .expect_get()
            .withf(move |id| *id == npc_id)
            .returning(move |_| Ok(Some(npc_for_get.clone())));

        // Narrative repo returns None - no active conversation to end
        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_end_active_conversation()
            .withf(move |p, n| *p == pc_id && *n == npc_id)
            .returning(|_, _| Ok(None));

        let use_case = super::EndConversation::new(
            Arc::new(CharacterOp::new(Arc::new(character_repo))),
            Arc::new(repositories::PlayerCharacter::new(Arc::new(pc_repo))),
            create_narrative_entity(narrative_repo),
        );

        let result = use_case
            .execute(pc_id, npc_id, None)
            .await
            .expect("EndConversation should succeed even with no active conversation");

        assert_eq!(result.pc_id, pc_id);
        assert_eq!(result.npc_id, npc_id);
        assert_eq!(result.conversation_id, None);
    }

    #[tokio::test]
    async fn when_narrative_repo_fails_then_still_succeeds_with_none_conversation_id() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let pc = wrldbldr_domain::PlayerCharacter::new("user", world_id, "TestPC", location_id, now)
            .with_id(pc_id);

        let npc = Character::new(world_id, CharacterName::new("TestNPC").unwrap(), CampbellArchetype::Mentor)
            .with_id(npc_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut character_repo = MockCharacterRepo::new();
        let npc_for_get = npc.clone();
        character_repo
            .expect_get()
            .withf(move |id| *id == npc_id)
            .returning(move |_| Ok(Some(npc_for_get.clone())));

        // Narrative repo returns an error
        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_end_active_conversation()
            .withf(move |p, n| *p == pc_id && *n == npc_id)
            .returning(|_, _| {
                Err(crate::infrastructure::ports::RepoError::database(
                    "end_conversation",
                    "Database connection lost",
                ))
            });

        let use_case = super::EndConversation::new(
            Arc::new(CharacterOp::new(Arc::new(character_repo))),
            Arc::new(repositories::PlayerCharacter::new(Arc::new(pc_repo))),
            create_narrative_entity(narrative_repo),
        );

        // Should still succeed - repo failure is logged but not propagated
        let result = use_case
            .execute(pc_id, npc_id, None)
            .await
            .expect("EndConversation should succeed even if narrative repo fails");

        assert_eq!(result.pc_id, pc_id);
        assert_eq!(result.npc_id, npc_id);
        assert_eq!(result.conversation_id, None); // None because repo failed
    }
}
