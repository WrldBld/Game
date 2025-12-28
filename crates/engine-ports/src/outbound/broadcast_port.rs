//! Broadcast Port - Outbound port for game event notifications
//!
//! This port abstracts the notification of game events to connected clients,
//! allowing use cases to trigger notifications without depending on WebSocket
//! infrastructure.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        APPLICATION LAYER                                 │
//! │                                                                          │
//! │  MovementUseCase::move_to_region()                                       │
//! │      │                                                                   │
//! │      └──> broadcast_port.broadcast(world_id, GameEvent::SceneChanged)   │
//! │                                                                          │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//!                   ┌────────────▼─────────────┐
//!                   │      BroadcastPort       │ (trait defined here)
//!                   │  broadcast(world, event) │
//!                   └────────────┬─────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                         ADAPTER LAYER                                    │
//! │                                                                          │
//! │  WebSocketBroadcastAdapter implements BroadcastPort                      │
//! │      │                                                                   │
//! │      ├──> Convert GameEvent to ServerMessage                            │
//! │      └──> Route via WorldConnectionManager                              │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Design Rationale (D5, D8)
//!
//! - Located in `engine-ports/outbound` (not domain) because it's an application-layer
//!   contract, not domain logic
//! - Takes `GameEvent` (domain type) not `ServerMessage` (protocol type) - proper abstraction
//! - Replaces the legacy `BroadcastSink` which took protocol types directly

use async_trait::async_trait;
use wrldbldr_domain::WorldId;

use super::game_events::GameEvent;

/// Port for broadcasting game events to connected clients
///
/// Implementations:
/// - Convert GameEvent to appropriate ServerMessage(s)
/// - Route to correct recipients based on event type
/// - Use WorldConnectionManager or similar for actual delivery
///
/// # Testing
///
/// Enable the `testing` feature to get mock implementations via mockall.
///
/// ```rust,ignore
/// #[cfg(test)]
/// use wrldbldr_engine_ports::outbound::MockBroadcastPort;
///
/// let mut mock = MockBroadcastPort::new();
/// mock.expect_broadcast()
///     .times(1)
///     .returning(|_, _| ());
/// ```
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait BroadcastPort: Send + Sync {
    /// Broadcast a game event
    ///
    /// The implementation routes the event to appropriate recipients:
    /// - DM-targeted events go to DMs
    /// - Player-targeted events (with user_id) go to specific players
    /// - World-wide events go to all participants
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world context for routing
    /// * `event` - The game event to broadcast
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // In a use case:
    /// self.broadcast_port.broadcast(
    ///     ctx.world_id,
    ///     GameEvent::SceneChanged {
    ///         user_id: ctx.user_id.clone(),
    ///         event: scene_event,
    ///     },
    /// ).await;
    /// ```
    async fn broadcast(&self, world_id: WorldId, event: GameEvent);
}
