// WebSocket handling - store methods prepared for future use
#![allow(dead_code)]

//! WebSocket handling for Player connections.
//!
//! Handles the WebSocket protocol between Engine and Player clients.

use std::{sync::{Arc, RwLock}, time::Duration};

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

use wrldbldr_domain::ConnectionId;

mod ws_actantial;
mod ws_approval;
mod ws_challenge;
mod ws_character;
mod ws_character_sheet;
mod ws_content;
mod ws_conversation;
mod ws_creator;
mod ws_dm;
mod ws_event_chain;
mod ws_inventory;
mod ws_items;
mod ws_location;
mod ws_lore;
mod ws_movement;
mod ws_narrative_event;
mod ws_npc;
mod ws_player;
mod ws_player_action;
mod ws_scene;
mod ws_session;
mod ws_skill;
mod ws_staging;
mod ws_stat;
mod ws_story_events;
mod ws_time;
mod ws_world;

pub mod error_sanitizer;

use wrldbldr_domain::{
    ActId, ChallengeId, CharacterId, EventChainId, GoalId, InteractionId, ItemId, LocationId,
    MoodState, NarrativeEventId, PlayerCharacterId, RegionId, SceneId, SkillId, StagingSource,
    UserId, WantId, WorldId,
};
use wrldbldr_shared::{
    ClientMessage, ErrorCode, RequestPayload, ResponseResult, ServerMessage,
    WorldRole as ProtoWorldRole,
};

