//! Use case execution context
//!
//! This context is passed from handlers to use cases, containing identity
//! and authorization information. Defined in ports layer so both adapters
//! and application layer can use it without circular dependencies.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        ADAPTER LAYER                                     │
//! │                                                                          │
//! │  HandlerContext::extract(state, client_id)                              │
//! │      │                                                                   │
//! │      └──> ctx.into() ──> UseCaseContext                                 │
//! │                                                                          │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                     APPLICATION LAYER (Use Cases)                        │
//! │                                                                          │
//! │  movement_use_case.move_to_region(ctx: UseCaseContext, ...)             │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Design Rationale (D2)
//!
//! Typed context variants (`DmContext`, `PlayerContext`) are in the adapter layer
//! where they provide compile-time guarantees about authorization. This `UseCaseContext`
//! is a unified type for passing to use cases, which perform their own validation
//! if needed.

use wrldbldr_domain::{PlayerCharacterId, WorldId};

/// Context for use case execution
///
/// Passed from handlers to use cases, contains identity and authorization info.
/// Defined in engine-ports to avoid circular dependencies between adapters and app.
#[derive(Debug, Clone)]
pub struct UseCaseContext {
    /// World this operation is for
    pub world_id: WorldId,
    /// User performing the operation
    pub user_id: String,
    /// Whether the user is a DM
    pub is_dm: bool,
    /// Player character ID (if user is playing a PC)
    pub pc_id: Option<PlayerCharacterId>,
}

impl UseCaseContext {
    /// Create a new context with all fields
    pub fn new(
        world_id: WorldId,
        user_id: String,
        is_dm: bool,
        pc_id: Option<PlayerCharacterId>,
    ) -> Self {
        Self {
            world_id,
            user_id,
            is_dm,
            pc_id,
        }
    }

    /// Create a DM context
    ///
    /// Use when you know the user is a DM (e.g., from `HandlerContext::require_dm()`).
    pub fn dm(world_id: WorldId, user_id: String) -> Self {
        Self {
            world_id,
            user_id,
            is_dm: true,
            pc_id: None,
        }
    }

    /// Create a player context
    ///
    /// Use when you know the user is a player with a PC
    /// (e.g., from `HandlerContext::require_player()`).
    pub fn player(world_id: WorldId, user_id: String, pc_id: PlayerCharacterId) -> Self {
        Self {
            world_id,
            user_id,
            is_dm: false,
            pc_id: Some(pc_id),
        }
    }

    /// Create a spectator context (not DM, no PC)
    ///
    /// Use for observers who can view but not interact.
    pub fn spectator(world_id: WorldId, user_id: String) -> Self {
        Self {
            world_id,
            user_id,
            is_dm: false,
            pc_id: None,
        }
    }

    /// Check if the context has a PC selected
    pub fn has_pc(&self) -> bool {
        self.pc_id.is_some()
    }

    /// Get PC ID or return an error
    ///
    /// Use in use cases that require a PC.
    pub fn require_pc(&self) -> Result<PlayerCharacterId, &'static str> {
        self.pc_id.ok_or("No player character selected")
    }

    /// Check if this is a DM context
    pub fn is_dm(&self) -> bool {
        self.is_dm
    }

    /// Get the world ID as a UUID
    pub fn world_id_uuid(&self) -> uuid::Uuid {
        *self.world_id.as_uuid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_dm_context() {
        let world_id = WorldId::from_uuid(Uuid::new_v4());
        let ctx = UseCaseContext::dm(world_id, "dm-user".to_string());

        assert!(ctx.is_dm());
        assert!(!ctx.has_pc());
        assert_eq!(ctx.user_id, "dm-user");
    }

    #[test]
    fn test_player_context() {
        let world_id = WorldId::from_uuid(Uuid::new_v4());
        let pc_id = PlayerCharacterId::from_uuid(Uuid::new_v4());
        let ctx = UseCaseContext::player(world_id, "player-user".to_string(), pc_id);

        assert!(!ctx.is_dm());
        assert!(ctx.has_pc());
        assert_eq!(ctx.pc_id, Some(pc_id));
    }

    #[test]
    fn test_require_pc() {
        let world_id = WorldId::from_uuid(Uuid::new_v4());
        let pc_id = PlayerCharacterId::from_uuid(Uuid::new_v4());

        let player_ctx = UseCaseContext::player(world_id, "player".to_string(), pc_id);
        assert!(player_ctx.require_pc().is_ok());

        let dm_ctx = UseCaseContext::dm(world_id, "dm".to_string());
        assert!(dm_ctx.require_pc().is_err());
    }
}
