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
use wrldbldr_protocol::{RequestPayload, ResponseResult};

// Re-export RequestContext from engine-dto for convenience
pub use wrldbldr_engine_dto::request_context::RequestContext;

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
