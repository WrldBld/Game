//! World Scene Port - Current scene tracking.
//!
//! This port handles tracking the current scene for a world.

use wrldbldr_domain::WorldId;

/// Port for tracking the current scene within a world.
///
/// The current scene determines which narrative context is active
/// and affects NPC behavior and available interactions.
///
/// All methods are synchronous as they operate on in-memory state.
/// Implementations must be thread-safe (Send + Sync).
pub trait WorldScenePort: Send + Sync {
    /// Get the current scene ID for a world.
    fn get_current_scene(&self, world_id: &WorldId) -> Option<String>;

    /// Set the current scene for a world.
    fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>);
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    pub WorldScenePort {}

    impl WorldScenePort for WorldScenePort {
        fn get_current_scene(&self, world_id: &WorldId) -> Option<String>;
        fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>);
    }
}
