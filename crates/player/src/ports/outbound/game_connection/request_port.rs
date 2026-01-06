//! Game Request Port - Handles request-response operations
//!
//! This trait defines operations for sending requests to the server
//! and awaiting responses, including health checks.
//!
//! # Shared Kernel Pattern
//!
//! This port uses protocol types (`RequestPayload`, `ResponseResult`, `RequestError`)
//! directly because they form a **Shared Kernel** - types that must be identical
//! on both Engine and Player sides for correct WebSocket communication.
//!
//! This is distinct from domain types (which each side defines independently).
//! The protocol crate exists specifically to share wire-format types across
//! the engine-player boundary.
//!
//! See `docs/architecture/hexagonal-architecture.md` for full explanation.
//!
//! Note: The async request methods use `async_trait` instead of returning
//! `Pin<Box<dyn Future>>` for better mockall compatibility.

use async_trait::async_trait;

// Shared Kernel: Protocol types used directly for wire-format compatibility
use wrldbldr_protocol::{RequestError, RequestPayload, ResponseResult};

use crate::outbound::GameConnectionPort;

/// Port for request-response operations
///
/// Handles sending requests to the server and awaiting responses.
/// This is the primary interface for WebSocket request-response patterns.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait GameRequestPort: Send + Sync {
    /// Send a request and await the response
    ///
    /// This is the primary method for WebSocket request-response operations.
    /// The implementation handles request_id generation, pending request tracking,
    /// and response correlation.
    ///
    /// # Arguments
    /// * `payload` - The request payload to send
    ///
    /// # Returns
    /// * `Ok(ResponseResult)` - The server's response
    /// * `Err(RequestError)` - If the request failed to send or timed out
    async fn request(&self, payload: RequestPayload) -> Result<ResponseResult, RequestError>;

    /// Send a request with a custom timeout
    ///
    /// # Arguments
    /// * `payload` - The request payload to send
    /// * `timeout_ms` - Timeout in milliseconds (default is from WRLDBLDR_REQUEST_TIMEOUT_MS env var or 120000)
    async fn request_with_timeout(
        &self,
        payload: RequestPayload,
        timeout_ms: u64,
    ) -> Result<ResponseResult, RequestError>;

    /// Request a manual ComfyUI health check
    fn check_comfyui_health(&self) -> anyhow::Result<()>;
}

// =============================================================================
// Blanket implementation: GameConnectionPort -> GameRequestPort
// =============================================================================

/// Blanket implementation allowing any `GameConnectionPort` to be used as `GameRequestPort`
#[async_trait]
impl<T: GameConnectionPort + ?Sized> GameRequestPort for T {
    async fn request(&self, payload: RequestPayload) -> Result<ResponseResult, RequestError> {
        GameConnectionPort::request(self, payload).await
    }

    async fn request_with_timeout(
        &self,
        payload: RequestPayload,
        timeout_ms: u64,
    ) -> Result<ResponseResult, RequestError> {
        GameConnectionPort::request_with_timeout(self, payload, timeout_ms).await
    }

    fn check_comfyui_health(&self) -> anyhow::Result<()> {
        GameConnectionPort::check_comfyui_health(self)
    }
}
