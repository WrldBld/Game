use std::sync::Arc;

use crate::infrastructure::ports::{
    ConnectedUserInfo, JoinWorldError as PortJoinWorldError, RepoError, SessionError,
    UserJoinedInfo, WorldRole,
};
use crate::stores::SessionStore as WorldSession;
use wrldbldr_domain::{ConnectionId, PlayerCharacterId, UserId, WorldId};

use super::types::{PlayerCharacterSummary, WorldSnapshot};
use super::{JoinWorld, JoinWorldError};

// =============================================================================
// Prepare/Commit Result Types
// =============================================================================

/// Result of the prepare phase: data fetched from DB, no state changes made.
/// Safe to serialize after this - if serialization fails, no cleanup needed.
#[derive(Debug, Clone)]
pub struct JoinWorldPrepared {
    pub world_id: WorldId,
    pub snapshot: WorldSnapshot,
    pub your_pc: Option<PlayerCharacterSummary>,
}

/// Result of the commit phase: connection registered, notification built.
/// Only call after serialization succeeds.
#[derive(Debug, Clone)]
pub struct JoinWorldCommitted {
    pub connected_users: Vec<ConnectedUserInfo>,
    pub user_joined: Option<UserJoinedInfo>,
}

/// IO dependencies for join-world flows (WS-state owned).
pub struct JoinWorldContext<'a> {
    pub session: &'a WorldSession,
}

/// Input for joining a world over WebSocket (domain types).
pub struct JoinWorldInput {
    pub connection_id: ConnectionId,
    pub world_id: WorldId,
    pub role: WorldRole,
    /// Stable user identifier from the client (e.g., browser storage).
    pub user_id: UserId,
    pub pc_id: Option<PlayerCharacterId>,
}

impl JoinWorldInput {
    /// Create input from protocol types (API layer conversion helper).
    ///
    /// Uses the From impl defined in protocol crate to convert WorldRole.
    /// Unknown protocol roles map to Spectator as the safe default.
    /// The user_id String is converted to UserId, using a fallback for empty strings.
    pub fn from_protocol(
        connection_id: ConnectionId,
        world_id: WorldId,
        role: wrldbldr_shared::WorldRole,
        user_id: String,
        pc_id: Option<PlayerCharacterId>,
    ) -> Self {
        // Convert String to UserId, using connection_id as fallback if empty
        let user_id = UserId::new(&user_id)
            .unwrap_or_else(|_| UserId::from_trusted(connection_id.to_string()));
        Self {
            connection_id,
            world_id,
            role: role.into(), // Uses From<protocol::WorldRole> for domain::WorldRole
            user_id,
            pc_id,
        }
    }
}

/// Use case for joining a world and updating connection state.
pub struct JoinWorldFlow {
    join_world: Arc<JoinWorld>,
}

impl JoinWorldFlow {
    pub fn new(join_world: Arc<JoinWorld>) -> Self {
        Self { join_world }
    }

    // =========================================================================
    // Prepare/Commit Pattern
    // =========================================================================

    /// Phase 1: Fetch data from database. No side effects.
    ///
    /// This is safe to call and then abandon if subsequent operations fail.
    /// No connection state is modified.
    pub async fn prepare(
        &self,
        world_id: WorldId,
        role: WorldRole,
        pc_id: Option<PlayerCharacterId>,
    ) -> Result<JoinWorldPrepared, JoinWorldFlowError> {
        let join_result = self
            .join_world
            .execute_with_role(world_id, role, pc_id)
            .await
            .map_err(JoinWorldFlowError::from)?;

        Ok(JoinWorldPrepared {
            world_id,
            snapshot: join_result.snapshot,
            your_pc: join_result.your_pc,
        })
    }

