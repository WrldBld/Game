//! Lore entity operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, CharacterId, LoreCategory, LoreChunkId, LoreId, WorldId};

use crate::infrastructure::ports::{LoreRepo, RepoError};

/// Lore entity operations.
pub struct Lore {
    repo: Arc<dyn LoreRepo>,
}

impl Lore {
    pub fn new(repo: Arc<dyn LoreRepo>) -> Self {
        Self { repo }
    }

    // CRUD operations

    pub async fn get(&self, id: LoreId) -> Result<Option<domain::Lore>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, lore: &domain::Lore) -> Result<(), RepoError> {
        self.repo.save(lore).await
    }

    pub async fn delete(&self, id: LoreId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }

    // Query operations

    pub async fn list_for_world(&self, world_id: WorldId) -> Result<Vec<domain::Lore>, RepoError> {
        self.repo.list_for_world(world_id).await
    }

    pub async fn list_by_category(
        &self,
        world_id: WorldId,
        category: LoreCategory,
    ) -> Result<Vec<domain::Lore>, RepoError> {
        self.repo.list_by_category(world_id, category).await
    }

    pub async fn list_common_knowledge(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::Lore>, RepoError> {
        self.repo.list_common_knowledge(world_id).await
    }

    pub async fn search_by_tags(
        &self,
        world_id: WorldId,
        tags: &[String],
    ) -> Result<Vec<domain::Lore>, RepoError> {
        self.repo.search_by_tags(world_id, tags).await
    }

    // Knowledge management

    pub async fn grant_knowledge(
        &self,
        knowledge: &domain::LoreKnowledge,
    ) -> Result<(), RepoError> {
        self.repo.grant_knowledge(knowledge).await
    }

    pub async fn revoke_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
    ) -> Result<(), RepoError> {
        self.repo.revoke_knowledge(character_id, lore_id).await
    }

    pub async fn get_character_knowledge(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<domain::LoreKnowledge>, RepoError> {
        self.repo.get_character_knowledge(character_id).await
    }

    pub async fn get_knowledge_for_lore(
        &self,
        lore_id: LoreId,
    ) -> Result<Vec<domain::LoreKnowledge>, RepoError> {
        self.repo.get_knowledge_for_lore(lore_id).await
    }

    pub async fn character_knows_lore(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
    ) -> Result<Option<domain::LoreKnowledge>, RepoError> {
        self.repo.character_knows_lore(character_id, lore_id).await
    }

    pub async fn add_chunks_to_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
        chunk_ids: &[LoreChunkId],
    ) -> Result<(), RepoError> {
        self.repo
            .add_chunks_to_knowledge(character_id, lore_id, chunk_ids)
            .await
    }
}
