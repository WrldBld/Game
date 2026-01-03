//! Neo4j staging repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{StagingRepo, RepoError};

pub struct Neo4jStagingRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jStagingRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl StagingRepo for Neo4jStagingRepo {
    async fn get_staged_npcs(&self, _region_id: RegionId) -> Result<Vec<StagedNpc>, RepoError> { todo!() }
    async fn stage_npc(&self, _region_id: RegionId, _character_id: CharacterId) -> Result<(), RepoError> { todo!() }
    async fn unstage_npc(&self, _region_id: RegionId, _character_id: CharacterId) -> Result<(), RepoError> { todo!() }
    async fn get_pending_staging(&self, _world_id: WorldId) -> Result<Vec<Staging>, RepoError> { todo!() }
    async fn save_pending_staging(&self, _staging: &Staging) -> Result<(), RepoError> { todo!() }
    async fn delete_pending_staging(&self, _id: StagingId) -> Result<(), RepoError> { todo!() }
}
