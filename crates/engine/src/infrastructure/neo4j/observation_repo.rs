//! Neo4j observation repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{ObservationRepo, RepoError};

pub struct Neo4jObservationRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jObservationRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl ObservationRepo for Neo4jObservationRepo {
    async fn get_observations(&self, _pc_id: PlayerCharacterId) -> Result<Vec<NpcObservation>, RepoError> { todo!() }
    async fn save_observation(&self, _observation: &NpcObservation) -> Result<(), RepoError> { todo!() }
    async fn has_observed(&self, _pc_id: PlayerCharacterId, _target_id: CharacterId) -> Result<bool, RepoError> { todo!() }
}
