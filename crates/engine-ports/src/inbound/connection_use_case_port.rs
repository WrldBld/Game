//! Connection Use Case Port
//!
//! Inbound port defining connection operations for the application layer.
//!
//! This port abstracts the ConnectionUseCase functionality, allowing adapters
//! to interact with connection lifecycle operations (join/leave world, spectator management)
//! without depending on the concrete implementation.
//!
//! # Methods
//!
//! - `join_world` - Join a world with a specific role (DM, Player, Spectator)
//! - `leave_world` - Leave the currently connected world
//! - `set_spectate_target` - Set spectate target for spectators
//!
//! # Note
//!
//! Unlike other use case ports that use `UseCaseContext`, this port receives
//! `connection_id` and `user_id` directly since connection operations establish
//! the session context itself.

use async_trait::async_trait;
#[cfg(any(test, feature = "testing"))]
use mockall::automock;
use uuid::Uuid;

use crate::outbound::{
    ConnectionError, JoinWorldInput, JoinWorldResult, LeaveWorldResult, SetSpectateTargetInput,
    SpectateTargetResult,
};

/// Port for connection use case operations
///
/// Defines the inbound interface for connection lifecycle management.
/// This port is implemented by the ConnectionUseCase in the application layer
/// and consumed by infrastructure adapters (e.g., WebSocket handlers).
///
/// # Architecture
///
/// ```text
/// WebSocket Handler --> ConnectionUseCasePort --> ConnectionUseCase
///                      (this trait)              (implementation)
/// ```
#[cfg_attr(any(test, feature = "testing"), automock)]
#[async_trait]
pub trait ConnectionUseCasePort: Send + Sync {
    /// Join a world with a specific role
    ///
    /// Registers the connection, joins the specified world, and returns
    /// the world snapshot along with connected user information.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Unique identifier for this connection
    /// * `user_id` - User identifier (e.g., authentication ID)
    /// * `input` - Join world parameters including world_id, role, and optional PC info
    ///
    /// # Returns
    ///
    /// * `Ok(JoinWorldResult)` - World snapshot, connected users, and session info
    /// * `Err(ConnectionError)` - If join fails (world not found, already connected, etc.)
    async fn join_world(
        &self,
        connection_id: Uuid,
        user_id: String,
        input: JoinWorldInput,
    ) -> Result<JoinWorldResult, ConnectionError>;

    /// Leave the currently connected world
    ///
    /// Removes the connection from the world and notifies other participants.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Unique identifier for this connection
    ///
    /// # Returns
    ///
    /// * `Ok(LeaveWorldResult)` - Confirmation of leaving
    /// * `Err(ConnectionError)` - If leave fails (not connected, etc.)
    async fn leave_world(&self, connection_id: Uuid) -> Result<LeaveWorldResult, ConnectionError>;

    /// Set spectate target for a spectator
    ///
    /// Changes which player character a spectator is following.
    /// Only valid for connections with the Spectator role.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Unique identifier for this connection
    /// * `input` - Spectate target parameters including the PC to follow
    ///
    /// # Returns
    ///
    /// * `Ok(SpectateTargetResult)` - Confirmation with target PC info
    /// * `Err(ConnectionError)` - If setting target fails (not a spectator, PC not found, etc.)
    async fn set_spectate_target(
        &self,
        connection_id: Uuid,
        input: SetSpectateTargetInput,
    ) -> Result<SpectateTargetResult, ConnectionError>;
}
