//! Connection lifecycle handlers for WebSocket connections.
//!
//! This module contains handlers for:
//! - Heartbeat/ping-pong
//! - World join/leave operations
//! - Spectator target management
//!
//! All handlers follow the pattern of taking `&AppState`, `client_id`, and
//! message-specific parameters, returning `Option<ServerMessage>`.

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use wrldbldr_protocol::{ParticipantRole, ServerMessage, WorldRole};

/// Handles heartbeat messages by returning a Pong response.
pub fn handle_heartbeat() -> Option<ServerMessage> {
    Some(ServerMessage::Pong)
}

/// Handles deprecated JoinSession messages.
///
/// This handler exists for backwards compatibility and always returns an error
/// directing clients to use JoinWorld instead.
pub fn handle_join_session(
    _user_id: String,
    _role: ParticipantRole,
    _world_id: Option<Uuid>,
) -> Option<ServerMessage> {
    tracing::warn!("JoinSession is deprecated, use JoinWorld instead");
    Some(ServerMessage::Error {
        code: "DEPRECATED".to_string(),
        message: "JoinSession is deprecated. Use JoinWorld instead.".to_string(),
    })
}

/// Handles JoinWorld requests.
///
/// This handler:
/// 1. Registers the connection if not already registered
/// 2. Joins the specified world with the given role
/// 3. Fetches PC data if the role is Player and a pc_id is provided
/// 4. Broadcasts UserJoined to other users in the world
/// 5. Returns WorldJoined with the world snapshot and connected users
pub async fn handle_join_world(
    state: &AppState,
    client_id: Uuid,
    world_id: Uuid,
    role: WorldRole,
    pc_id: Option<Uuid>,
    spectate_pc_id: Option<Uuid>,
) -> Option<ServerMessage> {
    // client_id is already a valid Uuid, use it directly as connection_id
    let connection_id = client_id;
    let client_id_str = client_id.to_string();

    tracing::info!(
        world_id = %world_id,
        role = ?role,
        pc_id = ?pc_id,
        spectate_pc_id = ?spectate_pc_id,
        connection_id = %connection_id,
        "JoinWorld request received"
    );

    // Register connection if not already registered
    // For now, use client_id as user_id (will be properly authenticated later)
    let user_id = client_id_str.clone();

    // Create a broadcast sender for this connection
    let (broadcast_tx, _broadcast_rx) = tokio::sync::broadcast::channel(64);
    state
        .world_connection_manager
        .register_connection(
            connection_id,
            client_id_str.clone(),
            user_id.clone(),
            broadcast_tx,
        )
        .await;

    // Join the world
    match state
        .world_connection_manager
        .join_world(connection_id, world_id, role, pc_id, spectate_pc_id)
        .await
    {
        Ok(connected_users) => {
            tracing::info!(
                world_id = %world_id,
                user_id = %user_id,
                connected_users = connected_users.len(),
                "User joined world successfully"
            );

            // Get world snapshot for the joiner
            let world_id_domain = wrldbldr_domain::WorldId::from_uuid(world_id);
            let snapshot = match state
                .core
                .world_service
                .export_world_snapshot(world_id_domain)
                .await
            {
                Ok(s) => serde_json::to_value(s).unwrap_or_default(),
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get world snapshot");
                    serde_json::json!({})
                }
            };

            // Fetch PC data if role is Player and pc_id is provided
            let pc_data = if role == WorldRole::Player {
                if let Some(pc_uuid) = pc_id {
                    let pc_id_domain = wrldbldr_domain::PlayerCharacterId::from_uuid(pc_uuid);
                    match state
                        .player
                        .player_character_service
                        .get_pc(pc_id_domain)
                        .await
                    {
                        Ok(Some(pc)) => {
                            tracing::debug!(pc_id = %pc_uuid, "Fetched PC data for Player join");
                            Some(serde_json::json!({
                                "id": pc.id.to_string(),
                                "name": pc.name,
                                "user_id": pc.user_id,
                                "world_id": pc.world_id.to_string(),
                                "current_location_id": pc.current_location_id.to_string(),
                                "current_region_id": pc.current_region_id.map(|r| r.to_string()),
                                "description": pc.description,
                                "sprite_asset": pc.sprite_asset,
                                "portrait_asset": pc.portrait_asset,
                            }))
                        }
                        Ok(None) => {
                            tracing::warn!(pc_id = %pc_uuid, "PC not found for Player join");
                            None
                        }
                        Err(e) => {
                            tracing::error!(pc_id = %pc_uuid, error = %e, "Failed to fetch PC data");
                            None
                        }
                    }
                } else {
                    tracing::debug!("Player joined without PC ID - will need to select PC");
                    None
                }
            } else {
                None
            };

            // Broadcast UserJoined to other users in the world
            let user_joined_msg = ServerMessage::UserJoined {
                user_id: user_id.clone(),
                username: None,
                role,
                pc: pc_data.clone(),
            };

            // Get all connections except this one and broadcast
            let world_connections = state
                .world_connection_manager
                .get_world_connections(world_id)
                .await;
            for other_conn_id in world_connections {
                if other_conn_id != connection_id {
                    state
                        .world_connection_manager
                        .send_to_connection(other_conn_id, user_joined_msg.clone())
                        .await;
                }
            }

            Some(ServerMessage::WorldJoined {
                world_id,
                snapshot,
                connected_users,
                your_role: role,
                your_pc: pc_data,
            })
        }
        Err(error) => {
            tracing::warn!(
                world_id = %world_id,
                error = ?error,
                "Failed to join world"
            );
            Some(ServerMessage::WorldJoinFailed { world_id, error })
        }
    }
}

