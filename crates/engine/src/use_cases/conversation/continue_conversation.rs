//! Continue conversation use case.
//!
//! Handles continuing an existing conversation between a player character and an NPC.
//! This use case validates the interaction context and enqueues the player's response
//! for LLM processing.

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{CharacterId, PlayerActionData, PlayerCharacterId, WorldId};

use crate::entities::{Character, Narrative, PlayerCharacter, Staging, World};
use crate::infrastructure::ports::{ClockPort, QueuePort};

// Re-use the shared ConversationError from start.rs
use super::start::ConversationError;

/// Response from continuing a conversation.
#[derive(Debug)]
pub struct ConversationContinued {
    /// ID of the queued player action
    pub action_queue_id: Uuid,
    /// The conversation is still active
    pub conversation_active: bool,
    /// The conversation ID for tracking
    pub conversation_id: Option<Uuid>,
}

/// Continue conversation use case.
///
/// Orchestrates: Context validation, player action queuing.
pub struct ContinueConversation {
    character: Arc<Character>,
    player_character: Arc<PlayerCharacter>,
    staging: Arc<Staging>,
    world: Arc<World>,
    narrative: Arc<Narrative>,
    queue: Arc<dyn QueuePort>,
    clock: Arc<dyn ClockPort>,
}

impl ContinueConversation {
    pub fn new(
        character: Arc<Character>,
        player_character: Arc<PlayerCharacter>,
        staging: Arc<Staging>,
        world: Arc<World>,
        narrative: Arc<Narrative>,
        queue: Arc<dyn QueuePort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            character,
            player_character,
            staging,
            world,
            narrative,
            queue,
            clock,
        }
    }

    /// Continue an existing conversation with an NPC.
    ///
    /// # Arguments
    /// * `world_id` - The world context
    /// * `pc_id` - The player character in the conversation
    /// * `npc_id` - The NPC being spoken to
    /// * `player_id` - The player's user ID
    /// * `player_message` - The player's response message
    /// * `conversation_id` - Optional conversation ID (if not provided, looks up active conversation)
    ///
    /// # Returns
    /// * `Ok(ConversationContinued)` - Response queued for processing
    /// * `Err(ConversationError)` - Failed to continue conversation
    pub async fn execute(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        player_id: String,
        player_message: String,
        conversation_id: Option<Uuid>,
    ) -> Result<ConversationContinued, ConversationError> {
        // 1. Validate the player character exists
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ConversationError::PlayerCharacterNotFound)?;

        // 2. Get the NPC
        let npc = self
            .character
            .get(npc_id)
            .await?
            .ok_or(ConversationError::NpcNotFound)?;

        // 3. Verify the NPC is still in the same region as the PC
        let pc_region_id = pc
            .current_region_id
            .ok_or(ConversationError::PlayerNotInRegion)?;

        // Get current game time for staging TTL check
        let world_data = self
            .world
            .get(world_id)
            .await?
            .ok_or(ConversationError::WorldNotFound)?;
        let current_game_time = world_data.game_time.current();

        let staged_npcs = self
            .staging
            .resolve_for_region(pc_region_id, current_game_time)
            .await?;
        let npc_in_region = staged_npcs
            .iter()
            .any(|staged| staged.character_id == npc_id);

        if !npc_in_region {
            // NPC left the region - conversation is over
            return Err(ConversationError::NpcLeftRegion);
        }

        // 4. Resolve conversation_id: use provided one or look up active conversation
        let resolved_conversation_id = if let Some(id) = conversation_id {
            // Verify the provided conversation is still active (not ended)
            // This prevents a race condition where a conversation could be ended
            // between the client sending a continue request and us processing it
            let is_active = self
                .narrative
                .is_conversation_active(id)
                .await
                .unwrap_or(false);

            if !is_active {
                tracing::warn!(
                    conversation_id = %id,
                    pc_id = %pc_id,
                    npc_id = %npc_id,
                    "Attempted to continue ended conversation"
                );
                return Err(ConversationError::ConversationEnded);
            }
            Some(id)
        } else {
            // Look up active conversation between PC and NPC
            match self
                .narrative
                .get_active_conversation_id(pc_id, npc_id)
                .await
            {
                Ok(Some(id)) => Some(id),
                Ok(None) => {
                    tracing::warn!(
                        pc_id = %pc_id,
                        npc_id = %npc_id,
                        "No active conversation found between PC and NPC"
                    );
                    return Err(ConversationError::NoActiveConversation);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        pc_id = %pc_id,
                        npc_id = %npc_id,
                        "Failed to look up active conversation"
                    );
                    return Err(ConversationError::Repo(e));
                }
            }
        };

        // 5. Enqueue the player action for processing
        // Note: target is the NPC ID (as string) so it can be parsed in build_prompt
        let action_data = PlayerActionData {
            world_id,
            player_id,
            pc_id: Some(pc_id),
            action_type: "talk".to_string(),
            target: Some(npc_id.to_string()),
            dialogue: Some(player_message),
            timestamp: self.clock.now(),
            conversation_id: resolved_conversation_id,
        };

        let action_queue_id = self
            .queue
            .enqueue_player_action(&action_data)
            .await
            .map_err(|e| ConversationError::QueueError(e.to_string()))?;

        Ok(ConversationContinued {
            action_queue_id,
            conversation_active: true,
            conversation_id: resolved_conversation_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;
    use wrldbldr_domain::{
        ApprovalRequestData, AssetGenerationData, CampbellArchetype, Character, CharacterId,
        LlmRequestData, LocationId, MoodState, PlayerActionData, PlayerCharacterId, RegionId,
        StagedNpc, Staging, StagingSource, WorldId,
    };

    use crate::entities;
    use crate::infrastructure::ports::{
        ClockPort, MockChallengeRepo, MockCharacterRepo, MockFlagRepo, MockLocationRepo,
        MockNarrativeRepo, MockObservationRepo, MockPlayerCharacterRepo, MockSceneRepo,
        MockStagingRepo, MockWorldRepo, QueueError, QueueItem, QueuePort,
    };

    struct FixedClock(chrono::DateTime<chrono::Utc>);

    impl ClockPort for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
    }

    #[derive(Debug)]
    struct RecordingQueuePort {
        enqueue_return_id: Uuid,
        player_actions: Mutex<Vec<PlayerActionData>>,
    }

    impl RecordingQueuePort {
        fn new(enqueue_return_id: Uuid) -> Self {
            Self {
                enqueue_return_id,
                player_actions: Mutex::new(Vec::new()),
            }
        }

        fn recorded_player_actions(&self) -> Vec<PlayerActionData> {
            self.player_actions.lock().expect("lock").clone()
        }
    }

    #[async_trait]
    impl QueuePort for RecordingQueuePort {
        async fn enqueue_player_action(&self, data: &PlayerActionData) -> Result<Uuid, QueueError> {
            self.player_actions.lock().expect("lock").push(data.clone());
            Ok(self.enqueue_return_id)
        }

        async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
            Ok(None)
        }

        async fn enqueue_llm_request(&self, _data: &LlmRequestData) -> Result<Uuid, QueueError> {
            Ok(Uuid::new_v4())
        }

        async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
            Ok(None)
        }

        async fn enqueue_dm_approval(
            &self,
            _data: &ApprovalRequestData,
        ) -> Result<Uuid, QueueError> {
            Ok(Uuid::new_v4())
        }

        async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
            Ok(None)
        }

        async fn enqueue_asset_generation(
            &self,
            _data: &AssetGenerationData,
        ) -> Result<Uuid, QueueError> {
            Ok(Uuid::new_v4())
        }

        async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
            Ok(None)
        }

        async fn mark_complete(&self, _id: Uuid) -> Result<(), QueueError> {
            Ok(())
        }

        async fn mark_failed(&self, _id: Uuid, _error: &str) -> Result<(), QueueError> {
            Ok(())
        }

        async fn get_pending_count(&self, _queue_type: &str) -> Result<usize, QueueError> {
            Ok(0)
        }

        async fn list_by_type(
            &self,
            _queue_type: &str,
            _limit: usize,
        ) -> Result<Vec<QueueItem>, QueueError> {
            Ok(vec![])
        }

        async fn set_result_json(&self, _id: Uuid, _result_json: &str) -> Result<(), QueueError> {
            Ok(())
        }

        async fn cancel_pending_llm_request_by_callback_id(
            &self,
            _callback_id: &str,
        ) -> Result<bool, QueueError> {
            Ok(false)
        }

        async fn get_approval_request(
            &self,
            _id: Uuid,
        ) -> Result<Option<ApprovalRequestData>, QueueError> {
            Ok(None)
        }

        async fn get_generation_read_state(
            &self,
            _user_id: &str,
            _world_id: WorldId,
        ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
            Ok(None)
        }

        async fn upsert_generation_read_state(
            &self,
            _user_id: &str,
            _world_id: WorldId,
            _read_batches: &[String],
            _read_suggestions: &[String],
        ) -> Result<(), QueueError> {
            Ok(())
        }

        async fn delete_by_callback_id(&self, _callback_id: &str) -> Result<bool, QueueError> {
            Ok(false)
        }
    }

    fn create_narrative_entity(narrative_repo: MockNarrativeRepo) -> Arc<entities::Narrative> {
        let now = Utc::now();
        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        Arc::new(entities::Narrative::new(
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
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(|_| Ok(None));

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let queue = Arc::new(RecordingQueuePort::new(Uuid::new_v4()));

        let use_case = super::ContinueConversation::new(
            Arc::new(entities::Character::new(Arc::new(MockCharacterRepo::new()))),
            Arc::new(entities::PlayerCharacter::new(Arc::new(pc_repo))),
            Arc::new(entities::Staging::new(Arc::new(MockStagingRepo::new()))),
            Arc::new(entities::World::new(
                Arc::new(MockWorldRepo::new()),
                clock.clone(),
            )),
            create_narrative_entity(MockNarrativeRepo::new()),
            queue.clone(),
            clock,
        );

        let err = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                "player".to_string(),
                "Hello again".to_string(),
                None,
            )
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            super::super::start::ConversationError::PlayerCharacterNotFound
        ));
        assert!(queue.recorded_player_actions().is_empty());
    }

    #[tokio::test]
    async fn when_npc_not_found_then_returns_npc_not_found() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("user", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = Some(region_id);

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

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let queue = Arc::new(RecordingQueuePort::new(Uuid::new_v4()));

        let use_case = super::ContinueConversation::new(
            Arc::new(entities::Character::new(Arc::new(character_repo))),
            Arc::new(entities::PlayerCharacter::new(Arc::new(pc_repo))),
            Arc::new(entities::Staging::new(Arc::new(MockStagingRepo::new()))),
            Arc::new(entities::World::new(
                Arc::new(MockWorldRepo::new()),
                clock.clone(),
            )),
            create_narrative_entity(MockNarrativeRepo::new()),
            queue.clone(),
            clock,
        );

        let err = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                "player".to_string(),
                "Hello again".to_string(),
                None,
            )
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            super::super::start::ConversationError::NpcNotFound
        ));
        assert!(queue.recorded_player_actions().is_empty());
    }

    #[tokio::test]
    async fn when_pc_not_in_region_then_returns_player_not_in_region() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        // PC has no current_region_id
        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("user", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = None;

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let npc = {
            let mut c = Character::new(world_id, "NPC", CampbellArchetype::Mentor);
            c.id = npc_id;
            c
        };

        let mut character_repo = MockCharacterRepo::new();
        let npc_for_get = npc.clone();
        character_repo
            .expect_get()
            .withf(move |id| *id == npc_id)
            .returning(move |_| Ok(Some(npc_for_get.clone())));

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let queue = Arc::new(RecordingQueuePort::new(Uuid::new_v4()));

        let use_case = super::ContinueConversation::new(
            Arc::new(entities::Character::new(Arc::new(character_repo))),
            Arc::new(entities::PlayerCharacter::new(Arc::new(pc_repo))),
            Arc::new(entities::Staging::new(Arc::new(MockStagingRepo::new()))),
            Arc::new(entities::World::new(
                Arc::new(MockWorldRepo::new()),
                clock.clone(),
            )),
            create_narrative_entity(MockNarrativeRepo::new()),
            queue.clone(),
            clock,
        );

        let err = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                "player".to_string(),
                "Hello again".to_string(),
                None,
            )
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            super::super::start::ConversationError::PlayerNotInRegion
        ));
        assert!(queue.recorded_player_actions().is_empty());
    }

    #[tokio::test]
    async fn when_world_not_found_then_returns_world_not_found() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("user", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = Some(region_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let npc = {
            let mut c = Character::new(world_id, "NPC", CampbellArchetype::Mentor);
            c.id = npc_id;
            c
        };

        let mut character_repo = MockCharacterRepo::new();
        let npc_for_get = npc.clone();
        character_repo
            .expect_get()
            .withf(move |id| *id == npc_id)
            .returning(move |_| Ok(Some(npc_for_get.clone())));

        let mut world_repo = MockWorldRepo::new();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(|_| Ok(None));

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let queue = Arc::new(RecordingQueuePort::new(Uuid::new_v4()));

        let use_case = super::ContinueConversation::new(
            Arc::new(entities::Character::new(Arc::new(character_repo))),
            Arc::new(entities::PlayerCharacter::new(Arc::new(pc_repo))),
            Arc::new(entities::Staging::new(Arc::new(MockStagingRepo::new()))),
            Arc::new(entities::World::new(Arc::new(world_repo), clock.clone())),
            create_narrative_entity(MockNarrativeRepo::new()),
            queue.clone(),
            clock,
        );

        let err = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                "player".to_string(),
                "Hello again".to_string(),
                None,
            )
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            super::super::start::ConversationError::WorldNotFound
        ));
        assert!(queue.recorded_player_actions().is_empty());
    }

    #[tokio::test]
    async fn when_npc_left_region_then_returns_npc_left_region() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let other_npc_id = CharacterId::new();

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("user", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = Some(region_id);

        let npc = {
            let mut c = Character::new(world_id, "NPC", CampbellArchetype::Mentor);
            c.id = npc_id;
            c
        };

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

        let mut world_repo = MockWorldRepo::new();
        let mut world = wrldbldr_domain::World::new("W", "D", now);
        world.id = world_id;
        let current_game_time = world.game_time.current();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world_for_get.clone())));

        // Staging has a different NPC, not the one we're trying to talk to
        let staged_npc = StagedNpc {
            character_id: other_npc_id, // Different NPC
            name: "Other NPC".to_string(),
            sprite_asset: None,
            portrait_asset: None,
            is_present: true,
            is_hidden_from_players: false,
            reasoning: "here".to_string(),
            mood: MoodState::Calm,
            has_incomplete_data: false,
        };
        let staging = Staging::new(
            region_id,
            location_id,
            world_id,
            current_game_time,
            "dm",
            StagingSource::DmCustomized,
            6,
            now,
        )
        .with_npcs(vec![staged_npc]);

        let mut staging_repo = MockStagingRepo::new();
        let staging_for_get = staging.clone();
        staging_repo
            .expect_get_active_staging()
            .withf(move |r, t| *r == region_id && *t == current_game_time)
            .returning(move |_, _| Ok(Some(staging_for_get.clone())));

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let queue = Arc::new(RecordingQueuePort::new(Uuid::new_v4()));

        let use_case = super::ContinueConversation::new(
            Arc::new(entities::Character::new(Arc::new(character_repo))),
            Arc::new(entities::PlayerCharacter::new(Arc::new(pc_repo))),
            Arc::new(entities::Staging::new(Arc::new(staging_repo))),
            Arc::new(entities::World::new(Arc::new(world_repo), clock.clone())),
            create_narrative_entity(MockNarrativeRepo::new()),
            queue.clone(),
            clock,
        );

        let err = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                "player".to_string(),
                "Hello again".to_string(),
                None,
            )
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            super::super::start::ConversationError::NpcLeftRegion
        ));
        assert!(queue.recorded_player_actions().is_empty());
    }

    #[tokio::test]
    async fn when_provided_conversation_ended_then_returns_conversation_ended() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let conversation_id = Uuid::new_v4();

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("user", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = Some(region_id);

        let npc = {
            let mut c = Character::new(world_id, "NPC", CampbellArchetype::Mentor);
            c.id = npc_id;
            c
        };

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

        let mut world_repo = MockWorldRepo::new();
        let mut world = wrldbldr_domain::World::new("W", "D", now);
        world.id = world_id;
        let current_game_time = world.game_time.current();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world_for_get.clone())));

        let staged_npc = StagedNpc {
            character_id: npc_id,
            name: npc.name.clone(),
            sprite_asset: None,
            portrait_asset: None,
            is_present: true,
            is_hidden_from_players: false,
            reasoning: "here".to_string(),
            mood: MoodState::Calm,
            has_incomplete_data: false,
        };
        let staging = Staging::new(
            region_id,
            location_id,
            world_id,
            current_game_time,
            "dm",
            StagingSource::DmCustomized,
            6,
            now,
        )
        .with_npcs(vec![staged_npc]);

        let mut staging_repo = MockStagingRepo::new();
        let staging_for_get = staging.clone();
        staging_repo
            .expect_get_active_staging()
            .withf(move |r, t| *r == region_id && *t == current_game_time)
            .returning(move |_, _| Ok(Some(staging_for_get.clone())));

        // Narrative repo says conversation is NOT active (ended)
        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_is_conversation_active()
            .withf(move |id| *id == conversation_id)
            .returning(|_| Ok(false));

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let queue = Arc::new(RecordingQueuePort::new(Uuid::new_v4()));

        let use_case = super::ContinueConversation::new(
            Arc::new(entities::Character::new(Arc::new(character_repo))),
            Arc::new(entities::PlayerCharacter::new(Arc::new(pc_repo))),
            Arc::new(entities::Staging::new(Arc::new(staging_repo))),
            Arc::new(entities::World::new(Arc::new(world_repo), clock.clone())),
            create_narrative_entity(narrative_repo),
            queue.clone(),
            clock,
        );

        let err = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                "player".to_string(),
                "Hello again".to_string(),
                Some(conversation_id),
            )
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            super::super::start::ConversationError::ConversationEnded
        ));
        assert!(queue.recorded_player_actions().is_empty());
    }

    #[tokio::test]
    async fn when_no_conversation_id_and_no_active_conversation_then_returns_no_active_conversation()
    {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("user", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = Some(region_id);

        let npc = {
            let mut c = Character::new(world_id, "NPC", CampbellArchetype::Mentor);
            c.id = npc_id;
            c
        };

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

        let mut world_repo = MockWorldRepo::new();
        let mut world = wrldbldr_domain::World::new("W", "D", now);
        world.id = world_id;
        let current_game_time = world.game_time.current();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world_for_get.clone())));

        let staged_npc = StagedNpc {
            character_id: npc_id,
            name: npc.name.clone(),
            sprite_asset: None,
            portrait_asset: None,
            is_present: true,
            is_hidden_from_players: false,
            reasoning: "here".to_string(),
            mood: MoodState::Calm,
            has_incomplete_data: false,
        };
        let staging = Staging::new(
            region_id,
            location_id,
            world_id,
            current_game_time,
            "dm",
            StagingSource::DmCustomized,
            6,
            now,
        )
        .with_npcs(vec![staged_npc]);

        let mut staging_repo = MockStagingRepo::new();
        let staging_for_get = staging.clone();
        staging_repo
            .expect_get_active_staging()
            .withf(move |r, t| *r == region_id && *t == current_game_time)
            .returning(move |_, _| Ok(Some(staging_for_get.clone())));

        // Narrative repo says no active conversation exists
        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_get_active_conversation_id()
            .withf(move |p, n| *p == pc_id && *n == npc_id)
            .returning(|_, _| Ok(None));

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let queue = Arc::new(RecordingQueuePort::new(Uuid::new_v4()));

        let use_case = super::ContinueConversation::new(
            Arc::new(entities::Character::new(Arc::new(character_repo))),
            Arc::new(entities::PlayerCharacter::new(Arc::new(pc_repo))),
            Arc::new(entities::Staging::new(Arc::new(staging_repo))),
            Arc::new(entities::World::new(Arc::new(world_repo), clock.clone())),
            create_narrative_entity(narrative_repo),
            queue.clone(),
            clock,
        );

        let err = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                "player".to_string(),
                "Hello again".to_string(),
                None, // No conversation_id provided
            )
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            super::super::start::ConversationError::NoActiveConversation
        ));
        assert!(queue.recorded_player_actions().is_empty());
    }

    #[tokio::test]
    async fn when_valid_with_conversation_id_then_enqueues_action_with_expected_payload() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let conversation_id = Uuid::new_v4();

        let player_id = "player".to_string();
        let player_message = "Hello again!".to_string();

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("user", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = Some(region_id);

        let npc = {
            let mut c = Character::new(world_id, "NPC", CampbellArchetype::Mentor);
            c.id = npc_id;
            c
        };

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

        let mut world_repo = MockWorldRepo::new();
        let mut world = wrldbldr_domain::World::new("W", "D", now);
        world.id = world_id;
        let current_game_time = world.game_time.current();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world_for_get.clone())));

        let staged_npc = StagedNpc {
            character_id: npc_id,
            name: npc.name.clone(),
            sprite_asset: None,
            portrait_asset: None,
            is_present: true,
            is_hidden_from_players: false,
            reasoning: "here".to_string(),
            mood: MoodState::Calm,
            has_incomplete_data: false,
        };
        let staging = Staging::new(
            region_id,
            location_id,
            world_id,
            current_game_time,
            "dm",
            StagingSource::DmCustomized,
            6,
            now,
        )
        .with_npcs(vec![staged_npc]);

        let mut staging_repo = MockStagingRepo::new();
        let staging_for_get = staging.clone();
        staging_repo
            .expect_get_active_staging()
            .withf(move |r, t| *r == region_id && *t == current_game_time)
            .returning(move |_, _| Ok(Some(staging_for_get.clone())));

        // Conversation is active
        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_is_conversation_active()
            .withf(move |id| *id == conversation_id)
            .returning(|_| Ok(true));

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let queue_id = Uuid::new_v4();
        let queue = Arc::new(RecordingQueuePort::new(queue_id));

        let use_case = super::ContinueConversation::new(
            Arc::new(entities::Character::new(Arc::new(character_repo))),
            Arc::new(entities::PlayerCharacter::new(Arc::new(pc_repo))),
            Arc::new(entities::Staging::new(Arc::new(staging_repo))),
            Arc::new(entities::World::new(Arc::new(world_repo), clock.clone())),
            create_narrative_entity(narrative_repo),
            queue.clone(),
            clock,
        );

        let result = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                player_id.clone(),
                player_message.clone(),
                Some(conversation_id),
            )
            .await
            .expect("ContinueConversation should succeed");

        assert_eq!(result.action_queue_id, queue_id);
        assert!(result.conversation_active);
        assert_eq!(result.conversation_id, Some(conversation_id));

        let recorded = queue.recorded_player_actions();
        assert_eq!(recorded.len(), 1);
        let action = &recorded[0];
        assert_eq!(action.world_id, world_id);
        assert_eq!(action.player_id, player_id);
        assert_eq!(action.pc_id, Some(pc_id));
        assert_eq!(action.action_type, "talk".to_string());
        assert_eq!(action.target, Some("NPC".to_string()));
        assert_eq!(action.dialogue, Some(player_message));
        assert_eq!(action.conversation_id, Some(conversation_id));
        assert_eq!(action.timestamp, now);
    }

    #[tokio::test]
    async fn when_valid_without_conversation_id_then_looks_up_active_and_enqueues() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let found_conversation_id = Uuid::new_v4();

        let player_id = "player".to_string();
        let player_message = "Continuing...".to_string();

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("user", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = Some(region_id);

        let npc = {
            let mut c = Character::new(world_id, "NPC", CampbellArchetype::Mentor);
            c.id = npc_id;
            c
        };

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

        let mut world_repo = MockWorldRepo::new();
        let mut world = wrldbldr_domain::World::new("W", "D", now);
        world.id = world_id;
        let current_game_time = world.game_time.current();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world_for_get.clone())));

        let staged_npc = StagedNpc {
            character_id: npc_id,
            name: npc.name.clone(),
            sprite_asset: None,
            portrait_asset: None,
            is_present: true,
            is_hidden_from_players: false,
            reasoning: "here".to_string(),
            mood: MoodState::Calm,
            has_incomplete_data: false,
        };
        let staging = Staging::new(
            region_id,
            location_id,
            world_id,
            current_game_time,
            "dm",
            StagingSource::DmCustomized,
            6,
            now,
        )
        .with_npcs(vec![staged_npc]);

        let mut staging_repo = MockStagingRepo::new();
        let staging_for_get = staging.clone();
        staging_repo
            .expect_get_active_staging()
            .withf(move |r, t| *r == region_id && *t == current_game_time)
            .returning(move |_, _| Ok(Some(staging_for_get.clone())));

        // Narrative repo returns an active conversation when looked up
        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_get_active_conversation_id()
            .withf(move |p, n| *p == pc_id && *n == npc_id)
            .returning(move |_, _| Ok(Some(found_conversation_id)));

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let queue_id = Uuid::new_v4();
        let queue = Arc::new(RecordingQueuePort::new(queue_id));

        let use_case = super::ContinueConversation::new(
            Arc::new(entities::Character::new(Arc::new(character_repo))),
            Arc::new(entities::PlayerCharacter::new(Arc::new(pc_repo))),
            Arc::new(entities::Staging::new(Arc::new(staging_repo))),
            Arc::new(entities::World::new(Arc::new(world_repo), clock.clone())),
            create_narrative_entity(narrative_repo),
            queue.clone(),
            clock,
        );

        let result = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                player_id.clone(),
                player_message.clone(),
                None, // No conversation_id, should look up
            )
            .await
            .expect("ContinueConversation should succeed");

        assert_eq!(result.action_queue_id, queue_id);
        assert!(result.conversation_active);
        assert_eq!(result.conversation_id, Some(found_conversation_id));

        let recorded = queue.recorded_player_actions();
        assert_eq!(recorded.len(), 1);
        let action = &recorded[0];
        assert_eq!(action.conversation_id, Some(found_conversation_id));
    }
}
