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

use crate::app::{App, Entities, UseCases};
use crate::infrastructure::ports::{
    ClockPort, ImageGenError, ImageGenPort, LlmError, LlmPort, QueueError, QueueItem, RandomPort,
};
use crate::infrastructure::ports::{
    MockAssetRepo, MockChallengeRepo, MockCharacterRepo, MockFlagRepo, MockItemRepo,
    MockLocationRepo, MockLocationStateRepo, MockLoreRepo, MockNarrativeRepo, MockObservationRepo,
    MockPlayerCharacterRepo, MockRegionStateRepo, MockSceneRepo, MockStagingRepo,
};

pub(crate) use crate::infrastructure::ports::{MockWorldRepo, QueuePort};

pub(crate) struct TestAppRepos {
    pub(crate) world_repo: MockWorldRepo,
    pub(crate) character_repo: MockCharacterRepo,
    pub(crate) player_character_repo: MockPlayerCharacterRepo,
    pub(crate) location_repo: MockLocationRepo,
    pub(crate) scene_repo: MockSceneRepo,
    pub(crate) challenge_repo: MockChallengeRepo,
    pub(crate) narrative_repo: MockNarrativeRepo,
    pub(crate) staging_repo: MockStagingRepo,
    pub(crate) observation_repo: MockObservationRepo,
    pub(crate) item_repo: MockItemRepo,
    pub(crate) asset_repo: MockAssetRepo,
    pub(crate) flag_repo: MockFlagRepo,
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

        Self {
            world_repo,
            character_repo,
            player_character_repo: MockPlayerCharacterRepo::new(),
            location_repo,
            scene_repo,
            challenge_repo: MockChallengeRepo::new(),
            narrative_repo: MockNarrativeRepo::new(),
            staging_repo: MockStagingRepo::new(),
            observation_repo: MockObservationRepo::new(),
            item_repo: MockItemRepo::new(),
            asset_repo: MockAssetRepo::new(),
            flag_repo: MockFlagRepo::new(),
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
        _data: &wrldbldr_domain::PlayerActionData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("noop".to_string()))
    }

    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_llm_request(
        &self,
        _data: &wrldbldr_domain::LlmRequestData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("noop".to_string()))
    }

    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_dm_approval(
        &self,
        _data: &wrldbldr_domain::ApprovalRequestData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("noop".to_string()))
    }

    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_asset_generation(
        &self,
        _data: &wrldbldr_domain::AssetGenerationData,
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
    ) -> Result<Option<wrldbldr_domain::ApprovalRequestData>, QueueError> {
        Ok(None)
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
    pub(crate) approvals: StdHashMap<Uuid, wrldbldr_domain::ApprovalRequestData>,
    pub(crate) completed: Vec<Uuid>,
    pub(crate) failed: Vec<(Uuid, String)>,
}

#[derive(Clone, Default)]
pub(crate) struct RecordingApprovalQueue {
    pub(crate) state: Arc<Mutex<RecordingApprovalQueueState>>,
}

