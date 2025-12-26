//! Scene-related WebSocket message handlers.
//!
//! This module contains handlers for scene management operations:
//! - Scene change requests (any connected player)
//! - Directorial updates (DM only)
//! - Approval decisions (DM only)

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use wrldbldr_engine_app::application::dto::DMAction;
use wrldbldr_protocol::{
    CharacterData, CharacterPosition, DirectorialContext, InteractionData, SceneData,
    ServerMessage,
};

/// Handles a request to change the current scene.
///
/// This handler:
/// 1. Validates the scene ID format
/// 2. Verifies the client is connected to a world
/// 3. Loads the scene with all its relations (location, characters)
/// 4. Loads available interactions for the scene
/// 5. Broadcasts the scene update to all players in the world
///
/// # Arguments
/// * `state` - The application state containing all services
/// * `client_id` - The WebSocket client ID making the request
/// * `scene_id` - The UUID string of the scene to change to
///
/// # Returns
/// * `None` on success (scene update is broadcast to all players)
/// * `Some(ServerMessage::Error)` on failure
pub async fn handle_request_scene_change(
    state: &AppState,
    client_id: Uuid,
    scene_id: String,
) -> Option<ServerMessage> {
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

    // Get the client's world connection
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "You must join a world before requesting scene changes".to_string(),
            });
        }
    };

    let world_id = match connection.world_id {
        Some(id) => id,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };

    // Load scene from database with relations
    let scene_with_relations = match state
        .core
        .scene_service
        .get_scene_with_relations(scene_uuid)
        .await
    {
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
    let interactions = match state
        .core
        .interaction_service
        .list_interactions(scene_uuid)
        .await
    {
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

    // Update world's current scene and broadcast via world connection manager
    let world_id_typed = wrldbldr_domain::WorldId::from_uuid(world_id);
    state
        .world_state
        .set_current_scene(&world_id_typed, Some(scene_id.clone()));
    state
        .world_connection_manager
        .broadcast_message_to_world(world_id, scene_update)
        .await;

    tracing::info!(
        "Scene change to {} broadcast to world {}",
        scene_id,
        world_id
    );

    None // SceneUpdate is broadcast, no direct response needed
}

/// Handles a directorial update from the DM.
///
/// **DM-only**: This handler requires the client to be the DM of the world.
///
/// Directorial updates allow the DM to set context that influences NPC behavior
/// and narrative generation, including NPC motivations, scene mood, and pacing hints.
///
/// # Arguments
/// * `state` - The application state containing all services
/// * `client_id` - The WebSocket client ID making the request
/// * `context` - The directorial context to apply
///
/// # Returns
/// * `None` on success
/// * `Some(ServerMessage::Error)` if not authorized or not connected
pub async fn handle_directorial_update(
    state: &AppState,
    client_id: Uuid,
    context: DirectorialContext,
) -> Option<ServerMessage> {
    tracing::debug!("Received directorial update");

    // Only DMs should send directorial updates
    let client_id_str = client_id.to_string();

    // Get connection
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    // Extract world_id
    let world_id = match connection.world_id {
        Some(id) => id,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can perform this action".to_string(),
        });
    }

    // Store directorial context in WorldStateManager
    let world_id_domain = wrldbldr_domain::WorldId::from_uuid(world_id);
    state
        .world_state
        .set_directorial_context(&world_id_domain, context.clone());

    tracing::info!(
        world_id = %world_id,
        npc_count = context.npc_motivations.len(),
        "DM updated directorial context"
    );

    None // No response needed
}

/// Handles an approval decision from the DM.
///
/// **DM-only**: This handler requires the client to be the DM of the world.
///
/// When AI-generated content requires DM approval before being shown to players,
/// this handler processes the DM's decision (approve, reject, or edit).
/// The decision is enqueued for asynchronous processing by the DM action queue worker.
///
/// # Arguments
/// * `state` - The application state containing all services
/// * `client_id` - The WebSocket client ID making the request
/// * `request_id` - The ID of the approval request being decided
/// * `decision` - The DM's decision (approve/reject/edit with modifications)
///
/// # Returns
/// * `None` on success (decision is queued for processing)
/// * `Some(ServerMessage::Error)` if not authorized, not connected, or queue error
pub async fn handle_approval_decision(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    decision: wrldbldr_protocol::ApprovalDecision,
) -> Option<ServerMessage> {
    tracing::debug!(
        "Received approval decision for {}: {:?}",
        request_id,
        decision
    );

    // Only DMs should approve - check via world connection manager
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            });
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can approve responses".to_string(),
        });
    }

    let dm_id = connection.user_id.clone();
    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };

    // Enqueue to DMActionQueue - returns immediately
    // The DM action queue worker will process this asynchronously
    let dm_action = DMAction::ApprovalDecision {
        request_id: request_id.clone(),
        decision: decision.clone(),
    };

    match state
        .queues
        .dm_action_queue_service
        .enqueue_action(&world_id, dm_id, dm_action)
        .await
    {
        Ok(_) => {
            tracing::info!("Enqueued approval decision for request {}", request_id);
            // Return acknowledgment - processing happens in background worker
            None
        }
        Err(e) => {
            tracing::error!("Failed to enqueue approval decision: {}", e);
            Some(ServerMessage::Error {
                code: "QUEUE_ERROR".to_string(),
                message: format!("Failed to queue approval: {}", e),
            })
        }
    }
}
