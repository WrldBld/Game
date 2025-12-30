//! World Lifecycle Port - World initialization and cleanup.
//!
//! This port handles world session lifecycle management.

use wrldbldr_domain::{GameTime, WorldId};

/// Port for managing world lifecycle within the state manager.
///
/// Handles initialization when a world connection is established
/// and cleanup when a world connection is closed.
///
/// All methods are synchronous as they operate on in-memory state.
/// Implementations must be thread-safe (Send + Sync).
pub trait WorldLifecyclePort: Send + Sync {
    /// Initialize state for a new world with the given starting time.
    ///
    /// This should be called when a world connection is established.
    fn initialize_world(&self, world_id: &WorldId, initial_time: GameTime);

    /// Clean up all state for a world.
    ///
    /// This should be called when a world connection is closed.
    fn cleanup_world(&self, world_id: &WorldId);

    /// Check if a world has been initialized.
    fn is_world_initialized(&self, world_id: &WorldId) -> bool;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    pub WorldLifecyclePort {}

    impl WorldLifecyclePort for WorldLifecyclePort {
        fn initialize_world(&self, world_id: &WorldId, initial_time: GameTime);
        fn cleanup_world(&self, world_id: &WorldId);
        fn is_world_initialized(&self, world_id: &WorldId) -> bool;
    }
}