/// Handles LeaveWorld requests.
///
/// This handler:
/// 1. Removes the connection from the world
/// 2. Broadcasts UserLeft to remaining users in the world
/// 3. Returns None (no response needed)
pub async fn handle_leave_world(state: &AppState, client_id: Uuid) -> Option<ServerMessage> {
    // client_id is already a valid Uuid, use it directly as connection_id
    let connection_id = client_id;

    tracing::info!(connection_id = %connection_id, "LeaveWorld request received");

    if let Some((world_id, _role)) = state
        .world_connection_manager
        .leave_world(connection_id)
        .await
    {
        // Broadcast UserLeft to remaining users
        if let Some(conn_info) = state
            .world_connection_manager
            .get_connection(connection_id)
            .await
        {
            let user_left_msg = ServerMessage::UserLeft {
                user_id: conn_info.user_id.clone(),
            };
            state
                .world_connection_manager
                .broadcast_to_world(world_id, user_left_msg)
                .await;
        }
        tracing::info!(world_id = %world_id, "User left world");
    }

    None // No response needed
}

/// Handles SetSpectateTarget requests for spectators.
///
/// This handler:
/// 1. Validates that the connection is a spectator
/// 2. Updates the spectate target in the connection manager
/// 3. Fetches the PC name for the response
/// 4. Returns SpectateTargetChanged on success
pub async fn handle_set_spectate_target(
    state: &AppState,
    client_id: Uuid,
    pc_id: Uuid,
) -> Option<ServerMessage> {
    // client_id is already a valid Uuid, use it directly as connection_id
    let connection_id = client_id;

    tracing::info!(
        pc_id = %pc_id,
        connection_id = %connection_id,
        "SetSpectateTarget request received"
    );

    if let Some(conn_info) = state
        .world_connection_manager
        .get_connection(connection_id)
        .await
    {
        if !conn_info.is_spectator() {
            tracing::warn!("SetSpectateTarget called by non-spectator");
            return Some(ServerMessage::Error {
                code: "not_spectator".to_string(),
                message: "Only spectators can change spectate target".to_string(),
            });
        }

        // Update spectate target in connection manager
        state
            .world_connection_manager
            .set_spectate_target(connection_id, Some(pc_id))
            .await;

        // Fetch PC name for the response
        let pc_id_domain = wrldbldr_domain::PlayerCharacterId::from_uuid(pc_id);
        let pc_name = match state
            .player
            .player_character_service
            .get_pc(pc_id_domain)
            .await
        {
            Ok(Some(pc)) => pc.name,
            Ok(None) => {
                tracing::warn!(pc_id = %pc_id, "Spectate target PC not found");
                return Some(ServerMessage::Error {
                    code: "pc_not_found".to_string(),
                    message: format!("Player character {} not found", pc_id),
                });
            }
            Err(e) => {
                tracing::error!(pc_id = %pc_id, error = %e, "Failed to fetch spectate target PC");
                return Some(ServerMessage::Error {
                    code: "internal_error".to_string(),
                    message: "Failed to fetch player character".to_string(),
                });
            }
        };

        tracing::info!(
            pc_id = %pc_id,
            pc_name = %pc_name,
            "Spectate target changed"
        );

        Some(ServerMessage::SpectateTargetChanged { pc_id, pc_name })
    } else {
        tracing::warn!("SetSpectateTarget from unknown connection");
        Some(ServerMessage::Error {
            code: "not_connected".to_string(),
            message: "Not connected to a world".to_string(),
        })
    }
}
