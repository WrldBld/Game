use super::*;

use std::{
    collections::HashMap as StdHashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::routing::get;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

use crate::app::{App, Repositories, UseCases};
use crate::infrastructure::ports::{
    ClockPort, ImageGenError, ImageGenPort, LlmError, LlmPort, QueueError, QueueItem, RandomPort,
};
use crate::infrastructure::ports::{
    MockActRepo, MockAssetRepo, MockChallengeRepo, MockCharacterRepo, MockFlagRepo, MockGoalRepo,
    MockContentRepo, MockInteractionRepo, MockItemRepo, MockLocationRepo, MockLocationStateRepo,
    MockLoreRepo, MockNarrativeRepo, MockObservationRepo, MockPlayerCharacterRepo,
    MockRegionStateRepo, MockSceneRepo, MockSettingsRepo, MockStagingRepo,
};
use crate::queue_types::{ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData};

pub(crate) use crate::infrastructure::ports::{MockWorldRepo, QueuePort};

pub(crate) struct TestAppRepos {
    pub(crate) world_repo: MockWorldRepo,
    pub(crate) character_repo: MockCharacterRepo,
    pub(crate) player_character_repo: MockPlayerCharacterRepo,
    pub(crate) location_repo: MockLocationRepo,
    pub(crate) scene_repo: MockSceneRepo,
    pub(crate) act_repo: MockActRepo,
    pub(crate) content_repo: MockContentRepo,
    pub(crate) interaction_repo: MockInteractionRepo,
    pub(crate) settings_repo: MockSettingsRepo,
    pub(crate) challenge_repo: MockChallengeRepo,
    pub(crate) narrative_repo: MockNarrativeRepo,
    pub(crate) staging_repo: MockStagingRepo,
    pub(crate) observation_repo: MockObservationRepo,
    pub(crate) item_repo: MockItemRepo,
    pub(crate) asset_repo: MockAssetRepo,
    pub(crate) flag_repo: MockFlagRepo,
    pub(crate) goal_repo: MockGoalRepo,
    pub(crate) lore_repo: MockLoreRepo,
    pub(crate) location_state_repo: MockLocationStateRepo,
    pub(crate) region_state_repo: MockRegionStateRepo,
}

impl TestAppRepos {
    pub(crate) fn new(world_repo: MockWorldRepo) -> Self {
        let mut character_repo = MockCharacterRepo::new();
        // Joining a world now includes a lightweight snapshot.
        // Default to an empty world surface unless tests override.
        character_repo
            .expect_list_in_world()
            .returning(|_world_id| Ok(Vec::new()));

        let mut location_repo = MockLocationRepo::new();
        location_repo
            .expect_list_locations_in_world()
            .returning(|_world_id| Ok(Vec::new()));

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_get_current()
            .returning(|_world_id| Ok(None));

        let mut narrative_repo = MockNarrativeRepo::new();
        narrative_repo
            .expect_record_dialogue_context()
            .returning(|_, _, _, _, _, _, _, _, _, _, _, _| Ok(()));

        Self {
            world_repo,
            character_repo,
            player_character_repo: MockPlayerCharacterRepo::new(),
            location_repo,
            scene_repo,
            act_repo: MockActRepo::new(),
            content_repo: MockContentRepo::new(),
            interaction_repo: MockInteractionRepo::new(),
            settings_repo: MockSettingsRepo::new(),
            challenge_repo: MockChallengeRepo::new(),
            narrative_repo,
            staging_repo: MockStagingRepo::new(),
            observation_repo: MockObservationRepo::new(),
            item_repo: MockItemRepo::new(),
            asset_repo: MockAssetRepo::new(),
            flag_repo: MockFlagRepo::new(),
            goal_repo: MockGoalRepo::new(),
            lore_repo: MockLoreRepo::new(),
            location_state_repo: MockLocationStateRepo::new(),
            region_state_repo: MockRegionStateRepo::new(),
        }
    }
}

pub(crate) struct NoopQueue;

#[async_trait::async_trait]
impl QueuePort for NoopQueue {
    async fn enqueue_player_action(
        &self,
        _data: &PlayerActionData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("noop".to_string()))
    }

    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_llm_request(
        &self,
        _data: &LlmRequestData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("noop".to_string()))
    }

    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_dm_approval(
        &self,
        _data: &ApprovalRequestData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("noop".to_string()))
    }

    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_asset_generation(
        &self,
        _data: &AssetGenerationData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("noop".to_string()))
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
        _world_id: wrldbldr_domain::WorldId,
    ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
        Ok(None)
    }

    async fn upsert_generation_read_state(
        &self,
        _user_id: &str,
        _world_id: wrldbldr_domain::WorldId,
        _read_batches: &[String],
        _read_suggestions: &[String],
    ) -> Result<(), QueueError> {
        Ok(())
    }

    async fn delete_by_callback_id(&self, _callback_id: &str) -> Result<bool, QueueError> {
        Ok(false)
    }
}

