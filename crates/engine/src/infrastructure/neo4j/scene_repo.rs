//! Neo4j scene repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{SceneRepo, RepoError};

pub struct Neo4jSceneRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jSceneRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl SceneRepo for Neo4jSceneRepo {
    async fn get(&self, _id: SceneId) -> Result<Option<Scene>, RepoError> { todo!() }
    async fn save(&self, _scene: &Scene) -> Result<(), RepoError> { todo!() }
    async fn get_current(&self, _world_id: WorldId) -> Result<Option<Scene>, RepoError> { todo!() }
    async fn set_current(&self, _world_id: WorldId, _scene_id: SceneId) -> Result<(), RepoError> { todo!() }
    async fn list_for_region(&self, _region_id: RegionId) -> Result<Vec<Scene>, RepoError> { todo!() }
    async fn get_featured_characters(&self, _scene_id: SceneId) -> Result<Vec<CharacterId>, RepoError> { todo!() }
    async fn set_featured_characters(&self, _scene_id: SceneId, _chars: &[CharacterId]) -> Result<(), RepoError> { todo!() }
}
