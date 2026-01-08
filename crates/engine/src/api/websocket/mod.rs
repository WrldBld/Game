//! WebSocket handling for Player connections.
//!
//! Handles the WebSocket protocol between Engine and Player clients.

use std::{collections::HashMap, sync::Arc};

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

mod ws_challenge;
mod ws_core;
mod ws_creator;
mod ws_event_chain;
mod ws_actantial;
mod ws_location;
mod ws_lore;
mod ws_narrative_event;
mod ws_player;
mod ws_scene;
mod ws_skill;
mod ws_story_events;

use crate::use_cases::narrative::EffectExecutionContext;
use wrldbldr_domain::{
    ActId, ChallengeId, CharacterId, EventChainId, GoalId, InteractionId, ItemId, LocationId,
    MoodState, NarrativeEventId, PlayerCharacterId, RegionId, SceneId, SkillId, StagingSource,
    WantId, WorldId,
};
use wrldbldr_protocol::{
    ClientMessage, ErrorCode, RequestPayload, ResponseResult, ServerMessage,
    WorldRole as ProtoWorldRole,
};

use super::connections::{ConnectionManager, WorldRole};
use crate::app::App;
use crate::use_cases::movement::{EnterRegionError, StagingStatus};
use crate::use_cases::staging::PendingStagingRequest;

/// Buffer size for per-connection message channel.
const CONNECTION_CHANNEL_BUFFER: usize = 256;