pub(crate) struct NoopLlm;

#[async_trait::async_trait]
impl LlmPort for NoopLlm {
    async fn generate(
        &self,
        _request: crate::infrastructure::ports::LlmRequest,
    ) -> Result<crate::infrastructure::ports::LlmResponse, LlmError> {
        Err(LlmError::RequestFailed("noop".to_string()))
    }

    async fn generate_with_tools(
        &self,
        _request: crate::infrastructure::ports::LlmRequest,
        _tools: Vec<crate::infrastructure::ports::ToolDefinition>,
    ) -> Result<crate::infrastructure::ports::LlmResponse, LlmError> {
        Err(LlmError::RequestFailed("noop".to_string()))
    }
}

pub(crate) struct NoopImageGen;

#[async_trait::async_trait]
impl ImageGenPort for NoopImageGen {
    async fn generate(
        &self,
        _request: crate::infrastructure::ports::ImageRequest,
    ) -> Result<crate::infrastructure::ports::ImageResult, ImageGenError> {
        Err(ImageGenError::Unavailable)
    }

    async fn check_health(&self) -> Result<bool, ImageGenError> {
        Ok(false)
    }
}

pub(crate) struct FixedClock {
    pub(crate) now: DateTime<Utc>,
}

impl ClockPort for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}

pub(crate) struct FixedRandom;

impl RandomPort for FixedRandom {
    fn gen_range(&self, _min: i32, _max: i32) -> i32 {
        1
    }

    fn gen_uuid(&self) -> Uuid {
        Uuid::nil()
    }
}

#[derive(Default)]
pub(crate) struct RecordingApprovalQueueState {
    pub(crate) approvals: StdHashMap<Uuid, ApprovalRequestData>,
    pub(crate) completed: Vec<Uuid>,
    pub(crate) failed: Vec<(Uuid, String)>,
}

#[derive(Clone, Default)]
pub(crate) struct RecordingApprovalQueue {
    pub(crate) state: Arc<Mutex<RecordingApprovalQueueState>>,
}

impl RecordingApprovalQueue {
    pub(crate) fn insert_approval(&self, id: Uuid, data: ApprovalRequestData) {
        let mut guard = self.state.lock().unwrap();
        guard.approvals.insert(id, data);
    }

    pub(crate) fn completed_contains(&self, id: Uuid) -> bool {
        let guard = self.state.lock().unwrap();
        guard.completed.contains(&id)
    }

    pub(crate) fn failed_contains(&self, id: Uuid) -> bool {
        let guard = self.state.lock().unwrap();
        guard.failed.iter().any(|(got, _)| *got == id)
    }
}

