//! WebSocket handling for Player connections.
//!
//! Handles the WebSocket protocol between Engine and Player clients.

use std::{sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

mod ws_actantial;
mod ws_approval;
mod ws_challenge;
mod ws_character_sheet;
mod ws_content;
mod ws_conversation;
mod ws_core;
mod ws_creator;
mod ws_dm;
mod ws_event_chain;
mod ws_inventory;
mod ws_location;
mod ws_lore;
mod ws_movement;
mod ws_narrative_event;
mod ws_player;
mod ws_player_action;
mod ws_scene;
mod ws_session;
mod ws_skill;
mod ws_staging;
mod ws_stat;
mod ws_story_events;
mod ws_time;

pub mod error_sanitizer;

use wrldbldr_domain::{
    ActId, ChallengeId, CharacterId, EventChainId, GoalId, InteractionId, ItemId, LocationId,
    MoodState, NarrativeEventId, PlayerCharacterId, RegionId, SceneId, SkillId, StagingSource,
    WantId, WorldId,
};
use wrldbldr_protocol::{
    ClientMessage, ErrorCode, RequestPayload, ResponseResult, ServerMessage,
    WorldRole as ProtoWorldRole,
};

use super::connections::ConnectionManager;
use crate::app::App;
use crate::infrastructure::cache::TtlCache;
use crate::infrastructure::ports::{
    PendingStagingRequest, PendingStagingStore, TimeSuggestion, TimeSuggestionStore,
};

/// Buffer size for per-connection message channel.
const CONNECTION_CHANNEL_BUFFER: usize = 256;

/// TTL for pending staging requests (1 hour).
const STAGING_REQUEST_TTL: Duration = Duration::from_secs(60 * 60);

/// TTL for time suggestions (30 minutes).
const TIME_SUGGESTION_TTL: Duration = Duration::from_secs(30 * 60);

/// TTL for generation read state (5 minutes).
const GENERATION_STATE_TTL: Duration = Duration::from_secs(5 * 60);

/// Combined state for WebSocket handlers.
pub struct WsState {
    pub app: Arc<App>,
    pub connections: Arc<ConnectionManager>,
    pub pending_time_suggestions: TimeSuggestionStoreImpl,
    pub pending_staging_requests: PendingStagingStoreImpl,
    pub generation_read_state: GenerationStateStoreImpl,
}

impl WsState {
    /// Cleanup expired entries from all TTL caches.
    /// Returns total number of entries removed.
    pub async fn cleanup_expired(&self) -> usize {
        let staging = self.pending_staging_requests.cleanup_expired().await;
        let time = self.pending_time_suggestions.cleanup_expired().await;
        let generation = self.generation_read_state.cleanup_expired().await;
        staging + time + generation
    }
}

// =============================================================================
// Store Implementations (TTL-based)
// =============================================================================

/// TTL-based implementation of PendingStagingStore (1 hour TTL).
pub struct PendingStagingStoreImpl {
    inner: TtlCache<String, PendingStagingRequest>,
}

impl PendingStagingStoreImpl {
    pub fn new() -> Self {
        Self {
            inner: TtlCache::new(STAGING_REQUEST_TTL),
        }
    }

    /// Insert a pending request.
    pub async fn insert(&self, key: String, request: PendingStagingRequest) {
        self.inner.insert(key, request).await;
    }

    /// Get a pending request by key.
    pub async fn get(&self, key: &str) -> Option<PendingStagingRequest> {
        self.inner.get(&key.to_string()).await
    }

    /// Remove and return a pending request.
    pub async fn remove(&self, key: &str) -> Option<PendingStagingRequest> {
        self.inner.remove(&key.to_string()).await
    }

    /// Check if a key exists.
    pub async fn contains(&self, key: &str) -> bool {
        self.inner.contains(&key.to_string()).await
    }

    /// Get all non-expired entries.
    pub async fn entries(&self) -> Vec<(String, PendingStagingRequest)> {
        self.inner.entries().await
    }

    /// Remove expired entries and return count.
    pub async fn cleanup_expired(&self) -> usize {
        self.inner.cleanup_expired().await
    }
}

impl Default for PendingStagingStoreImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PendingStagingStore for PendingStagingStoreImpl {
    async fn insert(&self, key: String, request: PendingStagingRequest) {
        self.inner.insert(key, request).await;
    }

    async fn get(&self, key: &str) -> Option<PendingStagingRequest> {
        self.inner.get(&key.to_string()).await
    }

    async fn remove(&self, key: &str) -> Option<PendingStagingRequest> {
        self.inner.remove(&key.to_string()).await
    }
}

/// TTL-based implementation of TimeSuggestionStore (30 minute TTL).
pub struct TimeSuggestionStoreImpl {
    inner: TtlCache<Uuid, TimeSuggestion>,
}

impl TimeSuggestionStoreImpl {
    pub fn new() -> Self {
        Self {
            inner: TtlCache::new(TIME_SUGGESTION_TTL),
        }
    }

    /// Insert a time suggestion.
    pub async fn insert(&self, key: Uuid, suggestion: TimeSuggestion) {
        self.inner.insert(key, suggestion).await;
    }

    /// Get a time suggestion by key.
    pub async fn get(&self, key: &Uuid) -> Option<TimeSuggestion> {
        self.inner.get(key).await
    }

    /// Remove and return a time suggestion.
    pub async fn remove(&self, key: &Uuid) -> Option<TimeSuggestion> {
        self.inner.remove(key).await
    }

    /// Remove all suggestions for a given PC.
    /// This prevents unbounded growth when a player performs multiple actions
    /// before the DM resolves the first suggestion.
    pub async fn remove_for_pc(&self, pc_id: wrldbldr_domain::PlayerCharacterId) {
        let entries = self.inner.entries().await;
        for (key, suggestion) in entries {
            if suggestion.pc_id == pc_id {
                self.inner.remove(&key).await;
            }
        }
    }

    /// Remove expired entries and return count.
    pub async fn cleanup_expired(&self) -> usize {
        self.inner.cleanup_expired().await
    }
}

impl Default for TimeSuggestionStoreImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TimeSuggestionStore for TimeSuggestionStoreImpl {
    async fn insert(&self, key: Uuid, suggestion: TimeSuggestion) {
        self.inner.insert(key, suggestion).await;
    }

    async fn remove(&self, key: Uuid) -> Option<TimeSuggestion> {
        self.inner.remove(&key).await
    }
}

/// TTL-based store for generation read state (5 minute TTL).
pub struct GenerationStateStoreImpl {
    inner: TtlCache<String, ws_creator::GenerationReadState>,
}

impl GenerationStateStoreImpl {
    pub fn new() -> Self {
        Self {
            inner: TtlCache::new(GENERATION_STATE_TTL),
        }
    }

    /// Get generation state by key.
    pub async fn get(&self, key: &str) -> Option<ws_creator::GenerationReadState> {
        self.inner.get(&key.to_string()).await
    }

    /// Insert or update generation state.
    pub async fn insert(&self, key: String, state: ws_creator::GenerationReadState) {
        self.inner.insert(key, state).await;
    }

    /// Remove and return generation state.
    pub async fn remove(&self, key: &str) -> Option<ws_creator::GenerationReadState> {
        self.inner.remove(&key.to_string()).await
    }

    /// Remove expired entries and return count.
    pub async fn cleanup_expired(&self) -> usize {
        self.inner.cleanup_expired().await
    }
}

impl Default for GenerationStateStoreImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler - entry point for new connections.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<WsState>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an individual WebSocket connection.
async fn handle_socket(socket: WebSocket, state: Arc<WsState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create a unique client ID for this connection
    let connection_id = Uuid::new_v4();
    let user_id = connection_id.to_string(); // Anonymous user for now

    // Create a bounded channel for sending messages to this client
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(CONNECTION_CHANNEL_BUFFER);

    // Register the connection
    state
        .connections
        .register(connection_id, user_id.clone(), tx.clone())
        .await;

    tracing::info!(connection_id = %connection_id, "WebSocket connection established");

    // Spawn a task to forward messages from the channel to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(Message::Text(text)) => match serde_json::from_str::<ClientMessage>(text.as_str()) {
                Ok(msg) => {
                    if let Some(response) =
                        handle_message(msg, &state, connection_id, tx.clone()).await
                    {
                        if tx.try_send(response).is_err() {
                            tracing::warn!(
                                connection_id = %connection_id,
                                "Failed to send response, channel full or closed"
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(connection_id = %connection_id, error = %e, "Failed to parse message");
                    let error = ServerMessage::Error {
                        code: "PARSE_ERROR".to_string(),
                        message: format!("Invalid message format: {}", e),
                    };
                    let _ = tx.try_send(error);
                }
            },
            Ok(Message::Ping(_)) => {
                let _ = tx.try_send(ServerMessage::Pong);
            }
            Ok(Message::Close(_)) => {
                tracing::info!(connection_id = %connection_id, "WebSocket closed by client");
                break;
            }
            Err(e) => {
                tracing::error!(connection_id = %connection_id, error = %e, "WebSocket error");
                break;
            }
            _ => {}
        }
    }

    // Clean up
    state.connections.unregister(connection_id).await;
    send_task.abort();

    tracing::info!(connection_id = %connection_id, "WebSocket connection terminated");
}

/// Dispatch a parsed client message to the appropriate handler.
async fn handle_message(
    msg: ClientMessage,
    state: &WsState,
    connection_id: Uuid,
    _sender: mpsc::Sender<ServerMessage>,
) -> Option<ServerMessage> {
    match msg {
        // Connection lifecycle
        ClientMessage::Heartbeat => Some(ServerMessage::Pong),

        ClientMessage::JoinWorld {
            world_id,
            role,
            user_id,
            pc_id,
            spectate_pc_id,
        } => {
            tracing::info!(
                connection_id = %connection_id,
                world_id = %world_id,
                ?role,
                %user_id,
                "JoinWorld message received"
            );
            ws_session::handle_join_world(
                state,
                connection_id,
                world_id,
                role,
                user_id,
                pc_id,
                spectate_pc_id,
            )
            .await
        }

        ClientMessage::LeaveWorld => ws_session::handle_leave_world(state, connection_id).await,

        // Movement
        ClientMessage::MoveToRegion { pc_id, region_id } => {
            ws_movement::handle_move_to_region(state, connection_id, pc_id, region_id).await
        }

        ClientMessage::ExitToLocation {
            pc_id,
            location_id,
            arrival_region_id,
        } => {
            ws_movement::handle_exit_to_location(
                state,
                connection_id,
                pc_id,
                location_id,
                arrival_region_id,
            )
            .await
        }

        // Inventory
        ClientMessage::EquipItem { pc_id, item_id } => {
            ws_inventory::handle_inventory_action(
                state,
                connection_id,
                ws_inventory::InventoryAction::Equip,
                &pc_id,
                &item_id,
                1,
            )
            .await
        }
        ClientMessage::UnequipItem { pc_id, item_id } => {
            ws_inventory::handle_inventory_action(
                state,
                connection_id,
                ws_inventory::InventoryAction::Unequip,
                &pc_id,
                &item_id,
                1,
            )
            .await
        }
        ClientMessage::DropItem {
            pc_id,
            item_id,
            quantity,
        } => {
            ws_inventory::handle_inventory_action(
                state,
                connection_id,
                ws_inventory::InventoryAction::Drop,
                &pc_id,
                &item_id,
                quantity,
            )
            .await
        }
        ClientMessage::PickupItem { pc_id, item_id } => {
            ws_inventory::handle_inventory_action(
                state,
                connection_id,
                ws_inventory::InventoryAction::Pickup,
                &pc_id,
                &item_id,
                1,
            )
            .await
        }

        // Request/Response pattern (CRUD operations)
        ClientMessage::Request {
            request_id,
            payload,
        } => handle_request(state, connection_id, request_id, payload).await,

        // Challenge handlers
        ClientMessage::ChallengeRoll { challenge_id, roll } => {
            ws_challenge::handle_challenge_roll(state, connection_id, challenge_id, roll).await
        }

        ClientMessage::ChallengeRollInput {
            challenge_id,
            input_type,
        } => {
            ws_challenge::handle_challenge_roll_input(
                state,
                connection_id,
                challenge_id,
                input_type,
            )
            .await
        }

        ClientMessage::TriggerChallenge {
            challenge_id,
            target_character_id,
        } => {
            ws_challenge::handle_trigger_challenge(
                state,
                connection_id,
                challenge_id,
                target_character_id,
            )
            .await
        }

        // Staging handlers
        ClientMessage::StagingApprovalResponse {
            request_id,
            approved_npcs,
            ttl_hours,
            source,
            location_state_id,
            region_state_id,
        } => {
            ws_staging::handle_staging_approval(
                state,
                connection_id,
                request_id,
                approved_npcs,
                ttl_hours,
                source,
                location_state_id,
                region_state_id,
            )
            .await
        }

        ClientMessage::StagingRegenerateRequest {
            request_id,
            guidance,
        } => {
            ws_staging::handle_staging_regenerate(state, connection_id, request_id, guidance).await
        }

        ClientMessage::PreStageRegion {
            region_id,
            npcs,
            ttl_hours,
            location_state_id,
            region_state_id,
        } => {
            ws_staging::handle_pre_stage_region(
                state,
                connection_id,
                region_id,
                npcs,
                ttl_hours,
                location_state_id,
                region_state_id,
            )
            .await
        }

        // Approval handlers
        ClientMessage::ApprovalDecision {
            request_id,
            decision,
        } => {
            ws_approval::handle_approval_decision(state, connection_id, request_id, decision).await
        }

        ClientMessage::ChallengeSuggestionDecision {
            request_id,
            approved,
            modified_difficulty,
        } => {
            ws_challenge::handle_challenge_suggestion_decision(
                state,
                connection_id,
                request_id,
                approved,
                modified_difficulty,
            )
            .await
        }

        ClientMessage::ChallengeOutcomeDecision {
            resolution_id,
            decision,
        } => {
            ws_challenge::handle_challenge_outcome_decision(
                state,
                connection_id,
                resolution_id,
                decision,
            )
            .await
        }

        ClientMessage::NarrativeEventSuggestionDecision {
            request_id,
            event_id,
            approved,
            selected_outcome,
        } => {
            ws_narrative_event::handle_narrative_event_decision(
                state,
                connection_id,
                request_id,
                event_id,
                approved,
                selected_outcome,
            )
            .await
        }

        // DM action handlers
        ClientMessage::DirectorialUpdate { context } => {
            ws_dm::handle_directorial_update(state, connection_id, context).await
        }

        ClientMessage::TriggerApproachEvent {
            npc_id,
            target_pc_id,
            description,
            reveal,
        } => {
            ws_dm::handle_trigger_approach_event(
                state,
                connection_id,
                npc_id,
                target_pc_id,
                description,
                reveal,
            )
            .await
        }

        ClientMessage::TriggerLocationEvent {
            region_id,
            description,
        } => {
            ws_dm::handle_trigger_location_event(state, connection_id, region_id, description).await
        }

        ClientMessage::ShareNpcLocation {
            pc_id,
            npc_id,
            location_id,
            region_id,
            notes,
        } => {
            ws_dm::handle_share_npc_location(
                state,
                connection_id,
                pc_id,
                npc_id,
                location_id,
                region_id,
                notes,
            )
            .await
        }

        // Time control handlers (DM only)
        ClientMessage::SetGameTime {
            world_id,
            day,
            hour,
            notify_players,
        } => {
            ws_time::handle_set_game_time(state, connection_id, world_id, day, hour, notify_players)
                .await
        }

        ClientMessage::SkipToPeriod { world_id, period } => {
            ws_time::handle_skip_to_period(state, connection_id, world_id, period).await
        }

        ClientMessage::PauseGameTime { world_id, paused } => {
            ws_time::handle_pause_game_time(state, connection_id, world_id, paused).await
        }

        ClientMessage::SetTimeMode { world_id, mode } => {
            ws_time::handle_set_time_mode(state, connection_id, world_id, mode).await
        }

        ClientMessage::SetTimeCosts { world_id, costs } => {
            ws_time::handle_set_time_costs(state, connection_id, world_id, costs).await
        }

        ClientMessage::RespondToTimeSuggestion {
            suggestion_id,
            decision,
        } => {
            ws_time::handle_respond_to_time_suggestion(
                state,
                connection_id,
                suggestion_id,
                decision,
            )
            .await
        }

        // Player action handler
        ClientMessage::PlayerAction {
            action_type,
            target,
            dialogue,
        } => {
            ws_player_action::handle_player_action(
                state,
                connection_id,
                action_type,
                target,
                dialogue,
            )
            .await
        }

        ClientMessage::StartConversation { npc_id, message } => {
            ws_conversation::handle_start_conversation(state, connection_id, npc_id, message).await
        }

        ClientMessage::ContinueConversation {
            npc_id,
            message,
            conversation_id,
        } => {
            ws_conversation::handle_continue_conversation(
                state,
                connection_id,
                npc_id,
                message,
                conversation_id,
            )
            .await
        }

        ClientMessage::PerformInteraction { interaction_id } => {
            ws_conversation::handle_perform_interaction(state, connection_id, interaction_id).await
        }

        // Forward compatibility - return error so client doesn't hang
        ClientMessage::Unknown => {
            tracing::warn!(connection_id = %connection_id, "Received unknown message type");
            Some(ServerMessage::Error {
                code: "UNKNOWN_MESSAGE".to_string(),
                message: "Unrecognized message type".to_string(),
            })
        }

        // All other message types - return not implemented for now
        _ => {
            tracing::debug!(connection_id = %connection_id, "Unhandled message type");
            Some(ServerMessage::Error {
                code: "NOT_IMPLEMENTED".to_string(),
                message: "This message type is not yet implemented".to_string(),
            })
        }
    }
}

async fn handle_request(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    payload: RequestPayload,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(ServerMessage::Response {
                request_id,
                result: ResponseResult::error(ErrorCode::BadRequest, "Connection not found"),
            });
        }
    };

    let dispatched: Result<ResponseResult, ServerMessage> = match payload {
        RequestPayload::Lore(req) => {
            ws_lore::handle_lore_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::StoryEvent(req) => {
            ws_story_events::handle_story_event_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::World(req) => {
            ws_core::handle_world_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Character(req) => {
            ws_core::handle_character_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Location(req) => {
            ws_location::handle_location_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Region(req) => {
            ws_location::handle_region_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Time(req) => {
            ws_core::handle_time_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Npc(req) => {
            ws_core::handle_npc_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Items(req) => {
            ws_core::handle_items_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::PlayerCharacter(req) => {
            ws_player::handle_player_character_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Relationship(req) => {
            ws_player::handle_relationship_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Observation(req) => {
            ws_player::handle_observation_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Generation(req) => {
            ws_creator::handle_generation_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Ai(req) => {
            ws_creator::handle_ai_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Expression(req) => {
            ws_creator::handle_expression_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Challenge(req) => {
            ws_challenge::handle_challenge_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::NarrativeEvent(req) => {
            ws_narrative_event::handle_narrative_event_request(state, &request_id, &conn_info, req)
                .await
        }
        RequestPayload::EventChain(req) => {
            ws_event_chain::handle_event_chain_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Goal(req) => {
            ws_actantial::handle_goal_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Want(req) => {
            ws_actantial::handle_want_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Actantial(req) => {
            ws_actantial::handle_actantial_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Scene(req) => {
            ws_scene::handle_scene_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Act(req) => {
            ws_scene::handle_act_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Interaction(req) => {
            ws_scene::handle_interaction_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Skill(req) => {
            ws_skill::handle_skill_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Stat(req) => {
            ws_stat::handle_stat_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::CharacterSheet(req) => {
            ws_character_sheet::handle_character_sheet_request(state, &request_id, &conn_info, req)
                .await
        }
        RequestPayload::Content(req) => {
            ws_content::handle_content_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Unknown => Ok(ResponseResult::error(
            ErrorCode::BadRequest,
            "This request type is not yet implemented",
        )),
    };

    match dispatched {
        Ok(result) => Some(ServerMessage::Response { request_id, result }),
        Err(e) => Some(e),
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn error_response(code: &str, message: &str) -> ServerMessage {
    ServerMessage::Error {
        code: code.to_string(),
        message: message.to_string(),
    }
}

fn parse_staging_source(source: &str) -> StagingSource {
    source.parse().unwrap_or(StagingSource::Unknown)
}

/// Parse a string ID into a typed domain ID, returning an error response on failure.
fn parse_id<T, F>(id_str: &str, from_uuid: F, error_msg: &str) -> Result<T, ServerMessage>
where
    F: FnOnce(Uuid) -> T,
{
    Uuid::parse_str(id_str)
        .map(from_uuid)
        .map_err(|_| error_response("INVALID_ID", error_msg))
}

// =============================================================================
// WebSocket Integration Tests
// =============================================================================

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod ws_integration_tests;

#[cfg(test)]
pub mod e2e_client;

#[cfg(test)]
pub mod e2e_scenarios;

// Legacy inline suite considers module-private items in this file.
// Kept temporarily behind cfg(any()) to avoid a very large deletion patch.
#[cfg(any())]
mod ws_integration_tests_inline {
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
        ClockPort, ImageGenError, ImageGenPort, LlmError, LlmPort, QueueError, QueueItem,
        QueuePort, RandomPort,
    };
    use crate::infrastructure::ports::{
        MockActRepo, MockAssetRepo, MockChallengeRepo, MockCharacterRepo, MockFlagRepo,
        MockContentRepo, MockGoalRepo, MockInteractionRepo, MockItemRepo, MockLocationRepo,
        MockLocationStateRepo, MockLoreRepo, MockNarrativeRepo, MockObservationRepo,
        MockPlayerCharacterRepo, MockRegionStateRepo, MockSceneRepo, MockSettingsRepo,
        MockStagingRepo,
        MockWorldRepo,
    };

    struct TestAppRepos {
        world_repo: MockWorldRepo,
        character_repo: MockCharacterRepo,
        player_character_repo: MockPlayerCharacterRepo,
        location_repo: MockLocationRepo,
        scene_repo: MockSceneRepo,
        act_repo: MockActRepo,
        content_repo: MockContentRepo,
        interaction_repo: MockInteractionRepo,
        settings_repo: MockSettingsRepo,
        challenge_repo: MockChallengeRepo,
        narrative_repo: MockNarrativeRepo,
        staging_repo: MockStagingRepo,
        observation_repo: MockObservationRepo,
        item_repo: MockItemRepo,
        asset_repo: MockAssetRepo,
        flag_repo: MockFlagRepo,
        goal_repo: MockGoalRepo,
        lore_repo: MockLoreRepo,
        location_state_repo: MockLocationStateRepo,
        region_state_repo: MockRegionStateRepo,
    }

    impl TestAppRepos {
        fn new(world_repo: MockWorldRepo) -> Self {
            Self {
                world_repo,
                character_repo: MockCharacterRepo::new(),
                player_character_repo: MockPlayerCharacterRepo::new(),
                location_repo: MockLocationRepo::new(),
                scene_repo: MockSceneRepo::new(),
                act_repo: MockActRepo::new(),
                content_repo: MockContentRepo::new(),
                interaction_repo: MockInteractionRepo::new(),
                settings_repo: MockSettingsRepo::new(),
                challenge_repo: MockChallengeRepo::new(),
                narrative_repo: MockNarrativeRepo::new(),
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

    struct NoopQueue;

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

    struct NoopLlm;

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

    struct NoopImageGen;

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

    struct FixedClock {
        now: DateTime<Utc>,
    }

    impl ClockPort for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.now
        }
    }

    struct FixedRandom;

    impl RandomPort for FixedRandom {
        fn gen_range(&self, _min: i32, _max: i32) -> i32 {
            1
        }

        fn gen_uuid(&self) -> Uuid {
            Uuid::nil()
        }
    }

    #[derive(Default)]
    struct RecordingApprovalQueueState {
        approvals: StdHashMap<Uuid, ApprovalRequestData>,
        completed: Vec<Uuid>,
        failed: Vec<(Uuid, String)>,
    }

    #[derive(Clone, Default)]
    struct RecordingApprovalQueue {
        state: Arc<Mutex<RecordingApprovalQueueState>>,
    }

    impl RecordingApprovalQueue {
        fn insert_approval(&self, id: Uuid, data: ApprovalRequestData) {
            let mut guard = self.state.lock().unwrap();
            guard.approvals.insert(id, data);
        }

        fn completed_contains(&self, id: Uuid) -> bool {
            let guard = self.state.lock().unwrap();
            guard.completed.contains(&id)
        }

        fn failed_contains(&self, id: Uuid) -> bool {
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

    struct FixedLlm {
        content: String,
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

    fn build_test_app_with_ports(
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

        let ai =
            crate::use_cases::AiUseCases::new(Arc::new(crate::use_cases::ai::SuggestionOps::new(
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
                approve_suggestion,
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
        let time_uc =
            crate::use_cases::TimeUseCases::new(suggest_time, time_control, time_suggestions);

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
        };

        Arc::new(App {
            repositories: repositories_container,
            use_cases,
            queue,
            llm,
            content: Arc::new(crate::use_cases::content::ContentService::new(
                Default::default(),
            )),
        })
    }

    fn build_test_app(repos: TestAppRepos, now: DateTime<Utc>) -> Arc<App> {
        build_test_app_with_ports(repos, now, Arc::new(NoopQueue), Arc::new(NoopLlm))
    }

    async fn spawn_ws_server(state: Arc<WsState>) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let router = axum::Router::new().route("/ws", get(ws_handler).with_state(state));

        let handle = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        (addr, handle)
    }

    async fn ws_connect(
        addr: SocketAddr,
    ) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>
    {
        let url = format!("ws://{}/ws", addr);
        let (ws, _resp) = connect_async(url).await.unwrap();
        ws
    }

    async fn ws_send_client(
        ws: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        msg: &wrldbldr_protocol::ClientMessage,
    ) {
        let json = serde_json::to_string(msg).unwrap();
        ws.send(WsMessage::Text(json.into())).await.unwrap();
    }

    async fn ws_recv_server(
        ws: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) -> wrldbldr_protocol::ServerMessage {
        loop {
            let msg = ws.next().await.unwrap().unwrap();
            match msg {
                WsMessage::Text(text) => {
                    return serde_json::from_str::<wrldbldr_protocol::ServerMessage>(&text)
                        .unwrap();
                }
                WsMessage::Binary(bin) => {
                    let text = String::from_utf8(bin).unwrap();
                    return serde_json::from_str::<wrldbldr_protocol::ServerMessage>(&text)
                        .unwrap();
                }
                _ => {}
            }
        }
    }

    async fn ws_expect_message<F>(
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

    async fn ws_expect_no_message_matching<F>(
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

    #[tokio::test]
    async fn when_dm_approves_time_suggestion_then_time_advances_and_broadcasts() {
        let now = chrono::Utc::now();

        let world_id = WorldId::new();
        let world_name = WorldName::new("Test World").unwrap();
        let world = wrldbldr_domain::World::new(world_name, now)
            .with_description(Description::new("desc").unwrap())
            .with_id(world_id);

        // World repo mock: always returns the same world and accepts saves.
        let mut world_repo = MockWorldRepo::new();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .returning(move |_| Ok(Some(world_for_get.clone())));

        world_repo.expect_save().returning(|_world| Ok(()));

        let repos = TestAppRepos::new(world_repo);
        let app = build_test_app(repos, now);
        let connections = Arc::new(ConnectionManager::new());

        let ws_state = Arc::new(WsState {
            app,
            connections,
            pending_time_suggestions: TimeSuggestionStoreImpl::new(),
            pending_staging_requests: PendingStagingStoreImpl::new(),
            generation_read_state: GenerationStateStoreImpl::new(),
        });

        let (addr, server) = spawn_ws_server(ws_state.clone()).await;

        let mut dm_ws = ws_connect(addr).await;
        let mut spectator_ws = ws_connect(addr).await;

        // DM joins.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Dm,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;

        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // Spectator joins (so we can assert broadcast reaches others too).
        ws_send_client(
            &mut spectator_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Spectator,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;

        let _ = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // DM will receive a UserJoined broadcast for the spectator.
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::UserJoined { .. })
        })
        .await;

        // Seed a pending time suggestion.
        let suggestion_id = Uuid::new_v4();
        let pc_id = PlayerCharacterId::new();

        let current_time = world.game_time.clone();
        let mut resulting_time = current_time.clone();
        resulting_time.advance_minutes(15);

        let suggestion = crate::use_cases::time::TimeSuggestion {
            id: suggestion_id,
            world_id,
            pc_id,
            pc_name: "PC".to_string(),
            action_type: "travel_region".to_string(),
            action_description: "to somewhere".to_string(),
            suggested_minutes: 15,
            current_time: current_time.clone(),
            resulting_time: resulting_time.clone(),
            period_change: None,
        };

        ws_state
            .pending_time_suggestions
            .insert(suggestion_id, suggestion)
            .await;

        // DM approves the suggestion (no direct response; only broadcast).
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::RespondToTimeSuggestion {
                suggestion_id: suggestion_id.to_string(),
                decision: wrldbldr_protocol::types::TimeSuggestionDecision::Approve,
            },
        )
        .await;

        let dm_broadcast = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::GameTimeAdvanced { .. })
        })
        .await;

        let spectator_broadcast =
            ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
                matches!(m, ServerMessage::GameTimeAdvanced { .. })
            })
            .await;

        // Basic sanity: both received the same broadcast variant.
        assert!(matches!(
            dm_broadcast,
            ServerMessage::GameTimeAdvanced { .. }
        ));
        assert!(matches!(
            spectator_broadcast,
            ServerMessage::GameTimeAdvanced { .. }
        ));

        server.abort();
    }

    #[tokio::test]
    async fn when_player_enters_unstaged_region_then_dm_can_approve_and_player_receives_staging_ready(
    ) {
        use wrldbldr_domain::value_objects::CampbellArchetype;
        use wrldbldr_domain::TimeMode;

        let now = chrono::Utc::now();

        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let visible_npc_id = CharacterId::new();
        let hidden_npc_id = CharacterId::new();

        // World (manual time, so movement doesn't generate time suggestions).
        let world_name = WorldName::new("Test World").unwrap();
        let mut world = wrldbldr_domain::World::new(world_name, now)
            .with_description(Description::new("desc").unwrap())
            .with_id(world_id);
        world.set_time_mode(TimeMode::Manual, now);

        // Domain fixtures.
        let mut location = wrldbldr_domain::Location::new(
            world_id,
            "Test Location",
            wrldbldr_domain::LocationType::Exterior,
        )
        .expect("valid location");
        location.id = location_id;

        let mut region = wrldbldr_domain::Region::new(location_id, "Unstaged Region");
        region.id = region_id;

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("player-1", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = None; // initial spawn; skip connection validation

        let mut visible_npc =
            wrldbldr_domain::Character::new(world_id, "Visible NPC", CampbellArchetype::Hero)
                .expect("valid character");
        visible_npc.id = visible_npc_id;
        let mut hidden_npc =
            wrldbldr_domain::Character::new(world_id, "Hidden NPC", CampbellArchetype::Herald)
                .expect("valid character");
        hidden_npc.id = hidden_npc_id;

        // World repo: serve the world for both time + visual state resolution.
        let mut world_repo = MockWorldRepo::new();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .returning(move |_| Ok(Some(world_for_get.clone())));
        world_repo.expect_save().returning(|_world| Ok(()));

        let mut repos = TestAppRepos::new(world_repo);

        // Movement needs PC + region + location.
        let pc_for_get = pc.clone();
        repos
            .player_character_repo
            .expect_get()
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        repos
            .player_character_repo
            .expect_get_inventory()
            .returning(|_| Ok(vec![]));

        repos
            .player_character_repo
            .expect_update_position()
            .returning(|_, _, _| Ok(()));

        let region_for_get = region.clone();
        repos
            .location_repo
            .expect_get_region()
            .returning(move |_| Ok(Some(region_for_get.clone())));

        let location_for_get = location.clone();
        repos
            .location_repo
            .expect_get_location()
            .returning(move |_| Ok(Some(location_for_get.clone())));

        repos
            .location_repo
            .expect_get_connections()
            .returning(|_| Ok(vec![]));

        repos
            .location_repo
            .expect_get_location_exits()
            .returning(|_| Ok(vec![]));

        // Unstaged region -> pending.
        repos
            .staging_repo
            .expect_get_active_staging()
            .returning(|_, _| Ok(None));

        repos
            .staging_repo
            .expect_get_staged_npcs()
            .returning(|_| Ok(vec![]));

        // Narrative triggers: keep empty so we don't need deeper narrative deps.
        repos
            .narrative_repo
            .expect_get_triggers_for_region()
            .returning(|_, _| Ok(vec![]));

        // Scene resolution: no scenes.
        repos
            .scene_repo
            .expect_get_completed_scenes()
            .returning(|_| Ok(vec![]));
        repos
            .scene_repo
            .expect_list_for_region()
            .returning(|_| Ok(vec![]));

        // Observations + flags: empty.
        repos
            .observation_repo
            .expect_get_observations()
            .returning(|_| Ok(vec![]));

        repos
            .observation_repo
            .expect_has_observed()
            .returning(|_, _| Ok(false));

        repos
            .observation_repo
            .expect_save_observation()
            .returning(|_| Ok(()));

        repos
            .observation_repo
            .expect_has_observed()
            .returning(|_, _| Ok(false));
        repos
            .observation_repo
            .expect_save_observation()
            .returning(|_| Ok(()));
        repos
            .flag_repo
            .expect_get_world_flags()
            .returning(|_| Box::pin(async { Ok(vec![]) }));
        repos
            .flag_repo
            .expect_get_pc_flags()
            .returning(|_| Box::pin(async { Ok(vec![]) }));

        // Visual state resolution: no states.
        repos
            .location_state_repo
            .expect_list_for_location()
            .returning(|_| Ok(vec![]));
        repos
            .region_state_repo
            .expect_list_for_region()
            .returning(|_| Ok(vec![]));
        repos
            .location_state_repo
            .expect_get_active()
            .returning(|_| Ok(None));
        repos
            .region_state_repo
            .expect_get_active()
            .returning(|_| Ok(None));

        // Items in region: empty.
        repos
            .item_repo
            .expect_list_in_region()
            .returning(|_| Ok(vec![]));

        // Staging approval persists full per-NPC info (including hidden flags).
        let region_id_for_staging = region_id;
        let location_id_for_staging = location_id;
        let world_id_for_staging = world_id;
        let visible_npc_id_for_staging = visible_npc_id;
        let hidden_npc_id_for_staging = hidden_npc_id;
        repos
            .staging_repo
            .expect_save_pending_staging()
            .withf(move |s| {
                s.region_id == region_id_for_staging
                    && s.location_id == location_id_for_staging
                    && s.world_id == world_id_for_staging
                    && s.ttl_hours == 24
                    && s.npcs.iter().any(|n| {
                        n.character_id == visible_npc_id_for_staging
                            && n.is_present
                            && !n.is_hidden_from_players
                    })
                    && s.npcs.iter().any(|n| {
                        n.character_id == hidden_npc_id_for_staging
                            && n.is_present
                            && n.is_hidden_from_players
                    })
            })
            .returning(|_| Ok(()));

        repos
            .staging_repo
            .expect_activate_staging()
            .withf(move |_staging_id, r| *r == region_id)
            .returning(|_, _| Ok(()));

        // Character details for StagingReady payload.
        let visible_npc_for_get = visible_npc.clone();
        let hidden_npc_for_get = hidden_npc.clone();
        repos.character_repo.expect_get().returning(move |id| {
            if id == visible_npc_for_get.id {
                Ok(Some(visible_npc_for_get.clone()))
            } else if id == hidden_npc_for_get.id {
                Ok(Some(hidden_npc_for_get.clone()))
            } else {
                Ok(None)
            }
        });

        repos
            .character_repo
            .expect_get_npcs_for_region()
            .returning(|_| Ok(vec![]));

        let app = build_test_app(repos, now);
        let connections = Arc::new(ConnectionManager::new());

        let ws_state = Arc::new(WsState {
            app,
            connections,
            pending_time_suggestions: TimeSuggestionStoreImpl::new(),
            pending_staging_requests: PendingStagingStoreImpl::new(),
            generation_read_state: GenerationStateStoreImpl::new(),
        });

        let (addr, server) = spawn_ws_server(ws_state.clone()).await;

        let mut dm_ws = ws_connect(addr).await;
        let mut player_ws = ws_connect(addr).await;

        // DM joins.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Dm,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // Player joins with PC.
        ws_send_client(
            &mut player_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Player,
                pc_id: Some(*pc_id.as_uuid()),
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // DM receives UserJoined broadcast.
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::UserJoined { .. })
        })
        .await;

        // Player moves into region with no active staging.
        ws_send_client(
            &mut player_ws,
            &ClientMessage::MoveToRegion {
                pc_id: pc_id.to_string(),
                region_id: region_id.to_string(),
            },
        )
        .await;

        let _pending = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::StagingPending { .. })
        })
        .await;

        // DM gets staging approval request.
        let approval_required = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::StagingApprovalRequired { .. })
        })
        .await;

        let approval_request_id = match approval_required {
            ServerMessage::StagingApprovalRequired { request_id, .. } => request_id,
            other => panic!("expected StagingApprovalRequired, got: {:?}", other),
        };

        // DM approves: one visible NPC + one hidden NPC.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::StagingApprovalResponse {
                request_id: approval_request_id,
                approved_npcs: vec![
                    wrldbldr_protocol::ApprovedNpcInfo {
                        character_id: visible_npc_id.to_string(),
                        is_present: true,
                        reasoning: None,
                        is_hidden_from_players: false,
                        mood: None,
                    },
                    wrldbldr_protocol::ApprovedNpcInfo {
                        character_id: hidden_npc_id.to_string(),
                        is_present: true,
                        reasoning: None,
                        is_hidden_from_players: true,
                        mood: None,
                    },
                ],
                ttl_hours: 24,
                source: "test".to_string(),
                location_state_id: None,
                region_state_id: None,
            },
        )
        .await;

        // Player receives StagingReady broadcast, containing only visible NPC.
        let staging_ready = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::StagingReady { .. })
        })
        .await;

        match staging_ready {
            ServerMessage::StagingReady {
                region_id: got_region_id,
                npcs_present,
                ..
            } => {
                assert_eq!(got_region_id, region_id.to_string());
                assert!(npcs_present
                    .iter()
                    .any(|n| n.character_id == visible_npc_id.to_string()));
                assert!(!npcs_present
                    .iter()
                    .any(|n| n.character_id == hidden_npc_id.to_string()));
            }
            other => panic!("expected StagingReady, got: {:?}", other),
        }

        server.abort();
    }

    #[tokio::test]
    async fn when_dm_accepts_approval_suggestion_then_marks_complete_and_broadcasts_dialogue() {
        let now = chrono::Utc::now();

        let world_id = WorldId::new();
        let world_name = WorldName::new("Test World").unwrap();
        let world = wrldbldr_domain::World::new(world_name, now)
            .with_description(Description::new("desc").unwrap())
            .with_id(world_id);

        let mut world_repo = MockWorldRepo::new();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .returning(move |_| Ok(Some(world_for_get.clone())));
        world_repo.expect_save().returning(|_world| Ok(()));

        let repos = TestAppRepos::new(world_repo);

        let queue = RecordingApprovalQueue::default();
        let queue_port: Arc<dyn QueuePort> = Arc::new(queue.clone());

        let app = build_test_app_with_ports(repos, now, queue_port, Arc::new(NoopLlm));
        let connections = Arc::new(ConnectionManager::new());

        let ws_state = Arc::new(WsState {
            app,
            connections,
            pending_time_suggestions: TimeSuggestionStoreImpl::new(),
            pending_staging_requests: PendingStagingStoreImpl::new(),
            generation_read_state: GenerationStateStoreImpl::new(),
        });

        let (addr, server) = spawn_ws_server(ws_state.clone()).await;

        let mut dm_ws = ws_connect(addr).await;
        let mut spectator_ws = ws_connect(addr).await;

        // DM joins.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Dm,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // Spectator joins (receives world broadcasts).
        ws_send_client(
            &mut spectator_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Spectator,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // DM receives UserJoined broadcast.
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::UserJoined { .. })
        })
        .await;

        // Seed an approval request.
        let approval_id = Uuid::new_v4();
        let npc_id = CharacterId::new();
        let proposed_dialogue = "Hello there".to_string();

        queue.insert_approval(
            approval_id,
            ApprovalRequestData {
                world_id,
                source_action_id: Uuid::new_v4(),
                decision_type: ApprovalDecisionType::NpcResponse,
                urgency: ApprovalUrgency::Normal,
                pc_id: None,
                npc_id: Some(npc_id),
                npc_name: "NPC".to_string(),
                proposed_dialogue: proposed_dialogue.clone(),
                internal_reasoning: "".to_string(),
                proposed_tools: vec![],
                retry_count: 0,
                challenge_suggestion: None,
                narrative_event_suggestion: None,
                challenge_outcome: None,
                player_dialogue: None,
                scene_id: None,
                location_id: None,
                game_time: None,
                topics: vec![],
                conversation_id: None,
            },
        );

        // DM accepts.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::ApprovalDecision {
                request_id: approval_id.to_string(),
                decision: wrldbldr_protocol::ApprovalDecision::Accept,
            },
        )
        .await;

        // DM sees ResponseApproved.
        let dm_msg = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::ResponseApproved { .. })
        })
        .await;
        match dm_msg {
            ServerMessage::ResponseApproved {
                npc_dialogue,
                executed_tools,
            } => {
                assert_eq!(npc_dialogue, proposed_dialogue);
                assert!(executed_tools.is_empty());
            }
            other => panic!("expected ResponseApproved, got: {:?}", other),
        }

        // World sees DialogueResponse.
        let world_msg = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::DialogueResponse { .. })
        })
        .await;
        match world_msg {
            ServerMessage::DialogueResponse {
                speaker_id, text, ..
            } => {
                assert_eq!(speaker_id, npc_id.to_string());
                assert_eq!(text, proposed_dialogue);
            }
            other => panic!("expected DialogueResponse, got: {:?}", other),
        }

        assert!(queue.completed_contains(approval_id));
        assert!(!queue.failed_contains(approval_id));

        server.abort();
    }

    #[tokio::test]
    async fn when_dm_rejects_approval_suggestion_then_marks_failed_and_does_not_broadcast_dialogue()
    {
        let now = chrono::Utc::now();

        let world_id = WorldId::new();
        let world_name = WorldName::new("Test World").unwrap();
        let world = wrldbldr_domain::World::new(world_name, now)
            .with_description(Description::new("desc").unwrap())
            .with_id(world_id);

        let mut world_repo = MockWorldRepo::new();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .returning(move |_| Ok(Some(world_for_get.clone())));
        world_repo.expect_save().returning(|_world| Ok(()));

        let repos = TestAppRepos::new(world_repo);

        let queue = RecordingApprovalQueue::default();
        let queue_port: Arc<dyn QueuePort> = Arc::new(queue.clone());
        let app = build_test_app_with_ports(repos, now, queue_port, Arc::new(NoopLlm));
        let connections = Arc::new(ConnectionManager::new());

        let ws_state = Arc::new(WsState {
            app,
            connections,
            pending_time_suggestions: TimeSuggestionStoreImpl::new(),
            pending_staging_requests: PendingStagingStoreImpl::new(),
            generation_read_state: GenerationStateStoreImpl::new(),
        });

        let (addr, server) = spawn_ws_server(ws_state.clone()).await;

        let mut dm_ws = ws_connect(addr).await;
        let mut spectator_ws = ws_connect(addr).await;

        // DM joins.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Dm,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // Spectator joins.
        ws_send_client(
            &mut spectator_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Spectator,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // DM receives UserJoined broadcast.
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::UserJoined { .. })
        })
        .await;

        // Seed an approval request.
        let approval_id = Uuid::new_v4();
        let npc_id = CharacterId::new();
        queue.insert_approval(
            approval_id,
            ApprovalRequestData {
                world_id,
                source_action_id: Uuid::new_v4(),
                decision_type: ApprovalDecisionType::NpcResponse,
                urgency: ApprovalUrgency::Normal,
                pc_id: None,
                npc_id: Some(npc_id),
                npc_name: "NPC".to_string(),
                proposed_dialogue: "Hello".to_string(),
                internal_reasoning: "".to_string(),
                proposed_tools: vec![],
                retry_count: 0,
                challenge_suggestion: None,
                narrative_event_suggestion: None,
                challenge_outcome: None,
                player_dialogue: None,
                scene_id: None,
                location_id: None,
                game_time: None,
                topics: vec![],
                conversation_id: None,
            },
        );

        // DM rejects.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::ApprovalDecision {
                request_id: approval_id.to_string(),
                decision: wrldbldr_protocol::ApprovalDecision::Reject {
                    feedback: "no".to_string(),
                },
            },
        )
        .await;

        // Ensure no DialogueResponse is broadcast.
        ws_expect_no_message_matching(&mut spectator_ws, Duration::from_millis(250), |m| {
            matches!(m, ServerMessage::DialogueResponse { .. })
        })
        .await;

        assert!(!queue.completed_contains(approval_id));
        assert!(queue.failed_contains(approval_id));

        server.abort();
    }

    #[tokio::test]
    async fn when_dm_modifies_approval_suggestion_then_marks_complete_and_broadcasts_modified_dialogue(
    ) {
        let now = chrono::Utc::now();

        let world_id = WorldId::new();
        let world_name = WorldName::new("Test World").unwrap();
        let world = wrldbldr_domain::World::new(world_name, now)
            .with_description(Description::new("desc").unwrap())
            .with_id(world_id);

        let mut world_repo = MockWorldRepo::new();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .returning(move |_| Ok(Some(world_for_get.clone())));
        world_repo.expect_save().returning(|_world| Ok(()));

        let repos = TestAppRepos::new(world_repo);

        let queue = RecordingApprovalQueue::default();
        let queue_port: Arc<dyn QueuePort> = Arc::new(queue.clone());
        let app = build_test_app_with_ports(repos, now, queue_port, Arc::new(NoopLlm));
        let connections = Arc::new(ConnectionManager::new());

        let ws_state = Arc::new(WsState {
            app,
            connections,
            pending_time_suggestions: TimeSuggestionStoreImpl::new(),
            pending_staging_requests: PendingStagingStoreImpl::new(),
            generation_read_state: GenerationStateStoreImpl::new(),
        });

        let (addr, server) = spawn_ws_server(ws_state.clone()).await;

        let mut dm_ws = ws_connect(addr).await;
        let mut spectator_ws = ws_connect(addr).await;

        // DM joins.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Dm,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // Spectator joins.
        ws_send_client(
            &mut spectator_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Spectator,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // DM receives UserJoined broadcast.
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::UserJoined { .. })
        })
        .await;

        // Seed an approval request.
        let approval_id = Uuid::new_v4();
        let npc_id = CharacterId::new();
        queue.insert_approval(
            approval_id,
            ApprovalRequestData {
                world_id,
                source_action_id: Uuid::new_v4(),
                decision_type: ApprovalDecisionType::NpcResponse,
                urgency: ApprovalUrgency::Normal,
                pc_id: None,
                npc_id: Some(npc_id),
                npc_name: "NPC".to_string(),
                proposed_dialogue: "Original".to_string(),
                internal_reasoning: "".to_string(),
                proposed_tools: vec![],
                retry_count: 0,
                challenge_suggestion: None,
                narrative_event_suggestion: None,
                challenge_outcome: None,
                player_dialogue: None,
                scene_id: None,
                location_id: None,
                game_time: None,
                topics: vec![],
                conversation_id: None,
            },
        );

        let modified_dialogue = "Modified dialogue".to_string();
        let approved_tools = vec!["tool_a".to_string(), "tool_b".to_string()];

        // DM modifies.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::ApprovalDecision {
                request_id: approval_id.to_string(),
                decision: wrldbldr_protocol::ApprovalDecision::AcceptWithModification {
                    modified_dialogue: modified_dialogue.clone(),
                    approved_tools: approved_tools.clone(),
                    rejected_tools: vec![],
                    item_recipients: std::collections::HashMap::new(),
                },
            },
        )
        .await;

        let dm_msg = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::ResponseApproved { .. })
        })
        .await;
        match dm_msg {
            ServerMessage::ResponseApproved {
                npc_dialogue,
                executed_tools,
            } => {
                assert_eq!(npc_dialogue, modified_dialogue);
                assert_eq!(executed_tools, approved_tools);
            }
            other => panic!("expected ResponseApproved, got: {:?}", other),
        }

        let world_msg = ws_expect_message(&mut spectator_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::DialogueResponse { .. })
        })
        .await;
        match world_msg {
            ServerMessage::DialogueResponse { text, .. } => {
                assert_eq!(text, modified_dialogue);
            }
            other => panic!("expected DialogueResponse, got: {:?}", other),
        }

        assert!(queue.completed_contains(approval_id));
        assert!(!queue.failed_contains(approval_id));

        server.abort();
    }

    #[tokio::test]
    async fn when_dm_prestages_region_then_player_entering_gets_scene_changed_without_staging_pending(
    ) {
        use wrldbldr_domain::value_objects::CampbellArchetype;
        use wrldbldr_domain::TimeMode;

        let now = chrono::Utc::now();

        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();

        let world_name = WorldName::new("Test World").unwrap();
        let mut world = wrldbldr_domain::World::new(world_name, now)
            .with_description(Description::new("desc").unwrap())
            .with_id(world_id);
        world.set_time_mode(TimeMode::Manual, now);

        let mut location = wrldbldr_domain::Location::new(
            world_id,
            "Test Location",
            wrldbldr_domain::LocationType::Exterior,
        )
        .expect("valid location");
        location.id = location_id;

        let mut region = wrldbldr_domain::Region::new(location_id, "Region");
        region.id = region_id;

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("player-1", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = None;

        let mut npc = wrldbldr_domain::Character::new(world_id, "NPC", CampbellArchetype::Hero)
            .expect("valid character");
        npc.id = npc_id;

        let mut world_repo = MockWorldRepo::new();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .returning(move |_| Ok(Some(world_for_get.clone())));
        world_repo.expect_save().returning(|_world| Ok(()));

        let mut repos = TestAppRepos::new(world_repo);

        // Join+movement needs PC+region+location.
        let pc_for_get = pc.clone();
        repos
            .player_character_repo
            .expect_get()
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        repos
            .player_character_repo
            .expect_get_inventory()
            .returning(|_| Ok(vec![]));

        repos
            .player_character_repo
            .expect_update_position()
            .returning(|_, _, _| Ok(()));

        let region_for_get = region.clone();
        repos
            .location_repo
            .expect_get_region()
            .returning(move |_| Ok(Some(region_for_get.clone())));

        let location_for_get = location.clone();
        repos
            .location_repo
            .expect_get_location()
            .returning(move |_| Ok(Some(location_for_get.clone())));

        repos
            .location_repo
            .expect_get_connections()
            .returning(|_| Ok(vec![]));

        repos
            .location_repo
            .expect_get_location_exits()
            .returning(|_| Ok(vec![]));

        repos
            .item_repo
            .expect_list_in_region()
            .returning(|_| Ok(vec![]));

        // Narrative triggers/scene/flags/observations: empty.
        repos
            .narrative_repo
            .expect_get_triggers_for_region()
            .returning(|_, _| Ok(vec![]));
        repos
            .scene_repo
            .expect_get_completed_scenes()
            .returning(|_| Ok(vec![]));
        repos
            .scene_repo
            .expect_list_for_region()
            .returning(|_| Ok(vec![]));
        repos
            .observation_repo
            .expect_get_observations()
            .returning(|_| Ok(vec![]));
        repos
            .observation_repo
            .expect_has_observed()
            .returning(|_, _| Ok(false));
        repos
            .observation_repo
            .expect_save_observation()
            .returning(|_| Ok(()));
        repos
            .flag_repo
            .expect_get_world_flags()
            .returning(|_| Box::pin(async { Ok(vec![]) }));
        repos
            .flag_repo
            .expect_get_pc_flags()
            .returning(|_| Box::pin(async { Ok(vec![]) }));

        // Visual state resolution: no states.
        repos
            .location_state_repo
            .expect_list_for_location()
            .returning(|_| Ok(vec![]));
        repos
            .region_state_repo
            .expect_list_for_region()
            .returning(|_| Ok(vec![]));
        repos
            .location_state_repo
            .expect_get_active()
            .returning(|_| Ok(None));
        repos
            .region_state_repo
            .expect_get_active()
            .returning(|_| Ok(None));

        // Character details used by PreStageRegion.
        let npc_for_get = npc.clone();
        repos.character_repo.expect_get().returning(move |id| {
            if id == npc_for_get.id {
                Ok(Some(npc_for_get.clone()))
            } else {
                Ok(None)
            }
        });

        // Stage activation should influence subsequent get_active_staging.
        #[derive(Default)]
        struct SharedStaging {
            pending: Option<wrldbldr_domain::Staging>,
            activated: bool,
        }

        let shared = Arc::new(Mutex::new(SharedStaging::default()));

        let shared_for_save = shared.clone();
        repos
            .staging_repo
            .expect_save_pending_staging()
            .returning(move |s| {
                let mut guard = shared_for_save.lock().unwrap();
                guard.pending = Some(s.clone());
                Ok(())
            });

        let shared_for_activate = shared.clone();
        repos
            .staging_repo
            .expect_activate_staging()
            .withf(move |_id, r| *r == region_id)
            .returning(move |_id, _region| {
                let mut guard = shared_for_activate.lock().unwrap();
                guard.activated = true;
                Ok(())
            });

        let shared_for_get_active = shared.clone();
        repos
            .staging_repo
            .expect_get_active_staging()
            .returning(move |rid, _now| {
                let guard = shared_for_get_active.lock().unwrap();
                if guard.activated {
                    Ok(guard.pending.clone().filter(|s| s.region_id == rid))
                } else {
                    Ok(None)
                }
            });

        repos
            .staging_repo
            .expect_get_staged_npcs()
            .returning(|_| Ok(vec![]));

        repos
            .character_repo
            .expect_get_npcs_for_region()
            .returning(|_| Ok(vec![]));

        let app = build_test_app(repos, now);
        let connections = Arc::new(ConnectionManager::new());

        let ws_state = Arc::new(WsState {
            app,
            connections,
            pending_time_suggestions: TimeSuggestionStoreImpl::new(),
            pending_staging_requests: PendingStagingStoreImpl::new(),
            generation_read_state: GenerationStateStoreImpl::new(),
        });

        let (addr, server) = spawn_ws_server(ws_state.clone()).await;
        let mut dm_ws = ws_connect(addr).await;
        let mut player_ws = ws_connect(addr).await;

        // DM joins.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Dm,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // Player joins with PC.
        ws_send_client(
            &mut player_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Player,
                pc_id: Some(*pc_id.as_uuid()),
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // DM receives UserJoined broadcast.
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::UserJoined { .. })
        })
        .await;

        // DM pre-stages the region.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::PreStageRegion {
                region_id: region_id.to_string(),
                npcs: vec![wrldbldr_protocol::ApprovedNpcInfo {
                    character_id: npc_id.to_string(),
                    is_present: true,
                    reasoning: Some("pre-staged".to_string()),
                    is_hidden_from_players: false,
                    mood: None,
                }],
                ttl_hours: 24,
                location_state_id: None,
                region_state_id: None,
            },
        )
        .await;

        // Player moves into region and should immediately receive SceneChanged (not StagingPending).
        ws_send_client(
            &mut player_ws,
            &ClientMessage::MoveToRegion {
                pc_id: pc_id.to_string(),
                region_id: region_id.to_string(),
            },
        )
        .await;

        let scene_changed = ws_expect_message(&mut player_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::SceneChanged { .. })
        })
        .await;
        match scene_changed {
            ServerMessage::SceneChanged { npcs_present, .. } => {
                assert!(npcs_present
                    .iter()
                    .any(|n| n.character_id == npc_id.to_string()));
            }
            other => panic!("expected SceneChanged, got: {:?}", other),
        }

        // DM should not receive a staging approval request as a result of the move.
        ws_expect_no_message_matching(&mut dm_ws, Duration::from_millis(250), |m| {
            matches!(m, ServerMessage::StagingApprovalRequired { .. })
        })
        .await;

        server.abort();
    }

    #[tokio::test]
    async fn when_dm_requests_staging_regenerate_then_returns_llm_suggestions_and_does_not_mutate_staging(
    ) {
        use crate::infrastructure::ports::{NpcRegionRelationType, NpcWithRegionInfo};

        let now = chrono::Utc::now();

        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let npc_id = CharacterId::new();

        let world_name = WorldName::new("Test World").unwrap();
        let world = wrldbldr_domain::World::new(world_name, now)
            .with_description(Description::new("desc").unwrap())
            .with_id(world_id);

        let mut location = wrldbldr_domain::Location::new(
            world_id,
            "Test Location",
            wrldbldr_domain::LocationType::Exterior,
        )
        .expect("valid location");
        location.id = location_id;

        let mut region = wrldbldr_domain::Region::new(location_id, "Test Region");
        region.id = region_id;

        let mut world_repo = MockWorldRepo::new();
        let world_for_get = world.clone();
        world_repo
            .expect_get()
            .returning(move |_| Ok(Some(world_for_get.clone())));
        world_repo.expect_save().returning(|_world| Ok(()));

        let mut repos = TestAppRepos::new(world_repo);

        let region_for_get = region.clone();
        repos
            .location_repo
            .expect_get_region()
            .returning(move |_| Ok(Some(region_for_get.clone())));

        let location_for_get = location.clone();
        repos
            .location_repo
            .expect_get_location()
            .returning(move |_| Ok(Some(location_for_get.clone())));

        // Candidates for LLM suggestions.
        repos
            .character_repo
            .expect_get_npcs_for_region()
            .returning(move |_| {
                Ok(vec![NpcWithRegionInfo {
                    character_id: npc_id,
                    name: "Alice".to_string(),
                    sprite_asset: None,
                    portrait_asset: None,
                    relationship_type: NpcRegionRelationType::Frequents,
                    shift: None,
                    frequency: Some("often".to_string()),
                    time_of_day: None,
                    reason: None,
                    default_mood: wrldbldr_domain::MoodState::default(),
                }])
            });

        // Regenerate should not touch staging persistence.
        repos.staging_repo.expect_save_pending_staging().times(0);
        repos.staging_repo.expect_activate_staging().times(0);

        let llm = Arc::new(FixedLlm {
            content: r#"[{"name":"Alice","reason":"She is here"}]"#.to_string(),
        });

        let app = build_test_app_with_ports(repos, now, Arc::new(NoopQueue), llm);
        let connections = Arc::new(ConnectionManager::new());

        let ws_state = Arc::new(WsState {
            app,
            connections,
            pending_time_suggestions: TimeSuggestionStoreImpl::new(),
            pending_staging_requests: PendingStagingStoreImpl::new(),
            generation_read_state: GenerationStateStoreImpl::new(),
        });

        // Seed a pending staging request correlation.
        let request_id = "req-123".to_string();
        ws_state
            .pending_staging_requests
            .insert(
                request_id.clone(),
                PendingStagingRequest {
                    region_id,
                    location_id,
                    world_id,
                    created_at: now,
                },
            )
            .await;

        let (addr, server) = spawn_ws_server(ws_state.clone()).await;
        let mut dm_ws = ws_connect(addr).await;

        // DM joins.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::JoinWorld {
                world_id: *world_id.as_uuid(),
                role: ProtoWorldRole::Dm,
                pc_id: None,
                spectate_pc_id: None,
            },
        )
        .await;
        let _ = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::WorldJoined { .. })
        })
        .await;

        // DM requests regeneration.
        ws_send_client(
            &mut dm_ws,
            &ClientMessage::StagingRegenerateRequest {
                request_id: request_id.clone(),
                guidance: "more drama".to_string(),
            },
        )
        .await;

        let regenerated = ws_expect_message(&mut dm_ws, Duration::from_secs(2), |m| {
            matches!(m, ServerMessage::StagingRegenerated { .. })
        })
        .await;

        match regenerated {
            ServerMessage::StagingRegenerated {
                request_id: got_id,
                llm_based_npcs,
            } => {
                assert_eq!(got_id, request_id);
                assert_eq!(llm_based_npcs.len(), 1);
                assert_eq!(llm_based_npcs[0].character_id, npc_id.to_string());
                assert!(llm_based_npcs[0].reasoning.contains("[LLM]"));
            }
            other => panic!("expected StagingRegenerated, got: {:?}", other),
        }

        server.abort();
    }
}

