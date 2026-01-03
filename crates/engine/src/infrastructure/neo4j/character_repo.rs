//! Neo4j character repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{CharacterRepo, RepoError};

pub struct Neo4jCharacterRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jCharacterRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl CharacterRepo for Neo4jCharacterRepo {
    async fn get(&self, _id: CharacterId) -> Result<Option<Character>, RepoError> { todo!() }
    async fn save(&self, _character: &Character) -> Result<(), RepoError> { todo!() }
    async fn delete(&self, _id: CharacterId) -> Result<(), RepoError> { todo!() }
    async fn list_in_region(&self, _region_id: RegionId) -> Result<Vec<Character>, RepoError> { todo!() }
    async fn list_in_world(&self, _world_id: WorldId) -> Result<Vec<Character>, RepoError> { todo!() }
    async fn list_npcs_in_world(&self, _world_id: WorldId) -> Result<Vec<Character>, RepoError> { todo!() }
    async fn update_position(&self, _id: CharacterId, _region_id: RegionId) -> Result<(), RepoError> { todo!() }
    async fn get_relationships(&self, _id: CharacterId) -> Result<Vec<Relationship>, RepoError> { todo!() }
    async fn save_relationship(&self, _relationship: &Relationship) -> Result<(), RepoError> { todo!() }
    async fn get_inventory(&self, _id: CharacterId) -> Result<Vec<Item>, RepoError> { todo!() }
    async fn add_to_inventory(&self, _cid: CharacterId, _iid: ItemId) -> Result<(), RepoError> { todo!() }
    async fn remove_from_inventory(&self, _cid: CharacterId, _iid: ItemId) -> Result<(), RepoError> { todo!() }
    async fn get_wants(&self, _id: CharacterId) -> Result<Vec<Want>, RepoError> { todo!() }
    async fn save_want(&self, _cid: CharacterId, _want: &Want) -> Result<(), RepoError> { todo!() }
    async fn get_disposition(&self, _npc_id: CharacterId, _pc_id: PlayerCharacterId) -> Result<Option<NpcDispositionState>, RepoError> { todo!() }
    async fn save_disposition(&self, _d: &NpcDispositionState) -> Result<(), RepoError> { todo!() }
    async fn get_actantial_context(&self, _id: CharacterId) -> Result<Option<ActantialContext>, RepoError> { todo!() }
    async fn save_actantial_context(&self, _id: CharacterId, _c: &ActantialContext) -> Result<(), RepoError> { todo!() }
}
