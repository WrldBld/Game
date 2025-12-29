//! Connection lifecycle handlers for WebSocket connections.
//!
//! Thin routing layer that delegates to ConnectionUseCase.
//! Handles: heartbeat, join world, leave world, spectate target.

use uuid::Uuid;

use crate::infrastructure::adapter_state::AdapterState;
use crate::infrastructure::websocket::IntoServerError;
use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::inbound::WorldRole as UseCaseWorldRole;
use wrldbldr_engine_ports::outbound::{
    ConnectionError, JoinWorldInput, SetSpectateTargetInput,
};
use wrldbldr_protocol::{JoinError, ServerMessage, WorldRole};

/// Handles heartbeat messages by returning a Pong response.
pub fn handle_heartbeat() -> Option<ServerMessage> {
    Some(ServerMessage::Pong)
}

/// Handles JoinWorld requests by delegating to ConnectionUseCase.
pub async fn handle_join_world(
    state: &AdapterState,
    client_id: Uuid,
    world_id: Uuid,
    role: WorldRole,
    pc_id: Option<Uuid>,
    spectate_pc_id: Option<Uuid>,
) -> Option<ServerMessage> {
    let user_id = client_id.to_string();

    let input = JoinWorldInput {
        world_id: WorldId::from_uuid(world_id),
        role: protocol_to_use_case_role(role),
        pc_id: pc_id.map(PlayerCharacterId::from_uuid),
        spectate_pc_id: spectate_pc_id.map(PlayerCharacterId::from_uuid),
    };

    match state
        .app
        .use_cases
        .connection
        .join_world(client_id, user_id, input)
        .await
    {
        Ok(result) => {
            let connected_users = result
                .connected_users
                .into_iter()
                .map(|u| wrldbldr_protocol::ConnectedUser {
                    user_id: u.user_id,
                    username: None,
                    role: use_case_to_protocol_role(u.role),
                    pc_id: u.pc_id.map(|id| id.to_string()),
                    connection_count: 1,
                })
                .collect();

            let your_pc = result.your_pc.map(|pc| {
                serde_json::json!({
                    "id": pc.id,
                    "name": pc.name,
                    "user_id": pc.user_id,
                    "world_id": pc.world_id,
                    "current_location_id": pc.current_location_id,
                    "current_region_id": pc.current_region_id,
                    "description": pc.description,
                    "sprite_asset": pc.sprite_asset,
                    "portrait_asset": pc.portrait_asset,
                })
            });

            Some(ServerMessage::WorldJoined {
                world_id,
                snapshot: result.snapshot,
                connected_users,
                your_role: use_case_to_protocol_role(result.your_role),
                your_pc,
            })
        }
        Err(e) => Some(ServerMessage::WorldJoinFailed {
            world_id,
            error: connection_error_to_join_error(e),
        }),
    }
}

/// Handles LeaveWorld requests by delegating to ConnectionUseCase.
pub async fn handle_leave_world(state: &AdapterState, client_id: Uuid) -> Option<ServerMessage> {
    let _ = state.app.use_cases.connection.leave_world(client_id).await;
    None // No response needed
}

/// Handles SetSpectateTarget requests by delegating to ConnectionUseCase.
pub async fn handle_set_spectate_target(
    state: &AdapterState,
    client_id: Uuid,
    pc_id: Uuid,
) -> Option<ServerMessage> {
    let input = SetSpectateTargetInput {
        pc_id: PlayerCharacterId::from_uuid(pc_id),
    };

    match state
        .app
        .use_cases
        .connection
        .set_spectate_target(client_id, input)
        .await
    {
        Ok(result) => Some(ServerMessage::SpectateTargetChanged {
            pc_id: *result.pc_id.as_uuid(),
            pc_name: result.pc_name,
        }),
        Err(e) => Some(e.into_server_error()),
    }
}

// =============================================================================
// Conversion Helpers
// =============================================================================

fn protocol_to_use_case_role(role: WorldRole) -> UseCaseWorldRole {
    match role {
        WorldRole::Dm => UseCaseWorldRole::DM,
        WorldRole::Player => UseCaseWorldRole::Player,
        WorldRole::Spectator | WorldRole::Unknown => {
            UseCaseWorldRole::Spectator // Default unknown to Spectator (least privileged)
        }
    }
}

fn use_case_to_protocol_role(role: UseCaseWorldRole) -> WorldRole {
    match role {
        UseCaseWorldRole::DM => WorldRole::Dm,
        UseCaseWorldRole::Player => WorldRole::Player,
        UseCaseWorldRole::Spectator => WorldRole::Spectator,
    }
}

fn connection_error_to_join_error(e: ConnectionError) -> JoinError {
    match e {
        ConnectionError::WorldNotFound(_) => JoinError::WorldNotFound,
        ConnectionError::AlreadyConnected => JoinError::DmAlreadyConnected {
            existing_user_id: String::new(),
        },
        _ => JoinError::Unauthorized, // Fallback for other errors
    }
}
