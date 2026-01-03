//! Neo4j world repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{WorldRepo, RepoError};

pub struct Neo4jWorldRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jWorldRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl WorldRepo for Neo4jWorldRepo {
    async fn get(&self, _id: WorldId) -> Result<Option<World>, RepoError> { todo!() }
    async fn save(&self, _world: &World) -> Result<(), RepoError> { todo!() }
    async fn list_all(&self) -> Result<Vec<World>, RepoError> { todo!() }
    async fn delete(&self, _id: WorldId) -> Result<(), RepoError> { todo!() }
}
