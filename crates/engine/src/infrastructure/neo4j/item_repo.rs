//! Neo4j item repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{ItemRepo, RepoError};

pub struct Neo4jItemRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jItemRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl ItemRepo for Neo4jItemRepo {
    async fn get(&self, _id: ItemId) -> Result<Option<Item>, RepoError> { todo!() }
    async fn save(&self, _item: &Item) -> Result<(), RepoError> { todo!() }
    async fn list_in_region(&self, _region_id: RegionId) -> Result<Vec<Item>, RepoError> { todo!() }
    async fn list_in_world(&self, _world_id: WorldId) -> Result<Vec<Item>, RepoError> { todo!() }
}
