use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::api::connections::{ConnectionError, ConnectionManager, WorldRole};
use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_protocol::{ConnectedUser, JoinError, WorldRole as ProtoWorldRole};

use super::{JoinWorld, JoinWorldError};

/// IO dependencies for join-world flows (WS-state owned).
pub struct JoinWorldContext<'a> {
    pub connections: &'a ConnectionManager,
}

/// Input for joining a world over WebSocket.
pub struct JoinWorldInput {
    pub connection_id: Uuid,
    pub world_id: WorldId,
    pub role: ProtoWorldRole,
    pub pc_id: Option<PlayerCharacterId>,
}

/// Use case for joining a world and updating connection state.
pub struct JoinWorldFlow {
    join_world: Arc<JoinWorld>,
}

impl JoinWorldFlow {
    pub fn new(join_world: Arc<JoinWorld>) -> Self {
        Self { join_world }
    }

    pub async fn execute(
        &self,
        ctx: &JoinWorldContext<'_>,
        input: JoinWorldInput,
    ) -> Result<JoinWorldFlowResult, JoinWorldFlowError> {
        let internal_role = match input.role {
            ProtoWorldRole::Dm => WorldRole::Dm,
            ProtoWorldRole::Player => WorldRole::Player,
            ProtoWorldRole::Spectator | ProtoWorldRole::Unknown => WorldRole::Spectator,
        };

        let join_result = self
            .join_world
            .execute_with_role(input.world_id, input.role, input.pc_id)
            .await
            .map_err(JoinWorldFlowError::from)?;

        ctx.connections
            .join_world(
                input.connection_id,
                input.world_id,
                internal_role,
                input.pc_id,
            )
            .await
            .map_err(JoinWorldFlowError::from)?;

        let connected_users = ctx
            .connections
            .get_world_connections(input.world_id)
            .await
            .into_iter()
            .map(|info| ConnectedUser {
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

        let user_joined = ctx
            .connections
            .get(input.connection_id)
            .await
            .map(|info| UserJoinedPayload {
                user_id: info.user_id,
                role: input.role,
                pc: join_result.your_pc.clone(),
            });

        Ok(JoinWorldFlowResult {
            world_id: input.world_id,
            snapshot: join_result.snapshot,
            connected_users,
            your_pc: join_result.your_pc,
            user_joined,
        })
    }
}

pub struct JoinWorldFlowResult {
    pub world_id: WorldId,
    pub snapshot: Value,
    pub connected_users: Vec<ConnectedUser>,
    pub your_pc: Option<Value>,
    pub user_joined: Option<UserJoinedPayload>,
}

pub struct UserJoinedPayload {
    pub user_id: String,
    pub role: ProtoWorldRole,
    pub pc: Option<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum JoinWorldFlowError {
    #[error("World not found")]
    WorldNotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Join error: {0:?}")]
    JoinError(JoinError),
}

impl From<JoinWorldError> for JoinWorldFlowError {
    fn from(err: JoinWorldError) -> Self {
        match err {
            JoinWorldError::WorldNotFound => JoinWorldFlowError::WorldNotFound,
            JoinWorldError::Repo(e) => JoinWorldFlowError::Repo(e),
        }
    }
}

impl From<ConnectionError> for JoinWorldFlowError {
    fn from(err: ConnectionError) -> Self {
        let join_error = match err {
            ConnectionError::DmAlreadyConnected => JoinError::DmAlreadyConnected {
                existing_user_id: String::new(),
            },
            _ => JoinError::Unknown,
        };

        JoinWorldFlowError::JoinError(join_error)
    }
}
