//! Start conversation use case.
//!
//! Handles initiating a conversation between a player character and an NPC.
//! This use case validates the interaction is possible and enqueues the
//! player action for LLM processing.

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{CharacterId, PlayerCharacterId, WorldId};

use crate::queue_types::PlayerActionData;

use crate::infrastructure::ports::{
    CharacterRepo, ClockPort, PlayerCharacterRepo, QueueError, QueuePort, RepoError, SceneRepo,
    StagingRepo, WorldRepo,
};

/// Result of starting a conversation.
#[derive(Debug)]
pub struct ConversationStarted {
    /// Unique ID for this conversation session
    pub conversation_id: Uuid,
    /// ID of the queued player action
    pub action_queue_id: Uuid,
    /// NPC name for display
    pub npc_name: String,
    /// NPC's current disposition toward the PC (if available)
    pub npc_disposition: Option<String>,
}

/// Start conversation use case.
///
/// Orchestrates: NPC validation, staging check, player action queuing.
#[allow(dead_code)]
pub struct StartConversation {
    character: Arc<dyn CharacterRepo>,
    player_character: Arc<dyn PlayerCharacterRepo>,
    staging: Arc<dyn StagingRepo>,
    scene: Arc<dyn SceneRepo>,
    world: Arc<dyn WorldRepo>,
    queue: Arc<dyn QueuePort>,
    clock: Arc<dyn ClockPort>,
}

