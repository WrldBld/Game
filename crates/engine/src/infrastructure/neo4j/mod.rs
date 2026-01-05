//! Neo4j database implementations.

use neo4rs::Graph;
use std::sync::Arc;

use crate::infrastructure::ports::ClockPort;

mod helpers;

mod character_repo;
mod location_repo;
mod scene_repo;
mod challenge_repo;
mod narrative_repo;
mod staging_repo;
mod observation_repo;
mod item_repo;
mod world_repo;
mod asset_repo;
mod player_character_repo;
mod flag_repo;
mod lore_repo;
mod location_state_repo;
mod region_state_repo;

pub use character_repo::Neo4jCharacterRepo;
pub use location_repo::Neo4jLocationRepo;
pub use scene_repo::Neo4jSceneRepo;
pub use challenge_repo::Neo4jChallengeRepo;
pub use narrative_repo::Neo4jNarrativeRepo;
pub use staging_repo::Neo4jStagingRepo;
pub use observation_repo::Neo4jObservationRepo;
pub use item_repo::Neo4jItemRepo;
pub use world_repo::Neo4jWorldRepo;
pub use asset_repo::Neo4jAssetRepo;
pub use player_character_repo::Neo4jPlayerCharacterRepo;
pub use flag_repo::Neo4jFlagRepo;
pub use lore_repo::Neo4jLoreRepo;
pub use location_state_repo::Neo4jLocationStateRepo;
pub use region_state_repo::Neo4jRegionStateRepo;

/// Create all Neo4j repositories from a graph connection.
pub struct Neo4jRepositories {
    pub character: Arc<Neo4jCharacterRepo>,
    pub player_character: Arc<Neo4jPlayerCharacterRepo>,
    pub location: Arc<Neo4jLocationRepo>,
    pub scene: Arc<Neo4jSceneRepo>,
    pub challenge: Arc<Neo4jChallengeRepo>,
    pub narrative: Arc<Neo4jNarrativeRepo>,
    pub staging: Arc<Neo4jStagingRepo>,
    pub observation: Arc<Neo4jObservationRepo>,
    pub item: Arc<Neo4jItemRepo>,
    pub world: Arc<Neo4jWorldRepo>,
    pub asset: Arc<Neo4jAssetRepo>,
    pub flag: Arc<Neo4jFlagRepo>,
    pub lore: Arc<Neo4jLoreRepo>,
    pub location_state: Arc<Neo4jLocationStateRepo>,
    pub region_state: Arc<Neo4jRegionStateRepo>,
}

impl Neo4jRepositories {
    pub fn new(graph: Graph, clock: Arc<dyn ClockPort>) -> Self {
        Self {
            character: Arc::new(Neo4jCharacterRepo::new(graph.clone())),
            player_character: Arc::new(Neo4jPlayerCharacterRepo::new(graph.clone(), clock.clone())),
            location: Arc::new(Neo4jLocationRepo::new(graph.clone())),
            scene: Arc::new(Neo4jSceneRepo::new(graph.clone())),
            challenge: Arc::new(Neo4jChallengeRepo::new(graph.clone())),
            narrative: Arc::new(Neo4jNarrativeRepo::new(graph.clone(), clock.clone())),
            staging: Arc::new(Neo4jStagingRepo::new(graph.clone(), clock.clone())),
            observation: Arc::new(Neo4jObservationRepo::new(graph.clone(), clock.clone())),
            item: Arc::new(Neo4jItemRepo::new(graph.clone())),
            world: Arc::new(Neo4jWorldRepo::new(graph.clone(), clock.clone())),
            asset: Arc::new(Neo4jAssetRepo::new(graph.clone())),
            flag: Arc::new(Neo4jFlagRepo::new(Arc::new(graph.clone()))),
            lore: Arc::new(Neo4jLoreRepo::new(graph.clone(), clock.clone())),
            location_state: Arc::new(Neo4jLocationStateRepo::new(graph.clone(), clock.clone())),
            region_state: Arc::new(Neo4jRegionStateRepo::new(graph, clock)),
        }
    }
}
