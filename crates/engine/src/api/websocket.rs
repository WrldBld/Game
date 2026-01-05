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
use chrono::Timelike;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use wrldbldr_domain::{ChallengeId, CharacterId, ItemId, LocationId, NarrativeEventId, PlayerCharacterId, RegionId, WorldId};
use crate::use_cases::narrative::EffectExecutionContext;
use wrldbldr_protocol::{
    ClientMessage, ErrorCode, ResponseResult, ServerMessage, WorldRole as ProtoWorldRole,
};

use crate::app::App;
use crate::use_cases::movement::{EnterRegionError, StagingStatus};
use crate::use_cases::visual_state::StateResolutionContext;
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
        ClientMessage::StagingApprovalResponse { request_id, approved_npcs, ttl_hours, source, location_state_id, region_state_id } => {
            handle_staging_approval(state, connection_id, request_id, approved_npcs, ttl_hours, source, location_state_id, region_state_id).await
        }
        
        ClientMessage::StagingRegenerateRequest { request_id, guidance } => {
            handle_staging_regenerate(state, connection_id, request_id, guidance).await
        }
        
        ClientMessage::PreStageRegion { region_id, npcs, ttl_hours, location_state_id, region_state_id } => {
            handle_pre_stage_region(state, connection_id, region_id, npcs, ttl_hours, location_state_id, region_state_id).await
        }

        // Approval handlers
        ClientMessage::ApprovalDecision { request_id, decision } => {
            handle_approval_decision(state, connection_id, request_id, decision).await
        }
        
        ClientMessage::ChallengeSuggestionDecision { request_id, approved, modified_difficulty } => {
            handle_challenge_suggestion_decision(state, connection_id, request_id, approved, modified_difficulty).await
        }
        
        ClientMessage::ChallengeOutcomeDecision { resolution_id, decision } => {
            handle_challenge_outcome_decision(state, connection_id, resolution_id, decision).await
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

        // Time control handlers (DM only)
        ClientMessage::SetGameTime { world_id, day, hour, notify_players } => {
            handle_set_game_time(state, connection_id, world_id, day, hour, notify_players).await
        }
        
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
        
        ClientMessage::RespondToTimeSuggestion { suggestion_id, decision } => {
            handle_respond_to_time_suggestion(state, connection_id, suggestion_id, decision).await
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
    
    // Fetch PC data if role is Player and pc_id is provided
    let your_pc = if matches!(role, ProtoWorldRole::Player) {
        if let Some(pc_id) = pc_id_typed {
            match state.app.entities.player_character.get(pc_id).await {
                Ok(Some(pc)) => {
                    Some(serde_json::json!({
                        "id": pc.id.to_string(),
                        "name": pc.name,
                        "description": pc.description,
                        "portrait_asset": pc.portrait_asset,
                        "sprite_asset": pc.sprite_asset,
                        "current_location_id": pc.current_location_id.to_string(),
                        "current_region_id": pc.current_region_id.map(|id| id.to_string()),
                    }))
                }
                Ok(None) => {
                    tracing::warn!(pc_id = %pc_id, "PC not found when joining world");
                    None
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to fetch PC data");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };
    
    Some(ServerMessage::WorldJoined {
        world_id,
        snapshot,
        connected_users,
        your_role: role,
        your_pc,
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
            
            // Check staging status
            match result.staging_status {
                StagingStatus::Pending { previous_staging } => {
                    // Send StagingPending to the player
                    let pending_msg = ServerMessage::StagingPending {
                        region_id: result.region.id.to_string(),
                        region_name: result.region.name.clone(),
                    };
                    
                    // Send StagingApprovalRequired to DMs
                    let request_id = Uuid::new_v4().to_string();
                    let now = chrono::Utc::now();
                    
                    // Get rule-based suggestions (NPCs that have relationships to this region)
                    let rule_based_npcs = generate_rule_based_suggestions(state, result.region.id).await;
                    
                    // Get LLM-based suggestions (async call to LLM for context-aware suggestions)
                    let llm_based_npcs = generate_llm_based_suggestions(
                        state,
                        result.region.id,
                        &result.region.name,
                        &location_name,
                        None, // No guidance on initial entry
                    ).await;
                    
                    // Resolve visual states for this region
                    let (resolved_visual_state, available_location_states, available_region_states) = 
                        resolve_visual_states_for_staging(
                            state,
                            conn_info.world_id,
                            result.region.location_id,
                            result.region.id,
                        ).await;
                    
                    let approval_msg = ServerMessage::StagingApprovalRequired {
                        request_id,
                        region_id: result.region.id.to_string(),
                        region_name: result.region.name.clone(),
                        location_id: result.region.location_id.to_string(),
                        location_name: location_name.clone(),
                        game_time: wrldbldr_protocol::types::GameTime {
                            day: 1,
                            hour: now.hour() as u8,
                            minute: now.minute() as u8,
                            is_paused: false,
                        },
                        previous_staging: previous_staging.map(|s| {
                            wrldbldr_protocol::PreviousStagingInfo {
                                staging_id: s.id.to_string(),
                                approved_at: s.approved_at.to_rfc3339(),
                                npcs: s.npcs.into_iter().map(|n| {
                                    wrldbldr_protocol::StagedNpcInfo {
                                        character_id: n.character_id.to_string(),
                                        name: n.name,
                                        sprite_asset: n.sprite_asset,
                                        portrait_asset: n.portrait_asset,
                                        is_present: n.is_present,
                                        reasoning: n.reasoning,
                                        is_hidden_from_players: n.is_hidden_from_players,
                                    }
                                }).collect(),
                            }
                        }),
                        rule_based_npcs,
                        llm_based_npcs,
                        default_ttl_hours: 24,
                        waiting_pcs: vec![
                            wrldbldr_protocol::WaitingPcInfo {
                                pc_id: result.pc.id.to_string(),
                                pc_name: result.pc.name.clone(),
                                player_id: result.pc.user_id.clone(),
                            }
                        ],
                        resolved_visual_state,
                        available_location_states,
                        available_region_states,
                    };
                    
                    // Broadcast staging approval and time suggestion to DMs
                    if let Some(world_id) = conn_info.world_id {
                        state.connections.broadcast_to_dms(world_id, approval_msg).await;
                        
                        // Also send time suggestion if present
                        if let Some(ref time_suggestion) = result.time_suggestion {
                            let suggestion_msg = ServerMessage::TimeSuggestion { data: time_suggestion.to_protocol() };
                            state.connections.broadcast_to_dms(world_id, suggestion_msg).await;
                        }
                    }
                    
                    Some(pending_msg)
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
                    
                    let npcs_present: Vec<wrldbldr_protocol::NpcPresenceData> = result.npcs
                        .into_iter()
                        .map(|npc| wrldbldr_protocol::NpcPresenceData {
                            character_id: npc.character_id.to_string(),
                            name: npc.name,
                            sprite_asset: npc.sprite_asset,
                            portrait_asset: npc.portrait_asset,
                        })
                        .collect();
                    
                    // Get navigation data
                    let navigation = build_navigation_data(
                        &state.app.entities.location,
                        region_uuid,
                    ).await;
                    
                    // Get items in the region
                    let region_items = build_region_items(
                        &state.app.entities.inventory,
                        region_uuid,
                    ).await;
                    
                    // Broadcast time suggestion to DMs if present
                    if let Some(ref time_suggestion) = result.time_suggestion {
                        if let Some(world_id) = conn_info.world_id {
                            let suggestion_msg = ServerMessage::TimeSuggestion { data: time_suggestion.to_protocol() };
                            state.connections.broadcast_to_dms(world_id, suggestion_msg).await;
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

/// Generate rule-based staging suggestions based on NPC relationships to a region.
///
/// Returns NPCs that have relationships to this region (home, work, frequents),
/// with reasoning based on the relationship type.
async fn generate_rule_based_suggestions(
    state: &WsState,
    region_id: RegionId,
) -> Vec<wrldbldr_protocol::StagedNpcInfo> {
    use crate::infrastructure::ports::NpcRegionRelationType;
    
    // Get NPCs that have relationships to this region
    let npcs_with_relationships = state.app.entities.character
        .get_npcs_for_region(region_id)
        .await
        .ok()
        .unwrap_or_default();
    
    // Convert to staging suggestions with reasoning
    let mut suggestions: Vec<wrldbldr_protocol::StagedNpcInfo> = npcs_with_relationships
        .into_iter()
        .filter(|n| n.relationship_type != NpcRegionRelationType::Avoids) // Filter out NPCs that avoid this region
        .map(|npc| {
            let reasoning = match npc.relationship_type {
                NpcRegionRelationType::HomeRegion => "Lives here".to_string(),
                NpcRegionRelationType::WorksAt => {
                    match npc.shift.as_deref() {
                        Some("day") => "Works here (day shift)".to_string(),
                        Some("night") => "Works here (night shift)".to_string(),
                        _ => "Works here".to_string(),
                    }
                }
                NpcRegionRelationType::Frequents => {
                    let freq = npc.frequency.as_deref().unwrap_or("sometimes");
                    let time = npc.time_of_day.as_deref();
                    match time {
                        Some(t) => format!("Frequents this area {} ({})", freq, t),
                        None => format!("Frequents this area ({})", freq),
                    }
                }
                NpcRegionRelationType::Avoids => "Avoids this area".to_string(), // Should be filtered out
            };
            
            wrldbldr_protocol::StagedNpcInfo {
                character_id: npc.character_id.to_string(),
                name: npc.name,
                sprite_asset: npc.sprite_asset,
                portrait_asset: npc.portrait_asset,
                is_present: true, // Suggest as present by default
                reasoning,
                is_hidden_from_players: false,
            }
        })
        .collect();
    
    // Also include currently staged NPCs that might not have explicit relationships
    if let Ok(staged_npcs) = state.app.entities.staging.get_staged_npcs(region_id).await {
        for staged in staged_npcs {
            // Only add if not already in suggestions
            if !suggestions.iter().any(|s| s.character_id == staged.character_id.to_string()) {
                suggestions.push(wrldbldr_protocol::StagedNpcInfo {
                    character_id: staged.character_id.to_string(),
                    name: staged.name,
                    sprite_asset: staged.sprite_asset,
                    portrait_asset: staged.portrait_asset,
                    is_present: staged.is_present,
                    reasoning: staged.reasoning,
                    is_hidden_from_players: staged.is_hidden_from_players,
                });
            }
        }
    }
    
    suggestions
}

/// Generate LLM-based NPC staging suggestions.
///
/// Uses the LLM to analyze which NPCs should be present based on:
/// - Region context (name, location, atmosphere)
/// - Time of day
/// - NPC descriptions and relationships
/// - Any DM guidance
async fn generate_llm_based_suggestions(
    state: &WsState,
    region_id: RegionId,
    region_name: &str,
    location_name: &str,
    guidance: Option<&str>,
) -> Vec<wrldbldr_protocol::StagedNpcInfo> {
    use crate::infrastructure::ports::{ChatMessage, LlmRequest, NpcRegionRelationType};
    
    // Get NPC candidates (those with relationships to this region)
    let npcs_with_relationships = match state.app.entities.character
        .get_npcs_for_region(region_id)
        .await
    {
        Ok(npcs) => npcs,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to get NPCs for LLM staging");
            return vec![];
        }
    };
    
    // Filter out NPCs that avoid this region
    let candidates: Vec<_> = npcs_with_relationships
        .into_iter()
        .filter(|n| n.relationship_type != NpcRegionRelationType::Avoids)
        .collect();
    
    if candidates.is_empty() {
        return vec![];
    }
    
    // Build the NPC list for the prompt
    let npc_list: String = candidates
        .iter()
        .enumerate()
        .map(|(i, npc)| {
            let relationship = match npc.relationship_type {
                NpcRegionRelationType::HomeRegion => "lives here",
                NpcRegionRelationType::WorksAt => "works here",
                NpcRegionRelationType::Frequents => "frequents this area",
                NpcRegionRelationType::Avoids => "avoids this area",
            };
            format!("{}. {} ({})", i + 1, npc.name, relationship)
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    // Build the prompt
    let guidance_text = guidance
        .filter(|g| !g.is_empty())
        .map(|g| format!("\n\nDM's guidance: {}", g))
        .unwrap_or_default();
    
    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        Select 1-4 NPCs that would logically be present. Only include NPCs from the provided list.";
    
    let user_prompt = format!(
        "Region: {} (in {})\n\nAvailable NPCs:\n{}{}\n\nWhich NPCs should be present? Respond with JSON only.",
        region_name, location_name, npc_list, guidance_text
    );
    
    // Call the LLM
    let request = LlmRequest::new(vec![ChatMessage::user(&user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);
    
    let response = match state.app.llm.generate(request).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::warn!(error = %e, "LLM staging suggestion failed");
            return vec![];
        }
    };
    
    // Parse the LLM response (simple JSON parsing)
    // Expected format: [{"name": "NPC Name", "reason": "Why they're here"}]
    let suggestions = parse_llm_staging_response(&response.content, &candidates);
    
    tracing::info!(
        region = %region_name,
        suggestion_count = suggestions.len(),
        "Generated LLM staging suggestions"
    );
    
    suggestions
}

/// Parse LLM staging response into StagedNpcInfo structs.
fn parse_llm_staging_response(
    content: &str,
    candidates: &[crate::infrastructure::ports::NpcWithRegionInfo],
) -> Vec<wrldbldr_protocol::StagedNpcInfo> {
    // Try to extract JSON array from the response
    let json_start = content.find('[');
    let json_end = content.rfind(']');
    
    let json_str = match (json_start, json_end) {
        (Some(start), Some(end)) if end > start => &content[start..=end],
        _ => {
            tracing::debug!("No valid JSON array found in LLM response");
            return vec![];
        }
    };
    
    // Parse JSON
    #[derive(serde::Deserialize)]
    struct LlmSuggestion {
        name: String,
        reason: String,
    }
    
    let parsed: Vec<LlmSuggestion> = match serde_json::from_str(json_str) {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!(error = %e, json = %json_str, "Failed to parse LLM staging JSON");
            return vec![];
        }
    };
    
    // Match suggestions to actual NPCs
    parsed
        .into_iter()
        .filter_map(|suggestion| {
            // Find matching NPC (case-insensitive)
            let npc = candidates.iter().find(|c| 
                c.name.to_lowercase() == suggestion.name.to_lowercase()
            )?;
            
            Some(wrldbldr_protocol::StagedNpcInfo {
                character_id: npc.character_id.to_string(),
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
                is_present: true,
                reasoning: format!("[LLM] {}", suggestion.reason),
                is_hidden_from_players: false,
            })
        })
        .collect()
}

/// Resolve visual states for a staging request.
///
/// Returns the auto-resolved visual state (if determinable) and all available
/// location/region states for DM selection.
async fn resolve_visual_states_for_staging(
    state: &WsState,
    world_id: Option<WorldId>,
    location_id: LocationId,
    region_id: RegionId,
) -> (
    Option<wrldbldr_protocol::types::ResolvedVisualStateData>,
    Vec<wrldbldr_protocol::types::StateOptionData>,
    Vec<wrldbldr_protocol::types::StateOptionData>,
) {
    // Get world to build context
    let world_id = match world_id {
        Some(id) => id,
        None => return (None, vec![], vec![]),
    };

    // Get the world for game time
    let game_time = match state.app.entities.world.get(world_id).await {
        Ok(Some(w)) => w.game_time,
        _ => return (None, vec![], vec![]),
    };

    // Get world flags
    let world_flags = state.app.entities.flag
        .get_world_flags(world_id)
        .await
        .unwrap_or_default();

    // Build resolution context
    let context = StateResolutionContext::new(world_id, game_time)
        .with_world_flags(world_flags);

    // Resolve visual states
    let resolution = match state.app.use_cases.visual_state.resolve
        .execute(location_id, region_id, &context)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to resolve visual states");
            return (None, vec![], vec![]);
        }
    };

    // Convert to protocol types
    let resolved = if resolution.is_complete {
        Some(wrldbldr_protocol::types::ResolvedVisualStateData {
            location_state: resolution.location_state.as_ref().map(|s| {
                wrldbldr_protocol::types::ResolvedStateInfoData {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    backdrop_override: s.backdrop_override.clone(),
                    atmosphere_override: s.atmosphere_override.clone(),
                    ambient_sound: s.ambient_sound.clone(),
                }
            }),
            region_state: resolution.region_state.as_ref().map(|s| {
                wrldbldr_protocol::types::ResolvedStateInfoData {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    backdrop_override: s.backdrop_override.clone(),
                    atmosphere_override: s.atmosphere_override.clone(),
                    ambient_sound: s.ambient_sound.clone(),
                }
            }),
        })
    } else {
        None // Incomplete resolution (needs LLM for soft rules)
    };

    // Convert available location states
    let available_location: Vec<wrldbldr_protocol::types::StateOptionData> = resolution
        .available_location_states
        .iter()
        .map(|s| {
            let match_reason = if s.evaluation.is_active {
                Some(s.evaluation.matched_rules.join(", "))
            } else {
                None
            };
            wrldbldr_protocol::types::StateOptionData {
                id: s.id.clone(),
                name: s.name.clone(),
                priority: 0, // TODO: Add priority to ResolvedStateInfo
                is_default: false,
                match_reason,
            }
        })
        .collect();

    // Convert available region states
    let available_region: Vec<wrldbldr_protocol::types::StateOptionData> = resolution
        .available_region_states
        .iter()
        .map(|s| {
            let match_reason = if s.evaluation.is_active {
                Some(s.evaluation.matched_rules.join(", "))
            } else {
                None
            };
            wrldbldr_protocol::types::StateOptionData {
                id: s.id.clone(),
                name: s.name.clone(),
                priority: 0,
                is_default: false,
                match_reason,
            }
        })
        .collect();

    (resolved, available_location, available_region)
}

/// Build visual state data from currently active states for StagingReady message.
async fn build_visual_state_for_staging(
    state: &WsState,
    location_id: wrldbldr_domain::LocationId,
    region_id: wrldbldr_domain::RegionId,
) -> Option<wrldbldr_protocol::types::ResolvedVisualStateData> {
    // Fetch active location state
    let location_state = state.app.entities.location_state
        .get_active(location_id)
        .await
        .ok()
        .flatten();
    
    // Fetch active region state
    let region_state = state.app.entities.region_state
        .get_active(region_id)
        .await
        .ok()
        .flatten();
    
    // If neither is set, return None
    if location_state.is_none() && region_state.is_none() {
        return None;
    }
    
    Some(wrldbldr_protocol::types::ResolvedVisualStateData {
        location_state: location_state.map(|s| {
            wrldbldr_protocol::types::ResolvedStateInfoData {
                id: s.id.to_string(),
                name: s.name,
                backdrop_override: s.backdrop_override,
                atmosphere_override: s.atmosphere_override,
                ambient_sound: s.ambient_sound,
            }
        }),
        region_state: region_state.map(|s| {
            wrldbldr_protocol::types::ResolvedStateInfoData {
                id: s.id.to_string(),
                name: s.name,
                backdrop_override: s.backdrop_override,
                atmosphere_override: s.atmosphere_override,
                ambient_sound: s.ambient_sound,
            }
        }),
    })
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
                .filter(|npc| npc.is_visible_to_players())
                .map(|npc| wrldbldr_protocol::NpcPresenceData {
                    character_id: npc.character_id.to_string(),
                    name: npc.name,
                    sprite_asset: npc.sprite_asset,
                    portrait_asset: npc.portrait_asset,
                })
                .collect();
            
            // Get navigation data for new region
            let navigation = build_navigation_data(
                &state.app.entities.location,
                result.region.id,
            ).await;
            
            // Get items in the region
            let region_items = build_region_items(
                &state.app.entities.inventory,
                result.region.id,
            ).await;
            
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
            let world_id_typed = match parse_world_id_for_request(&req_world_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            match state.app.entities.world.get(world_id_typed).await {
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
            let world_id_typed = match parse_world_id_for_request(&req_world_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            match state.app.entities.character.list_in_world(world_id_typed).await {
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
            let world_id_typed = match parse_world_id_for_request(&req_world_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            match state.app.entities.location.list_in_world(world_id_typed).await {
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
        
        // Game time queries
        RequestPayload::GetGameTime { world_id: req_world_id } => {
            let world_id_typed = match parse_world_id_for_request(&req_world_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(world)) => {
                    let gt = &world.game_time;
                    let game_time = wrldbldr_protocol::types::GameTime {
                        day: gt.day_ordinal(),
                        hour: gt.current().hour() as u8,
                        minute: gt.current().minute() as u8,
                        is_paused: gt.is_paused(),
                    };
                    ResponseResult::success(serde_json::json!({
                        "game_time": game_time,
                    }))
                }
                Ok(None) => ResponseResult::error(ErrorCode::NotFound, "World not found"),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        // Advance game time (DM only)
        RequestPayload::AdvanceGameTime { world_id: req_world_id, hours } => {
            // Verify DM authorization
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let world_id_typed = match parse_uuid_for_request(&req_world_id, &request_id, "Invalid world ID") {
                Ok(uuid) => WorldId::from_uuid(uuid),
                Err(e) => return Some(e),
            };
            
            // Get the world, advance time, and save
            let mut world = match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    });
                }
            };
            
            // Advance the game time
            world.game_time.advance_hours(hours);
            world.updated_at = chrono::Utc::now();
            
            // Save the world
            if let Err(e) = state.app.entities.world.save(&world).await {
                return Some(ServerMessage::Response {
                    request_id,
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                });
            }
            
            // Build the protocol GameTime
            let gt = &world.game_time;
            let game_time = wrldbldr_protocol::types::GameTime {
                day: gt.day_ordinal(),
                hour: gt.current().hour() as u8,
                minute: gt.current().minute() as u8,
                is_paused: gt.is_paused(),
            };
            
            // Broadcast GameTimeUpdated to all players in the world
            let update_msg = ServerMessage::GameTimeUpdated { game_time };
            state.connections.broadcast_to_world(world_id_typed, update_msg).await;
            
            tracing::info!(
                world_id = %world_id_typed,
                hours_advanced = hours,
                new_day = gt.day_ordinal(),
                new_hour = gt.current().hour(),
                "Game time advanced"
            );
            
            // Return success response to requester
            ResponseResult::success(serde_json::json!({
                "game_time": game_time,
                "hours_advanced": hours,
            }))
        }

        // Advance game time by minutes (DM only)
        RequestPayload::AdvanceGameTimeMinutes { world_id: req_world_id, minutes, reason } => {
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let world_id_typed = match parse_uuid_for_request(&req_world_id, &request_id, "Invalid world ID") {
                Ok(uuid) => WorldId::from_uuid(uuid),
                Err(e) => return Some(e),
            };
            
            let mut world = match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    });
                }
            };
            
            let previous_time = world.game_time.clone();
            let advance_reason = wrldbldr_domain::TimeAdvanceReason::DmManual { hours: minutes / 60 };
            let result = world.advance_time(minutes, advance_reason.clone(), chrono::Utc::now());
            
            if let Err(e) = state.app.entities.world.save(&world).await {
                return Some(ServerMessage::Response {
                    request_id,
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                });
            }
            
            // Broadcast GameTimeAdvanced to all players
            let advance_data = crate::use_cases::time::build_time_advance_data(
                &previous_time,
                &result.new_time,
                minutes,
                &advance_reason,
            );
            let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
            state.connections.broadcast_to_world(world_id_typed, update_msg).await;
            
            tracing::info!(
                world_id = %world_id_typed,
                minutes_advanced = minutes,
                "Game time advanced (minutes)"
            );
            
            let game_time = crate::use_cases::time::game_time_to_protocol(&world.game_time);
            ResponseResult::success(serde_json::json!({
                "game_time": game_time,
                "minutes_advanced": minutes,
            }))
        }

        // Set exact game time (DM only)
        RequestPayload::SetGameTime { world_id: req_world_id, day, hour, notify_players } => {
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let world_id_typed = match parse_uuid_for_request(&req_world_id, &request_id, "Invalid world ID") {
                Ok(uuid) => WorldId::from_uuid(uuid),
                Err(e) => return Some(e),
            };
            
            let mut world = match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    });
                }
            };
            
            let previous_time = world.game_time.clone();
            world.game_time.set_day_and_hour(day, hour as u32);
            world.updated_at = chrono::Utc::now();
            
            if let Err(e) = state.app.entities.world.save(&world).await {
                return Some(ServerMessage::Response {
                    request_id,
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                });
            }
            
            if notify_players {
                let reason = wrldbldr_domain::TimeAdvanceReason::DmSetTime;
                let advance_data = crate::use_cases::time::build_time_advance_data(
                    &previous_time,
                    &world.game_time,
                    0, // No specific minutes
                    &reason,
                );
                let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
                state.connections.broadcast_to_world(world_id_typed, update_msg).await;
            }
            
            tracing::info!(
                world_id = %world_id_typed,
                new_day = day,
                new_hour = hour,
                "Game time set"
            );
            
            let game_time = crate::use_cases::time::game_time_to_protocol(&world.game_time);
            ResponseResult::success(serde_json::json!({
                "game_time": game_time,
            }))
        }

        // Skip to next occurrence of time period (DM only)
        RequestPayload::SkipToPeriod { world_id: req_world_id, period } => {
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let world_id_typed = match parse_uuid_for_request(&req_world_id, &request_id, "Invalid world ID") {
                Ok(uuid) => WorldId::from_uuid(uuid),
                Err(e) => return Some(e),
            };
            
            // Parse the period string
            let target_period = match period.to_lowercase().as_str() {
                "morning" => wrldbldr_domain::TimeOfDay::Morning,
                "afternoon" => wrldbldr_domain::TimeOfDay::Afternoon,
                "evening" => wrldbldr_domain::TimeOfDay::Evening,
                "night" => wrldbldr_domain::TimeOfDay::Night,
                _ => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::BadRequest, "Invalid period. Use: morning, afternoon, evening, night"),
                    });
                }
            };
            
            let mut world = match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    });
                }
            };
            
            let previous_time = world.game_time.clone();
            let minutes_until = world.game_time.minutes_until_period(target_period);
            world.game_time.skip_to_period(target_period);
            world.updated_at = chrono::Utc::now();
            
            if let Err(e) = state.app.entities.world.save(&world).await {
                return Some(ServerMessage::Response {
                    request_id,
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                });
            }
            
            let reason = wrldbldr_domain::TimeAdvanceReason::DmSkipToPeriod { period: target_period };
            let advance_data = crate::use_cases::time::build_time_advance_data(
                &previous_time,
                &world.game_time,
                minutes_until,
                &reason,
            );
            let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
            state.connections.broadcast_to_world(world_id_typed, update_msg).await;
            
            tracing::info!(
                world_id = %world_id_typed,
                target_period = %target_period,
                "Skipped to time period"
            );
            
            let game_time = crate::use_cases::time::game_time_to_protocol(&world.game_time);
            ResponseResult::success(serde_json::json!({
                "game_time": game_time,
                "skipped_to": period,
            }))
        }

        // Get time configuration (any role)
        RequestPayload::GetTimeConfig { world_id: req_world_id } => {
            let world_id_typed = match parse_uuid_for_request(&req_world_id, &request_id, "Invalid world ID") {
                Ok(uuid) => WorldId::from_uuid(uuid),
                Err(e) => return Some(e),
            };
            
            match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(world)) => {
                    let config = &world.time_config;
                    ResponseResult::success(serde_json::json!({
                        "mode": format!("{:?}", config.mode).to_lowercase(),
                        "time_costs": {
                            "travel_location": config.time_costs.travel_location,
                            "travel_region": config.time_costs.travel_region,
                            "rest_short": config.time_costs.rest_short,
                            "rest_long": config.time_costs.rest_long,
                            "conversation": config.time_costs.conversation,
                            "challenge": config.time_costs.challenge,
                            "scene_transition": config.time_costs.scene_transition,
                        },
                        "show_time_to_players": config.show_time_to_players,
                    }))
                }
                Ok(None) => ResponseResult::error(ErrorCode::NotFound, "World not found"),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }

        // Update time configuration (DM only)
        RequestPayload::UpdateTimeConfig { world_id: req_world_id, config } => {
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let world_id_typed = match parse_uuid_for_request(&req_world_id, &request_id, "Invalid world ID") {
                Ok(uuid) => WorldId::from_uuid(uuid),
                Err(e) => return Some(e),
            };
            
            let mut world = match state.app.entities.world.get(world_id_typed).await {
                Ok(Some(w)) => w,
                Ok(None) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::NotFound, "World not found"),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Response {
                        request_id,
                        result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                    });
                }
            };
            
            // Convert protocol config to domain config
            let mode = match config.mode {
                wrldbldr_protocol::types::TimeMode::Manual => wrldbldr_domain::TimeMode::Manual,
                wrldbldr_protocol::types::TimeMode::Suggested => wrldbldr_domain::TimeMode::Suggested,
                wrldbldr_protocol::types::TimeMode::Auto => wrldbldr_domain::TimeMode::Auto,
            };
            
            let time_costs = wrldbldr_domain::TimeCostConfig {
                travel_location: config.time_costs.travel_location,
                travel_region: config.time_costs.travel_region,
                rest_short: config.time_costs.rest_short,
                rest_long: config.time_costs.rest_long,
                conversation: config.time_costs.conversation,
                challenge: config.time_costs.challenge,
                scene_transition: config.time_costs.scene_transition,
            };
            
            world.time_config = wrldbldr_domain::GameTimeConfig {
                mode,
                time_costs,
                show_time_to_players: config.show_time_to_players,
                time_format: wrldbldr_domain::TimeFormat::TwelveHour, // Default
            };
            world.updated_at = chrono::Utc::now();
            
            if let Err(e) = state.app.entities.world.save(&world).await {
                return Some(ServerMessage::Response {
                    request_id,
                    result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                });
            }
            
            // Broadcast config update to DMs
            let update_msg = ServerMessage::TimeConfigUpdated {
                world_id: world_id_typed.to_string(),
                config: config.clone(),
            };
            state.connections.broadcast_to_dms(world_id_typed, update_msg).await;
            
            tracing::info!(
                world_id = %world_id_typed,
                mode = ?mode,
                "Time config updated"
            );
            
            ResponseResult::success_empty()
        }
        
        // =====================================================================
        // NPC-Region Relationship Operations
        // =====================================================================
        
        RequestPayload::ListCharacterRegionRelationships { character_id } => {
            let char_id_typed = match parse_character_id_for_request(&character_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            match state.app.entities.character.get_region_relationships(char_id_typed).await {
                Ok(relationships) => {
                    let data: Vec<serde_json::Value> = relationships
                        .into_iter()
                        .map(|r| serde_json::json!({
                            "region_id": r.region_id.to_string(),
                            "relationship_type": format!("{}", r.relationship_type),
                            "shift": r.shift,
                            "frequency": r.frequency,
                            "time_of_day": r.time_of_day,
                            "reason": r.reason,
                        }))
                        .collect();
                    ResponseResult::success(data)
                }
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        RequestPayload::SetCharacterHomeRegion { character_id, region_id } => {
            // Verify DM authorization
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let char_uuid = match parse_character_id_for_request(&character_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            let region_uuid = match parse_region_id_for_request(&region_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            match state.app.entities.character.set_home_region(char_uuid, region_uuid).await {
                Ok(()) => ResponseResult::success(serde_json::json!({"success": true})),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        RequestPayload::SetCharacterWorkRegion { character_id, region_id } => {
            // Verify DM authorization
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let char_uuid = match parse_character_id_for_request(&character_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            let region_uuid = match parse_region_id_for_request(&region_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            // Note: shift parameter not in the protocol yet, using None
            match state.app.entities.character.set_work_region(char_uuid, region_uuid, None).await {
                Ok(()) => ResponseResult::success(serde_json::json!({"success": true})),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        RequestPayload::RemoveCharacterRegionRelationship { character_id, region_id, relationship_type } => {
            // Verify DM authorization
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let char_uuid = match parse_character_id_for_request(&character_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            let region_uuid = match parse_region_id_for_request(&region_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            match state.app.entities.character.remove_region_relationship(char_uuid, region_uuid, &relationship_type).await {
                Ok(()) => ResponseResult::success(serde_json::json!({"success": true})),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        RequestPayload::ListRegionNpcs { region_id } => {
            let region_id_typed = match parse_region_id_for_request(&region_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            match state.app.entities.character.get_npcs_for_region(region_id_typed).await {
                Ok(npcs) => {
                    let data: Vec<serde_json::Value> = npcs
                        .into_iter()
                        .map(|n| serde_json::json!({
                            "character_id": n.character_id.to_string(),
                            "name": n.name,
                            "sprite_asset": n.sprite_asset,
                            "portrait_asset": n.portrait_asset,
                            "relationship_type": format!("{}", n.relationship_type),
                            "shift": n.shift,
                            "frequency": n.frequency,
                            "time_of_day": n.time_of_day,
                            "reason": n.reason,
                        }))
                        .collect();
                    ResponseResult::success(data)
                }
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        // =====================================================================
        // Item Placement Operations (DM only)
        // =====================================================================
        
        RequestPayload::PlaceItemInRegion { region_id, item_id } => {
            // Verify DM authorization
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let region_uuid = match parse_region_id_for_request(&region_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            let item_uuid = match parse_item_id_for_request(&item_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            match state.app.entities.inventory.place_item_in_region(item_uuid, region_uuid).await {
                Ok(()) => ResponseResult::success(serde_json::json!({"success": true})),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        RequestPayload::CreateAndPlaceItem { world_id, region_id, data } => {
            // Verify DM authorization
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let world_uuid = match parse_world_id_for_request(&world_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            let region_uuid = match parse_region_id_for_request(&region_id, &request_id) {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            // Create the item
            let mut item = wrldbldr_domain::Item::new(world_uuid, data.name.clone());
            if let Some(desc) = data.description {
                item = item.with_description(desc);
            }
            if let Some(item_type) = data.item_type {
                item = item.with_type(item_type);
            }
            if let Some(props) = data.properties {
                item = item.with_properties(props.to_string());
            }
            
            // Save the item and place it in the region
            match state.app.entities.inventory.create_and_place_in_region(item, region_uuid).await {
                Ok(item_id) => ResponseResult::success(serde_json::json!({
                    "success": true,
                    "item_id": item_id.to_string(),
                })),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
            }
        }
        
        // Character inventory query
        RequestPayload::GetCharacterInventory { character_id } => {
            let char_uuid = match parse_uuid_for_request(&character_id, &request_id, "Invalid character ID") {
                Ok(id) => id,
                Err(e) => return Some(e),
            };
            
            // Try as PlayerCharacter first, then as NPC
            let pc_id = PlayerCharacterId::from_uuid(char_uuid);
            let items = match state.app.entities.inventory.get_pc_inventory(pc_id).await {
                Ok(items) => items,
                Err(_) => {
                    // Try as NPC
                    let npc_id = CharacterId::from_uuid(char_uuid);
                    match state.app.entities.inventory.get_character_inventory(npc_id).await {
                        Ok(items) => items,
                        Err(e) => {
                            return Some(ServerMessage::Response {
                                request_id,
                                result: ResponseResult::error(ErrorCode::InternalError, e.to_string()),
                            });
                        }
                    }
                }
            };
            
            let data: Vec<serde_json::Value> = items
                .into_iter()
                .map(|item| serde_json::json!({
                    "id": item.id,
                    "name": item.name,
                    "description": item.description,
                    "item_type": item.item_type,
                    "is_unique": item.is_unique,
                    "properties": item.properties,
                }))
                .collect();
            ResponseResult::success(data)
        }
        
        // =========================================================================
        // Lore Operations
        // =========================================================================
        RequestPayload::ListLore { world_id: req_world_id } => {
            let world_uuid = match Uuid::parse_str(&req_world_id) {
                Ok(u) => wrldbldr_domain::WorldId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world_id") }),
            };
            
            match state.app.entities.lore.list_for_world(world_uuid).await {
                Ok(lore_list) => {
                    let data: Vec<serde_json::Value> = lore_list
                        .into_iter()
                        .map(|l| serde_json::json!({
                            "id": l.id.to_string(),
                            "worldId": l.world_id.to_string(),
                            "title": l.title,
                            "summary": l.summary,
                            "category": format!("{}", l.category),
                            "isCommonKnowledge": l.is_common_knowledge,
                            "tags": l.tags,
                            "chunkCount": l.chunks.len(),
                            "createdAt": l.created_at.to_rfc3339(),
                            "updatedAt": l.updated_at.to_rfc3339(),
                        }))
                        .collect();
                    ResponseResult::success(data)
                }
                Err(e) => ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
            }
        }
        
        RequestPayload::GetLore { lore_id } => {
            let lore_uuid = match Uuid::parse_str(&lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id") }),
            };
            
            match state.app.entities.lore.get(lore_uuid).await {
                Ok(Some(lore)) => {
                    let chunks: Vec<serde_json::Value> = lore.chunks
                        .iter()
                        .map(|c| serde_json::json!({
                            "id": c.id.to_string(),
                            "order": c.order,
                            "title": c.title,
                            "content": c.content,
                            "discoveryHint": c.discovery_hint,
                        }))
                        .collect();
                    
                    ResponseResult::success(serde_json::json!({
                        "id": lore.id.to_string(),
                        "worldId": lore.world_id.to_string(),
                        "title": lore.title,
                        "summary": lore.summary,
                        "category": format!("{}", lore.category),
                        "isCommonKnowledge": lore.is_common_knowledge,
                        "tags": lore.tags,
                        "chunks": chunks,
                        "createdAt": lore.created_at.to_rfc3339(),
                        "updatedAt": lore.updated_at.to_rfc3339(),
                    }))
                }
                Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Lore not found"),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
            }
        }
        
        RequestPayload::CreateLore { world_id, data } => {
            // DM authorization required for lore creation
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let world_uuid = match Uuid::parse_str(&world_id) {
                Ok(u) => wrldbldr_domain::WorldId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid world_id") }),
            };
            
            let category = data.category
                .as_deref()
                .unwrap_or("common")
                .parse::<wrldbldr_domain::LoreCategory>()
                .unwrap_or(wrldbldr_domain::LoreCategory::Common);
            
            let now = chrono::Utc::now();
            let mut lore = wrldbldr_domain::Lore::new(world_uuid, &data.title, category, now);
            
            if let Some(summary) = &data.summary {
                lore = lore.with_summary(summary);
            }
            if let Some(tags) = &data.tags {
                lore = lore.with_tags(tags.clone());
            }
            if data.is_common_knowledge.unwrap_or(false) {
                lore = lore.as_common_knowledge();
            }
            
            // Add chunks if provided
            if let Some(chunks) = &data.chunks {
                let mut domain_chunks = Vec::new();
                for (i, chunk_data) in chunks.iter().enumerate() {
                    let mut chunk = wrldbldr_domain::LoreChunk::new(&chunk_data.content)
                        .with_order(chunk_data.order.unwrap_or(i as u32));
                    if let Some(title) = &chunk_data.title {
                        chunk = chunk.with_title(title);
                    }
                    if let Some(hint) = &chunk_data.discovery_hint {
                        chunk = chunk.with_discovery_hint(hint);
                    }
                    domain_chunks.push(chunk);
                }
                lore = lore.with_chunks(domain_chunks);
            }
            
            match state.app.entities.lore.save(&lore).await {
                Ok(()) => ResponseResult::success(serde_json::json!({
                    "id": lore.id.to_string(),
                    "title": lore.title,
                })),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
            }
        }
        
        RequestPayload::UpdateLore { lore_id, data } => {
            // DM authorization required for lore updates
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let lore_uuid = match Uuid::parse_str(&lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id") }),
            };
            
            let mut lore = match state.app.entities.lore.get(lore_uuid).await {
                Ok(Some(l)) => l,
                Ok(None) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::NotFound, "Lore not found") }),
                Err(e) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()) }),
            };
            
            if let Some(title) = &data.title {
                lore.title = title.clone();
            }
            if let Some(summary) = &data.summary {
                lore.summary = summary.clone();
            }
            if let Some(category_str) = &data.category {
                if let Ok(cat) = category_str.parse::<wrldbldr_domain::LoreCategory>() {
                    lore.category = cat;
                }
            }
            if let Some(tags) = &data.tags {
                lore.tags = tags.clone();
            }
            if let Some(is_common) = data.is_common_knowledge {
                lore.is_common_knowledge = is_common;
            }
            lore.updated_at = chrono::Utc::now();
            
            match state.app.entities.lore.save(&lore).await {
                Ok(()) => ResponseResult::success(serde_json::json!({
                    "id": lore.id.to_string(),
                    "title": lore.title,
                })),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
            }
        }
        
        RequestPayload::DeleteLore { lore_id } => {
            // DM authorization required for lore deletion
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let lore_uuid = match Uuid::parse_str(&lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id") }),
            };
            
            match state.app.entities.lore.delete(lore_uuid).await {
                Ok(()) => ResponseResult::success(serde_json::json!({ "deleted": true })),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
            }
        }
        
        RequestPayload::AddLoreChunk { lore_id, data } => {
            // DM authorization required for adding lore chunks
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let lore_uuid = match Uuid::parse_str(&lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id") }),
            };
            
            let mut lore = match state.app.entities.lore.get(lore_uuid).await {
                Ok(Some(l)) => l,
                Ok(None) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::NotFound, "Lore not found") }),
                Err(e) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::InternalError, &e.to_string()) }),
            };
            
            let mut chunk = wrldbldr_domain::LoreChunk::new(&data.content)
                .with_order(data.order.unwrap_or(lore.chunks.len() as u32));
            if let Some(title) = &data.title {
                chunk = chunk.with_title(title);
            }
            if let Some(hint) = &data.discovery_hint {
                chunk = chunk.with_discovery_hint(hint);
            }
            
            let chunk_id = chunk.id.to_string();
            lore.chunks.push(chunk);
            lore.updated_at = chrono::Utc::now();
            
            match state.app.entities.lore.save(&lore).await {
                Ok(()) => ResponseResult::success(serde_json::json!({
                    "chunkId": chunk_id,
                })),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
            }
        }
        
        RequestPayload::UpdateLoreChunk { chunk_id, data } => {
            // DM authorization required for updating lore chunks
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let chunk_uuid = match Uuid::parse_str(&chunk_id) {
                Ok(u) => wrldbldr_domain::LoreChunkId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid chunk_id") }),
            };
            
            // Note: This would require finding which lore contains this chunk
            // For simplicity, we return not implemented for now
            // A proper implementation would need a repo method to find lore by chunk ID
            let _ = chunk_uuid;
            let _ = data;
            ResponseResult::error(ErrorCode::BadRequest, "UpdateLoreChunk requires finding parent lore - not yet implemented")
        }
        
        RequestPayload::DeleteLoreChunk { chunk_id } => {
            // DM authorization required for deleting lore chunks
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let chunk_uuid = match Uuid::parse_str(&chunk_id) {
                Ok(u) => wrldbldr_domain::LoreChunkId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid chunk_id") }),
            };
            
            // Note: Same as UpdateLoreChunk - requires finding parent lore
            let _ = chunk_uuid;
            ResponseResult::error(ErrorCode::BadRequest, "DeleteLoreChunk requires finding parent lore - not yet implemented")
        }
        
        RequestPayload::GrantLoreKnowledge { character_id, lore_id, chunk_ids, discovery_source } => {
            // DM authorization required for granting lore knowledge
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let char_uuid = match Uuid::parse_str(&character_id) {
                Ok(u) => wrldbldr_domain::CharacterId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid character_id") }),
            };
            let lore_uuid = match Uuid::parse_str(&lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id") }),
            };
            
            // Convert discovery source from protocol to domain
            let domain_source = match discovery_source {
                wrldbldr_protocol::types::LoreDiscoverySourceData::ReadBook { book_name } => {
                    wrldbldr_domain::LoreDiscoverySource::ReadBook { book_name }
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::Conversation { npc_id, npc_name } => {
                    let npc_uuid = Uuid::parse_str(&npc_id)
                        .map(wrldbldr_domain::CharacterId::from_uuid)
                        .unwrap_or_else(|_| wrldbldr_domain::CharacterId::new());
                    wrldbldr_domain::LoreDiscoverySource::Conversation { npc_id: npc_uuid, npc_name }
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::Investigation => {
                    wrldbldr_domain::LoreDiscoverySource::Investigation
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::DmGranted { reason } => {
                    wrldbldr_domain::LoreDiscoverySource::DmGranted { reason }
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::CommonKnowledge => {
                    wrldbldr_domain::LoreDiscoverySource::CommonKnowledge
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::LlmDiscovered { context } => {
                    wrldbldr_domain::LoreDiscoverySource::LlmDiscovered { context }
                }
                wrldbldr_protocol::types::LoreDiscoverySourceData::Unknown => {
                    // Default to DM granted for unknown source types
                    wrldbldr_domain::LoreDiscoverySource::DmGranted { reason: Some("Unknown source type".to_string()) }
                }
            };
            
            let now = chrono::Utc::now();
            let knowledge = if let Some(ids) = chunk_ids {
                let chunk_uuids: Vec<wrldbldr_domain::LoreChunkId> = ids
                    .iter()
                    .filter_map(|id| Uuid::parse_str(id).ok().map(wrldbldr_domain::LoreChunkId::from_uuid))
                    .collect();
                wrldbldr_domain::LoreKnowledge::partial(lore_uuid, char_uuid, chunk_uuids, domain_source, now)
            } else {
                wrldbldr_domain::LoreKnowledge::full(lore_uuid, char_uuid, domain_source, now)
            };
            
            match state.app.entities.lore.grant_knowledge(&knowledge).await {
                Ok(()) => ResponseResult::success(serde_json::json!({ "granted": true })),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
            }
        }
        
        RequestPayload::RevokeLoreKnowledge { character_id, lore_id, chunk_ids: _ } => {
            // DM authorization required for revoking lore knowledge
            if let Err(e) = require_dm_for_request(&_conn_info, &request_id) {
                return Some(e);
            }
            
            let char_uuid = match Uuid::parse_str(&character_id) {
                Ok(u) => wrldbldr_domain::CharacterId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid character_id") }),
            };
            let lore_uuid = match Uuid::parse_str(&lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id") }),
            };
            
            // Note: chunk_ids for partial revocation would need additional repo support
            match state.app.entities.lore.revoke_knowledge(char_uuid, lore_uuid).await {
                Ok(()) => ResponseResult::success(serde_json::json!({ "revoked": true })),
                Err(e) => ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
            }
        }
        
        RequestPayload::GetCharacterLore { character_id } => {
            let char_uuid = match Uuid::parse_str(&character_id) {
                Ok(u) => wrldbldr_domain::CharacterId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid character_id") }),
            };
            
            match state.app.entities.lore.get_character_knowledge(char_uuid).await {
                Ok(knowledge_list) => {
                    let data: Vec<serde_json::Value> = knowledge_list
                        .into_iter()
                        .map(|k| serde_json::json!({
                            "loreId": k.lore_id.to_string(),
                            "characterId": k.character_id.to_string(),
                            "knownChunkIds": k.known_chunk_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                            "discoveredAt": k.discovered_at.to_rfc3339(),
                            "notes": k.notes,
                        }))
                        .collect();
                    ResponseResult::success(data)
                }
                Err(e) => ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
            }
        }
        
        RequestPayload::GetLoreKnowers { lore_id } => {
            let lore_uuid = match Uuid::parse_str(&lore_id) {
                Ok(u) => wrldbldr_domain::LoreId::from_uuid(u),
                Err(_) => return Some(ServerMessage::Response { request_id: request_id.clone(), result: ResponseResult::error(ErrorCode::BadRequest, "Invalid lore_id") }),
            };
            
            match state.app.entities.lore.get_knowledge_for_lore(lore_uuid).await {
                Ok(knowledge_list) => {
                    let data: Vec<serde_json::Value> = knowledge_list
                        .into_iter()
                        .map(|k| serde_json::json!({
                            "characterId": k.character_id.to_string(),
                            "knownChunkIds": k.known_chunk_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                            "discoveredAt": k.discovered_at.to_rfc3339(),
                        }))
                        .collect();
                    ResponseResult::success(data)
                }
                Err(e) => ResponseResult::error(ErrorCode::InternalError, &e.to_string()),
            }
        }
        
        // =========================================================================
        // Trigger Schema (for Visual Trigger Builder)
        // =========================================================================
        RequestPayload::GetTriggerSchema => {
            let schema = wrldbldr_protocol::TriggerSchema::generate();
            ResponseResult::success(schema)
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
    match state.app.use_cases.challenge.roll.execute(
        world_id,
        challenge_uuid,
        pc_id,
        Some(roll),
        0, // No modifier for legacy roll
    ).await {
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
                        outcome_triggers: result.outcome_triggers.iter().map(|t| {
                            wrldbldr_protocol::ProposedToolInfo {
                                id: t.id.clone(),
                                name: t.name.clone(),
                                description: t.description.clone(),
                                arguments: t.arguments.clone(),
                            }
                        }).collect(),
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
    match state.app.use_cases.challenge.roll.execute(
        world_id,
        challenge_uuid,
        pc_id,
        client_roll,
        modifier,
    ).await {
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
                        outcome_triggers: result.outcome_triggers.iter().map(|t| {
                            wrldbldr_protocol::ProposedToolInfo {
                                id: t.id.clone(),
                                name: t.name.clone(),
                                description: t.description.clone(),
                                arguments: t.arguments.clone(),
                            }
                        }).collect(),
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
    
    // Parse request_id as region_id (the request_id is typically the region being staged)
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
    let location_id = region.location_id;
    
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
            // Store selected visual states if provided
            if let Some(loc_state_str) = &location_state_id {
                if let Ok(loc_uuid) = Uuid::parse_str(loc_state_str) {
                    let loc_state_id = wrldbldr_domain::LocationStateId::from_uuid(loc_uuid);
                    if let Err(e) = state.app.entities.location_state.set_active(location_id, loc_state_id).await {
                        tracing::warn!(error = %e, "Failed to set active location state");
                    }
                }
            }
            
            if let Some(reg_state_str) = &region_state_id {
                if let Ok(reg_uuid) = Uuid::parse_str(reg_state_str) {
                    let reg_state_id = wrldbldr_domain::RegionStateId::from_uuid(reg_uuid);
                    if let Err(e) = state.app.entities.region_state.set_active(region_id, reg_state_id).await {
                        tracing::warn!(error = %e, "Failed to set active region state");
                    }
                }
            }
            
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
                
                // Fetch active visual states for the response
                let visual_state = build_visual_state_for_staging(state, location_id, region_id).await;
                
                // Broadcast StagingReady to all players in the world
                let staging_ready = ServerMessage::StagingReady {
                    region_id: request_id.clone(),
                    npcs_present,
                    visual_state,
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
    
    if let Err(e) = require_dm(&conn_info) {
        return Some(e);
    }
    
    // Parse the request_id as a RegionId (it's the region being staged)
    let region_id = match parse_region_id(&request_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    
    // Get region info for the LLM prompt
    let region = match state.app.entities.location.get_region(region_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return Some(error_response("NOT_FOUND", "Region not found")),
        Err(e) => return Some(error_response("REPO_ERROR", &e.to_string())),
    };
    
    // Get location name
    let location_name = state.app.entities.location
        .get_location(region.location_id)
        .await
        .ok()
        .flatten()
        .map(|l| l.name)
        .unwrap_or_else(|| "Unknown Location".to_string());
    
    tracing::info!(
        request_id = %request_id,
        region_id = %region_id,
        region_name = %region.name,
        guidance = %guidance,
        "Staging regeneration requested - calling LLM"
    );
    
    // Generate LLM-based suggestions with the DM's guidance
    let guidance_opt = if guidance.is_empty() { None } else { Some(guidance.as_str()) };
    let llm_based_npcs = generate_llm_based_suggestions(
        state,
        region_id,
        &region.name,
        &location_name,
        guidance_opt,
    ).await;
    
    tracing::info!(
        request_id = %request_id,
        npc_count = llm_based_npcs.len(),
        "Generated LLM staging suggestions"
    );
    
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
    _location_state_id: Option<String>,
    _region_state_id: Option<String>,
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
    match state.app.use_cases.approval.approve_suggestion.execute(approval_id, domain_decision).await {
        Ok(result) => {
            if result.approved {
                if let Some(world_id) = conn_info.world_id {
                    let dialogue = result.final_dialogue.clone().unwrap_or_default();
                    
                    // Record dialogue exchange to story events for persistence
                    if !dialogue.is_empty() {
                        if let Some(ref data) = approval_data {
                            if let Some(pc_id) = data.pc_id {
                                if let Some(npc_id) = data.npc_id {
                                    let player_dialogue = data.player_dialogue.clone().unwrap_or_default();
                                    if let Err(e) = state.app.entities.narrative.record_dialogue_exchange(
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
                                    ).await {
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
                        state.connections.broadcast_to_world(world_id, dialogue_msg).await;
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
        None => return Some(error_response("INVALID_DATA", "No challenge outcome data in approval request")),
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
            let pc_id = approval_data.pc_id;
            
            // Execute outcome triggers with PC context
            if let Err(e) = state.app.use_cases.challenge.resolve.execute_for_pc(
                challenge_id, 
                outcome_type.clone(),
                pc_id
            ).await {
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
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Edit { modified_description } => {
            // Get PC ID for trigger execution
            let pc_id = approval_data.pc_id;
            
            tracing::info!(
                challenge_id = %challenge_id,
                modified_description = %modified_description,
                "DM edited challenge outcome description"
            );
            
            // Execute outcome triggers with PC context
            if let Err(e) = state.app.use_cases.challenge.resolve.execute_for_pc(
                challenge_id, 
                outcome_type.clone(),
                pc_id
            ).await {
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
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Unknown => {
            Some(error_response("INVALID_DECISION", "Unknown challenge outcome decision type"))
        }
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
    let approval_data = state.app.queue.get_approval_request(approval_id).await.ok().flatten();
    
    match state.app.use_cases.approval.approve_suggestion.execute(approval_id, decision).await {
        Ok(_) => {
            if approved {
                if let Some(world_id) = conn_info.world_id {
                    // Parse the event ID to fetch the narrative event
                    let narrative_event_id = match parse_id(&event_id, NarrativeEventId::from_uuid, "Invalid event ID") {
                        Ok(id) => id,
                        Err(e) => return Some(e),
                    };
                    
                    // Fetch the narrative event
                    let event = match state.app.entities.narrative.get_event(narrative_event_id).await {
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
                        .or_else(|| approval_data.as_ref()
                            .and_then(|d| d.narrative_event_suggestion.as_ref())
                            .and_then(|s| s.suggested_outcome.clone()))
                        .or_else(|| event.default_outcome.clone())
                        .unwrap_or_else(|| event.outcomes.first().map(|o| o.name.clone()).unwrap_or_default());
                    
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
                                    current_scene_id: approval_data.as_ref().and_then(|d| d.scene_id),
                                };
                                
                                let summary = state.app.use_cases.narrative.execute_effects.execute(
                                    narrative_event_id,
                                    outcome_name.clone(),
                                    &outcome.effects,
                                    &context,
                                ).await;
                                
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
                        outcome_description: outcome.map(|o| o.description.clone()).unwrap_or_default(),
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
    
    if let Err(e) = state.app.entities.observation.save_observation(&observation).await {
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
    state.connections.broadcast_to_dms(world_id, queue_msg).await;
    
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
        state.connections.broadcast_to_world(world_id_typed, update_msg).await;
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
        _ => return Some(error_response("INVALID_PERIOD", "Use: morning, afternoon, evening, night")),
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
    
    let reason = wrldbldr_domain::TimeAdvanceReason::DmSkipToPeriod { period: target_period };
    let advance_data = crate::use_cases::time::build_time_advance_data(
        &previous_time,
        &world.game_time,
        minutes_until,
        &reason,
    );
    let update_msg = ServerMessage::GameTimeAdvanced { data: advance_data };
    state.connections.broadcast_to_world(world_id_typed, update_msg).await;
    
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
    state.connections.broadcast_to_world(world_id_typed, update_msg).await;
    
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
    
    let domain_mode = match mode {
        wrldbldr_protocol::types::TimeMode::Manual => wrldbldr_domain::TimeMode::Manual,
        wrldbldr_protocol::types::TimeMode::Suggested => wrldbldr_domain::TimeMode::Suggested,
        wrldbldr_protocol::types::TimeMode::Auto => wrldbldr_domain::TimeMode::Auto,
    };
    
    world.time_config.mode = domain_mode;
    world.updated_at = chrono::Utc::now();
    
    if let Err(e) = state.app.entities.world.save(&world).await {
        return Some(error_response("DATABASE_ERROR", &e.to_string()));
    }
    
    let update_msg = ServerMessage::TimeModeChanged {
        world_id: world_id_typed.to_string(),
        mode,
    };
    state.connections.broadcast_to_world(world_id_typed, update_msg).await;
    
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
    
    // TODO: Retrieve the pending suggestion from storage
    // For now, we'll log the decision
    tracing::info!(
        world_id = %world_id,
        suggestion_id = %suggestion_id,
        decision = ?decision,
        "Time suggestion response (storage not yet implemented)"
    );
    
    // In a full implementation:
    // 1. Look up the pending TimeSuggestion by ID
    // 2. Based on decision:
    //    - Approve: Advance time by suggested_minutes
    //    - Modify: Advance time by modified minutes
    //    - Skip: Do nothing
    // 3. Remove the suggestion from pending storage
    // 4. Broadcast GameTimeAdvanced if time was advanced
    
    match decision {
        wrldbldr_protocol::types::TimeSuggestionDecision::Skip => {
            tracing::debug!("Time suggestion skipped");
        }
        wrldbldr_protocol::types::TimeSuggestionDecision::Approve => {
            tracing::debug!("Time suggestion approved (would advance time)");
        }
        wrldbldr_protocol::types::TimeSuggestionDecision::Modify { minutes } => {
            tracing::debug!("Time suggestion modified to {} minutes", minutes);
        }
    }
    
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
    inventory_entity: &crate::entities::Inventory,
    region_id: RegionId,
) -> Vec<wrldbldr_protocol::RegionItemData> {
    match inventory_entity.list_in_region(region_id).await {
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

/// Parse a player character ID from a string.
fn parse_pc_id(id_str: &str) -> Result<PlayerCharacterId, ServerMessage> {
    parse_id(id_str, PlayerCharacterId::from_uuid, "Invalid PC ID format")
}

/// Parse a character ID from a string.
fn parse_character_id(id_str: &str) -> Result<CharacterId, ServerMessage> {
    parse_id(id_str, CharacterId::from_uuid, "Invalid character ID format")
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
    parse_id(id_str, ChallengeId::from_uuid, "Invalid challenge ID format")
}

/// Verify that the connection has DM authorization, returning an error response if not.
fn require_dm(conn_info: &super::connections::ConnectionInfo) -> Result<(), ServerMessage> {
    if conn_info.is_dm() {
        Ok(())
    } else {
        Err(error_response("UNAUTHORIZED", "Only DMs can perform this action"))
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
            result: ResponseResult::error(ErrorCode::Unauthorized, "Only DMs can perform this action"),
        })
    }
}

/// Parse a UUID from a string for Request/Response pattern.
fn parse_uuid_for_request(id_str: &str, request_id: &str, error_msg: &str) -> Result<Uuid, ServerMessage> {
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
fn parse_character_id_for_request(id_str: &str, request_id: &str) -> Result<CharacterId, ServerMessage> {
    parse_id_for_request(id_str, request_id, CharacterId::from_uuid, "Invalid character ID")
}

/// Parse a region ID for Request/Response pattern.
fn parse_region_id_for_request(id_str: &str, request_id: &str) -> Result<RegionId, ServerMessage> {
    parse_id_for_request(id_str, request_id, RegionId::from_uuid, "Invalid region ID")
}

/// Parse a location ID for Request/Response pattern.
fn parse_location_id_for_request(id_str: &str, request_id: &str) -> Result<LocationId, ServerMessage> {
    parse_id_for_request(id_str, request_id, LocationId::from_uuid, "Invalid location ID")
}

/// Parse an item ID for Request/Response pattern.
fn parse_item_id_for_request(id_str: &str, request_id: &str) -> Result<ItemId, ServerMessage> {
    parse_id_for_request(id_str, request_id, ItemId::from_uuid, "Invalid item ID")
}

/// Parse a challenge ID for Request/Response pattern.
fn parse_challenge_id_for_request(id_str: &str, request_id: &str) -> Result<ChallengeId, ServerMessage> {
    parse_id_for_request(id_str, request_id, ChallengeId::from_uuid, "Invalid challenge ID")
}
