//! Character entity operations.

use std::sync::Arc;
use wrldbldr_domain::{
    self as domain, ActantialContext, CharacterId, Item, NpcDispositionState, RegionId,
    Relationship, Want, WorldId,
};

use crate::infrastructure::ports::{CharacterRepo, RepoError};

/// Character entity operations.
///
/// Encapsulates all character-related queries and mutations.
pub struct Character {
    repo: Arc<dyn CharacterRepo>,
}

impl Character {
    pub fn new(repo: Arc<dyn CharacterRepo>) -> Self {
        Self { repo }
    }

    // =========================================================================
    // CRUD
    // =========================================================================

    pub async fn get(&self, id: CharacterId) -> Result<Option<domain::Character>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, character: &domain::Character) -> Result<(), RepoError> {
        self.repo.save(character).await
    }

    pub async fn delete(&self, id: CharacterId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }

    // =========================================================================
    // Queries
    // =========================================================================

    pub async fn list_in_region(&self, region_id: RegionId) -> Result<Vec<domain::Character>, RepoError> {
        self.repo.list_in_region(region_id).await
    }

    pub async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<domain::Character>, RepoError> {
        self.repo.list_in_world(world_id).await
    }

    pub async fn list_npcs_in_world(&self, world_id: WorldId) -> Result<Vec<domain::Character>, RepoError> {
        self.repo.list_npcs_in_world(world_id).await
    }

    // =========================================================================
    // Position
    // =========================================================================

    pub async fn update_position(&self, id: CharacterId, region_id: RegionId) -> Result<(), RepoError> {
        self.repo.update_position(id, region_id).await
    }

    // =========================================================================
    // Relationships
    // =========================================================================

    pub async fn get_relationships(&self, id: CharacterId) -> Result<Vec<Relationship>, RepoError> {
        self.repo.get_relationships(id).await
    }

    pub async fn save_relationship(&self, relationship: &Relationship) -> Result<(), RepoError> {
        self.repo.save_relationship(relationship).await
    }

    // =========================================================================
    // Inventory
    // =========================================================================

    pub async fn get_inventory(&self, id: CharacterId) -> Result<Vec<Item>, RepoError> {
        self.repo.get_inventory(id).await
    }

    pub async fn add_to_inventory(&self, character_id: CharacterId, item_id: wrldbldr_domain::ItemId) -> Result<(), RepoError> {
        self.repo.add_to_inventory(character_id, item_id).await
    }

    pub async fn remove_from_inventory(&self, character_id: CharacterId, item_id: wrldbldr_domain::ItemId) -> Result<(), RepoError> {
        self.repo.remove_from_inventory(character_id, item_id).await
    }

    // =========================================================================
    // Wants/Goals
    // =========================================================================

    pub async fn get_wants(&self, id: CharacterId) -> Result<Vec<Want>, RepoError> {
        self.repo.get_wants(id).await
    }

    pub async fn save_want(&self, character_id: CharacterId, want: &Want) -> Result<(), RepoError> {
        self.repo.save_want(character_id, want).await
    }

    // =========================================================================
    // Disposition
    // =========================================================================

    pub async fn get_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: wrldbldr_domain::PlayerCharacterId,
    ) -> Result<Option<NpcDispositionState>, RepoError> {
        self.repo.get_disposition(npc_id, pc_id).await
    }

    pub async fn save_disposition(
        &self,
        disposition: &NpcDispositionState,
    ) -> Result<(), RepoError> {
        self.repo.save_disposition(disposition).await
    }

    // =========================================================================
    // Actantial
    // =========================================================================

    pub async fn get_actantial_context(&self, id: CharacterId) -> Result<Option<ActantialContext>, RepoError> {
        self.repo.get_actantial_context(id).await
    }

    pub async fn save_actantial_context(&self, id: CharacterId, context: &ActantialContext) -> Result<(), RepoError> {
        self.repo.save_actantial_context(id, context).await
    }
}
