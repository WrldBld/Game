//! World use case port - Inbound interface for world operations
//!
//! This port is called by HTTP handlers for world export and other operations.
//! The implementation lives in engine-app.
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

use crate::outbound::PlayerWorldSnapshot;

/// Port for world use case operations
///
/// This trait provides read-only access to world data for use in
/// connection handling, state management, and prompt building.
///
/// Called by: HTTP handlers in export_routes.rs
/// Implemented by: WorldService in engine-app
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait WorldUseCasePort: Send + Sync {
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