impl StartConversation {
    pub fn new(
        character: Arc<dyn CharacterRepo>,
        player_character: Arc<dyn PlayerCharacterRepo>,
        staging: Arc<dyn StagingRepo>,
        scene: Arc<dyn SceneRepo>,
        world: Arc<dyn WorldRepo>,
        queue: Arc<dyn QueuePort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            character,
            player_character,
            staging,
            scene,
            world,
            queue,
            clock,
        }
    }

    /// Start a conversation with an NPC.
    ///
    /// # Arguments
    /// * `world_id` - The world context
    /// * `pc_id` - The player character initiating the conversation
    /// * `npc_id` - The NPC to converse with
    /// * `player_id` - The player's user ID
    /// * `initial_dialogue` - The player's opening message
    ///
    /// # Returns
    /// * `Ok(ConversationStarted)` - Conversation initiated, action queued
    /// * `Err(ConversationError)` - Failed to start conversation
    pub async fn execute(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        player_id: String,
        initial_dialogue: String,
    ) -> Result<ConversationStarted, ConversationError> {
        // 1. Validate the player character exists
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ConversationError::PlayerCharacterNotFound(pc_id))?;

        // 2. Get the NPC
        let npc = self
            .character
            .get(npc_id)
            .await?
            .ok_or(ConversationError::NpcNotFound(npc_id))?;

        // 3. Verify the NPC is in the same region as the PC
        let pc_region_id = pc
            .current_region_id()
            .ok_or(ConversationError::PlayerNotInRegion)?;

        // Get current game time for staging TTL check
        let world_data = self
            .world
            .get(world_id)
            .await?
            .ok_or(ConversationError::WorldNotFound(world_id))?;
        let current_game_time = world_data.game_time().current();

        // Check if NPC is staged in this region (with TTL check)
        // Get active staging and filter to visible NPCs
        let active_staging = self
            .staging
            .get_active_staging(pc_region_id, current_game_time)
            .await?;
        let staged_npcs = active_staging
            .map(|s| {
                s.npcs()
                    .iter()
                    .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let npc_in_region = staged_npcs
            .iter()
            .any(|staged| staged.character_id == npc_id);

        if !npc_in_region {
            return Err(ConversationError::NpcNotInRegion);
        }

        // 4. Generate conversation ID
        let conversation_id = Uuid::new_v4();

        // 5. Get NPC's current disposition toward the PC if available
        let npc_disposition = match self.character.get_disposition(npc_id, pc_id).await {
            Ok(disposition) => disposition.map(|d| d.disposition().to_string()),
            Err(e) => {
                tracing::warn!(
                    npc_id = %npc_id,
                    pc_id = %pc_id,
                    error = %e,
                    "Failed to get NPC disposition, continuing without it"
                );
                None
            }
        };

        // 6. Enqueue the player action for processing
        // Note: target is the NPC ID (as string) so it can be parsed in build_prompt
        let action_data = PlayerActionData {
            world_id,
            player_id,
            pc_id: Some(pc_id),
            action_type: "talk".to_string(),
            target: Some(npc_id.to_string()),
            dialogue: Some(initial_dialogue),
            timestamp: self.clock.now(),
            conversation_id: Some(conversation_id),
        };

        let action_queue_id = self.queue.enqueue_player_action(&action_data).await?;

        Ok(ConversationStarted {
            conversation_id,
            action_queue_id,
            npc_name: npc.name().to_string(),
            npc_disposition,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("Player character not found: {0}")]
    PlayerCharacterNotFound(PlayerCharacterId),
    #[error("NPC not found: {0}")]
    NpcNotFound(CharacterId),
    #[error("World not found: {0}")]
    WorldNotFound(WorldId),
    #[error("Player is not in a region")]
    PlayerNotInRegion,
    #[error("NPC is not in the player's region")]
    NpcNotInRegion,
    #[error("NPC has left the region")]
    NpcLeftRegion,
    #[error("Conversation has ended")]
    ConversationEnded,
    #[error("No active conversation found")]
    NoActiveConversation,
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;
    use wrldbldr_domain::{
        CampbellArchetype, Character, CharacterId, CharacterName, LocationId, MoodState,
        PlayerCharacterId, RegionId, StagedNpc, Staging, StagingSource, WorldId, WorldName,
    };

    use crate::queue_types::{
        ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData,
    };

    use crate::infrastructure::ports::{
        ClockPort, MockCharacterRepo, MockPlayerCharacterRepo, MockSceneRepo, MockStagingRepo,
        MockWorldRepo, QueueError, QueueItem, QueuePort,
    };
    use crate::repositories;
    use crate::repositories::{
        CharacterRepository, ClockService, QueueService, SceneRepository, StagingRepository,
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

    #[tokio::test]
    async fn when_npc_not_staged_then_returns_npc_not_in_region_and_does_not_enqueue() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let pc = wrldbldr_domain::PlayerCharacter::new(
            "user",
            world_id,
            CharacterName::new("PC").unwrap(),
            location_id,
            now,
        )
        .with_id(pc_id)
        .with_current_region(Some(region_id));

        let npc = Character::new(
            world_id,
            CharacterName::new("NPC").unwrap(),
            CampbellArchetype::Mentor,
        )
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
        character_repo
            .expect_get_disposition()
            .returning(|_, _| Ok(None));

        let mut world_repo = MockWorldRepo::new();
        let world_name = WorldName::new("W").unwrap();
        let world = wrldbldr_domain::World::new(world_name, now).with_id(world_id);
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world_for_get.clone())));

        let mut staging_repo = MockStagingRepo::new();
        staging_repo
            .expect_get_active_staging()
            .withf(move |r, _| *r == region_id)
            .returning(|_, _| Ok(None));

        let clock_port: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let clock = Arc::new(ClockService::new(clock_port.clone()));
        let queue_id = Uuid::new_v4();
        let queue_port = Arc::new(RecordingQueuePort::new(queue_id));
        let queue = Arc::new(QueueService::new(queue_port.clone()));

        let use_case = super::StartConversation::new(
            Arc::new(CharacterRepository::new(Arc::new(character_repo))),
            Arc::new(repositories::PlayerCharacterRepository::new(Arc::new(
                pc_repo,
            ))),
            Arc::new(StagingRepository::new(Arc::new(staging_repo))),
            Arc::new(SceneRepository::new(Arc::new(MockSceneRepo::new()))),
            Arc::new(repositories::WorldRepository::new(
                Arc::new(world_repo),
                clock_port.clone(),
            )),
            queue.clone(),
            clock.clone(),
        );

        let err = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                "player".to_string(),
                "Hello".to_string(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, super::ConversationError::NpcNotInRegion));
        assert!(queue_port.recorded_player_actions().is_empty());
    }

    #[tokio::test]
    async fn when_npc_staged_then_enqueues_speak_action_with_expected_payload() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let player_id = "player".to_string();
        let initial_dialogue = "Hello".to_string();

        let pc = wrldbldr_domain::PlayerCharacter::new(
            "user",
            world_id,
            CharacterName::new("PC").unwrap(),
            location_id,
            now,
        )
        .with_id(pc_id)
        .with_current_region(Some(region_id));

        let npc = Character::new(
            world_id,
            CharacterName::new("NPC").unwrap(),
            CampbellArchetype::Mentor,
        )
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
        character_repo
            .expect_get_disposition()
            .returning(|_, _| Ok(None));

        let mut world_repo = MockWorldRepo::new();
        let world_name = WorldName::new("W").unwrap();
        let world = wrldbldr_domain::World::new(world_name, now).with_id(world_id);
        let current_game_time = world.game_time().current();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world_for_get.clone())));

        let staged_npc =
            StagedNpc::new(npc_id, npc.name().to_string(), true, "here").with_mood(MoodState::Calm);
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

        let clock_port: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let clock = Arc::new(ClockService::new(clock_port.clone()));
        let queue_id = Uuid::new_v4();
        let queue_port = Arc::new(RecordingQueuePort::new(queue_id));
        let queue = Arc::new(QueueService::new(queue_port.clone()));

        let use_case = super::StartConversation::new(
            Arc::new(CharacterRepository::new(Arc::new(character_repo))),
            Arc::new(repositories::PlayerCharacterRepository::new(Arc::new(
                pc_repo,
            ))),
            Arc::new(StagingRepository::new(Arc::new(staging_repo))),
            Arc::new(SceneRepository::new(Arc::new(MockSceneRepo::new()))),
            Arc::new(repositories::WorldRepository::new(
                Arc::new(world_repo),
                clock_port.clone(),
            )),
            queue.clone(),
            clock.clone(),
        );

        let result = use_case
            .execute(
                world_id,
                pc_id,
                npc_id,
                player_id.clone(),
                initial_dialogue.clone(),
            )
            .await
            .expect("StartConversation should succeed");

        assert_eq!(result.action_queue_id, queue_id);
        assert!(!result.conversation_id.is_nil());
        assert_eq!(result.npc_name, "NPC".to_string());

        let recorded = queue_port.recorded_player_actions();
        assert_eq!(recorded.len(), 1);
        let action = &recorded[0];
        assert_eq!(action.world_id, world_id);
        assert_eq!(action.player_id, player_id);
        assert_eq!(action.pc_id, Some(pc_id));
        assert_eq!(action.action_type, "talk".to_string());
        assert_eq!(action.target, Some(npc_id.to_string())); // target is NPC ID
        assert_eq!(action.dialogue, Some(initial_dialogue));
        assert_eq!(action.timestamp, now);
    }
}
