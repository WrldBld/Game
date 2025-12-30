//! World Time Port - Game time management.
//!
//! This port handles game time operations for worlds.

use wrldbldr_domain::{GameTime, WorldId};

/// Port for managing game time within a world.
///
/// Game time is separate from real time and advances based on
/// in-game events and DM direction.
///
/// All methods are synchronous as they operate on in-memory state.
/// Implementations must be thread-safe (Send + Sync).
pub trait WorldTimePort: Send + Sync {
    /// Get the current game time for a world.
    ///
    /// Returns `None` if the world hasn't been initialized.
    fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime>;

    /// Set the game time for a world.
    ///
    /// This will initialize the world state if it doesn't exist.
    fn set_game_time(&self, world_id: &WorldId, time: GameTime);

    /// Advance game time by the specified hours and minutes.
    ///
    /// Returns the new game time, or `None` if the world doesn't exist.
    fn advance_game_time(&self, world_id: &WorldId, hours: i64, minutes: i64) -> Option<GameTime>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    pub WorldTimePort {}

    impl WorldTimePort for WorldTimePort {
        fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime>;
        fn set_game_time(&self, world_id: &WorldId, time: GameTime);
        fn advance_game_time(&self, world_id: &WorldId, hours: i64, minutes: i64) -> Option<GameTime>;
    }
}
