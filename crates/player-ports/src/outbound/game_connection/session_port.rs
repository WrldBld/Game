//! Session Command Port - Manages game session operations
//!
//! This trait handles joining worlds and registering callbacks for
//! connection state changes and server messages.
//!
//! Note: The callback registration methods (`on_state_change`, `on_message`) remain
//! on the main `GameConnectionPort` trait since mockall doesn't support `Fn` objects.
//! This trait focuses on the command operations that can be easily mocked.

use crate::outbound::GameConnectionPort;
use crate::session_types::ParticipantRole;

/// Port for session management commands
///
/// Handles joining game worlds. For event callbacks, use the main
/// `GameConnectionPort` trait which provides `on_state_change` and `on_message`.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait SessionCommandPort: Send + Sync {
    /// Join a world with the given user ID and role.
    ///
    /// # Arguments
    /// * `world_id` - The world to join (required)
    /// * `user_id` - The user's identifier
    /// * `role` - The participant role (Player or DM)
    fn join_world(
        &self,
        world_id: &str,
        user_id: &str,
        role: ParticipantRole,
    ) -> anyhow::Result<()>;
}

// =============================================================================
// Blanket implementation: GameConnectionPort -> SessionCommandPort
// =============================================================================

/// Blanket implementation allowing any `GameConnectionPort` to be used as `SessionCommandPort`
impl<T: GameConnectionPort + ?Sized> SessionCommandPort for T {
    fn join_world(
        &self,
        world_id: &str,
        user_id: &str,
        role: ParticipantRole,
    ) -> anyhow::Result<()> {
        GameConnectionPort::join_world(self, world_id, user_id, role)
    }
}
