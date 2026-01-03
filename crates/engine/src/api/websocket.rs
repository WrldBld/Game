//! WebSocket handling for Player connections.
//!
//! Handles the WebSocket protocol between Engine and Player clients.

use std::sync::Arc;

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

use wrldbldr_domain::{CharacterId, PlayerCharacterId, RegionId, WorldId};
use wrldbldr_protocol::{
    ClientMessage, ErrorCode, ResponseResult, ServerMessage, WorldRole as ProtoWorldRole,
};

use crate::app::App;
use crate::use_cases::movement::EnterRegionError;
use super::connections::{ConnectionManager, WorldRole};

/// Buffer size for per-connection message channel.
const CONNECTION_CHANNEL_BUFFER: usize = 256;

/// Combined state for WebSocket handlers.
pub struct WsState {
    pub app: Arc<App>,
    pub connections: Arc<ConnectionManager>,
}

/// WebSocket upgrade handler - entry point for new connections.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WsState>>,
) -> Response {
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
    state.connections.register(connection_id, user_id.clone(), tx.clone()).await;

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
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(msg) => {
                        if let Some(response) = handle_message(
                            msg,
                            &state,
                            connection_id,
                            tx.clone(),
                        ).await {
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
                }
            }
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
        
        ClientMessage::JoinWorld { world_id, role, pc_id, spectate_pc_id } => {
            handle_join_world(state, connection_id, world_id, role, pc_id, spectate_pc_id).await
        }
        
        ClientMessage::LeaveWorld => {
            state.connections.leave_world(connection_id).await;
            None
        }

        // Movement
        ClientMessage::MoveToRegion { pc_id, region_id } => {
            handle_move_to_region(state, connection_id, pc_id, region_id).await
        }
        
        ClientMessage::ExitToLocation { pc_id, location_id, arrival_region_id } => {
            handle_exit_to_location(state, connection_id, pc_id, location_id, arrival_region_id).await
        }

        // Inventory
        ClientMessage::EquipItem { pc_id, item_id } => {
            handle_inventory_action(state, connection_id, InventoryAction::Equip, &pc_id, &item_id, 1).await
        }
        ClientMessage::UnequipItem { pc_id, item_id } => {
            handle_inventory_action(state, connection_id, InventoryAction::Unequip, &pc_id, &item_id, 1).await
        }
        ClientMessage::DropItem { pc_id, item_id, quantity } => {
            handle_inventory_action(state, connection_id, InventoryAction::Drop, &pc_id, &item_id, quantity).await
        }
        ClientMessage::PickupItem { pc_id, item_id } => {
            handle_inventory_action(state, connection_id, InventoryAction::Pickup, &pc_id, &item_id, 1).await
        }

        // Request/Response pattern (CRUD operations)
        ClientMessage::Request { request_id, payload } => {
            handle_request(state, connection_id, request_id, payload).await
        }

        // Challenge handlers
        ClientMessage::ChallengeRoll { challenge_id, roll } => {
            handle_challenge_roll(state, connection_id, challenge_id, roll).await
        }
        
        ClientMessage::ChallengeRollInput { challenge_id, input_type } => {
            handle_challenge_roll_input(state, connection_id, challenge_id, input_type).await
        }
        
        ClientMessage::TriggerChallenge { challenge_id, target_character_id } => {
            handle_trigger_challenge(state, connection_id, challenge_id, target_character_id).await
        }

        // Staging handlers
        ClientMessage::StagingApprovalResponse { request_id, approved_npcs, ttl_hours, source } => {
            handle_staging_approval(state, connection_id, request_id, approved_npcs, ttl_hours, source).await
        }
        
        ClientMessage::StagingRegenerateRequest { request_id, guidance } => {
            handle_staging_regenerate(state, connection_id, request_id, guidance).await
        }
        
        ClientMessage::PreStageRegion { region_id, npcs, ttl_hours } => {
            handle_pre_stage_region(state, connection_id, region_id, npcs, ttl_hours).await
        }

        // Approval handlers
        ClientMessage::ApprovalDecision { request_id, decision } => {
            handle_approval_decision(state, connection_id, request_id, decision).await
        }
        
        ClientMessage::ChallengeSuggestionDecision { request_id, approved, modified_difficulty } => {
            handle_challenge_suggestion_decision(state, connection_id, request_id, approved, modified_difficulty).await
        }
        
        ClientMessage::NarrativeEventSuggestionDecision { request_id, event_id, approved, selected_outcome } => {
            handle_narrative_event_decision(state, connection_id, request_id, event_id, approved, selected_outcome).await
        }

        // DM action handlers
        ClientMessage::DirectorialUpdate { context } => {
            handle_directorial_update(state, connection_id, context).await
        }
        
        ClientMessage::TriggerApproachEvent { npc_id, target_pc_id, description, reveal } => {
            handle_trigger_approach_event(state, connection_id, npc_id, target_pc_id, description, reveal).await
        }
        
        ClientMessage::TriggerLocationEvent { region_id, description } => {
            handle_trigger_location_event(state, connection_id, region_id, description).await
        }
        
        ClientMessage::ShareNpcLocation { pc_id, npc_id, location_id, region_id, notes } => {
            handle_share_npc_location(state, connection_id, pc_id, npc_id, location_id, region_id, notes).await
        }

        // Player action handler
        ClientMessage::PlayerAction { action_type, target, dialogue } => {
            handle_player_action(state, connection_id, action_type, target, dialogue).await
        }

        // Forward compatibility
        ClientMessage::Unknown => {
            tracing::warn!(connection_id = %connection_id, "Received unknown message type");
            None
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
    
    // Verify world exists
    let world = match state.app.entities.world.get(world_id_typed).await {
        Ok(Some(w)) => w,
        Ok(None) => {
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_protocol::JoinError::WorldNotFound,
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch world");
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_protocol::JoinError::Unknown,
            });
        }
    };
    
    // Convert protocol role to internal role
    let internal_role = match role {
        ProtoWorldRole::Dm => WorldRole::Dm,
        ProtoWorldRole::Player => WorldRole::Player,
        ProtoWorldRole::Spectator | ProtoWorldRole::Unknown => WorldRole::Spectator,
    };
    
    let pc_id_typed = pc_id.map(PlayerCharacterId::from_uuid);
    
    // Join the world
    if let Err(e) = state.connections.join_world(connection_id, world_id_typed, internal_role, pc_id_typed).await {
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
    let connected_users = state.connections.get_world_connections(world_id_typed).await
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
    
    // Build world snapshot (simplified for now)
    let snapshot = serde_json::json!({
        "world": {
            "id": world.id,
            "name": world.name,
            "description": world.description,
        }
    });
    
    Some(ServerMessage::WorldJoined {
        world_id,
        snapshot,
        connected_users,
        your_role: role,
        your_pc: None, // TODO: Fetch PC data if role is Player
    })
}

