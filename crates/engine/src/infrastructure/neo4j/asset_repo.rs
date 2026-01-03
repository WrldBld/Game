//! Neo4j asset repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use uuid::Uuid;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{AssetRepo, RepoError};

pub struct Neo4jAssetRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jAssetRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl AssetRepo for Neo4jAssetRepo {
    async fn get(&self, _id: AssetId) -> Result<Option<GalleryAsset>, RepoError> { todo!() }
    async fn save(&self, _asset: &GalleryAsset) -> Result<(), RepoError> { todo!() }
    async fn list_for_entity(&self, _entity_type: &str, _entity_id: Uuid) -> Result<Vec<GalleryAsset>, RepoError> { todo!() }
    async fn set_active(&self, _entity_type: &str, _entity_id: Uuid, _asset_id: AssetId) -> Result<(), RepoError> { todo!() }
}
