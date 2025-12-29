//! Player character service port - Interface for player character operations
//!
//! This port abstracts player character business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::PlayerCharacter;
use wrldbldr_domain::{PlayerCharacterId, WorldId};

/// Port for player character service operations
///
/// This trait defines the read operations for player character management.
/// Adapters implement this trait by wrapping the PlayerCharacterService.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait PlayerCharacterServicePort: Send + Sync {
    /// Get a player character by ID
    ///
    /// Returns the player character if found, or None if not found.
    async fn get_player_character(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>>;

    /// Get a player character by world and user
    ///
    /// Returns the player character for the given user in the specified world,
    /// or None if not found.
    async fn get_by_world_and_user(
        &self,
        world_id: WorldId,
        user_id: &str,
    ) -> Result<Option<PlayerCharacter>>;

    /// List all player characters in a world
    ///
    /// Returns all player characters belonging to the specified world.
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<PlayerCharacter>>;
}