/// Combined state for WebSocket handlers.
pub struct WsState {
    pub app: Arc<App>,
    pub connections: Arc<ConnectionManager>,
    pub pending_time_suggestions:
        tokio::sync::RwLock<HashMap<Uuid, crate::use_cases::time::TimeSuggestion>>,
    pub pending_staging_requests: tokio::sync::RwLock<HashMap<String, PendingStagingRequest>>,
    pub generation_read_state:
        tokio::sync::RwLock<HashMap<String, ws_creator::GenerationReadState>>,
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
            pc_id,
            spectate_pc_id,
        } => handle_join_world(state, connection_id, world_id, role, pc_id, spectate_pc_id).await,

        ClientMessage::LeaveWorld => {
            // Broadcast UserLeft to other world members before leaving
            if let Some(conn_info) = state.connections.get(connection_id).await {
                if let Some(world_id) = conn_info.world_id {
                    let user_left_msg = ServerMessage::UserLeft {
                        user_id: conn_info.user_id,
                    };
                    state
                        .connections
                        .broadcast_to_world_except(world_id, connection_id, user_left_msg)
                        .await;
                }
            }
            state.connections.leave_world(connection_id).await;
            None
        }

        // Movement
        ClientMessage::MoveToRegion { pc_id, region_id } => {
            handle_move_to_region(state, connection_id, pc_id, region_id).await
        }

        ClientMessage::ExitToLocation {
            pc_id,
            location_id,
            arrival_region_id,
        } => {
            handle_exit_to_location(state, connection_id, pc_id, location_id, arrival_region_id)
                .await
        }

        // Inventory
        ClientMessage::EquipItem { pc_id, item_id } => {
            handle_inventory_action(
                state,
                connection_id,
                InventoryAction::Equip,
                &pc_id,
                &item_id,
                1,
            )
            .await
        }
        ClientMessage::UnequipItem { pc_id, item_id } => {
            handle_inventory_action(
                state,
                connection_id,
                InventoryAction::Unequip,
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
            handle_inventory_action(
                state,
                connection_id,
                InventoryAction::Drop,
                &pc_id,
                &item_id,
                quantity,
            )
            .await
        }
        ClientMessage::PickupItem { pc_id, item_id } => {
            handle_inventory_action(
                state,
                connection_id,
                InventoryAction::Pickup,
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
            handle_challenge_roll(state, connection_id, challenge_id, roll).await
        }

        ClientMessage::ChallengeRollInput {
            challenge_id,
            input_type,
        } => handle_challenge_roll_input(state, connection_id, challenge_id, input_type).await,

        ClientMessage::TriggerChallenge {
            challenge_id,
            target_character_id,
        } => {
            handle_trigger_challenge(state, connection_id, challenge_id, target_character_id).await
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
            handle_staging_approval(
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
        } => handle_staging_regenerate(state, connection_id, request_id, guidance).await,

        ClientMessage::PreStageRegion {
            region_id,
            npcs,
            ttl_hours,
            location_state_id,
            region_state_id,
        } => {
            handle_pre_stage_region(
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
        } => handle_approval_decision(state, connection_id, request_id, decision).await,

        ClientMessage::ChallengeSuggestionDecision {
            request_id,
            approved,
            modified_difficulty,
        } => {
            handle_challenge_suggestion_decision(
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
        } => handle_challenge_outcome_decision(state, connection_id, resolution_id, decision).await,

        ClientMessage::NarrativeEventSuggestionDecision {
            request_id,
            event_id,
            approved,
            selected_outcome,
        } => {
            handle_narrative_event_decision(
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
            handle_directorial_update(state, connection_id, context).await
        }

        ClientMessage::TriggerApproachEvent {
            npc_id,
            target_pc_id,
            description,
            reveal,
        } => {
            handle_trigger_approach_event(
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
        } => handle_trigger_location_event(state, connection_id, region_id, description).await,

        ClientMessage::ShareNpcLocation {
            pc_id,
            npc_id,
            location_id,
            region_id,
            notes,
        } => {
            handle_share_npc_location(
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
        } => handle_set_game_time(state, connection_id, world_id, day, hour, notify_players).await,

        ClientMessage::SkipToPeriod { world_id, period } => {
            handle_skip_to_period(state, connection_id, world_id, period).await
        }

        ClientMessage::PauseGameTime { world_id, paused } => {
            handle_pause_game_time(state, connection_id, world_id, paused).await
        }

        ClientMessage::SetTimeMode { world_id, mode } => {
            handle_set_time_mode(state, connection_id, world_id, mode).await
        }

        ClientMessage::SetTimeCosts { world_id, costs } => {
            handle_set_time_costs(state, connection_id, world_id, costs).await
        }

        ClientMessage::RespondToTimeSuggestion {
            suggestion_id,
            decision,
        } => handle_respond_to_time_suggestion(state, connection_id, suggestion_id, decision).await,

        // Player action handler
        ClientMessage::PlayerAction {
            action_type,
            target,
            dialogue,
        } => handle_player_action(state, connection_id, action_type, target, dialogue).await,

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

// =============================================================================
// Handler Implementations
// =============================================================================

async fn handle_join_world(
    state: &WsState,
    connection_id: Uuid,
    world_id: Uuid,
    role: ProtoWorldRole,
    pc_id: Option<Uuid>,
    _spectate_pc_id: Option<Uuid>,
) -> Option<ServerMessage> {
    let world_id_typed = WorldId::from_uuid(world_id);

    // Convert protocol role to internal role
    let internal_role = match role {
        ProtoWorldRole::Dm => WorldRole::Dm,
        ProtoWorldRole::Player => WorldRole::Player,
        ProtoWorldRole::Spectator | ProtoWorldRole::Unknown => WorldRole::Spectator,
    };

    let pc_id_typed = pc_id.map(PlayerCharacterId::from_uuid);
    let include_pc = matches!(role, ProtoWorldRole::Player);

    let join_result = match state
        .app
        .use_cases
        .session
        .join_world
        .execute(world_id_typed, pc_id_typed, include_pc)
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::session::JoinWorldError::WorldNotFound) => {
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_protocol::JoinError::WorldNotFound,
            });
        }
        Err(crate::use_cases::session::JoinWorldError::Repo(e)) => {
            tracing::error!(error = %e, "Failed to build world snapshot");
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_protocol::JoinError::Unknown,
            });
        }
    };

    // Join the world
    if let Err(e) = state
        .connections
        .join_world(connection_id, world_id_typed, internal_role, pc_id_typed)
        .await
    {
        return Some(ServerMessage::WorldJoinFailed {
            world_id,
            error: match e {
                super::connections::ConnectionError::DmAlreadyConnected => {
                    wrldbldr_protocol::JoinError::DmAlreadyConnected {
                        existing_user_id: String::new(),
                    }
                }
                _ => wrldbldr_protocol::JoinError::Unknown,
            },
        });
    }

    // Get connected users
    let connected_users = state
        .connections
        .get_world_connections(world_id_typed)
        .await
        .into_iter()
        .map(|info| wrldbldr_protocol::ConnectedUser {
            user_id: info.user_id,
            username: None,
            role: match info.role {
                WorldRole::Dm => ProtoWorldRole::Dm,
                WorldRole::Player => ProtoWorldRole::Player,
                WorldRole::Spectator => ProtoWorldRole::Spectator,
            },
            pc_id: info.pc_id.map(|id| id.to_string()),
            connection_count: 1,
        })
        .collect();

    // Get connection info to broadcast UserJoined to other world members
    if let Some(conn_info) = state.connections.get(connection_id).await {
        let user_joined_msg = ServerMessage::UserJoined {
            user_id: conn_info.user_id,
            username: None,
            role,
            pc: join_result.your_pc.clone(),
        };
        state
            .connections
            .broadcast_to_world_except(world_id_typed, connection_id, user_joined_msg)
            .await;
    }

    Some(ServerMessage::WorldJoined {
        world_id,
        snapshot: join_result.snapshot,
        connected_users,
        your_role: role,
        your_pc: join_result.your_pc,
    })
}

async fn handle_move_to_region(
    state: &WsState,
    connection_id: Uuid,
    pc_id: String,
    region_id: String,
) -> Option<ServerMessage> {
    // Parse IDs
    let pc_uuid = match parse_pc_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let region_uuid = match parse_region_id(&region_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get connection info to verify authorization
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Verify the PC belongs to this connection (or is DM)
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response("UNAUTHORIZED", "Cannot control this PC"));
    }

    // Execute movement use case
    match state
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_uuid, region_uuid)
        .await
    {
        Ok(result) => {
            // Get location name for the response
            let location_name = state
                .app
                .entities
                .location
                .get(result.region.location_id)
                .await
                .ok()
                .flatten()
                .map(|l| l.name.clone())
                .unwrap_or_else(|| "Unknown Location".to_string());

            // Check staging status
            match result.staging_status {
                StagingStatus::Pending { previous_staging } => {
                    let world_id = result.pc.world_id;
                    let ctx = crate::use_cases::staging::StagingApprovalContext {
                        connections: &state.connections,
                        pending_time_suggestions: &state.pending_time_suggestions,
                        pending_staging_requests: &state.pending_staging_requests,
                    };
                    let input = crate::use_cases::staging::StagingApprovalInput {
                        world_id,
                        region: result.region.clone(),
                        pc: result.pc.clone(),
                        previous_staging,
                        time_suggestion: result.time_suggestion.clone(),
                        guidance: None,
                    };

                    match state
                        .app
                        .use_cases
                        .staging
                        .request_approval
                        .execute(&ctx, input)
                        .await
                    {
                        Ok(msg) => Some(msg),
                        Err(e) => Some(error_response("STAGING_ERROR", &e.to_string())),
                    }
                }
                StagingStatus::Ready => {
                    // Build SceneChanged response with NPCs
                    let region_data = wrldbldr_protocol::RegionData {
                        id: result.region.id.to_string(),
                        name: result.region.name.clone(),
                        location_id: result.region.location_id.to_string(),
                        location_name,
                        backdrop_asset: result.region.backdrop_asset.clone(),
                        atmosphere: result.region.atmosphere.clone(),
                        map_asset: None,
                    };

                    let npcs_present: Vec<wrldbldr_protocol::NpcPresenceData> = result
                        .npcs
                        .into_iter()
                        .map(|npc| wrldbldr_protocol::NpcPresenceData {
                            character_id: npc.character_id.to_string(),
                            name: npc.name,
                            sprite_asset: npc.sprite_asset,
                            portrait_asset: npc.portrait_asset,
                        })
                        .collect();

                    // Get navigation data
                    let navigation =
                        build_navigation_data(&state.app.entities.location, region_uuid).await;

                    // Get items in the region
                    let region_items =
                        build_region_items(state.app.use_cases.inventory.ops.as_ref(), region_uuid)
                            .await;

                    // Broadcast time suggestion to DMs if present
                    if let Some(ref time_suggestion) = result.time_suggestion {
                        if let Some(world_id) = conn_info.world_id {
                            state
                                .pending_time_suggestions
                                .write()
                                .await
                                .insert(time_suggestion.id, time_suggestion.clone());
                            let suggestion_msg = ServerMessage::TimeSuggestion {
                                data: time_suggestion.to_protocol(),
                            };
                            state
                                .connections
                                .broadcast_to_dms(world_id, suggestion_msg)
                                .await;
                        }
                    }

                    Some(ServerMessage::SceneChanged {
                        pc_id: pc_id.clone(),
                        region: region_data,
                        npcs_present,
                        navigation,
                        region_items,
                    })
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Movement failed");
            match e {
                EnterRegionError::MovementBlocked(reason) => Some(ServerMessage::MovementBlocked {
                    pc_id: pc_id.clone(),
                    reason,
                }),
                _ => Some(error_response("MOVEMENT_FAILED", &e.to_string())),
            }
        }
    }
}

async fn handle_exit_to_location(
    state: &WsState,
    connection_id: Uuid,
    pc_id: String,
    location_id: String,
    arrival_region_id: Option<String>,
) -> Option<ServerMessage> {
    // Parse IDs
    let pc_uuid = match parse_pc_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let location_uuid = match parse_location_id(&location_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let arrival_uuid = match &arrival_region_id {
        Some(id) => match parse_region_id(id) {
            Ok(r) => Some(r),
            Err(e) => return Some(e),
        },
        None => None,
    };

    // Get connection info
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Verify authorization
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response("UNAUTHORIZED", "Cannot control this PC"));
    }

    // Execute movement use case
    match state
        .app
        .use_cases
        .movement
        .exit_location
        .execute(pc_uuid, location_uuid, arrival_uuid)
        .await
    {
        Ok(result) => {
            // Get location name for the response
            let location_name = state
                .app
                .entities
                .location
                .get(result.region.location_id)
                .await
                .ok()
                .flatten()
                .map(|l| l.name.clone())
                .unwrap_or_else(|| "Unknown Location".to_string());

            let region_data = wrldbldr_protocol::RegionData {
                id: result.region.id.to_string(),
                name: result.region.name.clone(),
                location_id: result.region.location_id.to_string(),
                location_name,
                backdrop_asset: result.region.backdrop_asset.clone(),
                atmosphere: result.region.atmosphere.clone(),
                map_asset: None,
            };

            let npcs_present: Vec<wrldbldr_protocol::NpcPresenceData> = result
                .npcs
                .into_iter()
                .filter(|npc| npc.is_visible_to_players())
                .map(|npc| wrldbldr_protocol::NpcPresenceData {
                    character_id: npc.character_id.to_string(),
                    name: npc.name,
                    sprite_asset: npc.sprite_asset,
                    portrait_asset: npc.portrait_asset,
                })
                .collect();

            // Get navigation data for new region
            let navigation =
                build_navigation_data(&state.app.entities.location, result.region.id).await;

            // Get items in the region
            let region_items =
                build_region_items(state.app.use_cases.inventory.ops.as_ref(), result.region.id)
                    .await;

            Some(ServerMessage::SceneChanged {
                pc_id: pc_id.clone(),
                region: region_data,
                npcs_present,
                navigation,
                region_items,
            })
        }
        Err(e) => {
            tracing::error!(error = %e, "Exit to location failed");
            Some(error_response("MOVEMENT_FAILED", &e.to_string()))
        }
    }
}

async fn handle_request(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    payload: RequestPayload,
) -> Option<ServerMessage> {
    // Get connection info
    let _conn_info = match state.connections.get(connection_id).await {
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
            ws_lore::handle_lore_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::StoryEvent(req) => {
            ws_story_events::handle_story_event_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::World(req) => {
            ws_core::handle_world_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Character(req) => {
            ws_core::handle_character_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Location(req) => {
            ws_location::handle_location_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Region(req) => {
            ws_location::handle_region_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Time(req) => {
            ws_core::handle_time_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Npc(req) => {
            ws_core::handle_npc_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Items(req) => {
            ws_core::handle_items_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::PlayerCharacter(req) => {
            ws_player::handle_player_character_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Relationship(req) => {
            ws_player::handle_relationship_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Observation(req) => {
            ws_player::handle_observation_request(state, &request_id, &_conn_info, req).await
        }

        RequestPayload::Generation(req) => {
            ws_creator::handle_generation_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Ai(req) => {
            ws_creator::handle_ai_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Expression(req) => {
            ws_creator::handle_expression_request(state, &request_id, &_conn_info, req).await
        }

        RequestPayload::Challenge(req) => {
            ws_challenge::handle_challenge_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::NarrativeEvent(req) => {
            ws_narrative_event::handle_narrative_event_request(state, &request_id, &_conn_info, req)
                .await
        }
        RequestPayload::EventChain(req) => {
            ws_event_chain::handle_event_chain_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Goal(req) => {
            ws_actantial::handle_goal_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Want(req) => {
            ws_actantial::handle_want_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Actantial(req) => {
            ws_actantial::handle_actantial_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Scene(req) => {
            ws_scene::handle_scene_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Act(req) => {
            ws_scene::handle_act_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Interaction(req) => {
            ws_scene::handle_interaction_request(state, &request_id, &_conn_info, req).await
        }
        RequestPayload::Skill(req) => {
            ws_skill::handle_skill_request(state, &request_id, &_conn_info, req).await
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
// Inventory Handler
// =============================================================================

#[derive(Debug)]
enum InventoryAction {
    Equip,
    Unequip,
    Drop,
    Pickup,
}

async fn handle_inventory_action(
    state: &WsState,
    connection_id: Uuid,
    action: InventoryAction,
    pc_id: &str,
    item_id: &str,
    quantity: u32,
) -> Option<ServerMessage> {
    // Parse IDs
    let pc_uuid = match parse_pc_id(pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let item_uuid = match parse_item_id(item_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get connection info
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Verify authorization
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response("UNAUTHORIZED", "Cannot control this PC"));
    }

    // Execute the inventory action
    let result = match action {
        InventoryAction::Equip => {
            state
                .app
                .entities
                .inventory
                .equip_item(pc_uuid, item_uuid)
                .await
        }
        InventoryAction::Unequip => {
            state
                .app
                .entities
                .inventory
                .unequip_item(pc_uuid, item_uuid)
                .await
        }
        InventoryAction::Drop => {
            state
                .app
                .entities
                .inventory
                .drop_item(pc_uuid, item_uuid, quantity)
                .await
        }
        InventoryAction::Pickup => {
            state
                .app
                .entities
                .inventory
                .pickup_item(pc_uuid, item_uuid)
                .await
        }
    };

    match result {
        Ok(action_result) => match action {
            InventoryAction::Equip => Some(ServerMessage::ItemEquipped {
                pc_id: pc_id.to_string(),
                item_id: item_id.to_string(),
                item_name: action_result.item_name,
            }),
            InventoryAction::Unequip => Some(ServerMessage::ItemUnequipped {
                pc_id: pc_id.to_string(),
                item_id: item_id.to_string(),
                item_name: action_result.item_name,
            }),
            InventoryAction::Drop => Some(ServerMessage::ItemDropped {
                pc_id: pc_id.to_string(),
                item_id: item_id.to_string(),
                item_name: action_result.item_name,
                quantity: action_result.quantity,
            }),
            InventoryAction::Pickup => Some(ServerMessage::ItemPickedUp {
                pc_id: pc_id.to_string(),
                item_id: item_id.to_string(),
                item_name: action_result.item_name,
            }),
        },
        Err(e) => {
            tracing::error!(error = %e, action = ?action, "Inventory action failed");
            Some(error_response("INVENTORY_ERROR", &e.to_string()))
        }
    }
}

// =============================================================================
// Challenge Handlers
// =============================================================================

async fn handle_challenge_roll(
    state: &WsState,
    connection_id: Uuid,
    challenge_id: String,
    roll: i32,
) -> Option<ServerMessage> {
    // Parse challenge ID
    let challenge_uuid = match parse_challenge_id(&challenge_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get connection info
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Get the world ID from connection
    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    // Get PC ID from connection (required for challenge rolls)
    let pc_id = match conn_info.pc_id {
        Some(id) => id,
        None => return Some(error_response("NO_PC", "Must have a PC to roll challenges")),
    };

    // Execute the roll challenge use case
    // For legacy ChallengeRoll, we use client-provided roll with 0 modifier
    match state
        .app
        .use_cases
        .challenge
        .roll
        .execute(
            world_id,
            challenge_uuid,
            pc_id,
            Some(roll),
            0, // No modifier for legacy roll
        )
        .await
    {
        Ok(result) => {
            // If approval is required, notify DMs
            if result.requires_approval {
                if let Some(approval_id) = result.approval_queue_id {
                    let dm_msg = ServerMessage::ChallengeOutcomePending {
                        resolution_id: approval_id.to_string(),
                        challenge_id: result.challenge_id.to_string(),
                        challenge_name: result.challenge_name.clone(),
                        character_id: result.character_id.to_string(),
                        character_name: result.character_name.clone(),
                        roll: result.roll,
                        modifier: result.modifier,
                        total: result.total,
                        outcome_type: format!("{:?}", result.outcome_type),
                        outcome_description: result.outcome_description.clone(),
                        outcome_triggers: result
                            .outcome_triggers
                            .iter()
                            .map(|t| wrldbldr_protocol::ProposedToolInfo {
                                id: t.id.clone(),
                                name: t.name.clone(),
                                description: t.description.clone(),
                                arguments: t.arguments.clone(),
                            })
                            .collect(),
                        roll_breakdown: result.roll_breakdown.clone(),
                    };
                    state.connections.broadcast_to_dms(world_id, dm_msg).await;
                }
            }

            Some(ServerMessage::ChallengeRollSubmitted {
                challenge_id,
                challenge_name: result.challenge_name,
                roll: result.roll,
                modifier: result.modifier,
                total: result.total,
                outcome_type: format!("{:?}", result.outcome_type),
                status: if result.requires_approval {
                    "pending_approval".to_string()
                } else {
                    "resolved".to_string()
                },
            })
        }
        Err(e) => {
            tracing::error!(error = %e, "Challenge roll failed");
            Some(error_response("CHALLENGE_ROLL_FAILED", &e.to_string()))
        }
    }
}

async fn handle_challenge_roll_input(
    state: &WsState,
    connection_id: Uuid,
    challenge_id: String,
    input_type: wrldbldr_protocol::DiceInputType,
) -> Option<ServerMessage> {
    // Parse challenge ID
    let challenge_uuid = match parse_challenge_id(&challenge_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get connection info
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Get the world ID from connection
    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    // Get PC ID from connection
    let pc_id = match conn_info.pc_id {
        Some(id) => id,
        None => return Some(error_response("NO_PC", "Must have a PC to roll challenges")),
    };

    // Determine roll value based on input type
    let (client_roll, modifier) = match input_type {
        wrldbldr_protocol::DiceInputType::Manual(value) => (Some(value), 0),
        wrldbldr_protocol::DiceInputType::Formula(formula) => {
            // For formula-based rolls, let the server roll
            // The formula could contain modifiers like "1d20+5"
            // For now, we'll parse simple modifiers from the formula
            let modifier = parse_modifier_from_formula(&formula);
            (None, modifier)
        }
        wrldbldr_protocol::DiceInputType::Unknown => {
            return Some(error_response("INVALID_INPUT", "Unknown dice input type"));
        }
    };

    // Execute the roll challenge use case
    match state
        .app
        .use_cases
        .challenge
        .roll
        .execute(world_id, challenge_uuid, pc_id, client_roll, modifier)
        .await
    {
        Ok(result) => {
            // If approval is required, notify DMs
            if result.requires_approval {
                if let Some(approval_id) = result.approval_queue_id {
                    let dm_msg = ServerMessage::ChallengeOutcomePending {
                        resolution_id: approval_id.to_string(),
                        challenge_id: result.challenge_id.to_string(),
                        challenge_name: result.challenge_name.clone(),
                        character_id: result.character_id.to_string(),
                        character_name: result.character_name.clone(),
                        roll: result.roll,
                        modifier: result.modifier,
                        total: result.total,
                        outcome_type: format!("{:?}", result.outcome_type),
                        outcome_description: result.outcome_description.clone(),
                        outcome_triggers: result
                            .outcome_triggers
                            .iter()
                            .map(|t| wrldbldr_protocol::ProposedToolInfo {
                                id: t.id.clone(),
                                name: t.name.clone(),
                                description: t.description.clone(),
                                arguments: t.arguments.clone(),
                            })
                            .collect(),
                        roll_breakdown: result.roll_breakdown.clone(),
                    };
                    state.connections.broadcast_to_dms(world_id, dm_msg).await;
                }
            }

            Some(ServerMessage::ChallengeRollSubmitted {
                challenge_id,
                challenge_name: result.challenge_name,
                roll: result.roll,
                modifier: result.modifier,
                total: result.total,
                outcome_type: format!("{:?}", result.outcome_type),
                status: if result.requires_approval {
                    "pending_approval".to_string()
                } else {
                    "resolved".to_string()
                },
            })
        }
        Err(e) => {
            tracing::error!(error = %e, "Challenge roll input failed");
            Some(error_response("CHALLENGE_ROLL_FAILED", &e.to_string()))
        }
    }
}

async fn handle_trigger_challenge(
    state: &WsState,
    connection_id: Uuid,
    challenge_id: String,
    target_character_id: String,
) -> Option<ServerMessage> {
    // Parse challenge ID
    let challenge_uuid = match parse_challenge_id(&challenge_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Parse target character ID (could be PC or NPC, but we use PlayerCharacterId for PCs)
    let _target_uuid = match parse_pc_id(&target_character_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get connection info
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Only DMs can trigger challenges manually
    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let prompt_data = match state
        .app
        .use_cases
        .challenge
        .trigger_prompt
        .execute(challenge_uuid)
        .await
    {
        Ok(data) => data,
        Err(crate::use_cases::challenge::ChallengeError::NotFound) => {
            return Some(error_response("NOT_FOUND", "Challenge not found"))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch challenge");
            return Some(error_response(
                "INTERNAL_ERROR",
                "Failed to fetch challenge",
            ));
        }
    };

    // Get target PC's connection to send them the challenge prompt
    // For now, we broadcast to the world - the client filters by pc_id
    if let Some(world_id) = conn_info.world_id {
        let prompt = ServerMessage::ChallengePrompt {
            challenge_id: prompt_data.challenge_id.to_string(),
            challenge_name: prompt_data.challenge_name.clone(),
            skill_name: prompt_data.skill_name.clone(),
            difficulty_display: prompt_data.difficulty_display.clone(),
            description: prompt_data.description.clone(),
            character_modifier: prompt_data.character_modifier,
            suggested_dice: prompt_data.suggested_dice.clone(),
            rule_system_hint: prompt_data.rule_system_hint.clone(),
        };

        // Broadcast to world connections (target player will see it)
        state.connections.broadcast_to_world(world_id, prompt).await;
    }

    // Confirm to DM that challenge was triggered
    Some(ServerMessage::AdHocChallengeCreated {
        challenge_id,
        challenge_name: prompt_data.challenge_name,
        target_pc_id: target_character_id,
    })
}

/// Parse modifier from a dice formula like "1d20+5" or "2d6-2"
fn parse_modifier_from_formula(formula: &str) -> i32 {
    // Simple parsing: look for +N or -N at the end
    if let Some(plus_idx) = formula.rfind('+') {
        if let Ok(modifier) = formula[plus_idx + 1..].trim().parse::<i32>() {
            return modifier;
        }
    }
    if let Some(minus_idx) = formula.rfind('-') {
        if let Ok(modifier) = formula[minus_idx + 1..].trim().parse::<i32>() {
            return -modifier;
        }
    }
    0
}

// =============================================================================
// Staging Handlers
// =============================================================================

fn parse_staging_source(source: &str) -> StagingSource {
    match source.to_lowercase().as_str() {
        "rule" | "rulebased" | "rule_based" => StagingSource::RuleBased,
        "llm" | "llmbased" | "llm_based" => StagingSource::LlmBased,
        "prestaged" | "pre_staged" | "prestage" | "pre_stage" => StagingSource::PreStaged,
        "dm" | "dmcustomized" | "dm_customized" | "custom" | "customized" => {
            StagingSource::DmCustomized
        }
        _ => StagingSource::DmCustomized,
    }
}

async fn handle_staging_approval(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    approved_npcs: Vec<wrldbldr_protocol::ApprovedNpcInfo>,
    ttl_hours: i32,
    source: String,
    location_state_id: Option<String>,
    region_state_id: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can approve staging
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // request_id is a correlation token; resolve it to a region_id.
    let pending = {
        let mut guard = state.pending_staging_requests.write().await;
        guard.remove(&request_id)
    };

    let (region_id, location_id) = if let Some(pending) = pending {
        (pending.region_id, pending.location_id)
    } else {
        // Backward-compat: allow request_id to be the region_id.
        let region_id = match parse_region_id(&request_id) {
            Ok(id) => id,
            Err(e) => return Some(e),
        };

        // Get region to find location_id (needed for setting location state)
        let region = match state.app.entities.location.get_region(region_id).await {
            Ok(Some(r)) => r,
            Ok(None) => return Some(error_response("NOT_FOUND", "Region not found")),
            Err(e) => return Some(error_response("REPO_ERROR", &e.to_string())),
        };
        (region_id, region.location_id)
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_CONNECTED", "World not joined")),
    };

    let input = crate::use_cases::staging::ApproveStagingInput {
        region_id,
        location_id,
        world_id,
        approved_by: conn_info.user_id.clone(),
        ttl_hours,
        source: parse_staging_source(&source),
        approved_npcs,
        location_state_id,
        region_state_id,
    };

    let payload = match state.app.use_cases.staging.approve.execute(input).await {
        Ok(result) => result,
        Err(e) => return Some(error_response("REPO_ERROR", &e.to_string())),
    };

    state
        .connections
        .broadcast_to_world(
            world_id,
            ServerMessage::StagingReady {
                region_id: payload.region_id.to_string(),
                npcs_present: payload.npcs_present,
                visual_state: payload.visual_state,
            },
        )
        .await;

    None // No direct response needed - we broadcasted
}

async fn handle_staging_regenerate(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    guidance: String,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can request regeneration
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // request_id is a correlation token; resolve it to a region_id.
    let pending = {
        let guard = state.pending_staging_requests.read().await;
        guard.get(&request_id).copied()
    };

    let region_id = if let Some(pending) = pending {
        pending.region_id
    } else {
        // Backward-compat: allow request_id to be the region_id.
        match parse_region_id(&request_id) {
            Ok(id) => id,
            Err(e) => return Some(e),
        }
    };

    let guidance_opt = if guidance.is_empty() {
        None
    } else {
        Some(guidance.as_str())
    };

    let llm_based_npcs = match state
        .app
        .use_cases
        .staging
        .regenerate
        .execute(region_id, guidance_opt)
        .await
    {
        Ok(npcs) => npcs,
        Err(crate::use_cases::staging::StagingError::RegionNotFound) => {
            return Some(error_response("NOT_FOUND", "Region not found"))
        }
        Err(e) => return Some(error_response("REPO_ERROR", &e.to_string())),
    };

    Some(ServerMessage::StagingRegenerated {
        request_id,
        llm_based_npcs,
    })
}

async fn handle_pre_stage_region(
    state: &WsState,
    connection_id: Uuid,
    region_id: String,
    npcs: Vec<wrldbldr_protocol::ApprovedNpcInfo>,
    ttl_hours: i32,
    location_state_id: Option<String>,
    region_state_id: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can pre-stage
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse region ID
    let region_uuid = match parse_region_id(&region_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_CONNECTED", "World not joined")),
    };

    // Determine location_id for this region.
    let region = match state.app.entities.location.get_region(region_uuid).await {
        Ok(Some(r)) => r,
        Ok(None) => return Some(error_response("NOT_FOUND", "Region not found")),
        Err(e) => return Some(error_response("REPO_ERROR", &e.to_string())),
    };
    let location_id = region.location_id;

    let input = crate::use_cases::staging::ApproveStagingInput {
        region_id: region_uuid,
        location_id,
        world_id,
        approved_by: conn_info.user_id.clone(),
        ttl_hours,
        source: StagingSource::PreStaged,
        approved_npcs: npcs,
        location_state_id,
        region_state_id,
    };

    if let Err(e) = state.app.use_cases.staging.approve.execute(input).await {
        return Some(error_response("REPO_ERROR", &e.to_string()));
    }

    None
}

// =============================================================================
// Approval Handlers
// =============================================================================

async fn handle_approval_decision(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    decision: wrldbldr_protocol::ApprovalDecision,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can make approval decisions
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse request ID as approval UUID
    let approval_id = match parse_id(&request_id, |u| u, "Invalid request ID") {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Convert protocol decision to domain decision
    let domain_decision = match decision {
        wrldbldr_protocol::ApprovalDecision::Accept => wrldbldr_domain::DmApprovalDecision::Accept,
        wrldbldr_protocol::ApprovalDecision::AcceptWithRecipients { item_recipients } => {
            wrldbldr_domain::DmApprovalDecision::AcceptWithRecipients { item_recipients }
        }
        wrldbldr_protocol::ApprovalDecision::Reject { feedback } => {
            wrldbldr_domain::DmApprovalDecision::Reject { feedback }
        }
        wrldbldr_protocol::ApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => wrldbldr_domain::DmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        },
        wrldbldr_protocol::ApprovalDecision::TakeOver { dm_response } => {
            wrldbldr_domain::DmApprovalDecision::TakeOver { dm_response }
        }
        wrldbldr_protocol::ApprovalDecision::Unknown => {
            return Some(error_response(
                "INVALID_DECISION",
                "Unknown approval decision type",
            ));
        }
    };

    // Get the original approval request data for dialogue recording
    let approval_data = match state.app.queue.get_approval_request(approval_id).await {
        Ok(Some(data)) => Some(data),
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to get approval request data for dialogue recording");
            None
        }
    };

    // Execute approval use case
    match state
        .app
        .use_cases
        .approval
        .approve_suggestion
        .execute(approval_id, domain_decision)
        .await
    {
        Ok(result) => {
            if result.approved {
                if let Some(world_id) = conn_info.world_id {
                    let dialogue = result.final_dialogue.clone().unwrap_or_default();

                    // Record dialogue exchange to story events for persistence
                    if !dialogue.is_empty() {
                        if let Some(ref data) = approval_data {
                            if let Some(pc_id) = data.pc_id {
                                if let Some(npc_id) = data.npc_id {
                                    let player_dialogue =
                                        data.player_dialogue.clone().unwrap_or_default();
                                    if let Err(e) = state
                                        .app
                                        .entities
                                        .narrative
                                        .record_dialogue_exchange(
                                            world_id,
                                            pc_id,
                                            npc_id,
                                            data.npc_name.clone(),
                                            player_dialogue,
                                            dialogue.clone(),
                                            data.topics.clone(),
                                            data.scene_id,
                                            data.location_id,
                                            data.game_time.clone(),
                                        )
                                        .await
                                    {
                                        tracing::error!(error = %e, "Failed to record dialogue exchange");
                                    }
                                }
                            }
                        }
                    }

                    // Send ResponseApproved to DMs (shows what tools were executed)
                    let dm_msg = ServerMessage::ResponseApproved {
                        npc_dialogue: dialogue.clone(),
                        executed_tools: result.approved_tools.clone(),
                    };
                    state.connections.broadcast_to_dms(world_id, dm_msg).await;

                    // Send DialogueResponse to all players (for visual novel display)
                    if !dialogue.is_empty() {
                        let dialogue_msg = ServerMessage::DialogueResponse {
                            speaker_id: result.npc_id.unwrap_or_default(),
                            speaker_name: result.npc_name.unwrap_or_else(|| "Unknown".to_string()),
                            text: dialogue,
                            choices: vec![], // Free-form input mode
                        };
                        state
                            .connections
                            .broadcast_to_world(world_id, dialogue_msg)
                            .await;
                    }
                }
            }
            None // No direct response - we broadcasted
        }
        Err(e) => {
            tracing::error!(error = %e, "Approval decision failed");
            Some(error_response("APPROVAL_ERROR", &e.to_string()))
        }
    }
}

async fn handle_challenge_suggestion_decision(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    approved: bool,
    _modified_difficulty: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can make decisions
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse request ID as approval UUID
    let approval_id = match parse_id(&request_id, |u| u, "Invalid request ID") {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let decision = if approved {
        wrldbldr_domain::DmApprovalDecision::Accept
    } else {
        wrldbldr_domain::DmApprovalDecision::Reject {
            feedback: "Challenge rejected by DM".to_string(),
        }
    };

    match state
        .app
        .use_cases
        .approval
        .approve_suggestion
        .execute(approval_id, decision)
        .await
    {
        Ok(_) => {
            if !approved {
                Some(ServerMessage::ChallengeDiscarded { request_id })
            } else {
                None
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Challenge suggestion decision failed");
            Some(error_response("APPROVAL_ERROR", &e.to_string()))
        }
    }
}

/// Handle DM decision on a challenge outcome (after dice roll, before triggers execute).
async fn handle_challenge_outcome_decision(
    state: &WsState,
    connection_id: Uuid,
    resolution_id: String,
    decision: wrldbldr_protocol::ChallengeOutcomeDecisionData,
) -> Option<ServerMessage> {
    // Only DMs can approve challenge outcomes
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse resolution ID as approval queue UUID
    let approval_id = match Uuid::parse_str(&resolution_id) {
        Ok(id) => id,
        Err(_) => return Some(error_response("INVALID_ID", "Invalid resolution ID format")),
    };

    // Get the approval request data containing challenge outcome details
    let approval_data = match state.app.queue.get_approval_request(approval_id).await {
        Ok(Some(data)) => data,
        Ok(None) => return Some(error_response("NOT_FOUND", "Approval request not found")),
        Err(e) => return Some(error_response("REPO_ERROR", &e.to_string())),
    };

    // Extract challenge outcome data
    let outcome_data = match &approval_data.challenge_outcome {
        Some(data) => data,
        None => {
            return Some(error_response(
                "INVALID_DATA",
                "No challenge outcome data in approval request",
            ))
        }
    };

    // Parse challenge ID from stored data
    let challenge_id = match parse_challenge_id(&outcome_data.challenge_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Parse outcome type from stored string
    let outcome_type = match outcome_data.outcome_type.as_str() {
        "CriticalSuccess" => wrldbldr_domain::OutcomeType::CriticalSuccess,
        "Success" => wrldbldr_domain::OutcomeType::Success,
        "Partial" => wrldbldr_domain::OutcomeType::Partial,
        "Failure" => wrldbldr_domain::OutcomeType::Failure,
        "CriticalFailure" => wrldbldr_domain::OutcomeType::CriticalFailure,
        _ => wrldbldr_domain::OutcomeType::Success, // Default fallback
    };

    match decision {
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Accept => {
            // Get PC ID for trigger execution
            let pc_id = match approval_data.pc_id {
                Some(id) => id,
                None => {
                    tracing::error!(
                        approval_id = %approval_id,
                        challenge_id = %challenge_id,
                        "Missing pc_id on challenge outcome approval request"
                    );
                    return Some(error_response(
                        "MISSING_PC_ID",
                        "Challenge outcome is missing target PC context",
                    ));
                }
            };

            // Execute outcome triggers with PC context
            if let Err(e) = state
                .app
                .use_cases
                .challenge
                .resolve
                .execute_for_pc(challenge_id, outcome_type.clone(), pc_id)
                .await
            {
                tracing::error!(error = %e, challenge_id = %challenge_id, "Failed to execute challenge outcome");
                return Some(error_response("RESOLVE_ERROR", &e.to_string()));
            }

            // Mark the approval request as processed
            if let Err(e) = state.app.queue.mark_complete(approval_id).await {
                tracing::warn!(error = %e, "Failed to mark approval request as complete");
            }

            // Broadcast ChallengeResolved to all players in the world
            if let Some(world_id) = conn_info.world_id {
                let outcome_str = match outcome_type {
                    wrldbldr_domain::OutcomeType::CriticalSuccess => "critical_success",
                    wrldbldr_domain::OutcomeType::Success => "success",
                    wrldbldr_domain::OutcomeType::Partial => "partial",
                    wrldbldr_domain::OutcomeType::Failure => "failure",
                    wrldbldr_domain::OutcomeType::CriticalFailure => "critical_failure",
                };

                let msg = ServerMessage::ChallengeResolved {
                    challenge_id: challenge_id.to_string(),
                    challenge_name: outcome_data.challenge_name.clone(),
                    character_name: outcome_data.character_name.clone(),
                    roll: outcome_data.roll,
                    modifier: outcome_data.modifier,
                    total: outcome_data.total,
                    outcome: outcome_str.to_string(),
                    outcome_description: outcome_data.outcome_description.clone(),
                    roll_breakdown: outcome_data.roll_breakdown.clone(),
                    individual_rolls: None,
                };
                state.connections.broadcast_to_world(world_id, msg).await;
            }

            None
        }
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Edit {
            modified_description,
        } => {
            // Get PC ID for trigger execution
            let pc_id = match approval_data.pc_id {
                Some(id) => id,
                None => {
                    tracing::error!(
                        approval_id = %approval_id,
                        challenge_id = %challenge_id,
                        "Missing pc_id on challenge outcome approval request"
                    );
                    return Some(error_response(
                        "MISSING_PC_ID",
                        "Challenge outcome is missing target PC context",
                    ));
                }
            };

            tracing::info!(
                challenge_id = %challenge_id,
                modified_description = %modified_description,
                "DM edited challenge outcome description"
            );

            // Execute outcome triggers with PC context
            if let Err(e) = state
                .app
                .use_cases
                .challenge
                .resolve
                .execute_for_pc(challenge_id, outcome_type.clone(), pc_id)
                .await
            {
                return Some(error_response("RESOLVE_ERROR", &e.to_string()));
            }

            // Mark the approval request as processed
            if let Err(e) = state.app.queue.mark_complete(approval_id).await {
                tracing::warn!(error = %e, "Failed to mark approval request as complete");
            }

            // Broadcast ChallengeResolved with modified description
            if let Some(world_id) = conn_info.world_id {
                let outcome_str = match outcome_type {
                    wrldbldr_domain::OutcomeType::CriticalSuccess => "critical_success",
                    wrldbldr_domain::OutcomeType::Success => "success",
                    wrldbldr_domain::OutcomeType::Partial => "partial",
                    wrldbldr_domain::OutcomeType::Failure => "failure",
                    wrldbldr_domain::OutcomeType::CriticalFailure => "critical_failure",
                };

                let msg = ServerMessage::ChallengeResolved {
                    challenge_id: challenge_id.to_string(),
                    challenge_name: outcome_data.challenge_name.clone(),
                    character_name: outcome_data.character_name.clone(),
                    roll: outcome_data.roll,
                    modifier: outcome_data.modifier,
                    total: outcome_data.total,
                    outcome: outcome_str.to_string(),
                    outcome_description: modified_description, // Use the DM's edited description
                    roll_breakdown: outcome_data.roll_breakdown.clone(),
                    individual_rolls: None,
                };
                state.connections.broadcast_to_world(world_id, msg).await;
            }

            None
        }
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Suggest { guidance } => {
            // Queue LLM request to generate alternative outcome descriptions
            let world_id = match conn_info.world_id {
                Some(id) => id,
                None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
            };

            let llm_request = wrldbldr_domain::LlmRequestData {
                request_type: wrldbldr_domain::LlmRequestType::OutcomeSuggestion {
                    resolution_id: approval_id,
                    world_id,
                    challenge_name: outcome_data.challenge_name.clone(),
                    current_description: outcome_data.outcome_description.clone(),
                    guidance: guidance.clone(),
                },
                world_id,
                pc_id: approval_data.pc_id,
                prompt: None,
                suggestion_context: Some(wrldbldr_domain::SuggestionContext {
                    entity_type: Some("challenge_outcome".to_string()),
                    entity_name: Some(outcome_data.challenge_name.clone()),
                    world_setting: None,
                    hints: guidance.clone(),
                    additional_context: Some(format!(
                        "Current outcome: {} ({})\nRoll: {} + {} = {}",
                        outcome_data.outcome_description,
                        outcome_data.outcome_type,
                        outcome_data.roll,
                        outcome_data.modifier,
                        outcome_data.total
                    )),
                    world_id: Some(world_id),
                }),
                callback_id: format!("outcome_suggestion:{}", approval_id),
            };

            match state.app.queue.enqueue_llm_request(&llm_request).await {
                Ok(request_id) => {
                    tracing::info!(
                        resolution_id = %resolution_id,
                        request_id = %request_id,
                        "Queued LLM request for outcome suggestions"
                    );

                    // Notify DM that suggestions are being generated
                    // Note: The actual OutcomeSuggestionReady will be sent when the LLM responds
                    // via the queue processor (requires background task implementation)
                    None
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to queue LLM request for outcome suggestions");
                    Some(error_response("QUEUE_ERROR", &e.to_string()))
                }
            }
        }
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Unknown => Some(error_response(
            "INVALID_DECISION",
            "Unknown challenge outcome decision type",
        )),
    }
}

async fn handle_narrative_event_decision(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    event_id: String,
    approved: bool,
    selected_outcome: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can make decisions
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse request ID as approval UUID
    let approval_id = match parse_id(&request_id, |u| u, "Invalid request ID") {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let decision = if approved {
        wrldbldr_domain::DmApprovalDecision::Accept
    } else {
        wrldbldr_domain::DmApprovalDecision::Reject {
            feedback: "Narrative event rejected by DM".to_string(),
        }
    };

    // Get the approval data for PC context
    let approval_data = state
        .app
        .queue
        .get_approval_request(approval_id)
        .await
        .ok()
        .flatten();

    match state
        .app
        .use_cases
        .approval
        .approve_suggestion
        .execute(approval_id, decision)
        .await
    {
        Ok(_) => {
            if approved {
                if let Some(world_id) = conn_info.world_id {
                    // Parse the event ID to fetch the narrative event
                    let narrative_event_id = match parse_id(
                        &event_id,
                        NarrativeEventId::from_uuid,
                        "Invalid event ID",
                    ) {
                        Ok(id) => id,
                        Err(e) => return Some(e),
                    };

                    // Fetch the narrative event
                    let event = match state
                        .app
                        .entities
                        .narrative
                        .get_event(narrative_event_id)
                        .await
                    {
                        Ok(Some(e)) => e,
                        Ok(None) => {
                            tracing::warn!(event_id = %event_id, "Narrative event not found");
                            // Still broadcast the trigger, just without effect execution
                            let msg = ServerMessage::NarrativeEventTriggered {
                                event_id: event_id.clone(),
                                event_name: String::new(),
                                outcome_description: String::new(),
                                scene_direction: String::new(),
                            };
                            state.connections.broadcast_to_world(world_id, msg).await;
                            return None;
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to fetch narrative event");
                            return Some(error_response("FETCH_ERROR", &e.to_string()));
                        }
                    };

                    // Find the selected outcome
                    let outcome_name = selected_outcome
                        .or_else(|| {
                            approval_data
                                .as_ref()
                                .and_then(|d| d.narrative_event_suggestion.as_ref())
                                .and_then(|s| s.suggested_outcome.clone())
                        })
                        .or_else(|| event.default_outcome.clone())
                        .unwrap_or_else(|| {
                            event
                                .outcomes
                                .first()
                                .map(|o| o.name.clone())
                                .unwrap_or_default()
                        });

                    let outcome = event.outcomes.iter().find(|o| o.name == outcome_name);

                    // Execute effects if we have an outcome with effects
                    if let Some(outcome) = outcome {
                        if !outcome.effects.is_empty() {
                            // Build execution context
                            let pc_id = approval_data.as_ref().and_then(|d| d.pc_id);

                            if let Some(pc_id) = pc_id {
                                let context = EffectExecutionContext {
                                    pc_id,
                                    world_id,
                                    current_scene_id: approval_data
                                        .as_ref()
                                        .and_then(|d| d.scene_id),
                                };

                                let summary = state
                                    .app
                                    .use_cases
                                    .narrative
                                    .execute_effects
                                    .execute(
                                        narrative_event_id,
                                        outcome_name.clone(),
                                        &outcome.effects,
                                        &context,
                                    )
                                    .await;

                                tracing::info!(
                                    event_id = %event_id,
                                    outcome = %outcome_name,
                                    success_count = summary.success_count,
                                    failure_count = summary.failure_count,
                                    "Executed narrative event effects"
                                );
                            } else {
                                tracing::warn!(
                                    event_id = %event_id,
                                    "No PC context for effect execution, skipping effects"
                                );
                            }
                        }
                    }

                    // Broadcast that the narrative event was triggered
                    let msg = ServerMessage::NarrativeEventTriggered {
                        event_id,
                        event_name: event.name.clone(),
                        outcome_description: outcome
                            .map(|o| o.description.clone())
                            .unwrap_or_default(),
                        scene_direction: event.scene_direction.clone(),
                    };
                    state.connections.broadcast_to_world(world_id, msg).await;
                }
            }
            None
        }
        Err(e) => {
            tracing::error!(error = %e, "Narrative event decision failed");
            Some(error_response("APPROVAL_ERROR", &e.to_string()))
        }
    }
}

// =============================================================================
// DM Action Handlers
// =============================================================================

async fn handle_directorial_update(
    state: &WsState,
    connection_id: Uuid,
    context: wrldbldr_protocol::DirectorialContext,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can update directorial context
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    // Log the directorial context update with all fields
    tracing::info!(
        world_id = %world_id,
        connection_id = %connection_id,
        scene_notes = %context.scene_notes,
        tone = %context.tone,
        npc_motivation_count = context.npc_motivations.len(),
        forbidden_topic_count = context.forbidden_topics.len(),
        "Directorial context stored"
    );

    // Log detailed NPC motivations at debug level
    for motivation in &context.npc_motivations {
        tracing::debug!(
            world_id = %world_id,
            character_id = %motivation.character_id,
            emotional_guidance = %motivation.emotional_guidance,
            immediate_goal = %motivation.immediate_goal,
            has_secret_agenda = motivation.secret_agenda.is_some(),
            "NPC motivation in directorial context"
        );
    }

    // Log forbidden topics at debug level
    if !context.forbidden_topics.is_empty() {
        tracing::debug!(
            world_id = %world_id,
            forbidden_topics = ?context.forbidden_topics,
            "Forbidden topics in directorial context"
        );
    }

    // Store directorial context in per-world cache for LLM prompts
    // This is session-scoped storage. For persistent storage, the context
    // would need to be saved to the Scene entity or a dedicated table.
    state.connections.set_directorial_context(world_id, context);

    tracing::info!(
        world_id = %world_id,
        "Directorial context stored for world"
    );

    None
}

async fn handle_trigger_approach_event(
    state: &WsState,
    connection_id: Uuid,
    npc_id: String,
    target_pc_id: String,
    description: String,
    reveal: bool,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can trigger approach events
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse target PC ID
    let pc_uuid = match parse_pc_id(&target_pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get NPC details
    let npc_uuid = match parse_character_id(&npc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let (npc_name, npc_sprite) = match state.app.entities.character.get(npc_uuid).await {
        Ok(Some(c)) => (c.name.clone(), c.sprite_asset.clone()),
        Ok(None) => (String::new(), None),
        Err(_) => (String::new(), None),
    };

    // Send approach event to target PC
    let msg = ServerMessage::ApproachEvent {
        npc_id,
        npc_name: if reveal {
            npc_name
        } else {
            "Unknown Figure".to_string()
        },
        npc_sprite: if reveal { npc_sprite } else { None },
        description,
        reveal,
    };

    state.connections.send_to_pc(pc_uuid, msg).await;
    None
}

async fn handle_trigger_location_event(
    state: &WsState,
    connection_id: Uuid,
    region_id: String,
    description: String,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can trigger location events
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Broadcast location event to all in the world
    if let Some(world_id) = conn_info.world_id {
        let msg = ServerMessage::LocationEvent {
            region_id,
            description,
        };
        state.connections.broadcast_to_world(world_id, msg).await;
    }

    None
}

async fn handle_share_npc_location(
    state: &WsState,
    connection_id: Uuid,
    pc_id: String,
    npc_id: String,
    location_id: String,
    region_id: String,
    notes: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can share NPC locations
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    // Parse PC ID
    let pc_uuid = match parse_pc_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get NPC and region names
    let npc_uuid = match parse_character_id(&npc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let location_uuid = match parse_location_id(&location_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let region_uuid = match parse_region_id(&region_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let npc_name = match state.app.entities.character.get(npc_uuid).await {
        Ok(Some(c)) => c.name,
        _ => "Unknown".to_string(),
    };

    let region_name = match state.app.entities.location.get_region(region_uuid).await {
        Ok(Some(r)) => r.name,
        _ => "Unknown".to_string(),
    };

    // Create and save the "heard about" observation
    let now = chrono::Utc::now();
    let observation = wrldbldr_domain::NpcObservation::heard_about(
        pc_uuid,
        npc_uuid,
        location_uuid,
        region_uuid,
        now, // game_time - using real time for now
        notes.clone(),
        now, // created_at
    );

    if let Err(e) = state
        .app
        .entities
        .observation
        .save_observation(&observation)
        .await
    {
        tracing::error!(error = %e, "Failed to save NPC observation");
        // Continue anyway - sending the message is still useful
    }

    // Send to target PC
    let msg = ServerMessage::NpcLocationShared {
        npc_id,
        npc_name,
        region_name,
        notes,
    };

    state.connections.send_to_pc(pc_uuid, msg).await;
    None
}

// =============================================================================
// Player Action Handler
// =============================================================================

async fn handle_player_action(
    state: &WsState,
    connection_id: Uuid,
    action_type: String,
    target: Option<String>,
    dialogue: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must join a world first")),
    };

    let pc_id = match conn_info.pc_id {
        Some(id) => id,
        None => return Some(error_response("NO_PC", "Must have a PC to perform actions")),
    };

    // Generate action ID
    let action_id = Uuid::new_v4().to_string();

    tracing::info!(
        connection_id = %connection_id,
        action_id = %action_id,
        action_type = %action_type,
        target = ?target,
        "Player action received"
    );

    // Acknowledge the action
    let ack = ServerMessage::ActionReceived {
        action_id: action_id.clone(),
        player_id: conn_info.user_id.clone(),
        action_type: action_type.clone(),
    };

    // Handle "talk" actions via conversation use case
    if action_type == "talk" {
        if let (Some(target_str), Some(dialogue_text)) = (target.as_ref(), dialogue.as_ref()) {
            // Parse target as NPC ID
            let npc_id = match parse_character_id(target_str) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };

            // Start conversation - validates NPC is in region and queues for LLM
            match state
                .app
                .use_cases
                .conversation
                .start
                .execute(
                    world_id,
                    pc_id,
                    npc_id,
                    conn_info.user_id.clone(),
                    dialogue_text.clone(),
                )
                .await
            {
                Ok(result) => {
                    tracing::info!(
                        conversation_id = %result.conversation_id,
                        action_queue_id = %result.action_queue_id,
                        npc = %result.npc_name,
                        disposition = ?result.npc_disposition,
                        "Conversation started, queued for LLM processing"
                    );
                    // Action is queued - actual NPC response comes later via approval flow
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to start conversation");
                    return Some(error_response("CONVERSATION_ERROR", &e.to_string()));
                }
            }
        } else {
            return Some(error_response(
                "MISSING_PARAMS",
                "Talk action requires target NPC ID and dialogue",
            ));
        }
    }

    // Notify DMs that action is queued
    let queue_msg = ServerMessage::ActionQueued {
        action_id,
        player_name: conn_info.user_id,
        action_type,
        queue_depth: 1, // Would need actual queue depth
    };
    state
        .connections
        .broadcast_to_dms(world_id, queue_msg)
        .await;

    Some(ack)
}

// =============================================================================
// Time Control Handlers
// =============================================================================

async fn handle_set_game_time(
    state: &WsState,
    connection_id: Uuid,
    world_id: String,
    day: u32,
    hour: u8,
    notify_players: bool,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let world_id_typed = match parse_world_id(&world_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let mut world = match state.app.entities.world.get(world_id_typed).await {
        Ok(Some(w)) => w,
        Ok(None) => return Some(error_response("NOT_FOUND", "World not found")),
        Err(e) => return Some(error_response("DATABASE_ERROR", &e.to_string())),
    };

    let previous_time = world.game_time.clone();
    world.game_time.set_day_and_hour(day, hour as u32);
    world.updated_at = chrono::Utc::now();

    if let Err(e) = state.app.entities.world.save(&world).await {
        return Some(error_response("DATABASE_ERROR", &e.to_string()));
    }

    if notify_players {
        let reason = wrldbldr_domain::TimeAdvanceReason::DmSetTime;
        let advance_data = crate::use_cases::time::build_time_advance_data(
            &previous_time,
            &world.game_time,
            0,
            &reason,
        );
        let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
        state
            .connections
            .broadcast_to_world(world_id_typed, update_msg)
            .await;
    }

    tracing::info!(world_id = %world_id_typed, day = day, hour = hour, "Game time set");
    None
}

async fn handle_skip_to_period(
    state: &WsState,
    connection_id: Uuid,
    world_id: String,
    period: String,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let world_id_typed = match parse_world_id(&world_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let target_period = match period.to_lowercase().as_str() {
        "morning" => wrldbldr_domain::TimeOfDay::Morning,
        "afternoon" => wrldbldr_domain::TimeOfDay::Afternoon,
        "evening" => wrldbldr_domain::TimeOfDay::Evening,
        "night" => wrldbldr_domain::TimeOfDay::Night,
        _ => {
            return Some(error_response(
                "INVALID_PERIOD",
                "Use: morning, afternoon, evening, night",
            ))
        }
    };

    let mut world = match state.app.entities.world.get(world_id_typed).await {
        Ok(Some(w)) => w,
        Ok(None) => return Some(error_response("NOT_FOUND", "World not found")),
        Err(e) => return Some(error_response("DATABASE_ERROR", &e.to_string())),
    };

    let previous_time = world.game_time.clone();
    let minutes_until = world.game_time.minutes_until_period(target_period);
    world.game_time.skip_to_period(target_period);
    world.updated_at = chrono::Utc::now();

    if let Err(e) = state.app.entities.world.save(&world).await {
        return Some(error_response("DATABASE_ERROR", &e.to_string()));
    }

    let reason = wrldbldr_domain::TimeAdvanceReason::DmSkipToPeriod {
        period: target_period,
    };
    let advance_data = crate::use_cases::time::build_time_advance_data(
        &previous_time,
        &world.game_time,
        minutes_until,
        &reason,
    );
    let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
    state
        .connections
        .broadcast_to_world(world_id_typed, update_msg)
        .await;

    tracing::info!(world_id = %world_id_typed, period = %target_period, "Skipped to period");
    None
}

async fn handle_pause_game_time(
    state: &WsState,
    connection_id: Uuid,
    world_id: String,
    paused: bool,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let world_id_typed = match parse_world_id(&world_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let mut world = match state.app.entities.world.get(world_id_typed).await {
        Ok(Some(w)) => w,
        Ok(None) => return Some(error_response("NOT_FOUND", "World not found")),
        Err(e) => return Some(error_response("DATABASE_ERROR", &e.to_string())),
    };

    world.game_time.set_paused(paused);
    world.updated_at = chrono::Utc::now();

    if let Err(e) = state.app.entities.world.save(&world).await {
        return Some(error_response("DATABASE_ERROR", &e.to_string()));
    }

    let update_msg = ServerMessage::GameTimePaused {
        world_id: world_id_typed.to_string(),
        paused,
    };
    state
        .connections
        .broadcast_to_world(world_id_typed, update_msg)
        .await;

    tracing::info!(world_id = %world_id_typed, paused = paused, "Game time pause state changed");
    None
}

async fn handle_set_time_mode(
    state: &WsState,
    connection_id: Uuid,
    world_id: String,
    mode: wrldbldr_protocol::types::TimeMode,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let world_id_typed = match parse_world_id(&world_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let mut world = match state.app.entities.world.get(world_id_typed).await {
        Ok(Some(w)) => w,
        Ok(None) => return Some(error_response("NOT_FOUND", "World not found")),
        Err(e) => return Some(error_response("DATABASE_ERROR", &e.to_string())),
    };

    let (domain_mode, broadcast_mode) = match mode {
        wrldbldr_protocol::types::TimeMode::Manual => (
            wrldbldr_domain::TimeMode::Manual,
            wrldbldr_protocol::types::TimeMode::Manual,
        ),
        wrldbldr_protocol::types::TimeMode::Suggested => (
            wrldbldr_domain::TimeMode::Suggested,
            wrldbldr_protocol::types::TimeMode::Suggested,
        ),
        // Auto is intentionally treated as Suggested to ensure time only advances
        // via explicit DM action/approval (never automatically).
        wrldbldr_protocol::types::TimeMode::Auto => (
            wrldbldr_domain::TimeMode::Suggested,
            wrldbldr_protocol::types::TimeMode::Suggested,
        ),
    };

    world.time_config.mode = domain_mode;
    world.updated_at = chrono::Utc::now();

    if let Err(e) = state.app.entities.world.save(&world).await {
        return Some(error_response("DATABASE_ERROR", &e.to_string()));
    }

    let update_msg = ServerMessage::TimeModeChanged {
        world_id: world_id_typed.to_string(),
        mode: broadcast_mode,
    };
    state
        .connections
        .broadcast_to_world(world_id_typed, update_msg)
        .await;

    tracing::info!(world_id = %world_id_typed, mode = ?mode, "Time mode changed");
    None
}

async fn handle_set_time_costs(
    state: &WsState,
    connection_id: Uuid,
    world_id: String,
    costs: wrldbldr_protocol::types::TimeCostConfig,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let world_id_typed = match parse_world_id(&world_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    let mut world = match state.app.entities.world.get(world_id_typed).await {
        Ok(Some(w)) => w,
        Ok(None) => return Some(error_response("NOT_FOUND", "World not found")),
        Err(e) => return Some(error_response("DATABASE_ERROR", &e.to_string())),
    };

    world.time_config.time_costs = wrldbldr_domain::TimeCostConfig {
        travel_location: costs.travel_location,
        travel_region: costs.travel_region,
        rest_short: costs.rest_short,
        rest_long: costs.rest_long,
        conversation: costs.conversation,
        challenge: costs.challenge,
        scene_transition: costs.scene_transition,
    };
    world.updated_at = chrono::Utc::now();

    if let Err(e) = state.app.entities.world.save(&world).await {
        return Some(error_response("DATABASE_ERROR", &e.to_string()));
    }

    tracing::info!(world_id = %world_id_typed, "Time costs updated");
    None
}

async fn handle_respond_to_time_suggestion(
    state: &WsState,
    connection_id: Uuid,
    suggestion_id: String,
    decision: wrldbldr_protocol::types::TimeSuggestionDecision,
) -> Option<ServerMessage> {
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }

    let world_id = match conn_info.world_id {
        Some(id) => id,
        None => return Some(error_response("NOT_IN_WORLD", "Must be in a world")),
    };

    let suggestion_uuid = match Uuid::parse_str(&suggestion_id) {
        Ok(id) => id,
        Err(_) => {
            return Some(error_response(
                "INVALID_SUGGESTION_ID",
                "Invalid time suggestion ID",
            ));
        }
    };

    let suggestion = {
        let mut guard = state.pending_time_suggestions.write().await;
        match guard.remove(&suggestion_uuid) {
            Some(s) => s,
            None => {
                return Some(error_response(
                    "TIME_SUGGESTION_NOT_FOUND",
                    "Time suggestion not found (maybe already resolved)",
                ));
            }
        }
    };

    if suggestion.world_id != world_id {
        tracing::warn!(
            world_id = %world_id,
            suggestion_world_id = %suggestion.world_id,
            suggestion_id = %suggestion_id,
            "Time suggestion world mismatch"
        );
        return Some(error_response(
            "TIME_SUGGESTION_WORLD_MISMATCH",
            "Time suggestion does not belong to this world",
        ));
    }

    let minutes_to_advance: u32 = match decision {
        wrldbldr_protocol::types::TimeSuggestionDecision::Skip
        | wrldbldr_protocol::types::TimeSuggestionDecision::Unknown => 0,
        wrldbldr_protocol::types::TimeSuggestionDecision::Approve => suggestion.suggested_minutes,
        wrldbldr_protocol::types::TimeSuggestionDecision::Modify { minutes } => minutes,
    };

    tracing::info!(
        world_id = %world_id,
        suggestion_id = %suggestion_id,
        minutes = minutes_to_advance,
        action_type = %suggestion.action_type,
        "Time suggestion resolved"
    );

    if minutes_to_advance == 0 {
        return None;
    }

    let reason = crate::use_cases::time::time_advance_reason_for_action(
        &suggestion.action_type,
        &suggestion.action_description,
    );

    let result = match state
        .app
        .entities
        .world
        .advance_time(world_id, minutes_to_advance, reason.clone())
        .await
    {
        Ok(r) => r,
        Err(e) => return Some(error_response("DATABASE_ERROR", &e.to_string())),
    };

    let advance_data = crate::use_cases::time::build_time_advance_data(
        &result.previous_time,
        &result.new_time,
        minutes_to_advance,
        &reason,
    );
    let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
    state
        .connections
        .broadcast_to_world(world_id, update_msg)
        .await;

    None
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

/// Build navigation data for a region, including connected regions and exits.
async fn build_navigation_data(
    location_entity: &crate::entities::Location,
    region_id: RegionId,
) -> wrldbldr_protocol::NavigationData {
    // Get region connections
    let connections = location_entity
        .get_connections(region_id)
        .await
        .ok()
        .unwrap_or_default();

    // Build connected regions with names
    let mut connected_regions = Vec::new();
    for c in connections {
        let region_name = location_entity
            .get_region(c.to_region)
            .await
            .ok()
            .flatten()
            .map(|r| r.name)
            .unwrap_or_else(|| "Unknown".to_string());

        connected_regions.push(wrldbldr_protocol::NavigationTarget {
            region_id: c.to_region.to_string(),
            name: region_name,
            is_locked: c.is_locked,
            lock_description: c.lock_description,
        });
    }

    // Get location exits
    let exits = location_entity
        .get_exits(region_id)
        .await
        .ok()
        .unwrap_or_default()
        .into_iter()
        .map(|e| wrldbldr_protocol::NavigationExit {
            location_id: e.location_id.to_string(),
            location_name: e.location_name,
            arrival_region_id: e.arrival_region_id.to_string(),
            description: e.description,
        })
        .collect();

    wrldbldr_protocol::NavigationData {
        connected_regions,
        exits,
    }
}

/// Build region items data (items that can be picked up in this region).
async fn build_region_items(
    inventory_ops: &crate::use_cases::inventory::InventoryOps,
    region_id: RegionId,
) -> Vec<wrldbldr_protocol::RegionItemData> {
    match inventory_ops.list_in_region(region_id).await {
        Ok(items) => items
            .into_iter()
            .map(|item| wrldbldr_protocol::RegionItemData {
                id: item.id.to_string(),
                name: item.name,
                description: item.description,
                item_type: item.item_type,
            })
            .collect(),
        Err(e) => {
            tracing::warn!(error = %e, region_id = %region_id, "Failed to fetch region items");
            vec![]
        }
    }
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

    use crate::app::{App, Entities, UseCases};
    use crate::infrastructure::ports::{
        ClockPort, ImageGenError, ImageGenPort, LlmError, LlmPort, QueueError, QueueItem,
        QueuePort, RandomPort,
    };
    use crate::infrastructure::ports::{
        MockActRepo, MockAssetRepo, MockChallengeRepo, MockCharacterRepo, MockFlagRepo,
        MockGoalRepo, MockInteractionRepo, MockItemRepo, MockLocationRepo, MockLocationStateRepo,
        MockLoreRepo, MockNarrativeRepo, MockObservationRepo, MockPlayerCharacterRepo,
        MockRegionStateRepo, MockSceneRepo, MockSettingsRepo, MockSkillRepo, MockStagingRepo,
        MockWorldRepo,
    };

    struct TestAppRepos {
        world_repo: MockWorldRepo,
        character_repo: MockCharacterRepo,
        player_character_repo: MockPlayerCharacterRepo,
        location_repo: MockLocationRepo,
        scene_repo: MockSceneRepo,
        act_repo: MockActRepo,
        skill_repo: MockSkillRepo,
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
                skill_repo: MockSkillRepo::new(),
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
        approvals: StdHashMap<Uuid, wrldbldr_domain::ApprovalRequestData>,
        completed: Vec<Uuid>,
        failed: Vec<(Uuid, String)>,
    }

    #[derive(Clone, Default)]
    struct RecordingApprovalQueue {
        state: Arc<Mutex<RecordingApprovalQueueState>>,
    }

    impl RecordingApprovalQueue {
        fn insert_approval(&self, id: Uuid, data: wrldbldr_domain::ApprovalRequestData) {
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
        let skill_repo = Arc::new(repos.skill_repo);
        let interaction_repo = Arc::new(repos.interaction_repo);
        let settings_repo = Arc::new(repos.settings_repo);
        let challenge_repo = Arc::new(repos.challenge_repo);
        let narrative_repo = Arc::new(repos.narrative_repo);
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
        let character = Arc::new(crate::entities::Character::new(character_repo.clone()));
        let player_character = Arc::new(crate::entities::PlayerCharacter::new(
            player_character_repo.clone(),
        ));
        let location = Arc::new(crate::entities::Location::new(location_repo.clone()));
        let scene = Arc::new(crate::entities::Scene::new(scene_repo.clone()));
        let act = Arc::new(crate::entities::Act::new(act_repo.clone()));
        let skill = Arc::new(crate::entities::Skill::new(skill_repo.clone()));
        let interaction = Arc::new(crate::entities::Interaction::new(interaction_repo.clone()));
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
        let goal = Arc::new(crate::entities::Goal::new(goal_repo.clone()));
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
            act: act.clone(),
            skill: skill.clone(),
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

        let actantial = crate::use_cases::ActantialUseCases::new(
            crate::use_cases::actantial::GoalOps::new(goal.clone()),
            crate::use_cases::actantial::WantOps::new(character.clone(), clock.clone()),
            crate::use_cases::actantial::ActantialContextOps::new(character.clone()),
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
            Arc::new(crate::use_cases::challenge::TriggerChallengePrompt::new(
                challenge.clone(),
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

        let time_uc = crate::use_cases::TimeUseCases::new(
            suggest_time,
            Arc::new(crate::use_cases::time::TimeControl::new(world.clone())),
        );

        let visual_state_uc = crate::use_cases::VisualStateUseCases::new(Arc::new(
            crate::use_cases::visual_state::ResolveVisualState::new(
                location_state.clone(),
                region_state.clone(),
                flag.clone(),
            ),
        ));

        let staging_uc = crate::use_cases::StagingUseCases::new(
            Arc::new(crate::use_cases::staging::RequestStagingApproval::new(
                character.clone(),
                staging.clone(),
                location.clone(),
                world.clone(),
                flag.clone(),
                visual_state_uc.resolve.clone(),
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
                location_state.clone(),
                region_state.clone(),
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
        );

        let inventory_uc = crate::use_cases::InventoryUseCases::new(Arc::new(
            crate::use_cases::inventory::InventoryOps::new(inventory.clone()),
        ));

        let story_events_uc = crate::use_cases::StoryEventUseCases::new(Arc::new(
            crate::use_cases::story_events::StoryEventOps::new(narrative.clone()),
        ));

        let lore_uc = crate::use_cases::LoreUseCases::new(Arc::new(
            crate::use_cases::lore::LoreOps::new(lore.clone()),
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
            crate::use_cases::management::SkillCrud::new(skill.clone()),
        );

        let settings = crate::use_cases::SettingsUseCases::new(Arc::new(
            crate::use_cases::settings::SettingsOps::new(settings_repo),
        ));

        let session = crate::use_cases::SessionUseCases::new(Arc::new(
            crate::use_cases::session::JoinWorld::new(
                world.clone(),
                location.clone(),
                character.clone(),
                scene.clone(),
                player_character.clone(),
            ),
        ));

        let use_cases = UseCases {
            movement,
            conversation,
            challenge: challenge_uc,
            approval,
            actantial,
            assets: assets_uc,
            world: world_uc,
            queues,
            narrative: narrative_uc,
            time: time_uc,
            visual_state: visual_state_uc,
            management,
            session,
            settings,
            staging: staging_uc,
            npc: npc_uc,
            inventory: inventory_uc,
            story_events: story_events_uc,
            lore: lore_uc,
        };

        Arc::new(App {
            entities,
            use_cases,
            queue,
            llm,
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
        let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
        world.id = world_id;

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
            pending_time_suggestions: tokio::sync::RwLock::new(HashMap::new()),
            pending_staging_requests: tokio::sync::RwLock::new(HashMap::new()),
            generation_read_state: tokio::sync::RwLock::new(HashMap::new()),
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

        {
            let mut guard = ws_state.pending_time_suggestions.write().await;
            guard.insert(suggestion_id, suggestion);
        }

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
        let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
        world.id = world_id;
        world.set_time_mode(TimeMode::Manual, now);

        // Domain fixtures.
        let mut location = wrldbldr_domain::Location::new(
            world_id,
            "Test Location",
            wrldbldr_domain::LocationType::Exterior,
        );
        location.id = location_id;

        let mut region = wrldbldr_domain::Region::new(location_id, "Unstaged Region");
        region.id = region_id;

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("player-1", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = None; // initial spawn; skip connection validation

        let mut visible_npc =
            wrldbldr_domain::Character::new(world_id, "Visible NPC", CampbellArchetype::Hero);
        visible_npc.id = visible_npc_id;
        let mut hidden_npc =
            wrldbldr_domain::Character::new(world_id, "Hidden NPC", CampbellArchetype::Herald);
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
            pending_time_suggestions: tokio::sync::RwLock::new(HashMap::new()),
            pending_staging_requests: tokio::sync::RwLock::new(HashMap::new()),
            generation_read_state: tokio::sync::RwLock::new(HashMap::new()),
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
        let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
        world.id = world_id;

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
            pending_time_suggestions: tokio::sync::RwLock::new(HashMap::new()),
            pending_staging_requests: tokio::sync::RwLock::new(HashMap::new()),
            generation_read_state: tokio::sync::RwLock::new(HashMap::new()),
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
            wrldbldr_domain::ApprovalRequestData {
                world_id,
                source_action_id: Uuid::new_v4(),
                decision_type: wrldbldr_domain::ApprovalDecisionType::NpcResponse,
                urgency: wrldbldr_domain::ApprovalUrgency::Normal,
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
        let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
        world.id = world_id;

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
            pending_time_suggestions: tokio::sync::RwLock::new(HashMap::new()),
            pending_staging_requests: tokio::sync::RwLock::new(HashMap::new()),
            generation_read_state: tokio::sync::RwLock::new(HashMap::new()),
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
            wrldbldr_domain::ApprovalRequestData {
                world_id,
                source_action_id: Uuid::new_v4(),
                decision_type: wrldbldr_domain::ApprovalDecisionType::NpcResponse,
                urgency: wrldbldr_domain::ApprovalUrgency::Normal,
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
        let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
        world.id = world_id;

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
            pending_time_suggestions: tokio::sync::RwLock::new(HashMap::new()),
            pending_staging_requests: tokio::sync::RwLock::new(HashMap::new()),
            generation_read_state: tokio::sync::RwLock::new(HashMap::new()),
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
            wrldbldr_domain::ApprovalRequestData {
                world_id,
                source_action_id: Uuid::new_v4(),
                decision_type: wrldbldr_domain::ApprovalDecisionType::NpcResponse,
                urgency: wrldbldr_domain::ApprovalUrgency::Normal,
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

        let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
        world.id = world_id;
        world.set_time_mode(TimeMode::Manual, now);

        let mut location = wrldbldr_domain::Location::new(
            world_id,
            "Test Location",
            wrldbldr_domain::LocationType::Exterior,
        );
        location.id = location_id;

        let mut region = wrldbldr_domain::Region::new(location_id, "Region");
        region.id = region_id;

        let mut pc =
            wrldbldr_domain::PlayerCharacter::new("player-1", world_id, "PC", location_id, now);
        pc.id = pc_id;
        pc.current_region_id = None;

        let mut npc = wrldbldr_domain::Character::new(world_id, "NPC", CampbellArchetype::Hero);
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
            pending_time_suggestions: tokio::sync::RwLock::new(HashMap::new()),
            pending_staging_requests: tokio::sync::RwLock::new(HashMap::new()),
            generation_read_state: tokio::sync::RwLock::new(HashMap::new()),
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

        let mut world = wrldbldr_domain::World::new("Test World", "desc", now);
        world.id = world_id;

        let mut location = wrldbldr_domain::Location::new(
            world_id,
            "Test Location",
            wrldbldr_domain::LocationType::Exterior,
        );
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
            pending_time_suggestions: tokio::sync::RwLock::new(HashMap::new()),
            pending_staging_requests: tokio::sync::RwLock::new(HashMap::new()),
            generation_read_state: tokio::sync::RwLock::new(HashMap::new()),
        });

        // Seed a pending staging request correlation.
        let request_id = "req-123".to_string();
        {
            let mut guard = ws_state.pending_staging_requests.write().await;
            guard.insert(
                request_id.clone(),
                PendingStagingRequest {
                    region_id,
                    location_id,
                },
            );
        }

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
fn parse_narrative_event_id(id_str: &str) -> Result<NarrativeEventId, ServerMessage> {
    parse_id(
        id_str,
        NarrativeEventId::from_uuid,
        "Invalid narrative event ID format",
    )
}

/// Parse an event chain ID from a string.
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
    Uuid::parse_str(id_str).map_err(|_| ServerMessage::Response {
        request_id: request_id.to_string(),
        result: ResponseResult::error(ErrorCode::BadRequest, error_msg),
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
