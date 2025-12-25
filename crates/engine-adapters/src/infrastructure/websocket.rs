//! WebSocket handler for Player connections
//!
//! Message types are aligned between Engine and Player for seamless communication.

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use chrono::Timelike;

use wrldbldr_engine_app::application::dto::{AdHocOutcomesDto, ChallengeOutcomeDecision, DMAction};
use wrldbldr_engine_app::application::services::scene_service::SceneService;
use wrldbldr_engine_app::application::services::scene_resolution_service::SceneResolutionService;
use wrldbldr_engine_app::application::services::player_character_service::PlayerCharacterService;
use wrldbldr_engine_app::application::services::location_service::LocationService;
use wrldbldr_engine_app::application::services::interaction_service::InteractionService;
use wrldbldr_engine_app::application::services::session_join_service as sjs;
use wrldbldr_engine_app::application::services::challenge_resolution_service as crs;
use wrldbldr_engine_app::application::services::MoodService;
use wrldbldr_engine_app::application::services::{
    ActantialContextService, CreateWantRequest, UpdateWantRequest, ActorTargetType,
};
use wrldbldr_engine_ports::outbound::{PlayerCharacterRepositoryPort, RegionRepositoryPort, SessionParticipantRole};
use crate::infrastructure::session::ClientId;
use crate::infrastructure::state::AppState;
use wrldbldr_domain::ActionId;
use wrldbldr_protocol::{
    CharacterData, CharacterPosition, ClientMessage, InteractionData, NpcMoodData, ParticipantInfo,
    ParticipantRole, SceneData, ServerMessage,
    // Actantial Model types (P1.5)
    WantData, WantTargetData, CreateWantData, UpdateWantData,
    ActantialActorData, ActantialViewData,
    NpcActantialContextData, SocialViewsData, SocialRelationData,
    GoalData,
    WantVisibilityData, ActorTypeData, ActantialRoleData, WantTargetTypeData,
};

// Conversion helpers for adapting between infrastructure message types and service DTOs

/// Convert wire format ParticipantRole to canonical SessionParticipantRole
fn wire_to_canonical_role(role: ParticipantRole) -> SessionParticipantRole {
    match role {
        ParticipantRole::DungeonMaster => SessionParticipantRole::DungeonMaster,
        ParticipantRole::Player => SessionParticipantRole::Player,
        ParticipantRole::Spectator => SessionParticipantRole::Spectator,
    }
}

/// Convert canonical SessionParticipantRole to wire format ParticipantRole
fn canonical_to_wire_role(role: SessionParticipantRole) -> ParticipantRole {
    match role {
        SessionParticipantRole::DungeonMaster => ParticipantRole::DungeonMaster,
        SessionParticipantRole::Player => ParticipantRole::Player,
        SessionParticipantRole::Spectator => ParticipantRole::Spectator,
    }
}

/// Convert session_join_service::ParticipantInfo to wire format wrldbldr_protocol::ParticipantInfo
fn from_service_participant(p: sjs::ParticipantInfo) -> ParticipantInfo {
    ParticipantInfo {
        user_id: p.user_id,
        role: canonical_to_wire_role(p.role),
        character_name: p.character_name,
    }
}

/// Convert wrldbldr_protocol::DiceInputType to challenge_resolution_service::DiceInputType
fn to_service_dice_input(input: wrldbldr_protocol::DiceInputType) -> crs::DiceInputType {
    match input {
        wrldbldr_protocol::DiceInputType::Formula(f) => crs::DiceInputType::Formula(f),
        wrldbldr_protocol::DiceInputType::Manual(v) => crs::DiceInputType::Manual(v),
    }
}

/// Convert wrldbldr_protocol::AdHocOutcomes to application dto AdHocOutcomesDto
fn to_adhoc_outcomes_dto(outcomes: wrldbldr_protocol::AdHocOutcomes) -> AdHocOutcomesDto {
    AdHocOutcomesDto {
        success: outcomes.success,
        failure: outcomes.failure,
        critical_success: outcomes.critical_success,
        critical_failure: outcomes.critical_failure,
    }
}

/// Try to deserialize a serde_json::Value into a ServerMessage
fn value_to_server_message(value: serde_json::Value) -> Option<ServerMessage> {
    serde_json::from_value(value).ok()
}

/// Convert wire format ChallengeOutcomeDecisionData to application DTO ChallengeOutcomeDecision
fn to_challenge_outcome_decision(decision: wrldbldr_protocol::ChallengeOutcomeDecisionData) -> ChallengeOutcomeDecision {
    match decision {
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Accept => ChallengeOutcomeDecision::Accept,
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Edit { modified_description } => {
            ChallengeOutcomeDecision::Edit { modified_description }
        }
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Suggest { guidance } => {
            ChallengeOutcomeDecision::Suggest { guidance }
        }
    }
}

// =============================================================================
// Actantial Model Conversion Helpers (P1.5)
// =============================================================================

/// Convert WantVisibilityData to domain WantVisibility
fn to_domain_visibility(v: WantVisibilityData) -> wrldbldr_domain::entities::WantVisibility {
    match v {
        WantVisibilityData::Known => wrldbldr_domain::entities::WantVisibility::Known,
        WantVisibilityData::Suspected => wrldbldr_domain::entities::WantVisibility::Suspected,
        WantVisibilityData::Hidden => wrldbldr_domain::entities::WantVisibility::Hidden,
    }
}

/// Convert domain WantVisibility to WantVisibilityData
fn from_domain_visibility(v: wrldbldr_domain::entities::WantVisibility) -> WantVisibilityData {
    match v {
        wrldbldr_domain::entities::WantVisibility::Known => WantVisibilityData::Known,
        wrldbldr_domain::entities::WantVisibility::Suspected => WantVisibilityData::Suspected,
        wrldbldr_domain::entities::WantVisibility::Hidden => WantVisibilityData::Hidden,
    }
}

/// Convert ActantialRoleData to domain ActantialRole
fn to_domain_role(r: ActantialRoleData) -> wrldbldr_domain::entities::ActantialRole {
    match r {
        ActantialRoleData::Helper => wrldbldr_domain::entities::ActantialRole::Helper,
        ActantialRoleData::Opponent => wrldbldr_domain::entities::ActantialRole::Opponent,
        ActantialRoleData::Sender => wrldbldr_domain::entities::ActantialRole::Sender,
        ActantialRoleData::Receiver => wrldbldr_domain::entities::ActantialRole::Receiver,
    }
}

/// Convert domain ActantialRole to ActantialRoleData
fn from_domain_role(r: wrldbldr_domain::entities::ActantialRole) -> ActantialRoleData {
    match r {
        wrldbldr_domain::entities::ActantialRole::Helper => ActantialRoleData::Helper,
        wrldbldr_domain::entities::ActantialRole::Opponent => ActantialRoleData::Opponent,
        wrldbldr_domain::entities::ActantialRole::Sender => ActantialRoleData::Sender,
        wrldbldr_domain::entities::ActantialRole::Receiver => ActantialRoleData::Receiver,
    }
}

/// Convert ActorTypeData to service ActorTargetType
fn to_service_actor_type(t: ActorTypeData) -> ActorTargetType {
    match t {
        ActorTypeData::Npc => ActorTargetType::Npc,
        ActorTypeData::Pc => ActorTargetType::Pc,
    }
}

/// Convert ActorTargetType to ActorTypeData
fn from_service_actor_type(t: ActorTargetType) -> ActorTypeData {
    match t {
        ActorTargetType::Npc => ActorTypeData::Npc,
        ActorTargetType::Pc => ActorTypeData::Pc,
    }
}

/// Convert WantTargetTypeData to string for service
fn target_type_to_string(t: WantTargetTypeData) -> String {
    match t {
        WantTargetTypeData::Character => "Character".to_string(),
        WantTargetTypeData::Item => "Item".to_string(),
        WantTargetTypeData::Goal => "Goal".to_string(),
    }
}

/// Convert WantContext to WantData for protocol
fn want_context_to_data(w: &wrldbldr_domain::value_objects::WantContext) -> WantData {
    let target = w.target.as_ref().map(|t| {
        let (target_type, description) = match t {
            wrldbldr_domain::value_objects::WantTarget::Character { .. } => {
                (WantTargetTypeData::Character, None)
            }
            wrldbldr_domain::value_objects::WantTarget::Item { .. } => {
                (WantTargetTypeData::Item, None)
            }
            wrldbldr_domain::value_objects::WantTarget::Goal { description, .. } => {
                (WantTargetTypeData::Goal, description.clone())
            }
        };
        WantTargetData {
            id: t.id().to_string(),
            name: t.name().to_string(),
            target_type,
            description,
        }
    });

    let helpers: Vec<ActantialActorData> = w.helpers.iter().map(|a| ActantialActorData {
        id: a.target.id_string(),
        name: a.name.clone(),
        actor_type: match a.target {
            wrldbldr_domain::value_objects::ActantialTarget::Npc(_) => ActorTypeData::Npc,
            wrldbldr_domain::value_objects::ActantialTarget::Pc(_) => ActorTypeData::Pc,
        },
        reason: a.reason.clone(),
    }).collect();

    let opponents: Vec<ActantialActorData> = w.opponents.iter().map(|a| ActantialActorData {
        id: a.target.id_string(),
        name: a.name.clone(),
        actor_type: match a.target {
            wrldbldr_domain::value_objects::ActantialTarget::Npc(_) => ActorTypeData::Npc,
            wrldbldr_domain::value_objects::ActantialTarget::Pc(_) => ActorTypeData::Pc,
        },
        reason: a.reason.clone(),
    }).collect();

    let sender = w.sender.as_ref().map(|a| ActantialActorData {
        id: a.target.id_string(),
        name: a.name.clone(),
        actor_type: match a.target {
            wrldbldr_domain::value_objects::ActantialTarget::Npc(_) => ActorTypeData::Npc,
            wrldbldr_domain::value_objects::ActantialTarget::Pc(_) => ActorTypeData::Pc,
        },
        reason: a.reason.clone(),
    });

    let receiver = w.receiver.as_ref().map(|a| ActantialActorData {
        id: a.target.id_string(),
        name: a.name.clone(),
        actor_type: match a.target {
            wrldbldr_domain::value_objects::ActantialTarget::Npc(_) => ActorTypeData::Npc,
            wrldbldr_domain::value_objects::ActantialTarget::Pc(_) => ActorTypeData::Pc,
        },
        reason: a.reason.clone(),
    });

    WantData {
        id: w.want_id.to_string(),
        description: w.description.clone(),
        intensity: w.intensity,
        priority: w.priority,
        visibility: from_domain_visibility(w.visibility),
        target,
        deflection_behavior: w.deflection_behavior.clone(),
        tells: w.tells.clone(),
        helpers,
        opponents,
        sender,
        receiver,
    }
}

/// Convert ActantialContext to NpcActantialContextData
fn actantial_context_to_data(ctx: &wrldbldr_domain::value_objects::ActantialContext) -> NpcActantialContextData {
    let wants: Vec<WantData> = ctx.wants.iter().map(want_context_to_data).collect();

    let allies: Vec<SocialRelationData> = ctx.social_views.allies.iter()
        .map(|(target, name, reasons)| SocialRelationData {
            id: target.id_string(),
            name: name.clone(),
            actor_type: match target {
                wrldbldr_domain::value_objects::ActantialTarget::Npc(_) => ActorTypeData::Npc,
                wrldbldr_domain::value_objects::ActantialTarget::Pc(_) => ActorTypeData::Pc,
            },
            reasons: reasons.clone(),
        })
        .collect();

    let enemies: Vec<SocialRelationData> = ctx.social_views.enemies.iter()
        .map(|(target, name, reasons)| SocialRelationData {
            id: target.id_string(),
            name: name.clone(),
            actor_type: match target {
                wrldbldr_domain::value_objects::ActantialTarget::Npc(_) => ActorTypeData::Npc,
                wrldbldr_domain::value_objects::ActantialTarget::Pc(_) => ActorTypeData::Pc,
            },
            reasons: reasons.clone(),
        })
        .collect();

    NpcActantialContextData {
        npc_id: ctx.character_id.to_string(),
        npc_name: ctx.character_name.clone(),
        wants,
        social_views: SocialViewsData { allies, enemies },
    }
}

/// Convert Goal to GoalData
fn goal_to_data(g: &wrldbldr_domain::entities::Goal) -> GoalData {
    GoalData {
        id: g.id.to_string(),
        name: g.name.clone(),
        description: g.description.clone(),
        usage_count: 0, // Will be filled in by caller if needed
    }
}

/// Fetch region items and convert to protocol format
async fn fetch_region_items(
    state: &AppState,
    region_id: wrldbldr_domain::RegionId,
) -> Vec<wrldbldr_protocol::RegionItemData> {
    match state.repository.regions().get_region_items(region_id).await {
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
            tracing::warn!(
                region_id = %region_id,
                error = %e,
                "Failed to fetch region items for SceneChanged"
            );
            vec![]
        }
    }
}

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create a unique client ID for this connection
    let client_id = ClientId::new();

    // Create a channel for sending messages to this client
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    tracing::info!("New WebSocket connection established: {}", client_id);

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
            Ok(Message::Text(text)) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(msg) => {
                    if let Some(response) = handle_message(msg, &state, client_id, tx.clone()).await
                    {
                        if tx.send(response).is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse message: {}", e);
                    let error = ServerMessage::Error {
                        code: "PARSE_ERROR".to_string(),
                        message: format!("Invalid message format: {}", e),
                    };
                    if tx.send(error).is_err() {
                        break;
                    }
                }
            },
            Ok(Message::Close(_)) => {
                tracing::info!("WebSocket connection closed by client: {}", client_id);
                break;
            }
            Ok(Message::Ping(data)) => {
                // Ping/Pong is handled by the send task through the channel
                let _ = tx.send(ServerMessage::Pong);
                let _ = data; // Acknowledge we received the ping data
            }
            Err(e) => {
                tracing::error!("WebSocket error for client {}: {}", client_id, e);
                break;
            }
            _ => {}
        }
    }

    // Clean up: remove client from session via async port
    if let Some((session_id, participant)) = state.async_session_port.client_leave_session(&client_id.to_string()).await {
        tracing::info!(
            "Client {} (user: {}) disconnected from session {}",
            client_id,
            participant.user_id,
            session_id
        );
    }

    // Cancel the send task
    send_task.abort();

    tracing::info!("WebSocket connection terminated: {}", client_id);
}

// NPC presence determination is now in domain::value_objects::region::RegionRelationshipType::is_npc_present