async fn handle_move_to_region(
    state: &WsState,
    connection_id: Uuid,
    pc_id: String,
    region_id: String,
) -> Option<ServerMessage> {
    // Parse IDs
    let pc_uuid = match Uuid::parse_str(&pc_id) {
        Ok(id) => PlayerCharacterId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid PC ID format")),
    };
    
    let region_uuid = match Uuid::parse_str(&region_id) {
        Ok(id) => RegionId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid region ID format")),
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
    match state.app.use_cases.movement.enter_region.execute(pc_uuid, region_uuid).await {
        Ok(result) => {
            // Get location name for the response
            let location_name = state.app.entities.location
                .get(result.region.location_id)
                .await
                .ok()
                .flatten()
                .map(|l| l.name.clone())
                .unwrap_or_else(|| "Unknown Location".to_string());
            
            // Build SceneChanged response
            let region_data = wrldbldr_protocol::RegionData {
                id: result.region.id.to_string(),
                name: result.region.name.clone(),
                location_id: result.region.location_id.to_string(),
                location_name,
                backdrop_asset: result.region.backdrop_asset.clone(),
                atmosphere: result.region.atmosphere.clone(),
                map_asset: None,
            };
            
            let npcs_present: Vec<wrldbldr_protocol::NpcPresenceData> = result.npcs
                .into_iter()
                .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
                .map(|npc| wrldbldr_protocol::NpcPresenceData {
                    character_id: npc.character_id.to_string(),
                    name: npc.name,
                    sprite_asset: npc.sprite_asset,
                    portrait_asset: npc.portrait_asset,
                })
                .collect();
            
            // Get navigation data - we need to look up region names
            let connections = state.app.entities.location
                .get_connections(region_uuid)
                .await
                .ok()
                .unwrap_or_default();
            
            let mut connected_regions = Vec::new();
            for c in connections {
                // Look up the target region name
                let region_name = state.app.entities.location
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
            
            // Get exits (connections to other locations)
            let exits = state.app.entities.location
                .get_exits(region_uuid)
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
            
            let navigation = wrldbldr_protocol::NavigationData {
                connected_regions,
                exits,
            };
            
            Some(ServerMessage::SceneChanged {
                pc_id: pc_id.clone(),
                region: region_data,
                npcs_present,
                navigation,
                region_items: vec![], // TODO: Implement region items
            })
        }
        Err(e) => {
            tracing::error!(error = %e, "Movement failed");
            // Check for specific error types
            match e {
                EnterRegionError::MovementBlocked(reason) => {
                    Some(ServerMessage::MovementBlocked {
                        pc_id: pc_id.clone(),
                        reason,
                    })
                }
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
    let pc_uuid = match Uuid::parse_str(&pc_id) {
        Ok(id) => PlayerCharacterId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid PC ID format")),
    };
    
    let location_uuid = match Uuid::parse_str(&location_id) {
        Ok(id) => wrldbldr_domain::LocationId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid location ID format")),
    };
    
    let arrival_uuid = match arrival_region_id {
        Some(ref id) => match Uuid::parse_str(id) {
            Ok(uuid) => Some(RegionId::from_uuid(uuid)),
            Err(_) => return Some(error_response("INVALID_ID", "Invalid arrival region ID format")),
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
    match state.app.use_cases.movement.exit_location.execute(pc_uuid, location_uuid, arrival_uuid).await {
        Ok(result) => {
            // Get location name for the response
            let location_name = state.app.entities.location
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
            
            let npcs_present: Vec<wrldbldr_protocol::NpcPresenceData> = result.npcs
                .into_iter()
                .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
                .map(|npc| wrldbldr_protocol::NpcPresenceData {
                    character_id: npc.character_id.to_string(),
                    name: npc.name,
                    sprite_asset: npc.sprite_asset,
                    portrait_asset: npc.portrait_asset,
                })
                .collect();
            
            // Get navigation data for new region
            let connections = state.app.entities.location
                .get_connections(result.region.id)
                .await
                .ok()
                .unwrap_or_default();
            
            let mut connected_regions = Vec::new();
            for c in connections {
                let region_name = state.app.entities.location
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
            
            let exits = state.app.entities.location
                .get_exits(result.region.id)
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
            
            let navigation = wrldbldr_protocol::NavigationData {
                connected_regions,
                exits,
            };
            
            Some(ServerMessage::SceneChanged {
                pc_id: pc_id.clone(),
                region: region_data,
                npcs_present,
                navigation,
                region_items: vec![],
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
    payload: wrldbldr_protocol::RequestPayload,
) -> Option<ServerMessage> {
    use wrldbldr_protocol::RequestPayload;
    
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
    
    let result = match payload {
        // World queries
        RequestPayload::ListWorlds => {
            match state.app.entities.world.list_all().await {
                Ok(worlds) => {
                    let data: Vec<serde_json::Value> = worlds
                        .into_iter()
                        .map(|w| serde_json::json!({
                            "id": w.id,
                            "name": w.name,
                            "description": w.description,
                        }))
                        .collect();
                    ResponseResult::success(data)
                }
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        RequestPayload::GetWorld { world_id: req_world_id } => {
            let uuid = match Uuid::parse_str(&req_world_id) {
                Ok(id) => id,
                Err(_) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world ID"),
                    });
                }
            };
            
            match state.app.entities.world.get(WorldId::from_uuid(uuid)).await {
                Ok(Some(world)) => ResponseResult::success(serde_json::json!({
                    "id": world.id,
                    "name": world.name,
                    "description": world.description,
                })),
                Ok(None) => ResponseResult::error(ErrorCode::NotFound, "World not found"),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        // Character queries
        RequestPayload::ListCharacters { world_id: req_world_id } => {
            let uuid = match Uuid::parse_str(&req_world_id) {
                Ok(id) => id,
                Err(_) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world ID"),
                    });
                }
            };
            
            match state.app.entities.character.list_in_world(WorldId::from_uuid(uuid)).await {
                Ok(chars) => {
                    let data: Vec<serde_json::Value> = chars
                        .into_iter()
                        .map(|c| serde_json::json!({
                            "id": c.id,
                            "name": c.name,
                            "description": c.description,
                            "is_active": c.is_active,
                        }))
                        .collect();
                    ResponseResult::success(data)
                }
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        // Location queries  
        RequestPayload::ListLocations { world_id: req_world_id } => {
            let uuid = match Uuid::parse_str(&req_world_id) {
                Ok(id) => id,
                Err(_) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world ID"),
                    });
                }
            };
            
            match state.app.entities.location.list_in_world(WorldId::from_uuid(uuid)).await {
                Ok(locations) => {
                    let data: Vec<serde_json::Value> = locations
                        .into_iter()
                        .map(|l| serde_json::json!({
                            "id": l.id,
                            "name": l.name,
                            "description": l.description,
                        }))
                        .collect();
                    ResponseResult::success(data)
                }
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        // Default - not implemented
        _ => ResponseResult::error(ErrorCode::BadRequest, "This request type is not yet implemented"),
    };
    
    Some(ServerMessage::Response { request_id, result })
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
    let pc_uuid = match Uuid::parse_str(pc_id) {
        Ok(id) => PlayerCharacterId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid PC ID format")),
    };
    
    let item_uuid = match Uuid::parse_str(item_id) {
        Ok(id) => wrldbldr_domain::ItemId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid item ID format")),
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
            state.app.entities.inventory.equip_item(pc_uuid, item_uuid).await
        }
        InventoryAction::Unequip => {
            state.app.entities.inventory.unequip_item(pc_uuid, item_uuid).await
        }
        InventoryAction::Drop => {
            state.app.entities.inventory.drop_item(pc_uuid, item_uuid, quantity).await
        }
        InventoryAction::Pickup => {
            state.app.entities.inventory.pickup_item(pc_uuid, item_uuid).await
        }
    };
    
    match result {
        Ok(action_result) => {
            match action {
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
            }
        }
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
    let challenge_uuid = match Uuid::parse_str(&challenge_id) {
        Ok(id) => wrldbldr_domain::ChallengeId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid challenge ID format")),
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
    match state.app.use_cases.challenge.roll.execute(
        world_id,
        challenge_uuid,
        pc_id,
        Some(roll),
        0, // No modifier for legacy roll
    ).await {
        Ok(result) => {
            Some(ServerMessage::ChallengeRollSubmitted {
                challenge_id,
                challenge_name: String::new(), // We don't have access to name from result
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
    let challenge_uuid = match Uuid::parse_str(&challenge_id) {
        Ok(id) => wrldbldr_domain::ChallengeId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid challenge ID format")),
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
    match state.app.use_cases.challenge.roll.execute(
        world_id,
        challenge_uuid,
        pc_id,
        client_roll,
        modifier,
    ).await {
        Ok(result) => {
            Some(ServerMessage::ChallengeRollSubmitted {
                challenge_id,
                challenge_name: String::new(),
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
    let challenge_uuid = match Uuid::parse_str(&challenge_id) {
        Ok(id) => wrldbldr_domain::ChallengeId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid challenge ID format")),
    };
    
    // Parse target character ID (could be PC or NPC, but we use PlayerCharacterId for PCs)
    let target_uuid = match Uuid::parse_str(&target_character_id) {
        Ok(id) => id,
        Err(_) => return Some(error_response("INVALID_ID", "Invalid target character ID format")),
    };
    
    // Get connection info
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };
    
    // Only DMs can trigger challenges manually
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can trigger challenges"));
    }
    
    // Get the challenge to send a prompt to the target player
    let challenge = match state.app.entities.challenge.get(challenge_uuid).await {
        Ok(Some(c)) => c,
        Ok(None) => return Some(error_response("NOT_FOUND", "Challenge not found")),
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch challenge");
            return Some(error_response("INTERNAL_ERROR", "Failed to fetch challenge"));
        }
    };
    
    // Get target PC's connection to send them the challenge prompt
    // For now, we broadcast to the world - the client filters by pc_id
    if let Some(world_id) = conn_info.world_id {
        // Build and send challenge prompt
        let difficulty_display = match &challenge.difficulty {
            wrldbldr_domain::Difficulty::DC(dc) => format!("DC {}", dc),
            wrldbldr_domain::Difficulty::Percentage(pct) => format!("{}%", pct),
            wrldbldr_domain::Difficulty::Opposed => "Opposed".to_string(),
            wrldbldr_domain::Difficulty::Descriptor(desc) => format!("{:?}", desc),
            wrldbldr_domain::Difficulty::Custom(custom) => custom.clone(),
        };
        
        let prompt = ServerMessage::ChallengePrompt {
            challenge_id: challenge_id.clone(),
            challenge_name: challenge.name.clone(),
            skill_name: String::new(), // Would need to fetch from relationship
            difficulty_display,
            description: challenge.description.clone(),
            character_modifier: 0, // Would need to calculate from PC stats
            suggested_dice: Some("1d20".to_string()),
            rule_system_hint: None,
        };
        
        // Broadcast to world connections (target player will see it)
        state.connections.broadcast_to_world(world_id, prompt).await;
    }
    
    // Confirm to DM that challenge was triggered
    Some(ServerMessage::AdHocChallengeCreated {
        challenge_id,
        challenge_name: challenge.name,
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

async fn handle_staging_approval(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    approved_npcs: Vec<wrldbldr_protocol::ApprovedNpcInfo>,
    _ttl_hours: i32,
    _source: String,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can approve staging
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };
    
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can approve staging"));
    }
    
    // Parse request_id as region_id (the request_id is typically the region being staged)
    let region_id = match Uuid::parse_str(&request_id) {
        Ok(id) => RegionId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid request/region ID")),
    };
    
    // Convert approved NPCs to CharacterIds
    let npc_ids: Vec<wrldbldr_domain::CharacterId> = approved_npcs
        .iter()
        .filter(|npc| npc.is_present)
        .filter_map(|npc| {
            Uuid::parse_str(&npc.character_id)
                .ok()
                .map(wrldbldr_domain::CharacterId::from_uuid)
        })
        .collect();
    
    // Execute staging approval use case
    match state.app.use_cases.approval.approve_staging.execute(region_id, npc_ids.clone()).await {
        Ok(_result) => {
            // Get the world ID to broadcast staging ready
            if let Some(world_id) = conn_info.world_id {
                // Get NPC details for the response
                let mut npcs_present = Vec::new();
                for npc_info in &approved_npcs {
                    if npc_info.is_present {
                        npcs_present.push(wrldbldr_protocol::NpcPresentInfo {
                            character_id: npc_info.character_id.clone(),
                            name: String::new(), // Would need to fetch from character entity
                            sprite_asset: None,
                            portrait_asset: None,
                            is_hidden_from_players: npc_info.is_hidden_from_players,
                        });
                    }
                }
                
                // Broadcast StagingReady to all players in the world
                let staging_ready = ServerMessage::StagingReady {
                    region_id: request_id.clone(),
                    npcs_present,
                };
                state.connections.broadcast_to_world(world_id, staging_ready).await;
            }
            
            None // No direct response needed - we broadcasted
        }
        Err(e) => {
            tracing::error!(error = %e, "Staging approval failed");
            Some(error_response("STAGING_ERROR", &e.to_string()))
        }
    }
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
    
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can regenerate staging"));
    }
    
    // Parse the request_id as a RegionId (it's the region being staged)
    let region_id = match Uuid::parse_str(&request_id) {
        Ok(id) => RegionId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid region ID")),
    };
    
    tracing::info!(
        request_id = %request_id,
        region_id = %region_id,
        guidance = %guidance,
        "Staging regeneration requested - generating rule-based suggestions"
    );
    
    // Get NPCs associated with this region from the character entity
    // This provides rule-based suggestions based on NPC relationships to the region
    // Note: Character entity contains NPCs only (PlayerCharacter is a separate entity)
    let npcs = match state.app.entities.character.list_in_region(region_id).await {
        Ok(characters) => characters,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch NPCs for region");
            return Some(error_response("STAGING_ERROR", &format!("Failed to fetch NPCs: {}", e)));
        }
    };
    
    // Convert characters to StagedNpcInfo with rule-based reasoning
    let llm_based_npcs: Vec<wrldbldr_protocol::StagedNpcInfo> = npcs
        .into_iter()
        .map(|npc| {
            // Generate reasoning based on the DM's guidance and NPC attributes
            let reasoning = if guidance.is_empty() {
                format!("{} is associated with this region", npc.name)
            } else {
                format!(
                    "{} - considering DM guidance: \"{}\"",
                    npc.name, guidance
                )
            };
            
            wrldbldr_protocol::StagedNpcInfo {
                character_id: npc.id.to_string(),
                name: npc.name,
                sprite_asset: npc.sprite_asset,
                portrait_asset: npc.portrait_asset,
                is_present: true, // Suggest all as present by default
                reasoning,
                is_hidden_from_players: false,
            }
        })
        .collect();
    
    tracing::info!(
        request_id = %request_id,
        npc_count = llm_based_npcs.len(),
        "Generated rule-based staging suggestions (LLM enhancement pending)"
    );
    
    // TODO: Queue LLM request for enhanced suggestions
    // When LLM integration is ready, this would:
    // 1. Build a prompt like:
    //    "You are helping a DM decide which NPCs should be present in a region.
    //     DM's guidance: {guidance}
    //     Available NPCs: [list of NPCs with their attributes]
    //     Suggest 2-4 NPCs that would make sense to be present. For each, provide:
    //     - Character name
    //     - Reason for being there
    //     - Whether they should be visible or hidden"
    // 2. Queue via LlmRequestData with LlmRequestType::Suggestion
    // 3. Return a "generating" status, with actual results sent via a separate message
    
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
    _ttl_hours: i32,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can pre-stage
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };
    
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can pre-stage regions"));
    }
    
    // Parse region ID
    let region_uuid = match Uuid::parse_str(&region_id) {
        Ok(id) => RegionId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid region ID")),
    };
    
    // Convert approved NPCs to CharacterIds
    let npc_ids: Vec<wrldbldr_domain::CharacterId> = npcs
        .iter()
        .filter(|npc| npc.is_present)
        .filter_map(|npc| {
            Uuid::parse_str(&npc.character_id)
                .ok()
                .map(wrldbldr_domain::CharacterId::from_uuid)
        })
        .collect();
    
    // Execute staging
    match state.app.use_cases.approval.approve_staging.execute(region_uuid, npc_ids).await {
        Ok(_) => None, // Success - no response needed for pre-staging
        Err(e) => {
            tracing::error!(error = %e, "Pre-staging failed");
            Some(error_response("STAGING_ERROR", &e.to_string()))
        }
    }
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
    
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can approve/reject suggestions"));
    }
    
    // Parse request ID
    let approval_id = match Uuid::parse_str(&request_id) {
        Ok(id) => id,
        Err(_) => return Some(error_response("INVALID_ID", "Invalid request ID")),
    };
    
    // Convert protocol decision to domain decision
    let domain_decision = match decision {
        wrldbldr_protocol::ApprovalDecision::Accept => {
            wrldbldr_domain::DmApprovalDecision::Accept
        }
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
        } => {
            wrldbldr_domain::DmApprovalDecision::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                rejected_tools,
                item_recipients,
            }
        }
        wrldbldr_protocol::ApprovalDecision::TakeOver { dm_response } => {
            wrldbldr_domain::DmApprovalDecision::TakeOver { dm_response }
        }
        wrldbldr_protocol::ApprovalDecision::Unknown => {
            return Some(error_response("INVALID_DECISION", "Unknown approval decision type"));
        }
    };
    
    // Execute approval use case
    match state.app.use_cases.approval.approve_suggestion.execute(approval_id, domain_decision).await {
        Ok(result) => {
            if result.approved {
                // Broadcast the approved response to the world
                if let Some(world_id) = conn_info.world_id {
                    let msg = ServerMessage::ResponseApproved {
                        npc_dialogue: result.final_dialogue.unwrap_or_default(),
                        executed_tools: result.approved_tools,
                    };
                    state.connections.broadcast_to_world(world_id, msg).await;
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
    
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can approve challenges"));
    }
    
    // Parse request ID
    let approval_id = match Uuid::parse_str(&request_id) {
        Ok(id) => id,
        Err(_) => return Some(error_response("INVALID_ID", "Invalid request ID")),
    };
    
    let decision = if approved {
        wrldbldr_domain::DmApprovalDecision::Accept
    } else {
        wrldbldr_domain::DmApprovalDecision::Reject {
            feedback: "Challenge rejected by DM".to_string(),
        }
    };
    
    match state.app.use_cases.approval.approve_suggestion.execute(approval_id, decision).await {
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

async fn handle_narrative_event_decision(
    state: &WsState,
    connection_id: Uuid,
    request_id: String,
    event_id: String,
    approved: bool,
    _selected_outcome: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can make decisions
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };
    
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can approve narrative events"));
    }
    
    // Parse request ID
    let approval_id = match Uuid::parse_str(&request_id) {
        Ok(id) => id,
        Err(_) => return Some(error_response("INVALID_ID", "Invalid request ID")),
    };
    
    let decision = if approved {
        wrldbldr_domain::DmApprovalDecision::Accept
    } else {
        wrldbldr_domain::DmApprovalDecision::Reject {
            feedback: "Narrative event rejected by DM".to_string(),
        }
    };
    
    match state.app.use_cases.approval.approve_suggestion.execute(approval_id, decision).await {
        Ok(_) => {
            if approved {
                // Broadcast that the narrative event was triggered
                if let Some(world_id) = conn_info.world_id {
                    let msg = ServerMessage::NarrativeEventTriggered {
                        event_id,
                        event_name: String::new(), // Would need to fetch
                        outcome_description: String::new(),
                        scene_direction: String::new(),
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
    
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can update directorial context"));
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
    
    // TODO: Persist directorial context to domain
    // The Scene entity has `directorial_notes: String` but not full DirectorialContext.
    // Options for future implementation:
    // 1. Add DirectorialContext fields to Scene domain type
    // 2. Create a separate DirectorialContext entity/value object
    // 3. Store in a world-scoped in-memory cache for LLM prompts
    
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
    
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can trigger approach events"));
    }
    
    // Parse target PC ID
    let pc_uuid = match Uuid::parse_str(&target_pc_id) {
        Ok(id) => PlayerCharacterId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid PC ID")),
    };
    
    // Get NPC details
    let npc_uuid = match Uuid::parse_str(&npc_id) {
        Ok(id) => wrldbldr_domain::CharacterId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid NPC ID")),
    };
    
    let (npc_name, npc_sprite) = match state.app.entities.character.get(npc_uuid).await {
        Ok(Some(c)) => (c.name.clone(), c.sprite_asset.clone()),
        Ok(None) => (String::new(), None),
        Err(_) => (String::new(), None),
    };
    
    // Send approach event to target PC
    let msg = ServerMessage::ApproachEvent {
        npc_id,
        npc_name: if reveal { npc_name } else { "Unknown Figure".to_string() },
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
    
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can trigger location events"));
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
    _location_id: String,
    region_id: String,
    notes: Option<String>,
) -> Option<ServerMessage> {
    // Get connection info - only DMs can share NPC locations
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };
    
    if !conn_info.is_dm() {
        return Some(error_response("UNAUTHORIZED", "Only DMs can share NPC locations"));
    }
    
    // Parse PC ID
    let pc_uuid = match Uuid::parse_str(&pc_id) {
        Ok(id) => PlayerCharacterId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid PC ID")),
    };
    
    // Get NPC and region names
    let npc_uuid = match Uuid::parse_str(&npc_id) {
        Ok(id) => wrldbldr_domain::CharacterId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid NPC ID")),
    };
    
    let region_uuid = match Uuid::parse_str(&region_id) {
        Ok(id) => RegionId::from_uuid(id),
        Err(_) => return Some(error_response("INVALID_ID", "Invalid region ID")),
    };
    
    let npc_name = match state.app.entities.character.get(npc_uuid).await {
        Ok(Some(c)) => c.name,
        _ => "Unknown".to_string(),
    };
    
    let region_name = match state.app.entities.location.get_region(region_uuid).await {
        Ok(Some(r)) => r.name,
        _ => "Unknown".to_string(),
    };
    
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
            let npc_id = match Uuid::parse_str(target_str) {
                Ok(id) => CharacterId::from_uuid(id),
                Err(_) => {
                    return Some(error_response("INVALID_ID", "Invalid NPC ID format"));
                }
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
    state.connections.broadcast_to_dms(world_id, queue_msg).await;
    
    Some(ack)
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