impl RecordingApprovalQueue {
    pub(crate) fn insert_approval(&self, id: Uuid, data: wrldbldr_domain::ApprovalRequestData) {
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
        _data: &wrldbldr_domain::PlayerActionData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("not implemented".to_string()))
    }

    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_llm_request(
        &self,
        _data: &wrldbldr_domain::LlmRequestData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("not implemented".to_string()))
    }

    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_dm_approval(
        &self,
        _data: &wrldbldr_domain::ApprovalRequestData,
    ) -> Result<Uuid, QueueError> {
        Err(QueueError::Error("not implemented".to_string()))
    }

    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_asset_generation(
        &self,
        _data: &wrldbldr_domain::AssetGenerationData,
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
    ) -> Result<Option<wrldbldr_domain::ApprovalRequestData>, QueueError> {
        let guard = self.state.lock().unwrap();
        Ok(guard.approvals.get(&id).cloned())
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
    let challenge_repo = Arc::new(repos.challenge_repo);
    let narrative_repo = Arc::new(repos.narrative_repo);
    let staging_repo = Arc::new(repos.staging_repo);
    let observation_repo = Arc::new(repos.observation_repo);
    let item_repo = Arc::new(repos.item_repo);
    let asset_repo = Arc::new(repos.asset_repo);
    let flag_repo = Arc::new(repos.flag_repo);
    let lore_repo = Arc::new(repos.lore_repo);
    let location_state_repo = Arc::new(repos.location_state_repo);
    let region_state_repo = Arc::new(repos.region_state_repo);

    // Entities
    let character = Arc::new(crate::entities::Character::new(character_repo.clone()));
    let player_character = Arc::new(crate::entities::PlayerCharacter::new(
        player_character_repo.clone(),
    ));
    let location = Arc::new(crate::entities::Location::new(location_repo.clone()));
    let scene = Arc::new(crate::entities::Scene::new(scene_repo.clone()));
    let challenge = Arc::new(crate::entities::Challenge::new(challenge_repo.clone()));
    let narrative = Arc::new(crate::entities::Narrative::new(
        narrative_repo.clone(),
        location_repo.clone(),
        player_character_repo.clone(),
        observation_repo.clone(),
        challenge_repo.clone(),
        flag_repo.clone(),
        scene_repo.clone(),
        clock.clone(),
    ));
    let staging = Arc::new(crate::entities::Staging::new(staging_repo.clone()));
    let observation = Arc::new(crate::entities::Observation::new(
        observation_repo.clone(),
        location_repo.clone(),
        clock.clone(),
    ));
    let inventory = Arc::new(crate::entities::Inventory::new(
        item_repo.clone(),
        character_repo.clone(),
        player_character_repo.clone(),
    ));
    let assets = Arc::new(crate::entities::Assets::new(asset_repo.clone(), image_gen));
    let world = Arc::new(crate::entities::World::new(world_repo, clock.clone()));
    let flag = Arc::new(crate::entities::Flag::new(flag_repo.clone()));
    let lore = Arc::new(crate::entities::Lore::new(lore_repo.clone()));
    let location_state = Arc::new(crate::entities::LocationStateEntity::new(
        location_state_repo.clone(),
    ));
    let region_state = Arc::new(crate::entities::RegionStateEntity::new(region_state_repo));

    let entities = Entities {
        character: character.clone(),
        player_character: player_character.clone(),
        location: location.clone(),
        scene: scene.clone(),
        challenge: challenge.clone(),
        narrative: narrative.clone(),
        staging: staging.clone(),
        observation: observation.clone(),
        inventory: inventory.clone(),
        assets: assets.clone(),
        world: world.clone(),
        flag: flag.clone(),
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

    let conversation = crate::use_cases::ConversationUseCases::new(
        Arc::new(crate::use_cases::conversation::StartConversation::new(
            character.clone(),
            player_character.clone(),
            staging.clone(),
            scene.clone(),
            world.clone(),
            queue.clone(),
            clock.clone(),
        )),
        Arc::new(crate::use_cases::conversation::ContinueConversation::new(
            character.clone(),
            player_character.clone(),
            staging.clone(),
            world.clone(),
            queue.clone(),
            clock.clone(),
        )),
        Arc::new(crate::use_cases::conversation::EndConversation::new(
            character.clone(),
            player_character.clone(),
        )),
    );

    let challenge_uc = crate::use_cases::ChallengeUseCases::new(
        Arc::new(crate::use_cases::challenge::RollChallenge::new(
            challenge.clone(),
            player_character.clone(),
            queue.clone(),
            random,
            clock.clone(),
        )),
        Arc::new(crate::use_cases::challenge::ResolveOutcome::new(
            challenge.clone(),
            inventory.clone(),
            observation.clone(),
            scene.clone(),
            player_character.clone(),
        )),
    );

    let approval = crate::use_cases::ApprovalUseCases::new(
        Arc::new(crate::use_cases::approval::ApproveStaging::new(
            staging.clone(),
        )),
        Arc::new(crate::use_cases::approval::ApproveSuggestion::new(
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
        )),
        Arc::new(crate::use_cases::queues::ProcessLlmRequest::new(
            queue.clone(),
            llm.clone(),
        )),
    );

    let narrative_uc = crate::use_cases::NarrativeUseCases::new(Arc::new(
        crate::use_cases::narrative::ExecuteEffects::new(
            inventory.clone(),
            challenge.clone(),
            narrative.clone(),
            character.clone(),
            observation.clone(),
            player_character.clone(),
            scene.clone(),
            flag.clone(),
            clock.clone(),
        ),
    ));

    let time_uc = crate::use_cases::TimeUseCases::new(suggest_time);

    let visual_state_uc = crate::use_cases::VisualStateUseCases::new(Arc::new(
        crate::use_cases::visual_state::ResolveVisualState::new(
            location_state.clone(),
            region_state.clone(),
            flag.clone(),
        ),
    ));

    let use_cases = UseCases {
        movement,
        conversation,
        challenge: challenge_uc,
        approval,
        assets: assets_uc,
        world: world_uc,
        queues,
        narrative: narrative_uc,
        time: time_uc,
        visual_state: visual_state_uc,
    };

    Arc::new(App {
        entities,
        use_cases,
        queue,
        llm,
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