#[async_trait::async_trait]
impl QueuePort for RecordingApprovalQueue {
    async fn enqueue_player_action(
        &self,
        _data: &PlayerActionData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("not implemented".to_string()))
    }

    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_llm_request(
        &self,
        _data: &LlmRequestData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("not implemented".to_string()))
    }

    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_dm_approval(
        &self,
        _data: &ApprovalRequestData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("not implemented".to_string()))
    }

    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_asset_generation(
        &self,
        _data: &AssetGenerationData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("not implemented".to_string()))
    }

    async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn mark_complete(&self, id: Uuid) -> Result<(), QueueError> {
        let mut guard = self.state.lock().unwrap();
        guard.completed.push(id);
        Ok(())
    }

    async fn mark_failed(&self, id: Uuid, error: &str) -> Result<(), QueueError> {
        let mut guard = self.state.lock().unwrap();
        guard.failed.push((id, error.to_string()));
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
        id: Uuid,
    ) -> Result<Option<ApprovalRequestData>, QueueError> {
        let guard = self.state.lock().unwrap();
        Ok(guard.approvals.get(&id).cloned())
    }

    async fn get_generation_read_state(
        &self,
        _user_id: &str,
        _world_id: wrldbldr_domain::WorldId,
    ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
        Ok(None)
    }

    async fn upsert_generation_read_state(
        &self,
        _user_id: &str,
        _world_id: wrldbldr_domain::WorldId,
        _read_batches: &[String],
        _read_suggestions: &[String],
    ) -> Result<(), QueueError> {
        Ok(())
    }

    async fn delete_by_callback_id(&self, _callback_id: &str) -> Result<bool, QueueError> {
        Ok(false)
    }
}

pub(crate) struct FixedLlm {
    pub(crate) content: String,
}

#[async_trait::async_trait]
impl LlmPort for FixedLlm {
    async fn generate(
        &self,
        _request: crate::infrastructure::ports::LlmRequest,
    ) -> Result<crate::infrastructure::ports::LlmResponse, LlmError> {
        Ok(crate::infrastructure::ports::LlmResponse {
            content: self.content.clone(),
            tool_calls: vec![],
            finish_reason: crate::infrastructure::ports::FinishReason::Stop,
            usage: None,
        })
    }

    async fn generate_with_tools(
        &self,
        request: crate::infrastructure::ports::LlmRequest,
        _tools: Vec<crate::infrastructure::ports::ToolDefinition>,
    ) -> Result<crate::infrastructure::ports::LlmResponse, LlmError> {
        self.generate(request).await
    }
}