/// Parse a player character ID from a string.
fn parse_pc_id(id_str: &str) -> Result<PlayerCharacterId, ServerMessage> {
    parse_id(id_str, PlayerCharacterId::from_uuid, "Invalid PC ID format")
}

/// Parse a character ID from a string.
fn parse_character_id(id_str: &str) -> Result<CharacterId, ServerMessage> {
    parse_id(
        id_str,
        CharacterId::from_uuid,
        "Invalid character ID format",
    )
}

/// Parse a region ID from a string.
fn parse_region_id(id_str: &str) -> Result<RegionId, ServerMessage> {
    parse_id(id_str, RegionId::from_uuid, "Invalid region ID format")
}

/// Parse a world ID from a string.
fn parse_world_id(id_str: &str) -> Result<WorldId, ServerMessage> {
    parse_id(id_str, WorldId::from_uuid, "Invalid world ID format")
}

/// Parse a location ID from a string.
fn parse_location_id(id_str: &str) -> Result<LocationId, ServerMessage> {
    parse_id(id_str, LocationId::from_uuid, "Invalid location ID format")
}

/// Parse an item ID from a string.
fn parse_item_id(id_str: &str) -> Result<ItemId, ServerMessage> {
    parse_id(id_str, ItemId::from_uuid, "Invalid item ID format")
}

