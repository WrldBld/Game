//! Neo4j player character repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{PlayerCharacterRepo, RepoError};

pub struct Neo4jPlayerCharacterRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jPlayerCharacterRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl PlayerCharacterRepo for Neo4jPlayerCharacterRepo {
    async fn get(&self, _id: PlayerCharacterId) -> Result<Option<PlayerCharacter>, RepoError> { todo!() }
    async fn save(&self, _pc: &PlayerCharacter) -> Result<(), RepoError> { todo!() }
    async fn delete(&self, _id: PlayerCharacterId) -> Result<(), RepoError> { todo!() }
    async fn list_in_world(&self, _world_id: WorldId) -> Result<Vec<PlayerCharacter>, RepoError> { todo!() }
    async fn get_by_user(&self, _world_id: WorldId, _user_id: &str) -> Result<Option<PlayerCharacter>, RepoError> { todo!() }
    async fn update_position(&self, _id: PlayerCharacterId, _loc: LocationId, _reg: RegionId) -> Result<(), RepoError> { todo!() }
    async fn get_inventory(&self, _id: PlayerCharacterId) -> Result<Vec<Item>, RepoError> { todo!() }
}
