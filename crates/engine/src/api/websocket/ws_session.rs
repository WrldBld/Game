use super::*;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::infrastructure::ports::JoinWorldError as PortJoinWorldError;

/// Convert domain JoinWorldError to protocol JoinError.
fn to_proto_join_error(err: PortJoinWorldError) -> wrldbldr_shared::JoinError {
    match err {
        PortJoinWorldError::DmAlreadyConnected { existing_user_id } => {
            wrldbldr_shared::JoinError::DmAlreadyConnected { existing_user_id }
        }
        PortJoinWorldError::PcNotFound { world_id, pc_id } => {
            tracing::warn!(world_id = %world_id, pc_id = %pc_id, "Player character not found");
            wrldbldr_shared::JoinError::Unknown
        }
        PortJoinWorldError::Unknown => wrldbldr_shared::JoinError::Unknown,
    }
}

pub(super) async fn handle_join_world(
    state: &WsState,
    connection_id: Uuid,
    world_id: Uuid,
    role: ProtoWorldRole,
    user_id: String,
    pc_id: Option<Uuid>,
    _spectate_pc_id: Option<Uuid>,
) -> Option<ServerMessage> {
    let world_id_typed = WorldId::from_uuid(world_id);

    let pc_id_typed = pc_id.map(PlayerCharacterId::from_uuid);
    let session = crate::repositories::WorldSession::new(state.connections.clone());
    let ctx = crate::use_cases::session::JoinWorldContext { session: &session };
    let input = crate::use_cases::session::JoinWorldInput::from_protocol(
        connection_id,
        world_id_typed,
        role,
        user_id,
        pc_id_typed,
    );

    let join_result = match state
        .app
        .use_cases
        .session
        .join_world_flow
        .execute(&ctx, input)
        .await
    {
        Ok(result) => result,
        Err(crate::use_cases::session::JoinWorldFlowError::WorldNotFound) => {
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_shared::JoinError::WorldNotFound,
            })
        }
        Err(crate::use_cases::session::JoinWorldFlowError::JoinError(e)) => {
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: to_proto_join_error(e),
            })
        }
        Err(crate::use_cases::session::JoinWorldFlowError::Repo(e)) => {
            // sanitize_repo_error logs internally; response uses generic error
            let _ = sanitize_repo_error(&e, "building world snapshot");
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_shared::JoinError::Unknown,
            });
        }
    };

    // Broadcast UserJoined to other world members
    if let Some(joined) = join_result.user_joined {
        let user_joined_msg = ServerMessage::UserJoined {
            user_id: joined.user_id,
            username: None,
            role: joined.role.into(), // Uses From<domain::WorldRole> for protocol::WorldRole
            pc: joined.pc,
        };
        state
            .connections
            .broadcast_to_world_except(world_id_typed, connection_id, user_joined_msg)
            .await;
    }

    // Convert connected users from domain to protocol
    let connected_users = join_result
        .connected_users
        .into_iter()
        .map(|u| wrldbldr_shared::ConnectedUser {
            user_id: u.user_id,
            username: u.username,
            role: u.role.into(), // Uses From<domain::WorldRole> for protocol::WorldRole
            pc_id: u.pc_id.map(|id| id.to_string()),
            connection_count: u.connection_count,
        })
        .collect();

    // Convert typed snapshot and PC to JSON for wire format
    let snapshot_json = match serde_json::to_value(&join_result.snapshot) {
        Ok(json) => json,
        Err(e) => {
            tracing::error!(error = %e, "Failed to serialize world snapshot");
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_shared::JoinError::Unknown,
            });
        }
    };
    let your_pc_json = match &join_result.your_pc {
        Some(pc) => match serde_json::to_value(pc) {
            Ok(json) => Some(json),
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize player character");
                return Some(ServerMessage::WorldJoinFailed {
                    world_id,
                    error: wrldbldr_shared::JoinError::Unknown,
                });
            }
        },
        None => None,
    };

    Some(ServerMessage::WorldJoined {
        world_id,
        snapshot: snapshot_json,
        connected_users,
        your_role: role,
        your_pc: your_pc_json,
    })
}

pub(super) async fn handle_leave_world(
    state: &WsState,
    connection_id: Uuid,
) -> Option<ServerMessage> {
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
