//! Player Character entity operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, Item, LocationId, PlayerCharacterId, RegionId, WorldId};

use crate::infrastructure::ports::{PlayerCharacterRepo, RepoError};

/// Player Character entity operations.
///
/// Encapsulates all player character-related queries and mutations.
pub struct PlayerCharacterRepository {
    repo: Arc<dyn PlayerCharacterRepo>,
}

impl PlayerCharacterRepository {
    pub fn new(repo: Arc<dyn PlayerCharacterRepo>) -> Self {
        Self { repo }
    }

    // =========================================================================
    // CRUD
    // =========================================================================

    pub async fn get(
        &self,
        id: PlayerCharacterId,
    ) -> Result<Option<domain::PlayerCharacter>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, pc: &domain::PlayerCharacter) -> Result<(), RepoError> {
        self.repo.save(pc).await
    }

    pub async fn delete(&self, id: PlayerCharacterId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }

    // =========================================================================
    // Queries
    // =========================================================================

    pub async fn list_in_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::PlayerCharacter>, RepoError> {
        self.repo.list_in_world(world_id).await
    }

    pub async fn get_by_user(
        &self,
        world_id: WorldId,
        user_id: &str,
    ) -> Result<Option<domain::PlayerCharacter>, RepoError> {
        self.repo.get_by_user(world_id, user_id).await
    }

    // =========================================================================
    // Position
    // =========================================================================

    pub async fn update_position(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
        region_id: RegionId,
    ) -> Result<(), RepoError> {
        self.repo.update_position(id, location_id, region_id).await
    }

    // =========================================================================
    // Inventory
    // =========================================================================

    pub async fn get_inventory(&self, id: PlayerCharacterId) -> Result<Vec<Item>, RepoError> {
        self.repo.get_inventory(id).await
    }

    // =========================================================================
    // Stats
    // =========================================================================

    /// Modify a character stat by the given amount.
    ///
    /// This is used by the ModifyCharacterStat trigger from challenge outcomes.
    /// Common stats include "hp", "sanity", "fatigue", etc.
    pub async fn modify_stat(
        &self,
        id: PlayerCharacterId,
        stat: &str,
        modifier: i32,
    ) -> Result<(), RepoError> {
        self.repo.modify_stat(id, stat, modifier).await
    }
}
