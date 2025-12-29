//! World service port - Interface for world operations
//!
//! This port abstracts world business logic from infrastructure adapters.
//! It exposes query methods for retrieving worlds and their current state.
//!
//! # Design Notes
//!
//! This port is designed for use by infrastructure adapters that need to query
//! world information. It focuses on read operations used by connection handlers,
//! prompt builders, and state management.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::entities::{Location, World};
use wrldbldr_domain::WorldId;

use super::PlayerWorldSnapshot;

/// Port for world service operations used by infrastructure adapters.
///
/// This trait provides read-only access to world data for use in
/// connection handling, state management, and prompt building.
///
/// # Usage
///
/// Infrastructure adapters should depend on this trait rather than importing
/// the service directly from engine-app, maintaining proper hexagonal
/// architecture boundaries.
#[async_trait]
pub trait WorldServicePort: Send + Sync {
    /// Get a world by ID.
    ///
    /// Returns `Ok(None)` if the world is not found.
    async fn get_world(&self, id: WorldId) -> Result<Option<World>>;

    /// List all available worlds.
    ///
    /// Returns worlds sorted by creation date (newest first).
    async fn list_worlds(&self) -> Result<Vec<World>>;

    /// Get the current location for a world.
    ///
    /// This retrieves the location where the player currently is,
    /// based on the world's state tracking.
    ///
    /// Returns `Ok(None)` if no current location is set.
    async fn get_current_location(&self, world_id: WorldId) -> Result<Option<Location>>;

    /// Export a world snapshot for Player clients.
    ///
    /// Returns a complete snapshot of the world state including all entities,
    /// characters, locations, and other data needed by the player client.
    async fn export_world_snapshot(&self, world_id: WorldId) -> Result<PlayerWorldSnapshot>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of WorldServicePort for testing.
    pub WorldServicePort {}

    #[async_trait]
    impl WorldServicePort for WorldServicePort {
        async fn get_world(&self, id: WorldId) -> Result<Option<World>>;
        async fn list_worlds(&self) -> Result<Vec<World>>;
        async fn get_current_location(&self, world_id: WorldId) -> Result<Option<Location>>;
        async fn export_world_snapshot(&self, world_id: WorldId) -> Result<PlayerWorldSnapshot>;
    }
}