/// Parse a challenge ID from a string.
fn parse_challenge_id(id_str: &str) -> Result<ChallengeId, ServerMessage> {
    parse_id(
        id_str,
        ChallengeId::from_uuid,
        "Invalid challenge ID format",
    )
}

/// Parse a narrative event ID from a string.
#[allow(dead_code)]
fn parse_narrative_event_id(id_str: &str) -> Result<NarrativeEventId, ServerMessage> {
    parse_id(
        id_str,
        NarrativeEventId::from_uuid,
        "Invalid narrative event ID format",
    )
}

/// Parse an event chain ID from a string.
#[allow(dead_code)]
fn parse_event_chain_id(id_str: &str) -> Result<EventChainId, ServerMessage> {
    parse_id(
        id_str,
        EventChainId::from_uuid,
        "Invalid event chain ID format",
    )
}

/// Verify that the connection has DM authorization, returning an error response if not.
fn require_dm(conn_info: &super::connections::ConnectionInfo) -> Result<(), ServerMessage> {
    if conn_info.is_dm() {
        Ok(())
    } else {
        Err(error_response(
            "UNAUTHORIZED",
            "Only DMs can perform this action",
        ))
    }
}

/// Verify that the connection has DM authorization for Request/Response pattern.
fn require_dm_for_request(
    conn_info: &super::connections::ConnectionInfo,
    request_id: &str,
) -> Result<(), ServerMessage> {
    if conn_info.is_dm() {
        Ok(())
    } else {
        tracing::warn!(
            connection_id = %conn_info.connection_id,
            user_id = %conn_info.user_id,
            role = ?conn_info.role,
            world_id = ?conn_info.world_id,
            request_id = %request_id,
            "DM authorization failed - connection does not have DM role"
        );
        Err(ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(
                ErrorCode::Unauthorized,
                "Only DMs can perform this action",
            ),
        })
    }
}

