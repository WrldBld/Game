//! Request Context DTO
//!
//! Context information for WebSocket request handling.

use uuid::Uuid;
use wrldbldr_protocol::{ErrorCode, ResponseResult, WorldRole};

/// Context for a WebSocket request
///
/// Contains information about the user making the request and their connection
/// to a world. This is passed to the request handler along with the payload.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique connection/socket identifier
    pub connection_id: Uuid,

    /// User making the request
    pub user_id: String,

    /// World the user is connected to (None if not in a world)
    pub world_id: Option<Uuid>,

    /// User's role in the current world
    pub role: Option<WorldRole>,

    /// Player character ID (for Player role)
    pub pc_id: Option<Uuid>,

    /// Whether this request originated from a DM connection
    pub is_dm: bool,

    /// Whether this user is spectating (read-only)
    pub is_spectating: bool,
}

impl RequestContext {
    /// Create a new request context for a user not yet in a world
    pub fn anonymous(connection_id: Uuid, user_id: String) -> Self {
        Self {
            connection_id,
            user_id,
            world_id: None,
            role: None,
            pc_id: None,
            is_dm: false,
            is_spectating: false,
        }
    }

    /// Create a context for a DM connection
    pub fn dm(connection_id: Uuid, user_id: String, world_id: Uuid) -> Self {
        Self {
            connection_id,
            user_id,
            world_id: Some(world_id),
            role: Some(WorldRole::Dm),
            pc_id: None,
            is_dm: true,
            is_spectating: false,
        }
    }

    /// Create a context for a Player connection
    pub fn player(connection_id: Uuid, user_id: String, world_id: Uuid, pc_id: Uuid) -> Self {
        Self {
            connection_id,
            user_id,
            world_id: Some(world_id),
            role: Some(WorldRole::Player),
            pc_id: Some(pc_id),
            is_dm: false,
            is_spectating: false,
        }
    }

    /// Create a context for a Spectator connection
    pub fn spectator(
        connection_id: Uuid,
        user_id: String,
        world_id: Uuid,
        spectate_pc_id: Uuid,
    ) -> Self {
        Self {
            connection_id,
            user_id,
            world_id: Some(world_id),
            role: Some(WorldRole::Spectator),
            pc_id: Some(spectate_pc_id),
            is_dm: false,
            is_spectating: true,
        }
    }

    /// Check if the user has permission to modify data
    pub fn can_modify(&self) -> bool {
        self.role.map(|r| r.can_modify()).unwrap_or(false)
    }

    /// Check if the user can perform DM-only actions
    pub fn can_dm_action(&self) -> bool {
        self.is_dm
    }

    /// Get the world ID, returning an error result if not in a world
    pub fn require_world(&self) -> Result<Uuid, ResponseResult> {
        self.world_id.ok_or_else(|| {
            ResponseResult::error(ErrorCode::BadRequest, "Not connected to a world")
        })
    }

    /// Require DM role, returning an error result if not DM
    pub fn require_dm(&self) -> Result<(), ResponseResult> {
        if self.is_dm {
            Ok(())
        } else {
            Err(ResponseResult::error(
                ErrorCode::Forbidden,
                "This action requires DM role",
            ))
        }
    }

    /// Require PC selection (Player or Spectator role)
    pub fn require_pc(&self) -> Result<Uuid, ResponseResult> {
        self.pc_id.ok_or_else(|| {
            ResponseResult::error(ErrorCode::BadRequest, "No player character selected")
        })
    }
}