    /// Phase 2: Register connection and build notification.
    ///
    /// Only call after serialization succeeds. This modifies connection state.
    ///
    /// # Arguments
    /// * `session` - The world session for connection management
    /// * `connection_id` - The WebSocket connection ID
    /// * `user_id` - The stable user identifier from client (typed)
    /// * `world_id` - The world being joined
    /// * `role` - The role in the world (DM, Player, Spectator)
    /// * `pc_id` - Player character ID (if role is Player)
    /// * `pc_json` - Pre-serialized PC data for the UserJoined notification
    pub async fn commit(
        &self,
        session: &WorldSession,
        connection_id: ConnectionId,
        user_id: UserId,
        world_id: WorldId,
        role: WorldRole,
        pc_id: Option<PlayerCharacterId>,
        pc_json: Option<serde_json::Value>,
    ) -> Result<JoinWorldCommitted, JoinWorldFlowError> {
        // Update user_id from the client (stable identifier from browser storage)
        session.set_user_id(connection_id, user_id).await;

        // Register the connection to the world
        session
            .join_world(connection_id, world_id, role, pc_id)
            .await
            .map_err(JoinWorldFlowError::from)?;

        // Get all connected users (including the one we just added)
        let connected_users = session
            .get_world_connections(world_id)
            .await
            .into_iter()
            .map(|info| ConnectedUserInfo {
                user_id: info.user_id,
                username: None,
                role: info.role,
                pc_id: info.pc_id,
                connection_count: 1,
            })
            .collect();

        // Build user joined notification from connection info
        let user_joined = session
            .get_connection(connection_id)
            .await
            .map(|info| UserJoinedInfo {
                user_id: info.user_id,
                role,
                pc: pc_json,
            });

        Ok(JoinWorldCommitted {
            connected_users,
            user_joined,
        })
    }

    // =========================================================================
    // Legacy Combined Method (for backward compatibility)
    // =========================================================================

    /// Combined prepare + commit for callers that don't need the separation.
    ///
    /// Note: This has the original race condition where serialization failure
    /// after connection registration leaves state inconsistent. Prefer using
    /// `prepare()` + `commit()` separately for new code.
    pub async fn execute(
        &self,
        ctx: &JoinWorldContext<'_>,
        input: JoinWorldInput,
    ) -> Result<JoinWorldFlowResult, JoinWorldFlowError> {
        // Update user_id from the client (stable identifier from browser storage)
        ctx.session
            .set_user_id(input.connection_id, input.user_id.clone())
            .await;

        let join_result = self
            .join_world
            .execute_with_role(input.world_id, input.role, input.pc_id)
            .await
            .map_err(JoinWorldFlowError::from)?;

        ctx.session
            .join_world(input.connection_id, input.world_id, input.role, input.pc_id)
            .await
            .map_err(JoinWorldFlowError::from)?;

        let connected_users = ctx
            .session
            .get_world_connections(input.world_id)
            .await
            .into_iter()
            .map(|info| ConnectedUserInfo {
                user_id: info.user_id,
                username: None,
                role: info.role,
                pc_id: info.pc_id,
                connection_count: 1,
            })
            .collect();

        // Convert PC summary to JSON for UserJoinedInfo (ports type uses Value)
        let pc_json = join_result
            .your_pc
            .as_ref()
            .and_then(|pc| serde_json::to_value(pc).ok());

        let user_joined = ctx
            .session
            .get_connection(input.connection_id)
            .await
            .map(|info| UserJoinedInfo {
                user_id: info.user_id,
                role: input.role,
                pc: pc_json.clone(),
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
    pub snapshot: WorldSnapshot,
    pub connected_users: Vec<ConnectedUserInfo>,
    pub your_pc: Option<PlayerCharacterSummary>,
    pub user_joined: Option<UserJoinedInfo>,
}

#[derive(Debug, thiserror::Error)]
pub enum JoinWorldFlowError {
    #[error("World not found")]
    WorldNotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Join error: {0}")]
    JoinError(PortJoinWorldError),
}

impl From<JoinWorldError> for JoinWorldFlowError {
    fn from(err: JoinWorldError) -> Self {
        match err {
            JoinWorldError::WorldNotFound(_) => JoinWorldFlowError::WorldNotFound,
            JoinWorldError::Repo(e) => JoinWorldFlowError::Repo(e),
        }
    }
}

impl From<SessionError> for JoinWorldFlowError {
    fn from(err: SessionError) -> Self {
        let join_error = match err {
            SessionError::DmAlreadyConnected => PortJoinWorldError::DmAlreadyConnected {
                existing_user_id: String::new(),
            },
            _ => PortJoinWorldError::Unknown,
        };

        JoinWorldFlowError::JoinError(join_error)
    }
}
