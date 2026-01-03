//! Neo4j location repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{LocationRepo, RepoError};

pub struct Neo4jLocationRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jLocationRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl LocationRepo for Neo4jLocationRepo {
    async fn get_location(&self, _id: LocationId) -> Result<Option<Location>, RepoError> { todo!() }
    async fn save_location(&self, _location: &Location) -> Result<(), RepoError> { todo!() }
    async fn list_locations_in_world(&self, _world_id: WorldId) -> Result<Vec<Location>, RepoError> { todo!() }
    async fn get_region(&self, _id: RegionId) -> Result<Option<Region>, RepoError> { todo!() }
    async fn save_region(&self, _region: &Region) -> Result<(), RepoError> { todo!() }
    async fn list_regions_in_location(&self, _location_id: LocationId) -> Result<Vec<Region>, RepoError> { todo!() }
    async fn get_connections(&self, _region_id: RegionId) -> Result<Vec<RegionConnection>, RepoError> { todo!() }
    async fn save_connection(&self, _connection: &RegionConnection) -> Result<(), RepoError> { todo!() }
    async fn get_location_exits(&self, _location_id: LocationId) -> Result<Vec<LocationConnection>, RepoError> { todo!() }
}