pub(crate) fn build_test_app_with_ports(
    repos: TestAppRepos,
    now: DateTime<Utc>,
    queue: Arc<dyn QueuePort>,
    llm: Arc<dyn LlmPort>,
) -> Arc<App> {
    let clock: Arc<dyn ClockPort> = Arc::new(FixedClock { now });
    let random: Arc<dyn RandomPort> = Arc::new(FixedRandom);
    let image_gen: Arc<dyn ImageGenPort> = Arc::new(NoopImageGen);

    // Repo mocks.
    let world_repo = Arc::new(repos.world_repo);
    let character_repo = Arc::new(repos.character_repo);
    let player_character_repo = Arc::new(repos.player_character_repo);
    let location_repo = Arc::new(repos.location_repo);
    let scene_repo = Arc::new(repos.scene_repo);
    let act_repo = Arc::new(repos.act_repo);
    let content_repo = Arc::new(repos.content_repo);
    let interaction_repo = Arc::new(repos.interaction_repo);
    let settings_repo = Arc::new(repos.settings_repo);
    let challenge_repo = Arc::new(repos.challenge_repo);
    let narrative_port = Arc::new(repos.narrative_repo);
    let staging_repo = Arc::new(repos.staging_repo);
    let observation_repo = Arc::new(repos.observation_repo);
    let item_repo = Arc::new(repos.item_repo);
    let asset_repo = Arc::new(repos.asset_repo);
    let flag_repo = Arc::new(repos.flag_repo);
    let goal_repo = Arc::new(repos.goal_repo);
    let lore_repo = Arc::new(repos.lore_repo);
    let location_state_repo = Arc::new(repos.location_state_repo);
    let region_state_repo = Arc::new(repos.region_state_repo);

    // Entities
    let character = Arc::new(crate::repositories::character::Character::new(
        character_repo.clone(),
    ));
    let player_character = Arc::new(crate::repositories::PlayerCharacter::new(
        player_character_repo.clone(),
    ));
    let location = Arc::new(crate::repositories::location::Location::new(
        location_repo.clone(),
    ));
    let scene = Arc::new(crate::repositories::scene::Scene::new(scene_repo.clone()));
    let act = Arc::new(crate::repositories::Act::new(act_repo.clone()));
    let content = Arc::new(crate::repositories::Content::new(content_repo.clone()));
    let interaction = Arc::new(crate::repositories::Interaction::new(
        interaction_repo.clone(),
    ));
    let challenge = Arc::new(crate::repositories::Challenge::new(challenge_repo.clone()));
    let observation = Arc::new(crate::repositories::Observation::new(
        observation_repo.clone(),
        location_repo.clone(),
        clock.clone(),
    ));
    let flag = Arc::new(crate::repositories::Flag::new(flag_repo.clone()));
    let world = Arc::new(crate::repositories::World::new(world_repo.clone(), clock.clone()));
    let narrative_repo = Arc::new(crate::repositories::Narrative::new(narrative_port));
    let narrative = Arc::new(crate::use_cases::narrative_operations::Narrative::new(
        narrative_repo,
        location.clone(),
        world.clone(),
        player_character.clone(),
        character.clone(),
        observation.clone(),
        challenge.clone(),
        flag.clone(),
        scene.clone(),
        clock.clone(),
    ));
    let staging = Arc::new(crate::repositories::staging::Staging::new(
        staging_repo.clone(),
    ));
    let inventory = Arc::new(crate::repositories::inventory::Inventory::new(
        item_repo.clone(),
        character_repo.clone(),
        player_character_repo.clone(),
    ));
    let assets = Arc::new(crate::repositories::Assets::new(
        asset_repo.clone(),
        image_gen,
    ));
    let goal = Arc::new(crate::repositories::Goal::new(goal_repo.clone()));
    let lore = Arc::new(crate::repositories::Lore::new(lore_repo.clone()));
    let location_state = Arc::new(crate::repositories::LocationStateEntity::new(
        location_state_repo.clone(),
    ));
    let region_state = Arc::new(crate::repositories::RegionStateEntity::new(
        region_state_repo,
    ));

    let repositories_container = Repositories {
        character: character.clone(),
        player_character: player_character.clone(),
        location: location.clone(),
        scene: scene.clone(),
        act: act.clone(),
        content: content.clone(),
        interaction: interaction.clone(),
        challenge: challenge.clone(),
        narrative: narrative.clone(),
        staging: staging.clone(),
        observation: observation.clone(),
        inventory: inventory.clone(),
        assets: assets.clone(),
        world: world.clone(),
        flag: flag.clone(),
        goal: goal.clone(),
        lore: lore.clone(),
        location_state: location_state.clone(),
        region_state: region_state.clone(),
    };

    // Use cases (not exercised by these tests, but required by App).
    let suggest_time = Arc::new(crate::use_cases::time::SuggestTime::new(
        world.clone(),
        clock.clone(),
    ));

    let movement = crate::use_cases::MovementUseCases::new(
        Arc::new(crate::use_cases::movement::EnterRegion::new(
            player_character.clone(),
            location.clone(),
            staging.clone(),
            observation.clone(),
            narrative.clone(),
            scene.clone(),
            inventory.clone(),
            flag.clone(),
            world.clone(),
            suggest_time.clone(),
        )),
        Arc::new(crate::use_cases::movement::ExitLocation::new(
            player_character.clone(),
            location.clone(),
            staging.clone(),
            observation.clone(),
            narrative.clone(),
            scene.clone(),
            inventory.clone(),
            flag.clone(),
            world.clone(),
            suggest_time.clone(),
        )),
    );

    let scene_change =
        crate::use_cases::SceneChangeBuilder::new(location.clone(), inventory.clone());

    let conversation_start = Arc::new(crate::use_cases::conversation::StartConversation::new(
        character.clone(),
        player_character.clone(),
        staging.clone(),
        scene.clone(),
        world.clone(),
        queue.clone(),
        clock.clone(),
    ));
    let conversation_continue =
        Arc::new(crate::use_cases::conversation::ContinueConversation::new(
            character.clone(),
            player_character.clone(),
            staging.clone(),
            world.clone(),
            narrative.clone(),
            queue.clone(),
            clock.clone(),
        ));
    let conversation_end = Arc::new(crate::use_cases::conversation::EndConversation::new(
        character.clone(),
        player_character.clone(),
        narrative.clone(),
    ));
    let conversation = crate::use_cases::ConversationUseCases::new(
        conversation_start.clone(),
        conversation_continue,
        conversation_end,
    );

    let player_action = crate::use_cases::PlayerActionUseCases::new(Arc::new(
        crate::use_cases::player_action::HandlePlayerAction::new(
            conversation_start,
            queue.clone(),
            clock.clone(),
        ),
    ));

    let actantial = crate::use_cases::ActantialUseCases::new(
        crate::use_cases::actantial::GoalOps::new(goal.clone()),
        crate::use_cases::actantial::WantOps::new(character.clone(), clock.clone()),
        crate::use_cases::actantial::ActantialContextOps::new(character.clone()),
    );

    let ai = crate::use_cases::AiUseCases::new(Arc::new(crate::use_cases::ai::SuggestionOps::new(
        queue.clone(),
        world.clone(),
        character.clone(),
    )));

    let resolve_outcome = Arc::new(crate::use_cases::challenge::ResolveOutcome::new(
        challenge.clone(),
        inventory.clone(),
        observation.clone(),
        scene.clone(),
        player_character.clone(),
    ));
    let outcome_decision = Arc::new(crate::use_cases::challenge::OutcomeDecision::new(
        queue.clone(),
        resolve_outcome.clone(),
    ));

    let challenge_uc = crate::use_cases::ChallengeUseCases::new(
        Arc::new(crate::use_cases::challenge::RollChallenge::new(
            challenge.clone(),
            player_character.clone(),
            queue.clone(),
            random,
            clock.clone(),
        )),
        resolve_outcome,
        Arc::new(crate::use_cases::challenge::TriggerChallengePrompt::new(
            challenge.clone(),
        )),
        outcome_decision,
        Arc::new(crate::use_cases::challenge::ChallengeOps::new(
            challenge.clone(),
        )),
    );

    let approve_suggestion = Arc::new(crate::use_cases::approval::ApproveSuggestion::new(
        queue.clone(),
    ));
    let approval = crate::use_cases::ApprovalUseCases::new(
        Arc::new(crate::use_cases::approval::ApproveStaging::new(
            staging.clone(),
        )),
        approve_suggestion.clone(),
        Arc::new(crate::use_cases::approval::ApprovalDecisionFlow::new(
            approve_suggestion.clone(),
            narrative.clone(),
            queue.clone(),
        )),
    );

    let assets_uc = crate::use_cases::AssetUseCases::new(
        Arc::new(crate::use_cases::assets::GenerateAsset::new(
            assets.clone(),
            queue.clone(),
            clock.clone(),
        )),
        Arc::new(crate::use_cases::assets::GenerateExpressionSheet::new(
            assets.clone(),
            character.clone(),
            queue.clone(),
            clock.clone(),
        )),
    );

    let world_uc = crate::use_cases::WorldUseCases::new(
        Arc::new(crate::use_cases::world::ExportWorld::new(
            world.clone(),
            location.clone(),
            character.clone(),
            inventory.clone(),
            narrative.clone(),
        )),
        Arc::new(crate::use_cases::world::ImportWorld::new(
            world.clone(),
            location.clone(),
            character.clone(),
            inventory.clone(),
            narrative.clone(),
        )),
    );

    let queues = crate::use_cases::QueueUseCases::new(
        Arc::new(crate::use_cases::queues::ProcessPlayerAction::new(
            queue.clone(),
            character.clone(),
            player_character.clone(),
            staging.clone(),
            scene.clone(),
            world.clone(),
            narrative.clone(),
            location.clone(),
            challenge.clone(),
        )),
        Arc::new(crate::use_cases::queues::ProcessLlmRequest::new(
            queue.clone(),
            llm.clone(),
        )),
    );

    let execute_effects = Arc::new(crate::use_cases::narrative::ExecuteEffects::new(
        inventory.clone(),
        challenge.clone(),
        narrative.clone(),
        character.clone(),
        observation.clone(),
        player_character.clone(),
        scene.clone(),
        flag.clone(),
        world.clone(),
        clock.clone(),
    ));
    let narrative_events = Arc::new(crate::use_cases::narrative::NarrativeEventOps::new(
        narrative.clone(),
        execute_effects.clone(),
        clock.clone(),
    ));
    let narrative_chains = Arc::new(crate::use_cases::narrative::EventChainOps::new(
        narrative.clone(),
    ));
    let narrative_decision = Arc::new(crate::use_cases::narrative::NarrativeDecisionFlow::new(
        approve_suggestion.clone(),
        queue.clone(),
        narrative.clone(),
        execute_effects.clone(),
    ));
    let narrative_uc = crate::use_cases::NarrativeUseCases::new(
        execute_effects,
        narrative_events,
        narrative_chains,
        narrative_decision,
    );

    let time_control =
        Arc::new(crate::use_cases::time::TimeControl::new(world.clone(), clock.clone()));
    let time_suggestions = Arc::new(crate::use_cases::time::TimeSuggestions::new(
        time_control.clone(),
    ));
    let time_uc = crate::use_cases::TimeUseCases::new(suggest_time, time_control, time_suggestions);

    let visual_state_uc = crate::use_cases::VisualStateUseCases::new(Arc::new(
        crate::use_cases::visual_state::ResolveVisualState::new(
            location_state.clone(),
            region_state.clone(),
            flag.clone(),
        ),
    ));

    let settings_entity = Arc::new(crate::repositories::Settings::new(settings_repo.clone()));

    let staging_uc = crate::use_cases::StagingUseCases::new(
        Arc::new(crate::use_cases::staging::RequestStagingApproval::new(
            character.clone(),
            staging.clone(),
            location.clone(),
            world.clone(),
            flag.clone(),
            visual_state_uc.resolve.clone(),
            settings_entity.clone(),
            llm.clone(),
        )),
        Arc::new(
            crate::use_cases::staging::RegenerateStagingSuggestions::new(
                location.clone(),
                character.clone(),
                llm.clone(),
            ),
        ),
        Arc::new(crate::use_cases::staging::ApproveStagingRequest::new(
            staging.clone(),
            world.clone(),
            character.clone(),
            location.clone(),
            location_state.clone(),
            region_state.clone(),
        )),
        Arc::new(crate::use_cases::staging::AutoApproveStagingTimeout::new(
            character.clone(),
            staging.clone(),
            world.clone(),
            location.clone(),
            location_state.clone(),
            region_state.clone(),
            settings_entity.clone(),
        )),
    );

    let npc_uc = crate::use_cases::NpcUseCases::new(
        Arc::new(crate::use_cases::npc::NpcDisposition::new(
            character.clone(),
            clock.clone(),
        )),
        Arc::new(crate::use_cases::npc::NpcMood::new(
            staging.clone(),
            character.clone(),
        )),
        Arc::new(crate::use_cases::npc::NpcRegionRelationships::new(
            character.clone(),
        )),
        Arc::new(crate::use_cases::npc::NpcLocationSharing::new(
            character.clone(),
            location.clone(),
            observation.clone(),
            clock.clone(),
        )),
        Arc::new(crate::use_cases::npc::NpcApproachEvents::new(
            character.clone(),
        )),
    );

    let story_events_uc = crate::use_cases::StoryEventUseCases::new(Arc::new(
        crate::use_cases::story_events::StoryEventOps::new(narrative.clone()),
    ));

    let lore_uc = crate::use_cases::LoreUseCases::new(Arc::new(
        crate::use_cases::lore::LoreOps::new(lore.clone()),
    ));

    let location_events_uc = crate::use_cases::LocationEventUseCases::new(Arc::new(
        crate::use_cases::location_events::TriggerLocationEvent::new(location.clone()),
    ));

    let management = crate::use_cases::ManagementUseCases::new(
        crate::use_cases::management::WorldCrud::new(world.clone(), clock.clone()),
        crate::use_cases::management::CharacterCrud::new(character.clone(), clock.clone()),
        crate::use_cases::management::LocationCrud::new(location.clone()),
        crate::use_cases::management::PlayerCharacterCrud::new(
            player_character.clone(),
            location.clone(),
            clock.clone(),
        ),
        crate::use_cases::management::RelationshipCrud::new(character.clone(), clock.clone()),
        crate::use_cases::management::ObservationCrud::new(
            observation.clone(),
            player_character.clone(),
            character.clone(),
            location.clone(),
            world.clone(),
            clock.clone(),
        ),
        crate::use_cases::management::ActCrud::new(act.clone()),
        crate::use_cases::management::SceneCrud::new(scene.clone()),
        crate::use_cases::management::InteractionCrud::new(interaction.clone()),
        crate::use_cases::management::SkillCrud::new(content.clone()),
    );

    let settings = settings_entity;

    let join_world = Arc::new(crate::use_cases::session::JoinWorld::new(
        world.clone(),
        location.clone(),
        character.clone(),
        scene.clone(),
        player_character.clone(),
    ));
    let join_world_flow = Arc::new(crate::use_cases::session::JoinWorldFlow::new(
        join_world.clone(),
    ));
    let directorial_update = Arc::new(crate::use_cases::session::DirectorialUpdate::new());
    let session =
        crate::use_cases::SessionUseCases::new(join_world, join_world_flow, directorial_update);

    // Create custom condition evaluator
    let custom_condition = Arc::new(crate::use_cases::CustomConditionEvaluator::new(llm.clone()));

    let use_cases = UseCases {
        movement,
        conversation,
        challenge: challenge_uc,
        approval,
        actantial,
        ai,
        assets: assets_uc,
        scene_change,
        world: world_uc,
        queues,
        narrative: narrative_uc,
        player_action,
        time: time_uc,
        visual_state: visual_state_uc,
        management,
        session,
        settings,
        staging: staging_uc,
        npc: npc_uc,
        story_events: story_events_uc,
        lore: lore_uc,
        location_events: location_events_uc,
        custom_condition,
    };

    // Create content service for game content (races, classes, spells, etc.)
    let content = Arc::new(crate::use_cases::content::ContentService::new(
        crate::use_cases::content::ContentServiceConfig::default(),
    ));

    Arc::new(App {
        repositories: repositories_container,
        use_cases,
        queue,
        llm,
        content,
    })
}

