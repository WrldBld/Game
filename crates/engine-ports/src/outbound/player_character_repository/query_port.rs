//! Query operations for PlayerCharacter entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{LocationId, PlayerCharacter, WorldId};

/// Query operations for finding player characters.
///
/// This trait covers lookup operations that return collections
/// of player characters based on various criteria.
#[async_trait]
pub trait PlayerCharacterQueryPort: Send + Sync {
    /// Get all player characters at a specific location
    async fn get_by_location(&self, location_id: LocationId) -> Result<Vec<PlayerCharacter>>;

    /// Get all player characters for a user in a world (for PC selection)
    async fn get_by_user_and_world(
        &self,
        user_id: &str,
        world_id: WorldId,
    ) -> Result<Vec<PlayerCharacter>>;

    /// Get all player characters in a world
    async fn get_all_by_world(&self, world_id: WorldId) -> Result<Vec<PlayerCharacter>>;

    /// Get all unbound player characters for a user (no session)
    async fn get_unbound_by_user(&self, user_id: &str) -> Result<Vec<PlayerCharacter>>;
}