/// Parse a UUID from a string for Request/Response pattern.
fn parse_uuid_for_request(
    id_str: &str,
    request_id: &str,
    error_msg: &str,
) -> Result<Uuid, ServerMessage> {
    Uuid::parse_str(id_str).map_err(|e| {
        tracing::debug!(input = %id_str, error = %e, "UUID parsing failed");
        ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(ErrorCode::BadRequest, error_msg),
        }
    })
}

/// Generic typed ID parser for Request/Response pattern.
fn parse_id_for_request<T, F>(
    id_str: &str,
    request_id: &str,
    from_uuid: F,
    error_msg: &str,
) -> Result<T, ServerMessage>
where
    F: FnOnce(Uuid) -> T,
{
    parse_uuid_for_request(id_str, request_id, error_msg).map(from_uuid)
}

/// Parse a world ID for Request/Response pattern.
fn parse_world_id_for_request(id_str: &str, request_id: &str) -> Result<WorldId, ServerMessage> {
    parse_id_for_request(id_str, request_id, WorldId::from_uuid, "Invalid world ID")
}

/// Parse a character ID for Request/Response pattern.
fn parse_character_id_for_request(
    id_str: &str,
    request_id: &str,
) -> Result<CharacterId, ServerMessage> {
    parse_id_for_request(
        id_str,
        request_id,
        CharacterId::from_uuid,
        "Invalid character ID",
    )
}

