//! Request Handler - Inbound port for WebSocket request/response pattern
//!
//! This module defines the trait for handling WebSocket requests in a type-safe,
//! async manner. The `RequestHandler` trait is the primary inbound port for the
//! WebSocket-first architecture.
//!
//! # Architecture
//!
//! ```text
//! WebSocket Message arrives
//!         |
//!         v
//! ┌───────────────────────┐
//! │  websocket.rs         │  (infrastructure - engine-adapters)
//! │  - Parse message      │
//! │  - Extract Request    │
//! └───────────┬───────────┘
//!             |
//!             v
//! ┌───────────────────────┐
//! │  RequestHandler trait │  (inbound port - engine-ports)
//! │  .handle(payload,ctx) │  <-- YOU ARE HERE
//! └───────────┬───────────┘
//!             |
//!             v
//! ┌───────────────────────┐
//! │  AppRequestHandler    │  (application - engine-app)
//! │  - Route to service   │
//! │  - Call service       │
//! │  - Broadcast changes  │
//! └───────────────────────┘
//! ```

use async_trait::async_trait;
use uuid::Uuid;
use wrldbldr_protocol::{RequestPayload, ResponseResult, WorldRole};

// =============================================================================
// Request Context
// =============================================================================

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
            ResponseResult::error(
                wrldbldr_protocol::ErrorCode::BadRequest,
                "Not connected to a world",
            )
        })
    }

    /// Require DM role, returning an error result if not DM
    pub fn require_dm(&self) -> Result<(), ResponseResult> {
        if self.is_dm {
            Ok(())
        } else {
            Err(ResponseResult::error(
                wrldbldr_protocol::ErrorCode::Forbidden,
                "This action requires DM role",
            ))
        }
    }

    /// Require PC selection (Player or Spectator role)
    pub fn require_pc(&self) -> Result<Uuid, ResponseResult> {
        self.pc_id.ok_or_else(|| {
            ResponseResult::error(
                wrldbldr_protocol::ErrorCode::BadRequest,
                "No player character selected",
            )
        })
    }
}

// =============================================================================
// Request Handler Trait
// =============================================================================

/// Handler for WebSocket requests
///
/// This is the primary inbound port for the WebSocket-first architecture.
/// Implementations receive requests and return responses.
///
/// # Example Implementation
///
/// ```ignore
/// use async_trait::async_trait;
/// use wrldbldr_engine_ports::inbound::{RequestHandler, RequestContext};
/// use wrldbldr_protocol::{RequestPayload, ResponseResult};
///
/// struct AppRequestHandler {
///     // ... service dependencies
/// }
///
/// #[async_trait]
/// impl RequestHandler for AppRequestHandler {
///     async fn handle(
///         &self,
///         payload: RequestPayload,
///         context: RequestContext,
///     ) -> ResponseResult {
///         match payload {
///             RequestPayload::ListWorlds => {
///                 // Call world service
///                 let worlds = self.world_service.list_worlds().await?;
///                 ResponseResult::success(worlds)
///             }
///             // ... other handlers
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Handle a request and return a response
    ///
    /// # Arguments
    ///
    /// * `payload` - The request payload (CRUD operation, action, etc.)
    /// * `context` - Context about the user and their connection
    ///
    /// # Returns
    ///
    /// A `ResponseResult` indicating success or failure. On success, the
    /// result may contain serialized data. On failure, it contains an
    /// error code and message.
    async fn handle(&self, payload: RequestPayload, context: RequestContext) -> ResponseResult;
}

// =============================================================================
// Broadcast Sink Trait
// =============================================================================

/// Sink for broadcasting entity changes to connected clients
///
/// When an entity is created, updated, or deleted, the handler can use this
/// trait to broadcast the change to all clients connected to the same world.
#[async_trait]
pub trait BroadcastSink: Send + Sync {
    /// Broadcast an entity change to all clients in a world
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to broadcast to
    /// * `change` - The entity change data
    async fn broadcast_entity_change(
        &self,
        world_id: Uuid,
        change: wrldbldr_protocol::EntityChangedData,
    );

    /// Send a message to a specific connection
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The connection to send to
    /// * `message` - The message to send
    async fn send_to_connection(
        &self,
        connection_id: Uuid,
        message: wrldbldr_protocol::ServerMessage,
    );

    /// Send a message to all connections for a specific user
    ///
    /// Useful for DMs with multiple screens.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user to send to
    /// * `world_id` - The world context
    /// * `message` - The message to send
    async fn send_to_user(
        &self,
        user_id: &str,
        world_id: Uuid,
        message: wrldbldr_protocol::ServerMessage,
    );

    /// Broadcast a message to all DMs in a world
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to broadcast to
    /// * `message` - The message to send
    async fn broadcast_to_dms(&self, world_id: Uuid, message: wrldbldr_protocol::ServerMessage);

    /// Broadcast a message to all players in a world
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to broadcast to
    /// * `message` - The message to send
    async fn broadcast_to_players(&self, world_id: Uuid, message: wrldbldr_protocol::ServerMessage);
}
