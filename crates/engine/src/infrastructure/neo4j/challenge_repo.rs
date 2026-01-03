//! Neo4j challenge repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{ChallengeRepo, RepoError};

pub struct Neo4jChallengeRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jChallengeRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl ChallengeRepo for Neo4jChallengeRepo {
    async fn get(&self, _id: ChallengeId) -> Result<Option<Challenge>, RepoError> { todo!() }
    async fn save(&self, _challenge: &Challenge) -> Result<(), RepoError> { todo!() }
    async fn list_for_scene(&self, _scene_id: SceneId) -> Result<Vec<Challenge>, RepoError> { todo!() }
    async fn list_pending_for_world(&self, _world_id: WorldId) -> Result<Vec<Challenge>, RepoError> { todo!() }
    async fn mark_resolved(&self, _id: ChallengeId) -> Result<(), RepoError> { todo!() }
}
