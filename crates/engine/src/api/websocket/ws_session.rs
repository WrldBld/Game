use wrldbldr_domain::{ConnectionId, UserId};

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
    connection_id: ConnectionId,
    world_id: Uuid,
    role: ProtoWorldRole,
    user_id: String,
    pc_id: Option<Uuid>,
    _spectate_pc_id: Option<Uuid>,
) -> Option<ServerMessage> {
    let world_id_typed = WorldId::from_uuid(world_id);
    let pc_id_typed = pc_id.map(PlayerCharacterId::from_uuid);
    let domain_role: wrldbldr_domain::WorldRole = role.into();

    // =========================================================================
    // Phase 1: Prepare (read-only, can fail safely)
    // =========================================================================
    let prepared = match state
        .app
        .use_cases
        .session
        .join_world_flow
        .prepare(world_id_typed, domain_role, pc_id_typed)
        .await
    {
        Ok(p) => p,
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
            let _ = sanitize_repo_error(&e, "preparing world snapshot");
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_shared::JoinError::Unknown,
            });
        }
    };

    // =========================================================================
    // Phase 2: Serialize (can fail safely, no state changed yet)
    // =========================================================================
    let snapshot_json = match serde_json::to_value(&prepared.snapshot) {
        Ok(json) => json,
        Err(e) => {
            tracing::error!(error = %e, "Failed to serialize world snapshot");
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_shared::JoinError::Unknown,
            });
        }
    };

    let your_pc_json = match &prepared.your_pc {
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

    // =========================================================================
    // Phase 3: Commit (only after serialization succeeds)
    // =========================================================================
    // Convert user_id String to UserId at API boundary
    let user_id_typed = UserId::new(&user_id)
        .unwrap_or_else(|_| UserId::from_trusted(connection_id.to_string()));
    let session = crate::stores::SessionStore::new(state.connections.clone());
    let committed = match state
        .app
        .use_cases
        .session
        .join_world_flow
        .commit(
            &session,
            connection_id,
            user_id_typed,
            world_id_typed,
            domain_role,
            pc_id_typed,
            your_pc_json.clone(),
        )
        .await
    {
        Ok(c) => c,
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
            let _ = sanitize_repo_error(&e, "committing world join");
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_shared::JoinError::Unknown,
            });
        }
    };

    // =========================================================================
    // Phase 4: Broadcast (only after commit succeeds)
    // =========================================================================
    if let Some(joined) = &committed.user_joined {
        let user_joined_msg = ServerMessage::UserJoined {
            user_id: joined.user_id.to_string(),
            username: None,
            role: joined.role.into(),
            pc: joined.pc.clone(),
        };
        state
            .connections
            .broadcast_to_world_except(world_id_typed, connection_id, user_joined_msg)
            .await;
    }

    // =========================================================================
    // Phase 5: Return success
    // =========================================================================
    let connected_users = committed
        .connected_users
        .into_iter()
        .map(|u| wrldbldr_shared::ConnectedUser {
            user_id: u.user_id.to_string(),
            username: u.username,
            role: u.role.into(),
            pc_id: u.pc_id.map(|id| id.to_string()),
            connection_count: u.connection_count,
        })
        .collect();

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
    connection_id: ConnectionId,
) -> Option<ServerMessage> {
    // Broadcast UserLeft to other world members before leaving
    if let Some(conn_info) = state.connections.get(connection_id).await {
        if let Some(world_id) = conn_info.world_id {
            let user_left_msg = ServerMessage::UserLeft {
                user_id: conn_info.user_id.to_string(),
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