pub(crate) fn build_test_app(repos: TestAppRepos, now: DateTime<Utc>) -> Arc<App> {
    build_test_app_with_ports(repos, now, Arc::new(NoopQueue), Arc::new(NoopLlm))
}

pub(crate) async fn spawn_ws_server(
    state: Arc<WsState>,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let router = axum::Router::new().route("/ws", get(ws_handler).with_state(state));

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (addr, handle)
}

pub(crate) async fn ws_connect(
    addr: SocketAddr,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let url = format!("ws://{}/ws", addr);
    let (ws, _resp) = connect_async(url).await.unwrap();
    ws
}

pub(crate) async fn ws_send_client(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    msg: &wrldbldr_protocol::ClientMessage,
) {
    let json = serde_json::to_string(msg).unwrap();
    ws.send(WsMessage::Text(json.into())).await.unwrap();
}

pub(crate) async fn ws_recv_server(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> wrldbldr_protocol::ServerMessage {
    loop {
        let msg = ws.next().await.unwrap().unwrap();
        match msg {
            WsMessage::Text(text) => {
                return serde_json::from_str::<wrldbldr_protocol::ServerMessage>(&text).unwrap();
            }
            WsMessage::Binary(bin) => {
                let text = String::from_utf8(bin).unwrap();
                return serde_json::from_str::<wrldbldr_protocol::ServerMessage>(&text).unwrap();
            }
            _ => {}
        }
    }
}

pub(crate) async fn ws_expect_message<F>(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    timeout: Duration,
    mut predicate: F,
) -> wrldbldr_protocol::ServerMessage
where
    F: FnMut(&wrldbldr_protocol::ServerMessage) -> bool,
{
    tokio::time::timeout(timeout, async {
        loop {
            let msg = ws_recv_server(ws).await;
            if predicate(&msg) {
                return msg;
            }
        }
    })
    .await
    .unwrap()
}

pub(crate) async fn ws_expect_no_message_matching<F>(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    timeout: Duration,
    mut predicate: F,
) where
    F: FnMut(&wrldbldr_protocol::ServerMessage) -> bool,
{
    let result = tokio::time::timeout(timeout, async {
        loop {
            let msg = ws_recv_server(ws).await;
            if predicate(&msg) {
                panic!("unexpected message: {:?}", msg);
            }
        }
    })
    .await;

    // We only succeed if we timed out without seeing a matching message.
    assert!(result.is_err());
}