/// Parse a region ID for Request/Response pattern.
fn parse_region_id_for_request(id_str: &str, request_id: &str) -> Result<RegionId, ServerMessage> {
    parse_id_for_request(id_str, request_id, RegionId::from_uuid, "Invalid region ID")
}

/// Parse a location ID for Request/Response pattern.
fn parse_location_id_for_request(
    id_str: &str,
    request_id: &str,
) -> Result<LocationId, ServerMessage> {
    parse_id_for_request(
        id_str,
        request_id,
        LocationId::from_uuid,
        "Invalid location ID",
    )
}

/// Parse an item ID for Request/Response pattern.
fn parse_item_id_for_request(id_str: &str, request_id: &str) -> Result<ItemId, ServerMessage> {
    parse_id_for_request(id_str, request_id, ItemId::from_uuid, "Invalid item ID")
}

/// Parse a goal ID for Request/Response pattern.
fn parse_goal_id_for_request(id_str: &str, request_id: &str) -> Result<GoalId, ServerMessage> {
    parse_id_for_request(id_str, request_id, GoalId::from_uuid, "Invalid goal ID")
}

/// Parse a want ID for Request/Response pattern.
fn parse_want_id_for_request(id_str: &str, request_id: &str) -> Result<WantId, ServerMessage> {
    parse_id_for_request(id_str, request_id, WantId::from_uuid, "Invalid want ID")
}

