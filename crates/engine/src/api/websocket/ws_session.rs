use super::*;

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
    let ctx = crate::use_cases::session::JoinWorldContext {
        connections: &state.connections,
    };
    let input = crate::use_cases::session::JoinWorldInput {
        connection_id,
        world_id: world_id_typed,
        role,
        user_id,
        pc_id: pc_id_typed,
    };

    let join_result = match state.app.use_cases.session.join_world_flow.execute(&ctx, input).await {
        Ok(result) => result,
        Err(crate::use_cases::session::JoinWorldFlowError::WorldNotFound) => {
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_protocol::JoinError::WorldNotFound,
            })
        }
        Err(crate::use_cases::session::JoinWorldFlowError::JoinError(e)) => {
            return Some(ServerMessage::WorldJoinFailed { world_id, error: e })
        }
        Err(crate::use_cases::session::JoinWorldFlowError::Repo(e)) => {
            tracing::error!(error = %e, "Failed to build world snapshot");
            return Some(ServerMessage::WorldJoinFailed {
                world_id,
                error: wrldbldr_protocol::JoinError::Unknown,
            });
        }
    };

    if let Some(joined) = join_result.user_joined {
        let user_joined_msg = ServerMessage::UserJoined {
            user_id: joined.user_id,
            username: None,
            role: joined.role,
            pc: joined.pc,
        };
        state
            .connections
            .broadcast_to_world_except(world_id_typed, connection_id, user_joined_msg)
            .await;
    }

    Some(ServerMessage::WorldJoined {
        world_id,
        snapshot: join_result.snapshot,
        connected_users: join_result.connected_users,
        your_role: role,
        your_pc: join_result.your_pc,
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
