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
// NOTE: BroadcastSink has been removed
// =============================================================================
//
// The legacy BroadcastSink trait has been replaced by BroadcastPort in
// engine-ports/src/outbound/broadcast_port.rs
//
// Key differences:
// - BroadcastSink took ServerMessage (protocol type) - violated hexagonal architecture
// - BroadcastPort takes GameEvent (domain type) - proper abstraction
// - BroadcastSink was in inbound ports (wrong semantically)
// - BroadcastPort is in outbound ports (correct - app pushes to infrastructure)