/// Parse a challenge ID for Request/Response pattern.
fn parse_challenge_id_for_request(
    id_str: &str,
    request_id: &str,
) -> Result<ChallengeId, ServerMessage> {
    parse_id_for_request(
        id_str,
        request_id,
        ChallengeId::from_uuid,
        "Invalid challenge ID",
    )
}

/// Parse a narrative event ID for the Request/Response pattern.
fn parse_narrative_event_id_for_request(
    id_str: &str,
    request_id: &str,
) -> Result<NarrativeEventId, ServerMessage> {
    parse_id_for_request(
        id_str,
        request_id,
        NarrativeEventId::from_uuid,
        "Invalid narrative event ID",
    )
}

/// Parse an event chain ID for the Request/Response pattern.
fn parse_event_chain_id_for_request(
    id_str: &str,
    request_id: &str,
) -> Result<EventChainId, ServerMessage> {
    parse_id_for_request(
        id_str,
        request_id,
        EventChainId::from_uuid,
        "Invalid event chain ID",
    )
}

fn parse_scene_id_for_request(id_str: &str, request_id: &str) -> Result<SceneId, ServerMessage> {
    parse_id_for_request(id_str, request_id, SceneId::from_uuid, "Invalid scene ID")
}

fn parse_act_id_for_request(id_str: &str, request_id: &str) -> Result<ActId, ServerMessage> {
    parse_id_for_request(id_str, request_id, ActId::from_uuid, "Invalid act ID")
}

fn parse_interaction_id_for_request(
    id_str: &str,
    request_id: &str,
) -> Result<InteractionId, ServerMessage> {
    parse_id_for_request(
        id_str,
        request_id,
        InteractionId::from_uuid,
        "Invalid interaction ID",
    )
}

fn parse_skill_id_for_request(id_str: &str, request_id: &str) -> Result<SkillId, ServerMessage> {
    parse_id_for_request(id_str, request_id, SkillId::from_uuid, "Invalid skill ID")
}