use super::connections::ConnectionManager;
use crate::app::App;
use crate::infrastructure::cache::TtlCache;
use crate::infrastructure::ports::{PendingStagingRequest, TimeSuggestion};

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
    pub pending_time_suggestions: Arc<TimeSuggestionStoreImpl>,
    pub pending_staging_requests: Arc<PendingStagingStoreImpl>,
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
    processed_ids: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl PendingStagingStoreImpl {
    pub fn new() -> Self {
        Self {
            inner: TtlCache::new(STAGING_REQUEST_TTL),
            processed_ids: Arc::new(RwLock::new(std::collections::HashSet::new())),
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

    /// Remove and mark as processed in one atomic operation.
    /// Returns the removed request if it existed and wasn't already processed.
    pub async fn remove_and_mark_processed(&self, key: &str) -> Option<PendingStagingRequest> {
        let key_str = key.to_string();

        // Atomic check-and-mark: if already processed, return None
        if !self.mark_processed(key) {
            return None;
        }

        // If we successfully marked it as processed, attempt to remove from cache
        // This handles the race where the request was just removed but not yet marked
        self.inner.remove(&key_str).await
    }

    /// Check if a request has already been processed.
    pub fn contains_processed(&self, key: &str) -> bool {
        let processed_ids = self.processed_ids.read().unwrap();
        processed_ids.contains(key)
    }

    /// Check if a key exists in the pending cache.
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

impl PendingStagingStoreImpl {
    /// Atomic check-and-mark operation to prevent double-approval.
    /// Returns true if the key was removed from processed_ids (was not already present).
    pub fn mark_processed(&self, key: &str) -> bool {
        let mut processed_ids = self.processed_ids.write().unwrap();
        processed_ids.remove(key)
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
    let connection_id = ConnectionId::new();
    let user_id = UserId::from_trusted(connection_id.to_string()); // Anonymous user for now

    // Create a bounded channel for sending messages to this client
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(CONNECTION_CHANNEL_BUFFER);

    // Register the connection
    state
        .connections
        .register(connection_id, user_id, tx.clone())
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
                    let error = error_response(
                        ErrorCode::BadRequest,
                        &format!("Invalid message format: {}", e),
                    );
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
    connection_id: ConnectionId,
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
            Some(error_response(
                ErrorCode::BadRequest,
                "Unrecognized message type",
            ))
        }

        // All other message types - return not implemented for now
        _ => {
            tracing::debug!(connection_id = %connection_id, "Unhandled message type");
            Some(error_response(
                ErrorCode::NotImplemented,
                "This message type is not yet implemented",
            ))
        }
    }
}

async fn handle_request(
    state: &WsState,
    connection_id: ConnectionId,
    request_id: String,
    payload: RequestPayload,
) -> Option<ServerMessage> {
    // Validate request_id length
    if request_id.is_empty() || request_id.len() > 100 {
        return Some(ServerMessage::Response {
            request_id: "invalid".to_string(),
            result: ResponseResult::error(ErrorCode::BadRequest, "Invalid request_id"),
        });
    }

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
            ws_world::handle_world_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Character(req) => {
            ws_character::handle_character_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Location(req) => {
            ws_location::handle_location_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Region(req) => {
            ws_location::handle_region_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Time(req) => {
            ws_time::handle_time_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Npc(req) => {
            ws_npc::handle_npc_request(state, &request_id, &conn_info, req).await
        }
        RequestPayload::Items(req) => {
            ws_items::handle_items_request(state, &request_id, &conn_info, req).await
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

fn error_response(code: ErrorCode, message: &str) -> ServerMessage {
    // Serialize ErrorCode to snake_case string (e.g., ErrorCode::NotFound -> "not_found")
    let code_str = serde_json::to_string(&code)
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to serialize error code: {}", e);
            "\"internal_error\"".to_string()
        })
        .trim_matches('"')
        .to_string();
    ServerMessage::Error {
        code: code_str,
        message: message.to_string(),
    }
}

fn parse_staging_source(source: &str) -> StagingSource {
    source.parse().unwrap_or_else(|_| {
        tracing::warn!("Invalid staging source '{}', defaulting to Unknown", source);
        StagingSource::Unknown
    })
}

/// Parse a string ID into a typed domain ID, returning an error response on failure.
fn parse_id<T, F>(id_str: &str, from_uuid: F, error_msg: &str) -> Result<T, ServerMessage>
where
    F: FnOnce(Uuid) -> T,
{
    Uuid::parse_str(id_str).map(from_uuid).map_err(|e| {
        tracing::warn!(input = %id_str, expected = "UUID", error = %e, "Invalid ID format");
        error_response(ErrorCode::ValidationError, error_msg)
    })
}

/// Apply pagination limits using settings-based defaults.
///
/// Returns (limit, offset) with proper bounds:
/// - Client-provided limit is respected if specified
/// - Environment variable overrides default if set
/// - Maximum limit is always enforced (hard cap)
///
/// # Example
/// ```ignore
/// let settings = state.app.settings().await;
/// let (limit, offset) = apply_pagination_limits(&settings, client_limit, client_offset);
/// ```
///
/// # Priority Order
/// 1. Client-provided limit (highest)
/// 2. Environment variable override (medium)
/// 3. Default setting (lowest)
/// 4. Maximum limit (hard cap, always applied)
pub fn apply_pagination_limits(
    settings: &crate::infrastructure::app_settings::AppSettings,
    client_limit: Option<u32>,
    client_offset: Option<u32>,
) -> (u32, Option<u32>) {
    let effective_default = settings.list_default_page_size_effective();
    let effective_max = settings.list_max_page_size_effective();

    // Client limit, or default, capped at max
    let limit = client_limit.unwrap_or(effective_default).min(effective_max);

    // Offset (default to 0)
    let offset = client_offset.unwrap_or(0);

    (limit, Some(offset))
}

// =============================================================================
// WebSocket Integration Tests
// =============================================================================

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod error_mapping_tests;

#[cfg(test)]
mod list_limits_tests;

#[cfg(test)]
mod ws_integration_tests;

#[cfg(test)]
pub mod e2e_client;

#[cfg(test)]
pub mod e2e_scenarios;

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
            ErrorCode::Unauthorized,
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
    // Reject obviously invalid UUID strings (too long)
    if id_str.len() > 100 {
        return Err(ServerMessage::Response {
            request_id: request_id.to_string(),
            result: ResponseResult::error(ErrorCode::BadRequest, "ID string too long"),
        });
    }

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