/// Handle a parsed client message
async fn handle_message(
    msg: ClientMessage,
    state: &AppState,
    client_id: ClientId,
    sender: mpsc::UnboundedSender<ServerMessage>,
) -> Option<ServerMessage> {
    match msg {
        ClientMessage::Heartbeat => Some(ServerMessage::Pong),

        ClientMessage::CheckComfyUIHealth => {
            // Trigger manual ComfyUI health check
            let comfyui_client = state.comfyui_client.clone();
            let async_session_port = state.async_session_port.clone();
            
            tokio::spawn(async move {
                let (state_str, message) = match comfyui_client.health_check().await {
                    Ok(true) => ("connected".to_string(), None),
                    Ok(false) => ("disconnected".to_string(), Some("ComfyUI is not responding".to_string())),
                    Err(e) => ("disconnected".to_string(), Some(format!("Health check failed: {}", e))),
                };
                
                // Broadcast to all connected clients
                let msg = ServerMessage::ComfyUIStateChanged {
                    state: state_str,
                    message,
                    retry_in_seconds: None,
                };
                
                // Get all session IDs and broadcast
                let session_ids = async_session_port.list_session_ids().await;
                for session_id in session_ids {
                    let _ = async_session_port.broadcast_to_session(
                        session_id,
                        serde_json::to_value(&msg).unwrap_or_default(),
                    ).await;
                }
            });
            
            None // Response sent asynchronously
        }

        ClientMessage::JoinSession {
            user_id,
            role,
            world_id,
        } => {
            tracing::info!(
                "User {} joining as {:?}, world: {:?}",
                user_id,
                role,
                world_id
            );

            // Create a JSON-value sender that forwards to the ServerMessage sender
            let (json_tx, mut json_rx) = mpsc::unbounded_channel::<serde_json::Value>();
            let server_tx = sender.clone();
            tokio::spawn(async move {
                while let Some(value) = json_rx.recv().await {
                    if let Ok(msg) = serde_json::from_value::<ServerMessage>(value) {
                        let _ = server_tx.send(msg);
                    }
                }
            });

            // Delegate to injected SessionJoinService to join or create a session
            match state.player.session_join_service.join_or_create_session_for_world(
                client_id.to_string(),
                user_id.clone(),
                wire_to_canonical_role(role),
                world_id,
                json_tx,
            )
            .await
            {
                Ok(session_joined_info) => {
                    // Broadcast PlayerJoined to other participants via async port
                    // Note: character_name is None at join time; it's set when player selects a character
                    let player_joined_msg = ServerMessage::PlayerJoined {
                        user_id: user_id.clone(),
                        role,
                        character_name: None,
                    };
                    if let Ok(msg_json) = serde_json::to_value(&player_joined_msg) {
                        let _ = state.async_session_port.broadcast_except(
                            session_joined_info.session_id,
                            msg_json,
                            &client_id.to_string(),
                        ).await;
                    }

                    // Convert service's ParticipantInfo to wrldbldr_protocol::ParticipantInfo
                    let participants: Vec<ParticipantInfo> = session_joined_info
                        .participants
                        .into_iter()
                        .map(from_service_participant)
                        .collect();

                    Some(ServerMessage::SessionJoined {
                        session_id: session_joined_info.session_id.to_string(),
                        role,
                        participants,
                        world_snapshot: session_joined_info.world_snapshot,
                    })
                }
                Err(e) => {
                    tracing::error!("Failed to join session: {}", e);
                    Some(ServerMessage::Error {
                        code: "SESSION_ERROR".to_string(),
                        message: format!("Failed to join session: {}", e),
                    })
                }
            }
        }

        ClientMessage::PlayerAction {
            action_type,
            target,
            dialogue,
        } => {
            tracing::debug!("Received player action: {} -> {:?}", action_type, target);

            // Generate a unique action ID for tracking
            let action_id = ActionId::new();
            let action_id_str = action_id.to_string();

            // Get the client's session and user info via async port
            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    tracing::warn!("Client {} sent action but is not in any session", client_id);
                    return Some(ServerMessage::Error {
                        code: "NOT_IN_SESSION".to_string(),
                        message: "You must join a session before performing actions".to_string(),
                    });
                }
            };
            let player_id = state.async_session_port.get_client_user_id(&client_id_str).await
                .unwrap_or_else(|| "unknown".to_string());

            // Handle Travel actions immediately (update location and resolve scene)
            if action_type == "travel" {
                if let Some(location_id_str) = target.as_ref() {
                    // Parse location ID
                    let location_uuid = match uuid::Uuid::parse_str(location_id_str) {
                        Ok(uuid) => wrldbldr_domain::LocationId::from_uuid(uuid),
                        Err(_) => {
                            return Some(ServerMessage::Error {
                                code: "INVALID_LOCATION_ID".to_string(),
                                message: "Invalid location ID format".to_string(),
                            });
                        }
                    };

                    // Get PC for this user
                    match state
                .player.player_character_service
                        .get_pc_by_user_and_session(&player_id, session_id)
                        .await
                    {
                        Ok(Some(pc)) => {
                            // Update PC location
                            if let Err(e) = state
                .player.player_character_service
                                .update_pc_location(pc.id, location_uuid)
                                .await
                            {
                                tracing::error!("Failed to update PC location: {}", e);
                                return Some(ServerMessage::Error {
                                    code: "LOCATION_UPDATE_FAILED".to_string(),
                                    message: format!("Failed to update location: {}", e),
                                });
                            }

                            // Resolve scene for the new location
                            match state
                .player.scene_resolution_service
                                .resolve_scene_for_pc(pc.id)
                                .await
                            {
                                Ok(Some(scene)) => {
                                    // Load scene with relations to build SceneUpdate
                                    match state.core.scene_service.get_scene_with_relations(scene.id).await {
                                        Ok(Some(scene_with_relations)) => {
                                            // Load interactions for the scene
                                            let interaction_templates = match state.core.interaction_service.list_interactions(scene.id).await {
                                                Ok(templates) => templates,
                                                Err(_) => vec![],
                                            };

                                            // Build interactions
                                            let interactions: Vec<InteractionData> = interaction_templates
                                                .iter()
                                                .map(|i| {
                                                    let target_name = match &i.target {
                                                        wrldbldr_domain::entities::InteractionTarget::Character(char_id) => {
                                                            Some(format!("Character {}", char_id))
                                                        },
                                                        wrldbldr_domain::entities::InteractionTarget::Item(item_id) => {
                                                            Some(format!("Item {}", item_id))
                                                        },
                                                        wrldbldr_domain::entities::InteractionTarget::Environment(desc) => {
                                                            Some(desc.clone())
                                                        },
                                                        wrldbldr_domain::entities::InteractionTarget::None => None,
                                                    };
                                                    InteractionData {
                                                        id: i.id.to_string(),
                                                        name: i.name.clone(),
                                                        target_name,
                                                        interaction_type: format!("{:?}", i.interaction_type),
                                                        is_available: i.is_available,
                                                    }
                                                })
                                                .collect();

                                            // Build character data
                                            let characters: Vec<CharacterData> = scene_with_relations
                                                .featured_characters
                                                .iter()
                                                .map(|c| CharacterData {
                                                    id: c.id.to_string(),
                                                    name: c.name.clone(),
                                                    sprite_asset: c.sprite_asset.clone(),
                                                    portrait_asset: c.portrait_asset.clone(),
                                                    position: CharacterPosition::Center,
                                                    is_speaking: false,
                                                    emotion: None, // Engine doesn't track emotion state yet
                                                })
                                                .collect();

                                            // Build SceneUpdate message
                                            let scene_update = ServerMessage::SceneUpdate {
                                                scene: SceneData {
                                                    id: scene_with_relations.scene.id.to_string(),
                                                    name: scene_with_relations.scene.name.clone(),
                                                    location_id: scene_with_relations.scene.location_id.to_string(),
                                                    location_name: scene_with_relations.location.name.clone(),
                                                    backdrop_asset: scene_with_relations
                                                        .scene
                                                        .backdrop_override
                                                        .or(scene_with_relations.location.backdrop_asset.clone()),
                                                    time_context: match &scene_with_relations.scene.time_context {
                                                        wrldbldr_domain::entities::TimeContext::Unspecified => "Unspecified".to_string(),
                                                        wrldbldr_domain::entities::TimeContext::TimeOfDay(tod) => format!("{:?}", tod),
                                                        wrldbldr_domain::entities::TimeContext::During(s) => s.clone(),
                                                        wrldbldr_domain::entities::TimeContext::Custom(s) => s.clone(),
                                                    },
                                                    directorial_notes: scene_with_relations.scene.directorial_notes.clone(),
                                                },
                                                characters,
                                                interactions,
                                            };

                                            // Update scene and send to player via async port
                                            let _ = state.async_session_port.update_session_scene(session_id, scene.id.to_string()).await;
                                            if let Ok(scene_msg) = serde_json::to_value(&scene_update) {
                                                let _ = state.async_session_port.send_to_participant(session_id, &player_id, scene_msg).await;
                                            }
                                            tracing::info!(
                                                "Sent scene update to player {} after travel to location {}",
                                                player_id,
                                                location_id_str
                                            );

                                            // Check for split party and notify DM
                                            if let Ok(resolution_result) = state
                .player.scene_resolution_service
                                                .resolve_scene_for_session(session_id)
                                                .await
                                            {
                                                if resolution_result.is_split_party {
                                                    // Get location details for notification
                                                    let mut split_locations = Vec::new();
                                                    let pcs = match state
                .player.player_character_service
                                                        .get_pcs_by_session(session_id)
                                                        .await
                                                    {
                                                        Ok(pcs) => pcs,
                                                        Err(_) => vec![],
                                                    };

                                                    // Group PCs by location
                                                    let mut location_pcs: std::collections::HashMap<String, Vec<&_>> = std::collections::HashMap::new();
                                                    for pc in &pcs {
                                                        location_pcs
                                                            .entry(pc.current_location_id.to_string())
                                                            .or_insert_with(Vec::new)
                                                            .push(pc);
                                                    }

                                                    // Build location info
                                                    for (loc_id_str, pcs_at_loc) in location_pcs.iter() {
                                                        if let Ok(location) = state
                                                            .core.location_service
                                                            .get_location(wrldbldr_domain::LocationId::from_uuid(
                                                                uuid::Uuid::parse_str(loc_id_str).unwrap_or_default()
                                                            ))
                                                            .await
                                                        {
                                                            if let Some(loc) = location {
                                                                split_locations.push(wrldbldr_protocol::SplitPartyLocation {
                                                                    location_id: loc_id_str.to_string(),
                                                                    location_name: loc.name,
                                                                    pc_count: pcs_at_loc.len(),
                                                                    pc_names: pcs_at_loc.iter().map(|pc| pc.name.clone()).collect(),
                                                                });
                                                            }
                                                        }
                                                    }

                                                    // Send notification to DM via async port
                                                    if state.async_session_port.session_has_dm(session_id).await {
                                                        let dm_msg = ServerMessage::SplitPartyNotification {
                                                            location_count: split_locations.len(),
                                                            locations: split_locations,
                                                        };
                                                        if let Ok(dm_json) = serde_json::to_value(&dm_msg) {
                                                            let _ = state.async_session_port.send_to_dm(session_id, dm_json).await;
                                                        }
                                                    }
                                                }
                                            }

                                            // Return acknowledgment
                                            return Some(ServerMessage::ActionReceived {
                                                action_id: action_id_str,
                                                player_id: player_id.clone(),
                                                action_type: action_type.clone(),
                                            });
                                        }
                                        Ok(None) => {
                                            tracing::warn!("Scene {} not found after resolution", scene.id);
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to load scene with relations: {}", e);
                                        }
                                    }
                                }
                                Ok(None) => {
                                    // No scene found, but location updated - still acknowledge
                                    tracing::warn!(
                                        "No scene found for location {} after travel",
                                        location_id_str
                                    );
                                }
                                Err(e) => {
                                    tracing::error!("Failed to resolve scene: {}", e);
                                }
                            }
                        }
                        Ok(None) => {
                            return Some(ServerMessage::Error {
                                code: "NO_PC".to_string(),
                                message: "You must create a character before traveling".to_string(),
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to get PC: {}", e);
                            return Some(ServerMessage::Error {
                                code: "PC_LOOKUP_FAILED".to_string(),
                                message: format!("Failed to find your character: {}", e),
                            });
                        }
                    }
                } else {
                    return Some(ServerMessage::Error {
                        code: "MISSING_TARGET".to_string(),
                        message: "Travel action requires a target location".to_string(),
                    });
                }
            }

            // Look up the player's character ID for challenge targeting
            let pc_id = match state
                .player.player_character_service
                .get_pc_by_user_and_session(&player_id, session_id)
                .await
            {
                Ok(Some(pc)) => Some(pc.id),
                Ok(None) => {
                    tracing::debug!("Player {} has no character selected in session {}", player_id, session_id);
                    None
                }
                Err(e) => {
                    tracing::warn!("Failed to look up PC for player {}: {}", player_id, e);
                    None
                }
            };

            // Enqueue to PlayerActionQueue - returns immediately
            match state
                .queues.player_action_queue_service
                .enqueue_action(
                        session_id,
                    player_id.clone(),
                    pc_id,
                        action_type.clone(),
                        target.clone(),
                        dialogue.clone(),
                    )
                .await
            {
                Ok(_) => {
                    // Get queue depth for status update
                    let depth = state
                        .queues.player_action_queue_service
                        .depth()
                        .await
                        .unwrap_or(0);

                    // Send ActionQueued event to DM via async port
                    if state.async_session_port.session_has_dm(session_id).await {
                        let dm_msg = ServerMessage::ActionQueued {
                            action_id: action_id_str.clone(),
                            player_name: player_id.clone(),
                            action_type: action_type.clone(),
                            queue_depth: depth,
                        };
                        if let Ok(dm_json) = serde_json::to_value(&dm_msg) {
                            let _ = state.async_session_port.send_to_dm(session_id, dm_json).await;
                        }
                    }

                tracing::info!(
                        "Enqueued action {} from player {} in session {}: {} -> {:?}",
                    action_id_str,
                    player_id,
                    session_id,
                    action_type,
                    target
                );

                // Send ActionReceived acknowledgment to the player
                let _ = sender.send(ServerMessage::ActionReceived {
                    action_id: action_id_str,
                    player_id,
                    action_type: action_type.clone(),
                });
                }
                Err(e) => {
                    tracing::error!("Failed to enqueue player action: {}", e);
                    return Some(ServerMessage::Error {
                        code: "QUEUE_ERROR".to_string(),
                        message: format!("Failed to queue action: {}", e),
                    });
                }
            }

            None // No response from here; responses come from LLM processing or DM approval
        }

        ClientMessage::RequestSceneChange { scene_id } => {
            tracing::debug!("Scene change requested: {}", scene_id);

            // Parse scene_id
            let scene_uuid = match uuid::Uuid::parse_str(&scene_id) {
                Ok(uuid) => wrldbldr_domain::SceneId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_SCENE_ID".to_string(),
                        message: "Invalid scene ID format".to_string(),
                    });
                }
            };

            // Get the client's session via async port
            let session_id = match state.async_session_port.get_client_session(&client_id.to_string()).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NOT_IN_SESSION".to_string(),
                        message: "You must join a session before requesting scene changes".to_string(),
                    });
                }
            };

            // Load scene from database with relations
            let scene_with_relations = match state.core.scene_service.get_scene_with_relations(scene_uuid).await {
                Ok(Some(scene_data)) => scene_data,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "SCENE_NOT_FOUND".to_string(),
                        message: format!("Scene {} not found", scene_id),
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to load scene: {}", e);
                    return Some(ServerMessage::Error {
                        code: "SCENE_LOAD_ERROR".to_string(),
                        message: "Failed to load scene".to_string(),
                    });
                }
            };

            // Load interactions for the scene
            let interactions = match state.core.interaction_service.list_interactions(scene_uuid).await {
                Ok(interactions) => interactions
                    .into_iter()
                    .map(|i| {
                        let target_name = match &i.target {
                            wrldbldr_domain::entities::InteractionTarget::Character(_) => {
                                Some("Character".to_string())
                            }
                            wrldbldr_domain::entities::InteractionTarget::Item(_) => {
                                Some("Item".to_string())
                            }
                            wrldbldr_domain::entities::InteractionTarget::Environment(name) => {
                                Some(name.clone())
                            }
                            wrldbldr_domain::entities::InteractionTarget::None => None,
                        };
                        InteractionData {
                            id: i.id.to_string(),
                            name: i.name.clone(),
                            interaction_type: format!("{:?}", i.interaction_type),
                            target_name,
                            is_available: i.is_available,
                        }
                    })
                    .collect(),
                Err(e) => {
                    tracing::warn!("Failed to load interactions for scene: {}", e);
                    vec![]
                }
            };

            // Build character data from featured characters
            let characters: Vec<CharacterData> = scene_with_relations
                .featured_characters
                .iter()
                .map(|c| CharacterData {
                    id: c.id.to_string(),
                    name: c.name.clone(),
                    sprite_asset: c.sprite_asset.clone(),
                    portrait_asset: c.portrait_asset.clone(),
                    position: CharacterPosition::Center, // Default position, could be enhanced
                    is_speaking: false,
                    emotion: None, // Engine doesn't track emotion state yet
                })
                .collect();

            // Build SceneUpdate message
            let scene_update = ServerMessage::SceneUpdate {
                scene: SceneData {
                    id: scene_with_relations.scene.id.to_string(),
                    name: scene_with_relations.scene.name.clone(),
                    location_id: scene_with_relations.scene.location_id.to_string(),
                    location_name: scene_with_relations.location.name.clone(),
                    backdrop_asset: scene_with_relations
                        .scene
                        .backdrop_override
                        .or(scene_with_relations.location.backdrop_asset.clone()),
                    time_context: match &scene_with_relations.scene.time_context {
                        wrldbldr_domain::entities::TimeContext::Unspecified => "Unspecified".to_string(),
                        wrldbldr_domain::entities::TimeContext::TimeOfDay(tod) => format!("{:?}", tod),
                        wrldbldr_domain::entities::TimeContext::During(s) => s.clone(),
                        wrldbldr_domain::entities::TimeContext::Custom(s) => s.clone(),
                    },
                    directorial_notes: scene_with_relations.scene.directorial_notes.clone(),
                },
                characters,
                interactions,
            };

            // Update session's current scene and broadcast via async port
            let _ = state.async_session_port.update_session_scene(session_id, scene_id.clone()).await;
            if let Ok(scene_json) = serde_json::to_value(&scene_update) {
                let _ = state.async_session_port.broadcast_to_session(session_id, scene_json).await;
            }

            tracing::info!("Scene change to {} broadcast to session {}", scene_id, session_id);

            None // SceneUpdate is broadcast, no direct response needed
        }

        ClientMessage::DirectorialUpdate { context: _ } => {
            tracing::debug!("Received directorial update");

            // Only DMs should send directorial updates - check via async port
            let client_id_str = client_id.to_string();
            if state.async_session_port.is_client_dm(&client_id_str).await {
                if let Some(session_id) = state.async_session_port.get_client_session(&client_id_str).await {
                    // TODO: Update directorial context and store in session
                    tracing::info!(
                        "DM updated directorial context for session {}",
                        session_id
                    );
                }
            }

            None // No response needed
        }

        ClientMessage::ApprovalDecision {
            request_id,
            decision,
        } => {
            tracing::debug!(
                "Received approval decision for {}: {:?}",
                request_id,
                decision
            );

            // Only DMs should approve - check via async port
            let client_id_str = client_id.to_string();
            let session_id = state.async_session_port.get_client_session(&client_id_str).await;
            let is_dm = state.async_session_port.is_client_dm(&client_id_str).await;
            let dm_id = if is_dm {
                state.async_session_port.get_client_user_id(&client_id_str).await
            } else {
                None
            };

            if let (Some(session_id), Some(dm_id)) = (session_id, dm_id) {
                // Enqueue to DMActionQueue - returns immediately
                // The DM action queue worker will process this asynchronously
                let dm_action = DMAction::ApprovalDecision {
                    request_id: request_id.clone(),
                    decision: decision.clone(),
                };

                match state
                    .queues.dm_action_queue_service
                    .enqueue_action(session_id, dm_id, dm_action)
                    .await
                {
                    Ok(_) => {
                        tracing::info!("Enqueued approval decision for request {}", request_id);
                        // Return acknowledgment - processing happens in background worker
                        return None;
                    }
                    Err(e) => {
                        tracing::error!("Failed to enqueue approval decision: {}", e);
                        return Some(ServerMessage::Error {
                            code: "QUEUE_ERROR".to_string(),
                            message: format!("Failed to queue approval: {}", e),
                        });
                    }
                }
            } else {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can approve responses".to_string(),
                });
            }
        }

        ClientMessage::ChallengeRoll { challenge_id, roll } => {
            tracing::debug!(
                "Received challenge roll: {} for challenge {}",
                roll,
                challenge_id
            );
            state
                .game.challenge_resolution_service
                .handle_roll(client_id.to_string(), challenge_id, roll)
                .await
                .and_then(value_to_server_message)
        }

        ClientMessage::ChallengeRollInput {
            challenge_id,
            input_type,
        } => {
            tracing::debug!(
                "Received challenge roll input: {:?} for challenge {}",
                input_type,
                challenge_id
            );
            state
                .game.challenge_resolution_service
                .handle_roll_input(client_id.to_string(), challenge_id, to_service_dice_input(input_type))
                .await
                .and_then(value_to_server_message)
        }

        ClientMessage::TriggerChallenge {
            challenge_id,
            target_character_id,
        } => {
            state
                .game.challenge_resolution_service
                .handle_trigger(client_id.to_string(), challenge_id, target_character_id)
                .await
                .and_then(value_to_server_message)
        }

        ClientMessage::ChallengeSuggestionDecision {
            request_id,
            approved,
            modified_difficulty,
        } => state
            .game.challenge_resolution_service
            .handle_suggestion_decision(client_id.to_string(), request_id, approved, modified_difficulty)
            .await
            .and_then(value_to_server_message),

        ClientMessage::NarrativeEventSuggestionDecision {
            request_id,
            event_id,
            approved,
            selected_outcome,
        } => state
            .game.narrative_event_approval_service
            .handle_decision(
                client_id.to_string(),
                request_id,
                event_id,
                approved,
                selected_outcome,
            )
            .await
            .and_then(value_to_server_message),

        ClientMessage::RegenerateOutcome {
            request_id,
            outcome_type,
            guidance,
        } => {
            tracing::debug!(
                "DM requested outcome regeneration for request {} outcome {:?}",
                request_id,
                outcome_type
            );

            // Best-effort: look up the approval item for context
            let maybe_approval = state
                .queues.dm_approval_queue_service
                .get_by_id(&request_id)
                .await
                .ok()
                .flatten();

            let base_flavor = if let Some(item) = maybe_approval {
                format!(
                    "{} (regenerated)",
                    item.payload.proposed_dialogue.trim()
                )
            } else {
                "Regenerated outcome (no approval context found)".to_string()
            };

            let flavor_text = if let Some(g) = guidance {
                if g.trim().is_empty() {
                    base_flavor
                } else {
                    format!("{} — Guidance: {}", base_flavor, g.trim())
                }
            } else {
                base_flavor
            };

            let outcome_type_str = outcome_type.unwrap_or_else(|| "all".to_string());

            Some(ServerMessage::OutcomeRegenerated {
                request_id,
                outcome_type: outcome_type_str,
                new_outcome: wrldbldr_protocol::OutcomeDetailData {
                    flavor_text,
                    scene_direction: "DM: narrate this regenerated outcome to the table."
                        .to_string(),
                    proposed_tools: Vec::new(),
                },
            })
        }

        ClientMessage::DiscardChallenge {
            request_id,
            feedback,
        } => {
            tracing::info!(
                "DM discarded challenge for request {}, feedback: {:?}",
                request_id,
                feedback
            );
            // Remove the challenge suggestion from the approval queue
            // The approval will be re-queued for a non-challenge response
            state
                .queues.dm_approval_queue_service
                .discard_challenge(&client_id.to_string(), &request_id)
                .await;
            Some(ServerMessage::ChallengeDiscarded { request_id })
        }

        ClientMessage::CreateAdHocChallenge {
            challenge_name,
            skill_name,
            difficulty,
            target_pc_id,
            outcomes,
        } => {
            tracing::info!(
                "DM creating ad-hoc challenge '{}' for PC {}",
                challenge_name,
                target_pc_id
            );
            state
                .game.challenge_resolution_service
                .handle_adhoc_challenge(
                    client_id.to_string(),
                    challenge_name,
                    skill_name,
                    difficulty,
                    target_pc_id,
                    to_adhoc_outcomes_dto(outcomes),
                )
                .await
                .and_then(value_to_server_message)
        }

        // =====================================================================
        // Challenge Outcome Approval (P3.3)
        // =====================================================================
        ClientMessage::ChallengeOutcomeDecision {
            resolution_id,
            decision,
        } => {
            tracing::info!(
                "DM decision on challenge outcome {}: {:?}",
                resolution_id,
                decision
            );

            // Only DMs should approve - check via async port
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can approve challenge outcomes".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Convert wire decision to service decision
            let svc_decision = to_challenge_outcome_decision(decision);

            // Process the decision via the approval service
            match state.game.challenge_outcome_approval_service
                .process_decision(session_id, &resolution_id, svc_decision)
                .await
            {
                Ok(()) => {
                    // Success - resolution broadcast is handled by the service
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to process challenge outcome decision: {}", e);
                    Some(ServerMessage::Error {
                        code: "APPROVAL_ERROR".to_string(),
                        message: format!("Failed to process decision: {}", e),
                    })
                }
            }
        }

        ClientMessage::RequestOutcomeSuggestion {
            resolution_id,
            guidance,
        } => {
            tracing::info!(
                "DM requesting outcome suggestion for {}: {:?}",
                resolution_id,
                guidance
            );

            // Only DMs should request suggestions
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can request outcome suggestions".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Process as a Suggest decision - the service will handle LLM generation
            let svc_decision = ChallengeOutcomeDecision::Suggest { guidance };

            match state.game.challenge_outcome_approval_service
                .process_decision(session_id, &resolution_id, svc_decision)
                .await
            {
                Ok(()) => {
                    // Success - the service will send OutcomeSuggestionReady when LLM completes
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to request outcome suggestions: {}", e);
                    Some(ServerMessage::Error {
                        code: "SUGGESTION_ERROR".to_string(),
                        message: format!("Failed to request suggestions: {}", e),
                    })
                }
            }
        }

        ClientMessage::RequestOutcomeBranches {
            resolution_id,
            guidance,
        } => {
            tracing::info!(
                "DM requesting outcome branches for {}: {:?}",
                resolution_id,
                guidance
            );

            // Only DMs should request branches
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can request outcome branches".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Request branches via the approval service
            match state.game.challenge_outcome_approval_service
                .request_branches(session_id, &resolution_id, guidance)
                .await
            {
                Ok(()) => {
                    // Success - the service will send OutcomeBranchesReady when LLM completes
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to request outcome branches: {}", e);
                    Some(ServerMessage::Error {
                        code: "BRANCH_ERROR".to_string(),
                        message: format!("Failed to request branches: {}", e),
                    })
                }
            }
        }

        ClientMessage::SelectOutcomeBranch {
            resolution_id,
            branch_id,
            modified_description,
        } => {
            tracing::info!(
                "DM selecting branch {} for resolution {}",
                branch_id,
                resolution_id
            );

            // Only DMs should select branches
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can select outcome branches".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Process branch selection via the approval service
            match state.game.challenge_outcome_approval_service
                .select_branch(session_id, &resolution_id, &branch_id, modified_description)
                .await
            {
                Ok(()) => {
                    // Success - challenge is resolved
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to select outcome branch: {}", e);
                    Some(ServerMessage::Error {
                        code: "BRANCH_SELECT_ERROR".to_string(),
                        message: format!("Failed to select branch: {}", e),
                    })
                }
            }
        }

        // =========================================================================
        // Phase 23D: NPC Location Sharing (HeardAbout observations)
        // =========================================================================

        ClientMessage::ShareNpcLocation {
            pc_id,
            npc_id,
            location_id,
            region_id,
            notes,
        } => {
            tracing::info!(
                "DM sharing NPC {} location with PC {}",
                npc_id,
                pc_id
            );

            // Only DMs can share NPC locations
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can share NPC locations".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse IDs
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };
            let location_uuid = match uuid::Uuid::parse_str(&location_id) {
                Ok(uuid) => wrldbldr_domain::LocationId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_LOCATION_ID".to_string(),
                        message: "Invalid location ID format".to_string(),
                    });
                }
            };
            let region_uuid = match uuid::Uuid::parse_str(&region_id) {
                Ok(uuid) => wrldbldr_domain::RegionId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_REGION_ID".to_string(),
                        message: "Invalid region ID format".to_string(),
                    });
                }
            };

            // Get game time from session
            let game_time = {
                let sessions = state.sessions.read().await;
                match sessions.get_session(session_id) {
                    Some(session) => session.game_time().current(),
                    None => {
                        return Some(ServerMessage::Error {
                            code: "SESSION_NOT_FOUND".to_string(),
                            message: "Session not found".to_string(),
                        });
                    }
                }
            };

            // Create HeardAbout observation
            let observation = wrldbldr_domain::entities::NpcObservation::heard_about(
                pc_uuid,
                npc_uuid,
                location_uuid,
                region_uuid,
                game_time,
                notes,
            );

            // Store the observation
            match state.repository.observations().upsert(&observation).await {
                Ok(()) => {
                    tracing::info!(
                        "Created HeardAbout observation: PC {} now knows NPC {} was at region {}",
                        pc_id,
                        npc_id,
                        region_id
                    );
                    // Could broadcast to the player here if we had their client ID
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to create observation: {}", e);
                    Some(ServerMessage::Error {
                        code: "OBSERVATION_ERROR".to_string(),
                        message: format!("Failed to record observation: {}", e),
                    })
                }
            }
        }

        // =========================================================================
        // Phase 23E: DM Event System
        // =========================================================================

        ClientMessage::TriggerApproachEvent {
            npc_id,
            target_pc_id,
            description,
            reveal,
        } => {
            tracing::info!(
                "DM triggering approach event: NPC {} approaching PC {}",
                npc_id,
                target_pc_id
            );

            // Only DMs can trigger approach events
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can trigger approach events".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse NPC ID and get NPC details
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };

            // Get NPC details
            let npc = match state.repository.characters().get(npc_uuid).await {
                Ok(Some(npc)) => npc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "NPC_NOT_FOUND".to_string(),
                        message: "NPC not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch NPC: {}", e),
                    });
                }
            };

            // Parse PC ID and get PC details (for region)
            let pc_uuid = match uuid::Uuid::parse_str(&target_pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            let pc = match state.repository.player_characters().get(pc_uuid).await {
                Ok(Some(pc)) => pc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "PC_NOT_FOUND".to_string(),
                        message: "Player character not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch PC: {}", e),
                    });
                }
            };

            // Create Direct observation for the PC (they now see the NPC)
            if let Some(region_id) = pc.current_region_id {
                let game_time = {
                    let sessions = state.sessions.read().await;
                    sessions.get_session(session_id)
                        .map(|s| s.game_time().current())
                        .unwrap_or_else(chrono::Utc::now)
                };

                let observation = if reveal {
                    wrldbldr_domain::entities::NpcObservation::direct(
                        pc_uuid,
                        npc_uuid,
                        pc.current_location_id,
                        region_id,
                        game_time,
                    )
                } else {
                    wrldbldr_domain::entities::NpcObservation::direct_unrevealed(
                        pc_uuid,
                        npc_uuid,
                        pc.current_location_id,
                        region_id,
                        game_time,
                    )
                };

                if let Err(e) = state.repository.observations().upsert(&observation).await {
                    tracing::warn!("Failed to create observation for approach event: {}", e);
                }
            }

            // Build the ApproachEvent message
            let (npc_name, npc_sprite) = if reveal {
                (npc.name.clone(), npc.sprite_asset.clone())
            } else {
                ("Unknown Figure".to_string(), None)
            };

            let approach_event = ServerMessage::ApproachEvent {
                npc_id: npc_id.clone(),
                npc_name,
                npc_sprite,
                description,
                reveal,
            };

            // Send to the target PC's user only (not broadcast to all)
            if let Ok(msg_json) = serde_json::to_value(&approach_event) {
                let _ = state.async_session_port.send_to_participant(session_id, &pc.user_id, msg_json).await;
            }

            tracing::info!(
                "Approach event triggered: {} approached by {}",
                target_pc_id,
                npc.name
            );
            None
        }

        ClientMessage::TriggerLocationEvent {
            region_id,
            description,
        } => {
            tracing::info!(
                "DM triggering location event in region {}",
                region_id
            );

            // Only DMs can trigger location events
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can trigger location events".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Build the LocationEvent message
            let location_event = ServerMessage::LocationEvent {
                region_id: region_id.clone(),
                description,
            };

            // Broadcast to all in session - clients filter by their current region
            if let Ok(msg_json) = serde_json::to_value(&location_event) {
                // Use broadcast_except with empty string to broadcast to all
                let _ = state.async_session_port.broadcast_except(session_id, msg_json, "").await;
            }

            tracing::info!("Location event triggered in region {}", region_id);
            None
        }

        // =========================================================================
        // Phase 23F: Game Time Control
        // =========================================================================

        ClientMessage::AdvanceGameTime { hours } => {
            tracing::info!("DM advancing game time by {} hours", hours);

            // Only DMs can advance time
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can advance game time".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Advance game time in session
            let game_time = {
                let mut sessions = state.sessions.write().await;
                match sessions.get_session_mut(session_id) {
                    Some(session) => {
                        session.advance_time_hours(hours);

                        let gt = session.game_time();
                        wrldbldr_protocol::GameTime::new(
                            gt.day_ordinal(),
                            gt.current().hour() as u8,
                            gt.current().minute() as u8,
                            gt.is_paused(),
                        )
                    }
                    None => {
                        return Some(ServerMessage::Error {
                            code: "SESSION_NOT_FOUND".to_string(),
                            message: "Session not found".to_string(),
                        });
                    }
                }
            };

            // Build the GameTimeUpdated message
            let time_updated = ServerMessage::GameTimeUpdated { game_time };

            // Broadcast to all in session
            if let Ok(msg_json) = serde_json::to_value(&time_updated) {
                // Use broadcast_except with empty string to broadcast to all
                let _ = state.async_session_port.broadcast_except(session_id, msg_json, "").await;
            }

            tracing::info!("Game time advanced by {} hours", hours);
            None
        }

        // =========================================================================
        // Phase 23C: Navigation
        // =========================================================================

        ClientMessage::SelectPlayerCharacter { pc_id } => {
            tracing::info!("Player selecting PC {}", pc_id);

            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse PC ID
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            // Get PC details
            let pc = match state.repository.player_characters().get(pc_uuid).await {
                Ok(Some(pc)) => pc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "PC_NOT_FOUND".to_string(),
                        message: "Player character not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch PC: {}", e),
                    });
                }
            };

            // Send PcSelected response
            Some(ServerMessage::PcSelected {
                pc_id: pc_id.clone(),
                pc_name: pc.name.clone(),
                location_id: pc.current_location_id.to_string(),
                region_id: pc.current_region_id.map(|r| r.to_string()),
            })
        }

        ClientMessage::MoveToRegion { pc_id, region_id } => {
            tracing::info!("PC {} moving to region {}", pc_id, region_id);

            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse IDs
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };
            let region_uuid = match uuid::Uuid::parse_str(&region_id) {
                Ok(uuid) => wrldbldr_domain::RegionId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_REGION_ID".to_string(),
                        message: "Invalid region ID format".to_string(),
                    });
                }
            };

            // Get current PC to check if movement is valid
            let pc = match state.repository.player_characters().get(pc_uuid).await {
                Ok(Some(pc)) => pc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "PC_NOT_FOUND".to_string(),
                        message: "Player character not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch PC: {}", e),
                    });
                }
            };

            // Check if movement is allowed (check for locked connections)
            if let Some(current_region_id) = pc.current_region_id {
                let connections = match state.repository.regions().get_connections(current_region_id).await {
                    Ok(conns) => conns,
                    Err(e) => {
                        return Some(ServerMessage::Error {
                            code: "DATABASE_ERROR".to_string(),
                            message: format!("Failed to fetch connections: {}", e),
                        });
                    }
                };

                // Find the connection to target region
                if let Some(conn) = connections.iter().find(|c| c.to_region == region_uuid) {
                    if conn.is_locked {
                        return Some(ServerMessage::MovementBlocked {
                            pc_id: pc_id.clone(),
                            reason: conn.lock_description.clone().unwrap_or_else(|| "The path is locked.".to_string()),
                        });
                    }
                }
            }

            // Get target region
            let target_region = match state.repository.regions().get(region_uuid).await {
                Ok(Some(region)) => region,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "REGION_NOT_FOUND".to_string(),
                        message: "Target region not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch region: {}", e),
                    });
                }
            };

            // Update PC position
            if let Err(e) = state.repository.player_characters()
                .update_position(pc_uuid, target_region.location_id, Some(region_uuid))
                .await
            {
                return Some(ServerMessage::Error {
                    code: "UPDATE_ERROR".to_string(),
                    message: format!("Failed to update position: {}", e),
                });
            }

            // Get location for name and staging settings
            let location = state.repository.locations().get(target_region.location_id).await
                .ok().flatten();
            let location_name = location.as_ref().map(|l| l.name.clone()).unwrap_or_default();
            let backdrop = target_region.backdrop_asset.clone()
                .or_else(|| location.as_ref().and_then(|l| l.backdrop_asset.clone()));
            let map_asset = location.as_ref().and_then(|l| l.map_asset.clone());
            let world_id = location.as_ref().map(|l| l.world_id).unwrap_or_else(wrldbldr_domain::WorldId::new);
            let default_ttl = location.as_ref().map(|l| l.presence_cache_ttl_hours).unwrap_or(3);
            let use_llm = location.as_ref().map(|l| l.use_llm_presence).unwrap_or(true);

            // Get game time
            let game_time = {
                let sessions = state.sessions.read().await;
                sessions.get_session(session_id)
                    .map(|s| s.game_time().clone())
                    .unwrap_or_default()
            };

            // Get user ID for this PC
            let user_id = state.async_session_port.get_client_user_id(&client_id_str).await
                .unwrap_or_else(|| client_id_str.clone());

            // =====================================================================
            // Staging System Integration
            // =====================================================================
            
            // Check for existing valid staging
            let existing_staging = state.staging_service.get_current_staging(region_uuid, &game_time).await.ok().flatten();

            let npcs_present: Vec<wrldbldr_protocol::NpcPresenceData> = if let Some(staging) = existing_staging {
                // Use existing staging
                tracing::debug!("Using existing staging {} for region {}", staging.id, region_uuid);
                staging.npcs
                    .into_iter()
                    .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
                    .map(|npc| wrldbldr_protocol::NpcPresenceData {
                        character_id: npc.character_id.to_string(),
                        name: npc.name,
                        sprite_asset: npc.sprite_asset,
                        portrait_asset: npc.portrait_asset,
                    })
                    .collect()
            } else {
                // No valid staging - check if there's already a pending approval for this region
                let has_pending = {
                    let sessions = state.sessions.read().await;
                    sessions.get_session(session_id)
                        .and_then(|s| s.get_pending_staging_for_region(region_uuid))
                        .is_some()
                };

                if has_pending {
                    // Add this PC to the waiting list and send StagingPending
                    {
                        let mut sessions = state.sessions.write().await;
                        if let Some(session) = sessions.get_session_mut(session_id) {
                            if let Some(pending) = session.get_pending_staging_for_region_mut(region_uuid) {
                                pending.add_waiting_pc(pc_uuid, pc.name.clone(), client_id, user_id.clone());
                            }
                        }
                    }

                    // Send StagingPending to this player
                    let _ = sender.send(ServerMessage::StagingPending {
                        region_id: region_uuid.to_string(),
                        region_name: target_region.name.clone(),
                    });

                    tracing::info!(
                        pc_id = %pc_id,
                        region_id = %region_id,
                        "PC added to existing pending staging, waiting for DM approval"
                    );

                    return None; // Return early, will send SceneChanged when staging approved
                }

                // No pending staging - generate a new proposal
                tracing::info!("No valid staging for region {}, generating proposal", region_uuid);

                // Send StagingPending to player immediately
                let _ = sender.send(ServerMessage::StagingPending {
                    region_id: region_uuid.to_string(),
                    region_name: target_region.name.clone(),
                });

                // Generate staging proposal
                let proposal = match state.staging_service.generate_proposal(
                    world_id,
                    region_uuid,
                    target_region.location_id,
                    &location_name,
                    &game_time,
                    default_ttl,
                    None, // No DM guidance yet
                ).await {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("Failed to generate staging proposal: {}", e);
                        // Fall back to simple rule-based presence
                        let npc_relationships = state.repository.characters()
                            .get_npcs_related_to_region(region_uuid)
                            .await
                            .unwrap_or_default();

                        let time_of_day = game_time.time_of_day();
                        let region_items = fetch_region_items(&state, region_uuid).await;
                        return Some(ServerMessage::SceneChanged {
                            pc_id: pc_id.clone(),
                            region: wrldbldr_protocol::RegionData {
                                id: region_uuid.to_string(),
                                name: target_region.name.clone(),
                                location_id: target_region.location_id.to_string(),
                                location_name: location_name.clone(),
                                backdrop_asset: backdrop.clone(),
                                atmosphere: target_region.atmosphere.clone(),
                                map_asset: map_asset.clone(),
                            },
                            npcs_present: npc_relationships
                                .into_iter()
                                .filter_map(|(character, rel_type)| {
                                    if rel_type.is_npc_present(time_of_day) {
                                        Some(wrldbldr_protocol::NpcPresenceData {
                                            character_id: character.id.to_string(),
                                            name: character.name,
                                            sprite_asset: character.sprite_asset,
                                            portrait_asset: character.portrait_asset,
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                            navigation: wrldbldr_protocol::NavigationData {
                                connected_regions: Vec::new(),
                                exits: Vec::new(),
                            },
                            region_items,
                        });
                    }
                };

                let request_id = proposal.request_id.clone();

                // Get previous staging for reference
                let previous_staging = state.staging_service.get_previous_staging(region_uuid).await.ok().flatten();
                let previous_staging_info = previous_staging.map(|s| wrldbldr_protocol::PreviousStagingInfo {
                    staging_id: s.id.to_string(),
                    approved_at: s.approved_at.to_rfc3339(),
                    npcs: s.npcs.into_iter().map(|npc| wrldbldr_protocol::StagedNpcInfo {
                        character_id: npc.character_id.to_string(),
                        name: npc.name,
                        sprite_asset: npc.sprite_asset,
                        portrait_asset: npc.portrait_asset,
                        is_present: npc.is_present,
                        reasoning: npc.reasoning,
                        is_hidden_from_players: npc.is_hidden_from_players,
                    }).collect(),
                });

                // Convert proposal NPCs to protocol format
                let rule_based_npcs: Vec<wrldbldr_protocol::StagedNpcInfo> = proposal.rule_based_npcs
                    .iter()
                    .map(|npc| wrldbldr_protocol::StagedNpcInfo {
                        character_id: npc.character_id.clone(),
                        name: npc.name.clone(),
                        sprite_asset: npc.sprite_asset.clone(),
                        portrait_asset: npc.portrait_asset.clone(),
                        is_present: npc.is_present,
                        reasoning: npc.reasoning.clone(),
                        is_hidden_from_players: false,
                    })
                    .collect();

                let llm_based_npcs: Vec<wrldbldr_protocol::StagedNpcInfo> = if use_llm {
                    proposal.llm_based_npcs
                        .iter()
                        .map(|npc| wrldbldr_protocol::StagedNpcInfo {
                            character_id: npc.character_id.clone(),
                            name: npc.name.clone(),
                            sprite_asset: npc.sprite_asset.clone(),
                            portrait_asset: npc.portrait_asset.clone(),
                            is_present: npc.is_present,
                            reasoning: npc.reasoning.clone(),
                            is_hidden_from_players: npc.is_hidden_from_players,
                        })
                        .collect()
                } else {
                    Vec::new() // Don't include LLM suggestions if disabled
                };

                // Store pending staging approval
                let mut pending_approval = crate::infrastructure::session::PendingStagingApproval::new(
                    request_id.clone(),
                    region_uuid,
                    target_region.location_id,
                    world_id,
                    target_region.name.clone(),
                    location_name.clone(),
                    proposal,
                );
                pending_approval.add_waiting_pc(pc_uuid, pc.name.clone(), client_id, user_id.clone());

                {
                    let mut sessions = state.sessions.write().await;
                    if let Some(session) = sessions.get_session_mut(session_id) {
                        session.add_pending_staging_approval(pending_approval);
                    }
                }

                // Send StagingApprovalRequired to DM
                let approval_msg = ServerMessage::StagingApprovalRequired {
                    request_id,
                    region_id: region_uuid.to_string(),
                    region_name: target_region.name.clone(),
                    location_id: target_region.location_id.to_string(),
                    location_name: location_name.clone(),
                    game_time: wrldbldr_protocol::GameTime::new(
                        game_time.day_ordinal(),
                        game_time.current().hour() as u8,
                        game_time.current().minute() as u8,
                        game_time.is_paused(),
                    ),
                    previous_staging: previous_staging_info,
                    rule_based_npcs,
                    llm_based_npcs,
                    default_ttl_hours: default_ttl,
                    waiting_pcs: vec![wrldbldr_protocol::WaitingPcInfo {
                        pc_id: pc_id.clone(),
                        pc_name: pc.name.clone(),
                        player_id: user_id,
                    }],
                };

                {
                    let sessions = state.sessions.read().await;
                    if let Some(session) = sessions.get_session(session_id) {
                        session.send_to_dm(&approval_msg);
                    }
                }

                tracing::info!(
                    pc_id = %pc_id,
                    region_id = %region_id,
                    "Staging approval request sent to DM"
                );

                return None; // Return early, will send SceneChanged when staging approved
            };

            // Get navigation options (only reached if we have valid staging)
            let connections = state.repository.regions().get_connections(region_uuid).await.unwrap_or_default();
            let exits = state.repository.regions().get_exits(region_uuid).await.unwrap_or_default();

            let mut connected_regions = Vec::new();
            for conn in connections {
                if let Ok(Some(target)) = state.repository.regions().get(conn.to_region).await {
                    connected_regions.push(wrldbldr_protocol::NavigationTarget {
                        region_id: conn.to_region.to_string(),
                        name: target.name,
                        is_locked: conn.is_locked,
                        lock_description: conn.lock_description,
                    });
                }
            }

            let mut exit_targets = Vec::new();
            for exit in exits {
                if let Ok(Some(target_loc)) = state.repository.locations().get(exit.to_location).await {
                    exit_targets.push(wrldbldr_protocol::NavigationExit {
                        location_id: exit.to_location.to_string(),
                        location_name: target_loc.name,
                        arrival_region_id: exit.arrival_region_id.to_string(),
                        description: exit.description,
                    });
                }
            }

            let region_items = fetch_region_items(&state, region_uuid).await;
            Some(ServerMessage::SceneChanged {
                pc_id,
                region: wrldbldr_protocol::RegionData {
                    id: region_uuid.to_string(),
                    name: target_region.name,
                    location_id: target_region.location_id.to_string(),
                    location_name,
                    backdrop_asset: backdrop,
                    atmosphere: target_region.atmosphere,
                    map_asset,
                },
                npcs_present,
                navigation: wrldbldr_protocol::NavigationData {
                    connected_regions,
                    exits: exit_targets,
                },
                region_items,
            })
        }

        ClientMessage::ExitToLocation { pc_id, location_id, arrival_region_id } => {
            tracing::info!("PC {} exiting to location {}", pc_id, location_id);

            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse IDs
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };
            let location_uuid = match uuid::Uuid::parse_str(&location_id) {
                Ok(uuid) => wrldbldr_domain::LocationId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_LOCATION_ID".to_string(),
                        message: "Invalid location ID format".to_string(),
                    });
                }
            };

            // Get target location
            let target_location = match state.repository.locations().get(location_uuid).await {
                Ok(Some(loc)) => loc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "LOCATION_NOT_FOUND".to_string(),
                        message: "Target location not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch location: {}", e),
                    });
                }
            };

            // Determine arrival region
            let arrival_region_uuid = if let Some(arrival_id) = arrival_region_id {
                match uuid::Uuid::parse_str(&arrival_id) {
                    Ok(uuid) => wrldbldr_domain::RegionId::from_uuid(uuid),
                    Err(_) => {
                        return Some(ServerMessage::Error {
                            code: "INVALID_REGION_ID".to_string(),
                            message: "Invalid arrival region ID format".to_string(),
                        });
                    }
                }
            } else if let Some(default_region) = target_location.default_region_id {
                default_region
            } else {
                // Find first spawn point in location
                let regions = state.repository.regions().list_by_location(location_uuid).await.unwrap_or_default();
                match regions.into_iter().find(|r| r.is_spawn_point).map(|r| r.id) {
                    Some(id) => id,
                    None => {
                        return Some(ServerMessage::Error {
                            code: "NO_ARRIVAL_REGION".to_string(),
                            message: "No arrival region specified and location has no default or spawn point".to_string(),
                        });
                    }
                }
            };

            // Get arrival region
            let arrival_region = match state.repository.regions().get(arrival_region_uuid).await {
                Ok(Some(region)) => region,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "REGION_NOT_FOUND".to_string(),
                        message: "Arrival region not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch region: {}", e),
                    });
                }
            };

            // Update PC position
            if let Err(e) = state.repository.player_characters()
                .update_position(pc_uuid, location_uuid, Some(arrival_region_uuid))
                .await
            {
                return Some(ServerMessage::Error {
                    code: "UPDATE_ERROR".to_string(),
                    message: format!("Failed to update position: {}", e),
                });
            }

            // Get backdrop and map asset
            let backdrop = arrival_region.backdrop_asset.clone()
                .or_else(|| target_location.backdrop_asset.clone());
            let map_asset = target_location.map_asset.clone();

            // Get staging settings from location
            let location_name = target_location.name.clone();
            let world_id = target_location.world_id;
            let default_ttl = target_location.presence_cache_ttl_hours;
            let use_llm = target_location.use_llm_presence;

            // Get game time
            let game_time = {
                let sessions = state.sessions.read().await;
                sessions
                    .get_session(session_id)
                    .map(|s| s.game_time().clone())
                    .unwrap_or_default()
            };

            // Get PC details (name used for waiting list)
            let pc = match state.repository.player_characters().get(pc_uuid).await {
                Ok(Some(pc)) => pc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "PC_NOT_FOUND".to_string(),
                        message: "Player character not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch PC: {}", e),
                    });
                }
            };

            // Get user ID for this PC
            let user_id = state
                .async_session_port
                .get_client_user_id(&client_id_str)
                .await
                .unwrap_or_else(|| client_id_str.clone());

            // =====================================================================
            // Staging System Integration
            // =====================================================================

            // Check for existing valid staging
            let existing_staging = state
                .staging_service
                .get_current_staging(arrival_region_uuid, &game_time)
                .await
                .ok()
                .flatten();

            let npcs_present: Vec<wrldbldr_protocol::NpcPresenceData> = if let Some(staging) = existing_staging {
                // Use existing staging
                tracing::debug!(
                    "Using existing staging {} for region {}",
                    staging.id,
                    arrival_region_uuid
                );
                staging
                    .npcs
                    .into_iter()
                    .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
                    .map(|npc| wrldbldr_protocol::NpcPresenceData {
                        character_id: npc.character_id.to_string(),
                        name: npc.name,
                        sprite_asset: npc.sprite_asset,
                        portrait_asset: npc.portrait_asset,
                    })
                    .collect()
            } else {
                // No valid staging - check if there's already a pending approval for this region
                let has_pending = {
                    let sessions = state.sessions.read().await;
                    sessions
                        .get_session(session_id)
                        .and_then(|s| s.get_pending_staging_for_region(arrival_region_uuid))
                        .is_some()
                };

                if has_pending {
                    // Add this PC to the waiting list and send StagingPending
                    {
                        let mut sessions = state.sessions.write().await;
                        if let Some(session) = sessions.get_session_mut(session_id) {
                            if let Some(pending) = session.get_pending_staging_for_region_mut(arrival_region_uuid) {
                                pending.add_waiting_pc(pc_uuid, pc.name.clone(), client_id, user_id.clone());
                            }
                        }
                    }

                    // Send StagingPending to this player
                    let _ = sender.send(ServerMessage::StagingPending {
                        region_id: arrival_region_uuid.to_string(),
                        region_name: arrival_region.name.clone(),
                    });

                    tracing::info!(
                        pc_id = %pc_id,
                        region_id = %arrival_region_uuid,
                        "PC added to existing pending staging, waiting for DM approval"
                    );

                    return None; // Return early, will send SceneChanged when staging approved
                }

                // No pending staging - generate a new proposal
                tracing::info!(
                    "No valid staging for region {}, generating proposal",
                    arrival_region_uuid
                );

                // Send StagingPending to player immediately
                let _ = sender.send(ServerMessage::StagingPending {
                    region_id: arrival_region_uuid.to_string(),
                    region_name: arrival_region.name.clone(),
                });

                // Generate staging proposal
                let proposal = match state
                    .staging_service
                    .generate_proposal(
                        world_id,
                        arrival_region_uuid,
                        location_uuid,
                        &location_name,
                        &game_time,
                        default_ttl,
                        None, // No DM guidance yet
                    )
                    .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("Failed to generate staging proposal: {}", e);
                        // Fall back to simple rule-based presence
                        let npc_relationships = state
                            .repository
                            .characters()
                            .get_npcs_related_to_region(arrival_region_uuid)
                            .await
                            .unwrap_or_default();

                        let time_of_day = game_time.time_of_day();
                        let region_items = fetch_region_items(&state, arrival_region_uuid).await;
                        return Some(ServerMessage::SceneChanged {
                            pc_id: pc_id.clone(),
                            region: wrldbldr_protocol::RegionData {
                                id: arrival_region_uuid.to_string(),
                                name: arrival_region.name.clone(),
                                location_id: location_uuid.to_string(),
                                location_name: location_name.clone(),
                                backdrop_asset: backdrop.clone(),
                                atmosphere: arrival_region.atmosphere.clone(),
                                map_asset: map_asset.clone(),
                            },
                            npcs_present: npc_relationships
                                .into_iter()
                                .filter_map(|(character, rel_type)| {
                                    if rel_type.is_npc_present(time_of_day) {
                                        Some(wrldbldr_protocol::NpcPresenceData {
                                            character_id: character.id.to_string(),
                                            name: character.name,
                                            sprite_asset: character.sprite_asset,
                                            portrait_asset: character.portrait_asset,
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                            navigation: wrldbldr_protocol::NavigationData {
                                connected_regions: Vec::new(),
                                exits: Vec::new(),
                            },
                            region_items,
                        });
                    }
                };

                let request_id = proposal.request_id.clone();

                // Get previous staging for reference
                let previous_staging = state
                    .staging_service
                    .get_previous_staging(arrival_region_uuid)
                    .await
                    .ok()
                    .flatten();
                let previous_staging_info = previous_staging.map(|s| wrldbldr_protocol::PreviousStagingInfo {
                    staging_id: s.id.to_string(),
                    approved_at: s.approved_at.to_rfc3339(),
                    npcs: s
                        .npcs
                        .into_iter()
                        .map(|npc| wrldbldr_protocol::StagedNpcInfo {
                            character_id: npc.character_id.to_string(),
                            name: npc.name,
                            sprite_asset: npc.sprite_asset,
                            portrait_asset: npc.portrait_asset,
                            is_present: npc.is_present,
                            reasoning: npc.reasoning,
                            is_hidden_from_players: npc.is_hidden_from_players,
                        })
                        .collect(),
                });

                // Convert proposal NPCs to protocol format
                let rule_based_npcs: Vec<wrldbldr_protocol::StagedNpcInfo> = proposal
                    .rule_based_npcs
                    .iter()
                    .map(|npc| wrldbldr_protocol::StagedNpcInfo {
                        character_id: npc.character_id.clone(),
                        name: npc.name.clone(),
                        sprite_asset: npc.sprite_asset.clone(),
                        portrait_asset: npc.portrait_asset.clone(),
                        is_present: npc.is_present,
                        reasoning: npc.reasoning.clone(),
                        is_hidden_from_players: false,
                    })
                    .collect();

                let llm_based_npcs: Vec<wrldbldr_protocol::StagedNpcInfo> = if use_llm {
                    proposal
                        .llm_based_npcs
                        .iter()
                        .map(|npc| wrldbldr_protocol::StagedNpcInfo {
                            character_id: npc.character_id.clone(),
                            name: npc.name.clone(),
                            sprite_asset: npc.sprite_asset.clone(),
                            portrait_asset: npc.portrait_asset.clone(),
                            is_present: npc.is_present,
                            reasoning: npc.reasoning.clone(),
                            is_hidden_from_players: npc.is_hidden_from_players,
                        })
                        .collect()
                } else {
                    Vec::new() // Don't include LLM suggestions if disabled
                };

                // Store pending staging approval
                let mut pending_approval = crate::infrastructure::session::PendingStagingApproval::new(
                    request_id.clone(),
                    arrival_region_uuid,
                    location_uuid,
                    world_id,
                    arrival_region.name.clone(),
                    location_name.clone(),
                    proposal,
                );
                pending_approval.add_waiting_pc(pc_uuid, pc.name.clone(), client_id, user_id.clone());

                {
                    let mut sessions = state.sessions.write().await;
                    if let Some(session) = sessions.get_session_mut(session_id) {
                        session.add_pending_staging_approval(pending_approval);
                    }
                }

                // Send StagingApprovalRequired to DM
                let approval_msg = ServerMessage::StagingApprovalRequired {
                    request_id,
                    region_id: arrival_region_uuid.to_string(),
                    region_name: arrival_region.name.clone(),
                    location_id: location_uuid.to_string(),
                    location_name: location_name.clone(),
                    game_time: wrldbldr_protocol::GameTime::new(
                        game_time.day_ordinal(),
                        game_time.current().hour() as u8,
                        game_time.current().minute() as u8,
                        game_time.is_paused(),
                    ),
                    previous_staging: previous_staging_info,
                    rule_based_npcs,
                    llm_based_npcs,
                    default_ttl_hours: default_ttl,
                    waiting_pcs: vec![wrldbldr_protocol::WaitingPcInfo {
                        pc_id: pc_id.clone(),
                        pc_name: pc.name.clone(),
                        player_id: user_id,
                    }],
                };

                {
                    let sessions = state.sessions.read().await;
                    if let Some(session) = sessions.get_session(session_id) {
                        session.send_to_dm(&approval_msg);
                    }
                }

                tracing::info!(
                    pc_id = %pc_id,
                    region_id = %arrival_region_uuid,
                    "Staging approval request sent to DM"
                );

                return None; // Return early, will send SceneChanged when staging approved
            };

            // Get navigation options
            let connections = state.repository.regions().get_connections(arrival_region_uuid).await.unwrap_or_default();
            let exits = state.repository.regions().get_exits(arrival_region_uuid).await.unwrap_or_default();

            let mut connected_regions = Vec::new();
            for conn in connections {
                if let Ok(Some(target)) = state.repository.regions().get(conn.to_region).await {
                    connected_regions.push(wrldbldr_protocol::NavigationTarget {
                        region_id: conn.to_region.to_string(),
                        name: target.name,
                        is_locked: conn.is_locked,
                        lock_description: conn.lock_description,
                    });
                }
            }

            let mut exit_targets = Vec::new();
            for exit in exits {
                if let Ok(Some(target_loc)) = state.repository.locations().get(exit.to_location).await {
                    exit_targets.push(wrldbldr_protocol::NavigationExit {
                        location_id: exit.to_location.to_string(),
                        location_name: target_loc.name,
                        arrival_region_id: exit.arrival_region_id.to_string(),
                        description: exit.description,
                    });
                }
            }

            let region_items = fetch_region_items(&state, arrival_region_uuid).await;
            Some(ServerMessage::SceneChanged {
                pc_id,
                region: wrldbldr_protocol::RegionData {
                    id: arrival_region_uuid.to_string(),
                    name: arrival_region.name,
                    location_id: location_uuid.to_string(),
                    location_name: target_location.name,
                    backdrop_asset: backdrop,
                    atmosphere: arrival_region.atmosphere,
                    map_asset,
                },
                npcs_present,
                navigation: wrldbldr_protocol::NavigationData {
                    connected_regions,
                    exits: exit_targets,
                },
                region_items,
            })
        }

        // =====================================================================
        // Staging System (NPC Presence Approval)
        // =====================================================================

        ClientMessage::StagingApprovalResponse {
            request_id,
            approved_npcs,
            ttl_hours,
            source,
        } => {
            tracing::info!(
                request_id = %request_id,
                npc_count = approved_npcs.len(),
                ttl_hours = ttl_hours,
                source = %source,
                "Staging approval response received"
            );

            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Get the pending staging approval
            let pending = {
                let sessions = state.sessions.read().await;
                let session = match sessions.get_session(session_id) {
                    Some(s) => s,
                    None => {
                        return Some(ServerMessage::Error {
                            code: "SESSION_NOT_FOUND".to_string(),
                            message: "Session not found".to_string(),
                        });
                    }
                };
                session.get_pending_staging_approval(&request_id).cloned()
            };

            let pending = match pending {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: "STAGING_NOT_FOUND".to_string(),
                        message: format!("Pending staging request {} not found", request_id),
                    });
                }
            };

            // Parse staging source
            let staging_source = match source.as_str() {
                "rule" => wrldbldr_domain::entities::StagingSource::RuleBased,
                "llm" => wrldbldr_domain::entities::StagingSource::LlmBased,
                "custom" => wrldbldr_domain::entities::StagingSource::DmCustomized,
                _ => wrldbldr_domain::entities::StagingSource::DmCustomized,
            };

            // Get character data for approved NPCs
            let mut approved_npc_data = Vec::new();
            for npc_info in &approved_npcs {
                let char_id = match uuid::Uuid::parse_str(&npc_info.character_id) {
                    Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                    Err(_) => continue,
                };

                // Find character in proposal to get name and assets
                let (name, sprite, portrait) = pending.proposal.rule_based_npcs
                    .iter()
                    .chain(pending.proposal.llm_based_npcs.iter())
                    .find(|n| n.character_id == npc_info.character_id)
                    .map(|n| (n.name.clone(), n.sprite_asset.clone(), n.portrait_asset.clone()))
                    .unwrap_or_else(|| ("Unknown".to_string(), None, None));

                approved_npc_data.push(wrldbldr_engine_app::application::services::staging_service::ApprovedNpcData {
                    character_id: char_id,
                    name,
                    sprite_asset: sprite,
                    portrait_asset: portrait,
                    is_present: npc_info.is_present,
                    is_hidden_from_players: npc_info.is_hidden_from_players,
                    reasoning: npc_info.reasoning.clone().unwrap_or_else(|| "DM approved".to_string()),
                });
            }

            // Get game time
            let game_time = {
                let sessions = state.sessions.read().await;
                sessions.get_session(session_id)
                    .map(|s| s.game_time().clone())
                    .unwrap_or_default()
            };

            // Get DM user ID for approved_by
            let dm_user_id = state.async_session_port.get_client_user_id(&client_id_str).await
                .unwrap_or_else(|| client_id_str.clone());

            // Approve the staging
            let staging = match state.staging_service.approve_staging(
                pending.region_id,
                pending.location_id,
                pending.world_id,
                &game_time,
                approved_npc_data,
                ttl_hours,
                staging_source,
                &dm_user_id,
                None,
            ).await {
                Ok(s) => s,
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "STAGING_APPROVAL_FAILED".to_string(),
                        message: format!("Failed to approve staging: {}", e),
                    });
                }
            };

            // Build the NPC presence list for players
            let npcs_present: Vec<wrldbldr_protocol::NpcPresentInfo> = staging.npcs
                .iter()
                .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
                .map(|npc| wrldbldr_protocol::NpcPresentInfo {
                    character_id: npc.character_id.to_string(),
                    name: npc.name.clone(),
                    sprite_asset: npc.sprite_asset.clone(),
                    portrait_asset: npc.portrait_asset.clone(),
                    is_hidden_from_players: false,
                })
                .collect();

            // Send StagingReady to all waiting PCs
            let staging_ready = ServerMessage::StagingReady {
                region_id: pending.region_id.to_string(),
                npcs_present: npcs_present.clone(),
            };

            {
                let sessions = state.sessions.read().await;
                if let Some(session) = sessions.get_session(session_id) {
                    for waiting_pc in &pending.waiting_pcs {
                        session.send_to_client(waiting_pc.client_id, &staging_ready);
                        
                        // Also send SceneChanged with the NPCs
                        // Get region and location data for the scene change
                        let map_asset = state.repository.locations().get(pending.location_id).await
                            .ok()
                            .flatten()
                            .and_then(|loc| loc.map_asset);
                        if let Ok(Some(region)) = state.repository.regions().get(pending.region_id).await {
                            let connections = state.repository.regions().get_connections(pending.region_id).await.unwrap_or_default();
                            let exits = state.repository.regions().get_exits(pending.region_id).await.unwrap_or_default();

                            let mut connected_regions = Vec::new();
                            for conn in connections {
                                if let Ok(Some(target)) = state.repository.regions().get(conn.to_region).await {
                                    connected_regions.push(wrldbldr_protocol::NavigationTarget {
                                        region_id: conn.to_region.to_string(),
                                        name: target.name,
                                        is_locked: conn.is_locked,
                                        lock_description: conn.lock_description,
                                    });
                                }
                            }

                            let mut exit_targets = Vec::new();
                            for exit in exits {
                                if let Ok(Some(target_loc)) = state.repository.locations().get(exit.to_location).await {
                                    exit_targets.push(wrldbldr_protocol::NavigationExit {
                                        location_id: exit.to_location.to_string(),
                                        location_name: target_loc.name,
                                        arrival_region_id: exit.arrival_region_id.to_string(),
                                        description: exit.description,
                                    });
                                }
                            }

                            let region_items = fetch_region_items(&state, pending.region_id).await;
                            let scene_changed = ServerMessage::SceneChanged {
                                pc_id: waiting_pc.pc_id.to_string(),
                                region: wrldbldr_protocol::RegionData {
                                    id: pending.region_id.to_string(),
                                    name: region.name.clone(),
                                    location_id: pending.location_id.to_string(),
                                    location_name: pending.location_name.clone(),
                                    backdrop_asset: region.backdrop_asset.clone(),
                                    atmosphere: region.atmosphere.clone(),
                                    map_asset: map_asset.clone(),
                                },
                                npcs_present: npcs_present.iter().map(|npc| wrldbldr_protocol::NpcPresenceData {
                                    character_id: npc.character_id.clone(),
                                    name: npc.name.clone(),
                                    sprite_asset: npc.sprite_asset.clone(),
                                    portrait_asset: npc.portrait_asset.clone(),
                                }).collect(),
                                navigation: wrldbldr_protocol::NavigationData {
                                    connected_regions,
                                    exits: exit_targets,
                                },
                                region_items,
                            };
                            session.send_to_client(waiting_pc.client_id, &scene_changed);
                        }
                    }
                }
            }

            // Remove the pending staging approval
            {
                let mut sessions = state.sessions.write().await;
                if let Some(session) = sessions.get_session_mut(session_id) {
                    session.remove_pending_staging_approval(&request_id);
                }
            }

            tracing::info!(
                request_id = %request_id,
                region_id = %pending.region_id,
                waiting_pcs = pending.waiting_pcs.len(),
                "Staging approved and sent to waiting PCs"
            );

            None // No direct response to DM
        }

        ClientMessage::StagingRegenerateRequest {
            request_id,
            guidance,
        } => {
            tracing::info!(
                request_id = %request_id,
                guidance = %guidance,
                "Staging regenerate request received"
            );

            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Get the pending staging approval
            let pending = {
                let sessions = state.sessions.read().await;
                let session = match sessions.get_session(session_id) {
                    Some(s) => s,
                    None => {
                        return Some(ServerMessage::Error {
                            code: "SESSION_NOT_FOUND".to_string(),
                            message: "Session not found".to_string(),
                        });
                    }
                };
                session.get_pending_staging_approval(&request_id).cloned()
            };

            let pending = match pending {
                Some(p) => p,
                None => {
                    return Some(ServerMessage::Error {
                        code: "STAGING_NOT_FOUND".to_string(),
                        message: format!("Pending staging request {} not found", request_id),
                    });
                }
            };

            // Get game time
            let game_time = {
                let sessions = state.sessions.read().await;
                sessions.get_session(session_id)
                    .map(|s| s.game_time().clone())
                    .unwrap_or_default()
            };

            // Regenerate LLM suggestions
            let new_suggestions = match state.staging_service.regenerate_suggestions(
                pending.world_id,
                pending.region_id,
                &pending.location_name,
                &game_time,
                &guidance,
            ).await {
                Ok(s) => s,
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "REGENERATION_FAILED".to_string(),
                        message: format!("Failed to regenerate suggestions: {}", e),
                    });
                }
            };

            // Convert to protocol format
            let llm_based_npcs: Vec<wrldbldr_protocol::StagedNpcInfo> = new_suggestions
                .into_iter()
                .map(|npc| wrldbldr_protocol::StagedNpcInfo {
                    character_id: npc.character_id,
                    name: npc.name,
                    sprite_asset: npc.sprite_asset,
                    portrait_asset: npc.portrait_asset,
                    is_present: npc.is_present,
                    reasoning: npc.reasoning,
                    is_hidden_from_players: npc.is_hidden_from_players,
                })
                .collect();

            tracing::info!(
                request_id = %request_id,
                new_count = llm_based_npcs.len(),
                "Staging suggestions regenerated"
            );

            Some(ServerMessage::StagingRegenerated {
                request_id,
                llm_based_npcs,
            })
        }

        ClientMessage::PreStageRegion {
            region_id,
            npcs,
            ttl_hours,
        } => {
            tracing::info!(
                region_id = %region_id,
                npc_count = npcs.len(),
                ttl_hours = ttl_hours,
                "Pre-stage region request received"
            );

            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse region ID
            let region_uuid = match uuid::Uuid::parse_str(&region_id) {
                Ok(uuid) => wrldbldr_domain::RegionId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_REGION_ID".to_string(),
                        message: "Invalid region ID format".to_string(),
                    });
                }
            };

            // Get region and location
            let region = match state.repository.regions().get(region_uuid).await {
                Ok(Some(r)) => r,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "REGION_NOT_FOUND".to_string(),
                        message: "Region not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch region: {}", e),
                    });
                }
            };

            let location = match state.repository.locations().get(region.location_id).await {
                Ok(Some(l)) => l,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "LOCATION_NOT_FOUND".to_string(),
                        message: "Location not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch location: {}", e),
                    });
                }
            };

            // Get game time
            let game_time = {
                let sessions = state.sessions.read().await;
                sessions.get_session(session_id)
                    .map(|s| s.game_time().clone())
                    .unwrap_or_default()
            };

            // Get DM user ID
            let dm_user_id = state.async_session_port.get_client_user_id(&client_id_str).await
                .unwrap_or_else(|| client_id_str.clone());

            // Build approved NPC data
            let mut approved_npc_data = Vec::new();
            for npc_info in &npcs {
                let char_id = match uuid::Uuid::parse_str(&npc_info.character_id) {
                    Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                    Err(_) => continue,
                };

                // Fetch character for name and assets
                let (name, sprite, portrait) = match state.repository.characters().get(char_id).await {
                    Ok(Some(c)) => (c.name, c.sprite_asset, c.portrait_asset),
                    _ => ("Unknown".to_string(), None, None),
                };

                approved_npc_data.push(wrldbldr_engine_app::application::services::staging_service::ApprovedNpcData {
                    character_id: char_id,
                    name,
                    sprite_asset: sprite,
                    portrait_asset: portrait,
                    is_present: npc_info.is_present,
                    is_hidden_from_players: npc_info.is_hidden_from_players,
                    reasoning: npc_info.reasoning.clone().unwrap_or_else(|| "Pre-staged by DM".to_string()),
                });
            }

            // Pre-stage the region
            match state.staging_service.pre_stage_region(
                region_uuid,
                region.location_id,
                location.world_id,
                &game_time,
                approved_npc_data,
                ttl_hours,
                &dm_user_id,
            ).await {
                Ok(staging) => {
                    tracing::info!(
                        staging_id = %staging.id,
                        region_id = %region_id,
                        npc_count = staging.npcs.len(),
                        "Region pre-staged successfully"
                    );
                    None // Success, no response needed
                }
                Err(e) => {
                    Some(ServerMessage::Error {
                        code: "PRESTAGE_FAILED".to_string(),
                        message: format!("Failed to pre-stage region: {}", e),
                    })
                }
            }
        }

        // =====================================================================
        // Inventory Actions
        // =====================================================================

        ClientMessage::EquipItem { pc_id, item_id } => {
            tracing::info!(pc_id = %pc_id, item_id = %item_id, "Equip item request");

            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            let item_uuid = match uuid::Uuid::parse_str(&item_id) {
                Ok(uuid) => wrldbldr_domain::ItemId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_ITEM_ID".to_string(),
                        message: "Invalid item ID format".to_string(),
                    });
                }
            };

            // Get the item to find its name and verify ownership
            let item = match state.repository.player_characters().get_inventory_item(pc_uuid, item_uuid).await {
                Ok(Some(item)) => item,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "ITEM_NOT_FOUND".to_string(),
                        message: "Item not found in inventory".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch item: {}", e),
                    });
                }
            };

            // Update the item to be equipped
            if let Err(e) = state.repository.player_characters().update_inventory_item(
                pc_uuid,
                item_uuid,
                item.quantity, // keep quantity unchanged
                true, // is_equipped = true
            ).await {
                return Some(ServerMessage::Error {
                    code: "UPDATE_ERROR".to_string(),
                    message: format!("Failed to equip item: {}", e),
                });
            }

            Some(ServerMessage::ItemEquipped {
                pc_id,
                item_id,
                item_name: item.item.name,
            })
        }

        ClientMessage::UnequipItem { pc_id, item_id } => {
            tracing::info!(pc_id = %pc_id, item_id = %item_id, "Unequip item request");

            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            let item_uuid = match uuid::Uuid::parse_str(&item_id) {
                Ok(uuid) => wrldbldr_domain::ItemId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_ITEM_ID".to_string(),
                        message: "Invalid item ID format".to_string(),
                    });
                }
            };

            // Get the item to find its name
            let item = match state.repository.player_characters().get_inventory_item(pc_uuid, item_uuid).await {
                Ok(Some(item)) => item,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "ITEM_NOT_FOUND".to_string(),
                        message: "Item not found in inventory".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch item: {}", e),
                    });
                }
            };

            // Update the item to be unequipped
            if let Err(e) = state.repository.player_characters().update_inventory_item(
                pc_uuid,
                item_uuid,
                item.quantity, // keep quantity unchanged
                false, // is_equipped = false
            ).await {
                return Some(ServerMessage::Error {
                    code: "UPDATE_ERROR".to_string(),
                    message: format!("Failed to unequip item: {}", e),
                });
            }

            Some(ServerMessage::ItemUnequipped {
                pc_id,
                item_id,
                item_name: item.item.name,
            })
        }

        ClientMessage::DropItem { pc_id, item_id, quantity } => {
            tracing::info!(pc_id = %pc_id, item_id = %item_id, quantity = quantity, "Drop item request");

            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            let item_uuid = match uuid::Uuid::parse_str(&item_id) {
                Ok(uuid) => wrldbldr_domain::ItemId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_ITEM_ID".to_string(),
                        message: "Invalid item ID format".to_string(),
                    });
                }
            };

            // Get the item to find its name and current quantity
            let item = match state.repository.player_characters().get_inventory_item(pc_uuid, item_uuid).await {
                Ok(Some(item)) => item,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "ITEM_NOT_FOUND".to_string(),
                        message: "Item not found in inventory".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch item: {}", e),
                    });
                }
            };

            let item_name = item.item.name.clone();
            let dropped_quantity = quantity.min(item.quantity);

            // Get PC's current region to place the dropped item
            let pc = match state.repository.player_characters().get(pc_uuid).await {
                Ok(Some(pc)) => pc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "PC_NOT_FOUND".to_string(),
                        message: "Player character not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch PC: {}", e),
                    });
                }
            };

            let current_region_id = match pc.current_region_id {
                Some(region_id) => region_id,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_REGION".to_string(),
                        message: "PC is not in a region, cannot drop item".to_string(),
                    });
                }
            };

            // Place the item in the region
            if let Err(e) = state.repository.regions().add_item_to_region(current_region_id, item_uuid).await {
                return Some(ServerMessage::Error {
                    code: "DROP_ERROR".to_string(),
                    message: format!("Failed to place item in region: {}", e),
                });
            }

            // Remove from PC inventory (or reduce quantity)
            if dropped_quantity >= item.quantity {
                // Remove the item entirely from inventory
                if let Err(e) = state.repository.player_characters().remove_inventory_item(pc_uuid, item_uuid).await {
                    // Try to undo the region placement
                    let _ = state.repository.regions().remove_item_from_region(current_region_id, item_uuid).await;
                    return Some(ServerMessage::Error {
                        code: "DELETE_ERROR".to_string(),
                        message: format!("Failed to drop item: {}", e),
                    });
                }
            } else {
                // Reduce quantity in inventory
                let new_quantity = item.quantity - dropped_quantity;
                if let Err(e) = state.repository.player_characters().update_inventory_item(
                    pc_uuid,
                    item_uuid,
                    new_quantity,
                    item.equipped, // keep equipped status unchanged
                ).await {
                    // Try to undo the region placement
                    let _ = state.repository.regions().remove_item_from_region(current_region_id, item_uuid).await;
                    return Some(ServerMessage::Error {
                        code: "UPDATE_ERROR".to_string(),
                        message: format!("Failed to update item quantity: {}", e),
                    });
                }
            }

            tracing::info!(
                pc_id = %pc_id,
                item_id = %item_id,
                item_name = %item_name,
                region_id = %current_region_id,
                quantity = dropped_quantity,
                "Item dropped in region"
            );

            Some(ServerMessage::ItemDropped {
                pc_id,
                item_id,
                item_name,
                quantity: dropped_quantity,
            })
        }

        ClientMessage::PickupItem { pc_id, item_id } => {
            tracing::info!(pc_id = %pc_id, item_id = %item_id, "Pickup item request");

            // Validate input parameters
            if pc_id.trim().is_empty() {
                tracing::warn!("Empty PC ID provided for pickup request");
                return Some(ServerMessage::Error {
                    code: "INVALID_PC_ID".to_string(),
                    message: "PC ID cannot be empty".to_string(),
                });
            }

            if item_id.trim().is_empty() {
                tracing::warn!("Empty item ID provided for pickup request");
                return Some(ServerMessage::Error {
                    code: "INVALID_ITEM_ID".to_string(),
                    message: "Item ID cannot be empty".to_string(),
                });
            }

            // Parse UUIDs
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(e) => {
                    tracing::warn!(pc_id = %pc_id, error = %e, "Invalid PC ID format for pickup");
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            let item_uuid = match uuid::Uuid::parse_str(&item_id) {
                Ok(uuid) => wrldbldr_domain::ItemId::from_uuid(uuid),
                Err(e) => {
                    tracing::warn!(item_id = %item_id, error = %e, "Invalid item ID format for pickup");
                    return Some(ServerMessage::Error {
                        code: "INVALID_ITEM_ID".to_string(),
                        message: "Invalid item ID format".to_string(),
                    });
                }
            };

            // Get PC's current region
            let pc = match state.repository.player_characters().get(pc_uuid).await {
                Ok(Some(pc)) => pc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "PC_NOT_FOUND".to_string(),
                        message: "Player character not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch PC: {}", e),
                    });
                }
            };

            let current_region_id = match pc.current_region_id {
                Some(region_id) => region_id,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_REGION".to_string(),
                        message: "PC is not in a region, cannot pick up items".to_string(),
                    });
                }
            };

            // Get region items to verify item is present and get item details
            let region_items = match state.repository.regions().get_region_items(current_region_id).await {
                Ok(items) => items,
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch region items: {}", e),
                    });
                }
            };

            let item = match region_items.iter().find(|i| i.id == item_uuid) {
                Some(item) => item.clone(),
                None => {
                    tracing::warn!(
                        pc_id = %pc_id,
                        item_id = %item_id,
                        region_id = %current_region_id,
                        available_items = region_items.len(),
                        "Attempted to pick up item not in region"
                    );
                    return Some(ServerMessage::Error {
                        code: "ITEM_NOT_IN_REGION".to_string(),
                        message: "Item is not in this region".to_string(),
                    });
                }
            };

            // Additional validation: Check if PC already has this specific item
            // This prevents edge cases where client and server state are out of sync
            match state.repository.player_characters().get_inventory_item(pc_uuid, item_uuid).await {
                Ok(Some(_existing_item)) => {
                    tracing::warn!(
                        pc_id = %pc_id,
                        item_id = %item_id,
                        item_name = %item.name,
                        "PC already has this item in inventory, refusing pickup"
                    );
                    return Some(ServerMessage::Error {
                        code: "ITEM_ALREADY_OWNED".to_string(),
                        message: "You already have this item in your inventory".to_string(),
                    });
                }
                Ok(None) => {
                    // Good, PC doesn't have this item
                    tracing::debug!(pc_id = %pc_id, item_id = %item_id, "Validated PC doesn't already have item");
                }
                Err(e) => {
                    tracing::error!(pc_id = %pc_id, item_id = %item_id, error = %e, "Failed to check PC inventory for duplicate item");
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to validate inventory state: {}", e),
                    });
                }
            }

            // Remove from region first (atomic operation)
            if let Err(e) = state.repository.regions().remove_item_from_region(current_region_id, item_uuid).await {
                return Some(ServerMessage::Error {
                    code: "PICKUP_ERROR".to_string(),
                    message: format!("Failed to remove item from region: {}", e),
                });
            }

            // Add to PC inventory
            if let Err(e) = state.repository.player_characters().add_inventory_item(
                pc_uuid,
                item_uuid,
                1, // quantity - items in regions are single instances
                false, // not equipped by default
                Some(wrldbldr_domain::entities::AcquisitionMethod::Found),
            ).await {
                // Rollback: put item back in region
                let rollback_result = state.repository.regions().add_item_to_region(current_region_id, item_uuid).await;
                if let Err(rollback_error) = rollback_result {
                    tracing::error!(
                        original_error = %e,
                        rollback_error = %rollback_error,
                        "Failed to rollback region placement after inventory error"
                    );
                }
                return Some(ServerMessage::Error {
                    code: "INVENTORY_ERROR".to_string(),
                    message: format!("Failed to add item to inventory: {}", e),
                });
            }

            tracing::info!(
                pc_id = %pc_id,
                item_id = %item_id,
                item_name = %item.name,
                region_id = %current_region_id,
                "Item picked up from region"
            );

            Some(ServerMessage::ItemPickedUp {
                pc_id,
                item_id,
                item_name: item.name,
            })
        }

        // =========================================================================
        // NPC Mood Control (P1.4)
        // =========================================================================

        ClientMessage::SetNpcMood {
            npc_id,
            pc_id,
            mood,
            reason,
        } => {
            // Only DM can set NPC moods
            let client_id_str = client_id.to_string();
            let is_dm = state.async_session_port.is_client_dm(&client_id_str).await;
            if !is_dm {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can set NPC moods".to_string(),
                });
            }

            // Parse IDs
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            // Parse mood level
            let mood_level = match mood.to_lowercase().as_str() {
                "friendly" => wrldbldr_domain::value_objects::MoodLevel::Friendly,
                "neutral" => wrldbldr_domain::value_objects::MoodLevel::Neutral,
                "suspicious" => wrldbldr_domain::value_objects::MoodLevel::Suspicious,
                "hostile" => wrldbldr_domain::value_objects::MoodLevel::Hostile,
                "afraid" => wrldbldr_domain::value_objects::MoodLevel::Afraid,
                "grateful" => wrldbldr_domain::value_objects::MoodLevel::Grateful,
                "annoyed" => wrldbldr_domain::value_objects::MoodLevel::Annoyed,
                "curious" => wrldbldr_domain::value_objects::MoodLevel::Curious,
                "melancholic" => wrldbldr_domain::value_objects::MoodLevel::Melancholic,
                _ => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_MOOD".to_string(),
                        message: format!("Invalid mood: {}. Valid moods: friendly, neutral, suspicious, hostile, afraid, grateful, annoyed, curious, melancholic", mood),
                    });
                }
            };

            // Set the mood via service
            match state.game.mood_service.set_mood(npc_uuid, pc_uuid, mood_level, reason.clone()).await {
                Ok(mood_state) => {
                    // Get NPC name for the response
                    let npc_name = match state.repository.characters().get(npc_uuid).await {
                        Ok(Some(c)) => c.name,
                        _ => "Unknown NPC".to_string(),
                    };

                    tracing::info!(
                        npc_id = %npc_id,
                        pc_id = %pc_id,
                        mood = ?mood_level,
                        reason = ?reason,
                        "DM set NPC mood"
                    );

                    Some(ServerMessage::NpcMoodChanged {
                        npc_id,
                        npc_name,
                        pc_id,
                        mood: format!("{:?}", mood_state.mood),
                        relationship: format!("{:?}", mood_state.relationship),
                        reason,
                    })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to set NPC mood");
                    Some(ServerMessage::Error {
                        code: "MOOD_SET_ERROR".to_string(),
                        message: format!("Failed to set mood: {}", e),
                    })
                }
            }
        }

        ClientMessage::SetNpcRelationship {
            npc_id,
            pc_id,
            relationship,
        } => {
            // Only DM can set NPC relationships
            let client_id_str = client_id.to_string();
            let is_dm = state.async_session_port.is_client_dm(&client_id_str).await;
            if !is_dm {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can set NPC relationships".to_string(),
                });
            }

            // Parse IDs
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            // Parse relationship level
            let relationship_level = match relationship.to_lowercase().as_str() {
                "ally" => wrldbldr_domain::value_objects::RelationshipLevel::Ally,
                "friend" => wrldbldr_domain::value_objects::RelationshipLevel::Friend,
                "acquaintance" => wrldbldr_domain::value_objects::RelationshipLevel::Acquaintance,
                "stranger" => wrldbldr_domain::value_objects::RelationshipLevel::Stranger,
                "rival" => wrldbldr_domain::value_objects::RelationshipLevel::Rival,
                "enemy" => wrldbldr_domain::value_objects::RelationshipLevel::Enemy,
                "nemesis" => wrldbldr_domain::value_objects::RelationshipLevel::Nemesis,
                _ => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_RELATIONSHIP".to_string(),
                        message: format!("Invalid relationship: {}. Valid levels: ally, friend, acquaintance, stranger, rival, enemy, nemesis", relationship),
                    });
                }
            };

            // Set the relationship via service
            match state.game.mood_service.set_relationship(npc_uuid, pc_uuid, relationship_level).await {
                Ok(mood_state) => {
                    // Get NPC name for the response
                    let npc_name = match state.repository.characters().get(npc_uuid).await {
                        Ok(Some(c)) => c.name,
                        _ => "Unknown NPC".to_string(),
                    };

                    tracing::info!(
                        npc_id = %npc_id,
                        pc_id = %pc_id,
                        relationship = ?relationship_level,
                        "DM set NPC relationship"
                    );

                    Some(ServerMessage::NpcMoodChanged {
                        npc_id,
                        npc_name,
                        pc_id,
                        mood: format!("{:?}", mood_state.mood),
                        relationship: format!("{:?}", mood_state.relationship),
                        reason: None,
                    })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to set NPC relationship");
                    Some(ServerMessage::Error {
                        code: "RELATIONSHIP_SET_ERROR".to_string(),
                        message: format!("Failed to set relationship: {}", e),
                    })
                }
            }
        }

        ClientMessage::GetNpcMoods { pc_id } => {
            // Only DM can get all NPC moods
            let client_id_str = client_id.to_string();
            let is_dm = state.async_session_port.is_client_dm(&client_id_str).await;
            if !is_dm {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can view all NPC moods".to_string(),
                });
            }

            // Parse PC ID
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            // Get all NPC moods for this PC
            match state.game.mood_service.get_all_relationships(pc_uuid).await {
                Ok(mood_states) => {
                    // Build response with NPC names
                    let mut moods = Vec::new();
                    for mood_state in mood_states {
                        let npc_name = match state.repository.characters().get(mood_state.npc_id).await {
                            Ok(Some(c)) => c.name,
                            _ => "Unknown NPC".to_string(),
                        };
                        moods.push(NpcMoodData {
                            npc_id: mood_state.npc_id.to_string(),
                            npc_name,
                            mood: format!("{:?}", mood_state.mood),
                            relationship: format!("{:?}", mood_state.relationship),
                            sentiment: mood_state.sentiment,
                            last_reason: mood_state.mood_reason,
                        });
                    }

                    Some(ServerMessage::NpcMoodsResponse {
                        pc_id,
                        moods,
                    })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get NPC moods");
                    Some(ServerMessage::Error {
                        code: "MOODS_GET_ERROR".to_string(),
                        message: format!("Failed to get moods: {}", e),
                    })
                }
            }
        }

        // =========================================================================
        // Actantial Model / Motivations (P1.5)
        // =========================================================================

        ClientMessage::CreateNpcWant { npc_id, want } => {
            // Only DM can create NPC wants
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can create NPC wants".to_string(),
                });
            }

            // Parse NPC ID
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };

            // Build create request
            let create_req = CreateWantRequest {
                description: want.description,
                intensity: want.intensity,
                priority: want.priority,
                visibility: to_domain_visibility(want.visibility),
                target_id: want.target_id,
                target_type: want.target_type.map(|t| target_type_to_string(t)),
                deflection_behavior: want.deflection_behavior,
                tells: want.tells,
            };

            // Create want
            match state.game.actantial_context_service.create_want(npc_uuid, create_req).await {
                Ok(want_id) => {
                    tracing::info!(npc_id = %npc_id, want_id = %want_id, "DM created NPC want");

                    // Get full context to return the created want
                    match state.game.actantial_context_service.get_context(npc_uuid).await {
                        Ok(ctx) => {
                            if let Some(w) = ctx.wants.iter().find(|w| w.want_id == want_id.to_uuid()) {
                                Some(ServerMessage::NpcWantCreated {
                                    npc_id,
                                    want: want_context_to_data(w),
                                })
                            } else {
                                Some(ServerMessage::Error {
                                    code: "WANT_NOT_FOUND".to_string(),
                                    message: "Want created but not found in context".to_string(),
                                })
                            }
                        }
                        Err(e) => Some(ServerMessage::Error {
                            code: "CONTEXT_ERROR".to_string(),
                            message: format!("Failed to fetch context: {}", e),
                        })
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to create NPC want");
                    Some(ServerMessage::Error {
                        code: "CREATE_WANT_ERROR".to_string(),
                        message: format!("Failed to create want: {}", e),
                    })
                }
            }
        }

        ClientMessage::UpdateNpcWant { npc_id, want_id, updates } => {
            // Only DM can update NPC wants
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can update NPC wants".to_string(),
                });
            }

            // Parse IDs
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };

            let want_uuid = match uuid::Uuid::parse_str(&want_id) {
                Ok(uuid) => wrldbldr_domain::WantId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_WANT_ID".to_string(),
                        message: "Invalid want ID format".to_string(),
                    });
                }
            };

            // Build update request
            let update_req = UpdateWantRequest {
                description: updates.description,
                intensity: updates.intensity,
                priority: updates.priority,
                visibility: updates.visibility.map(to_domain_visibility),
                deflection_behavior: updates.deflection_behavior,
                tells: updates.tells,
            };

            // Update want
            match state.game.actantial_context_service.update_want(want_uuid, update_req).await {
                Ok(()) => {
                    tracing::info!(npc_id = %npc_id, want_id = %want_id, "DM updated NPC want");

                    // Get updated want from context
                    match state.game.actantial_context_service.get_context(npc_uuid).await {
                        Ok(ctx) => {
                            if let Some(w) = ctx.wants.iter().find(|w| w.want_id == want_uuid.to_uuid()) {
                                Some(ServerMessage::NpcWantUpdated {
                                    npc_id,
                                    want: want_context_to_data(w),
                                })
                            } else {
                                Some(ServerMessage::Error {
                                    code: "WANT_NOT_FOUND".to_string(),
                                    message: "Want updated but not found in context".to_string(),
                                })
                            }
                        }
                        Err(e) => Some(ServerMessage::Error {
                            code: "CONTEXT_ERROR".to_string(),
                            message: format!("Failed to fetch context: {}", e),
                        })
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to update NPC want");
                    Some(ServerMessage::Error {
                        code: "UPDATE_WANT_ERROR".to_string(),
                        message: format!("Failed to update want: {}", e),
                    })
                }
            }
        }

        ClientMessage::DeleteNpcWant { npc_id, want_id } => {
            // Only DM can delete NPC wants
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can delete NPC wants".to_string(),
                });
            }

            // Parse want ID
            let want_uuid = match uuid::Uuid::parse_str(&want_id) {
                Ok(uuid) => wrldbldr_domain::WantId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_WANT_ID".to_string(),
                        message: "Invalid want ID format".to_string(),
                    });
                }
            };

            // Delete want
            match state.game.actantial_context_service.delete_want(want_uuid).await {
                Ok(()) => {
                    tracing::info!(npc_id = %npc_id, want_id = %want_id, "DM deleted NPC want");
                    Some(ServerMessage::NpcWantDeleted { npc_id, want_id })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to delete NPC want");
                    Some(ServerMessage::Error {
                        code: "DELETE_WANT_ERROR".to_string(),
                        message: format!("Failed to delete want: {}", e),
                    })
                }
            }
        }

        ClientMessage::SetWantTarget { want_id, target_id, target_type } => {
            // Only DM can set want targets
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can set want targets".to_string(),
                });
            }

            // Parse want ID
            let want_uuid = match uuid::Uuid::parse_str(&want_id) {
                Ok(uuid) => wrldbldr_domain::WantId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_WANT_ID".to_string(),
                        message: "Invalid want ID format".to_string(),
                    });
                }
            };

            let target_type_str = target_type_to_string(target_type);

            // Set target
            match state.game.actantial_context_service.set_want_target(want_uuid, &target_id, &target_type_str).await {
                Ok(()) => {
                    tracing::info!(want_id = %want_id, target_id = %target_id, "DM set want target");

                    // Build target response
                    let target = WantTargetData {
                        id: target_id.clone(),
                        name: target_id.clone(), // Name will be resolved by UI if needed
                        target_type,
                        description: None,
                    };

                    Some(ServerMessage::WantTargetSet { want_id, target })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to set want target");
                    Some(ServerMessage::Error {
                        code: "SET_TARGET_ERROR".to_string(),
                        message: format!("Failed to set target: {}", e),
                    })
                }
            }
        }

        ClientMessage::RemoveWantTarget { want_id } => {
            // Only DM can remove want targets
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can remove want targets".to_string(),
                });
            }

            // Parse want ID
            let want_uuid = match uuid::Uuid::parse_str(&want_id) {
                Ok(uuid) => wrldbldr_domain::WantId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_WANT_ID".to_string(),
                        message: "Invalid want ID format".to_string(),
                    });
                }
            };

            // Remove target
            match state.game.actantial_context_service.remove_want_target(want_uuid).await {
                Ok(()) => {
                    tracing::info!(want_id = %want_id, "DM removed want target");
                    Some(ServerMessage::WantTargetRemoved { want_id })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to remove want target");
                    Some(ServerMessage::Error {
                        code: "REMOVE_TARGET_ERROR".to_string(),
                        message: format!("Failed to remove target: {}", e),
                    })
                }
            }
        }

        ClientMessage::AddActantialView { npc_id, want_id, target_id, target_type, role, reason } => {
            // Only DM can add actantial views
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can add actantial views".to_string(),
                });
            }

            // Parse IDs
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };

            let want_uuid = match uuid::Uuid::parse_str(&want_id) {
                Ok(uuid) => wrldbldr_domain::WantId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_WANT_ID".to_string(),
                        message: "Invalid want ID format".to_string(),
                    });
                }
            };

            let domain_role = to_domain_role(role);
            let service_actor_type = to_service_actor_type(target_type);

            // Add view
            match state.game.actantial_context_service.add_actantial_view(
                npc_uuid, want_uuid, &target_id, service_actor_type, domain_role, reason.clone()
            ).await {
                Ok(()) => {
                    tracing::info!(npc_id = %npc_id, want_id = %want_id, target_id = %target_id, role = ?role, "DM added actantial view");

                    // Resolve target name
                    let target_name = match target_type {
                        ActorTypeData::Npc => {
                            if let Ok(uuid) = uuid::Uuid::parse_str(&target_id) {
                                let char_id = wrldbldr_domain::CharacterId::from_uuid(uuid);
                                match state.repository.characters().get(char_id).await {
                                    Ok(Some(c)) => c.name,
                                    _ => "Unknown NPC".to_string(),
                                }
                            } else {
                                "Unknown".to_string()
                            }
                        }
                        ActorTypeData::Pc => {
                            if let Ok(uuid) = uuid::Uuid::parse_str(&target_id) {
                                let pc_id = wrldbldr_domain::PlayerCharacterId::from_uuid(uuid);
                                match state.repository.player_characters().get(pc_id).await {
                                    Ok(Some(pc)) => pc.name,
                                    _ => "Unknown PC".to_string(),
                                }
                            } else {
                                "Unknown".to_string()
                            }
                        }
                    };

                    let view = ActantialViewData {
                        want_id: want_id.clone(),
                        target_id: target_id.clone(),
                        target_name,
                        target_type,
                        role,
                        reason,
                    };

                    Some(ServerMessage::ActantialViewAdded { npc_id, view })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to add actantial view");
                    Some(ServerMessage::Error {
                        code: "ADD_VIEW_ERROR".to_string(),
                        message: format!("Failed to add view: {}", e),
                    })
                }
            }
        }

        ClientMessage::RemoveActantialView { npc_id, want_id, target_id, role } => {
            // Only DM can remove actantial views
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can remove actantial views".to_string(),
                });
            }

            // Parse IDs
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };

            let want_uuid = match uuid::Uuid::parse_str(&want_id) {
                Ok(uuid) => wrldbldr_domain::WantId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_WANT_ID".to_string(),
                        message: "Invalid want ID format".to_string(),
                    });
                }
            };

            let domain_role = to_domain_role(role);

            // Try NPC first, then PC (we don't know the type from the message)
            let npc_result = if let Ok(uuid) = uuid::Uuid::parse_str(&target_id) {
                let char_id = wrldbldr_domain::CharacterId::from_uuid(uuid);
                state.game.actantial_context_service.remove_actantial_view(
                    npc_uuid, want_uuid, &target_id, ActorTargetType::Npc, domain_role
                ).await
            } else {
                Err(anyhow::anyhow!("Invalid target ID"))
            };

            match npc_result {
                Ok(()) => {
                    tracing::info!(npc_id = %npc_id, want_id = %want_id, target_id = %target_id, role = ?role, "DM removed actantial view");
                    Some(ServerMessage::ActantialViewRemoved { npc_id, want_id, target_id, role })
                }
                Err(_) => {
                    // Try as PC
                    match state.game.actantial_context_service.remove_actantial_view(
                        npc_uuid, want_uuid, &target_id, ActorTargetType::Pc, domain_role
                    ).await {
                        Ok(()) => {
                            tracing::info!(npc_id = %npc_id, want_id = %want_id, target_id = %target_id, role = ?role, "DM removed actantial view (PC)");
                            Some(ServerMessage::ActantialViewRemoved { npc_id, want_id, target_id, role })
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to remove actantial view");
                            Some(ServerMessage::Error {
                                code: "REMOVE_VIEW_ERROR".to_string(),
                                message: format!("Failed to remove view: {}", e),
                            })
                        }
                    }
                }
            }
        }

        ClientMessage::GetNpcActantialContext { npc_id } => {
            // Only DM can get NPC actantial context
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can view NPC actantial context".to_string(),
                });
            }

            // Parse NPC ID
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };

            // Get context
            match state.game.actantial_context_service.get_context(npc_uuid).await {
                Ok(ctx) => {
                    tracing::info!(npc_id = %npc_id, want_count = ctx.wants.len(), "Fetched NPC actantial context");
                    let context = actantial_context_to_data(&ctx);
                    Some(ServerMessage::NpcActantialContextResponse { npc_id, context })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get NPC actantial context");
                    Some(ServerMessage::Error {
                        code: "CONTEXT_ERROR".to_string(),
                        message: format!("Failed to get context: {}", e),
                    })
                }
            }
        }

        ClientMessage::GetWorldGoals { world_id } => {
            // Only DM can get world goals
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can view world goals".to_string(),
                });
            }

            // Parse world ID
            let world_uuid = match uuid::Uuid::parse_str(&world_id) {
                Ok(uuid) => wrldbldr_domain::WorldId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_WORLD_ID".to_string(),
                        message: "Invalid world ID format".to_string(),
                    });
                }
            };

            // Get goals
            match state.game.actantial_context_service.get_world_goals(world_uuid).await {
                Ok(goals) => {
                    tracing::info!(world_id = %world_id, goal_count = goals.len(), "Fetched world goals");
                    let goal_data: Vec<GoalData> = goals.iter().map(goal_to_data).collect();
                    Some(ServerMessage::WorldGoalsResponse { world_id, goals: goal_data })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get world goals");
                    Some(ServerMessage::Error {
                        code: "GOALS_ERROR".to_string(),
                        message: format!("Failed to get goals: {}", e),
                    })
                }
            }
        }

        ClientMessage::CreateGoal { world_id, goal } => {
            // Only DM can create goals
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can create goals".to_string(),
                });
            }

            // Parse world ID
            let world_uuid = match uuid::Uuid::parse_str(&world_id) {
                Ok(uuid) => wrldbldr_domain::WorldId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_WORLD_ID".to_string(),
                        message: "Invalid world ID format".to_string(),
                    });
                }
            };

            // Create goal
            match state.game.actantial_context_service.create_goal(world_uuid, goal.name.clone(), goal.description.clone()).await {
                Ok(goal_id) => {
                    tracing::info!(world_id = %world_id, goal_id = %goal_id, name = %goal.name, "DM created goal");

                    let created_goal = GoalData {
                        id: goal_id.to_string(),
                        name: goal.name,
                        description: goal.description,
                        usage_count: 0,
                    };

                    Some(ServerMessage::GoalCreated { world_id, goal: created_goal })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to create goal");
                    Some(ServerMessage::Error {
                        code: "CREATE_GOAL_ERROR".to_string(),
                        message: format!("Failed to create goal: {}", e),
                    })
                }
            }
        }

        ClientMessage::UpdateGoal { goal_id, updates } => {
            // Only DM can update goals
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can update goals".to_string(),
                });
            }

            // Parse goal ID
            let goal_uuid = match uuid::Uuid::parse_str(&goal_id) {
                Ok(uuid) => wrldbldr_domain::GoalId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_GOAL_ID".to_string(),
                        message: "Invalid goal ID format".to_string(),
                    });
                }
            };

            // Update goal
            match state.game.actantial_context_service.update_goal(goal_uuid, updates.name.clone(), updates.description.clone()).await {
                Ok(()) => {
                    tracing::info!(goal_id = %goal_id, "DM updated goal");

                    // Fetch updated goal
                    match state.repository.goals().get(goal_uuid).await {
                        Ok(Some(g)) => {
                            Some(ServerMessage::GoalUpdated { goal: goal_to_data(&g) })
                        }
                        _ => Some(ServerMessage::GoalUpdated {
                            goal: GoalData {
                                id: goal_id,
                                name: updates.name.unwrap_or_default(),
                                description: updates.description,
                                usage_count: 0,
                            }
                        })
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to update goal");
                    Some(ServerMessage::Error {
                        code: "UPDATE_GOAL_ERROR".to_string(),
                        message: format!("Failed to update goal: {}", e),
                    })
                }
            }
        }

        ClientMessage::DeleteGoal { goal_id } => {
            // Only DM can delete goals
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can delete goals".to_string(),
                });
            }

            // Parse goal ID
            let goal_uuid = match uuid::Uuid::parse_str(&goal_id) {
                Ok(uuid) => wrldbldr_domain::GoalId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_GOAL_ID".to_string(),
                        message: "Invalid goal ID format".to_string(),
                    });
                }
            };

            // Delete goal
            match state.game.actantial_context_service.delete_goal(goal_uuid).await {
                Ok(()) => {
                    tracing::info!(goal_id = %goal_id, "DM deleted goal");
                    Some(ServerMessage::GoalDeleted { goal_id })
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to delete goal");
                    Some(ServerMessage::Error {
                        code: "DELETE_GOAL_ERROR".to_string(),
                        message: format!("Failed to delete goal: {}", e),
                    })
                }
            }
        }

        // =========================================================================
        // Actantial Suggestions (P1.5) - TODO: Implement with async queue
        // These are placeholders that will be implemented in Step 4c/4d
        // =========================================================================

        ClientMessage::SuggestDeflectionBehavior { npc_id, want_id, want_description: _ } => {
            // Only DM can request suggestions
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can request suggestions".to_string(),
                });
            }

            // TODO: Implement with LLM queue integration
            // For now, return placeholder suggestions
            tracing::info!(npc_id = %npc_id, want_id = %want_id, "DM requested deflection behavior suggestions");

            Some(ServerMessage::DeflectionSuggestions {
                npc_id,
                want_id,
                suggestions: vec![
                    "Deflect with a sad smile; change subject to present dangers".to_string(),
                    "Give a vague, non-committal response about the past".to_string(),
                    "Become visibly uncomfortable; firmly redirect conversation".to_string(),
                ],
            })
        }

        ClientMessage::SuggestBehavioralTells { npc_id, want_id, want_description: _ } => {
            // Only DM can request suggestions
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can request suggestions".to_string(),
                });
            }

            // TODO: Implement with LLM queue integration
            tracing::info!(npc_id = %npc_id, want_id = %want_id, "DM requested behavioral tells suggestions");

            Some(ServerMessage::TellsSuggestions {
                npc_id,
                want_id,
                suggestions: vec![
                    "Avoids eye contact when the topic arises".to_string(),
                    "Unconsciously touches a hidden keepsake".to_string(),
                    "Voice becomes slightly strained when lying about it".to_string(),
                ],
            })
        }

        ClientMessage::SuggestWantDescription { npc_id, context: _ } => {
            // Only DM can request suggestions
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can request suggestions".to_string(),
                });
            }

            // TODO: Implement with LLM queue integration
            tracing::info!(npc_id = %npc_id, "DM requested want description suggestions");

            Some(ServerMessage::WantDescriptionSuggestions {
                npc_id,
                suggestions: vec![
                    "Seeks redemption for past misdeeds".to_string(),
                    "Desires recognition from their peers".to_string(),
                    "Yearns to protect their loved ones at any cost".to_string(),
                    "Craves knowledge of forbidden secrets".to_string(),
                ],
            })
        }

        ClientMessage::SuggestActantialReason { npc_id, want_id, target_id, role } => {
            // Only DM can request suggestions
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can request suggestions".to_string(),
                });
            }

            // TODO: Implement with LLM queue integration
            tracing::info!(npc_id = %npc_id, want_id = %want_id, target_id = %target_id, role = ?role, "DM requested actantial reason suggestions");

            let suggestions = match role {
                ActantialRoleData::Helper => vec![
                    "Has proven loyal in past endeavors".to_string(),
                    "Shares similar goals and values".to_string(),
                    "Owes a debt that can be called upon".to_string(),
                ],
                ActantialRoleData::Opponent => vec![
                    "Stands directly in the way of achieving the goal".to_string(),
                    "Has conflicting interests that cannot be reconciled".to_string(),
                    "Represents everything they despise".to_string(),
                ],
                ActantialRoleData::Sender => vec![
                    "Originally inspired this desire through their actions".to_string(),
                    "Tasked them with this mission or quest".to_string(),
                    "Their words planted the seed of this motivation".to_string(),
                ],
                ActantialRoleData::Receiver => vec![
                    "Will benefit most when this goal is achieved".to_string(),
                    "Is the intended recipient of these efforts".to_string(),
                    "Their wellbeing depends on success".to_string(),
                ],
            };

            Some(ServerMessage::ActantialReasonSuggestions {
                npc_id,
                want_id,
                target_id,
                role,
                suggestions,
            })
        }
    }
}

// Note: Unit tests for pickup functionality are included in the protocol crate tests
// and integration tests. The WebSocket handler tests require extensive mocking of
// repository dependencies, which is complex for this implementation.
// 
// The pickup implementation follows the proven DropItem pattern and includes:
// - Input validation (empty strings, invalid UUIDs)
// - Region validation (PC must be in region, item must be in region)  
// - Duplicate item validation (PC can't already own the item)
// - Atomic operations with rollback (remove from region, add to inventory)
// - Comprehensive error handling and logging
// - UI integration with conversation log and inventory refresh
//
// For testing the core logic, see:
// - Protocol message tests: crates/protocol/src/messages.rs
// - Repository operation tests: crates/engine-adapters/src/infrastructure/persistence/
// - End-to-end integration tests: Manual testing with development environment

