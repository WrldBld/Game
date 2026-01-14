//! Neo4j database implementations.

use std::sync::Arc;

use crate::infrastructure::ports::ClockPort;

mod graph;
#[cfg(test)]
mod graph_test;
mod helpers;
pub mod query_helpers;
mod schema;

#[cfg(test)]
pub use graph_test::Neo4jGraph;
#[cfg(not(test))]
pub use graph::Neo4jGraph;
pub use graph::Neo4jRowStream;
pub use schema::ensure_schema;

mod act_repo;
mod asset_repo;
mod challenge_repo;
mod character_repo;
mod flag_repo;
mod goal_repo;
mod interaction_repo;
mod item_repo;
mod location_repo;
mod location_state_repo;
mod lore_repo;
mod narrative_repo;
mod observation_repo;
mod player_character_repo;
mod region_state_repo;
mod scene_repo;
mod skill_repo;
mod staging_repo;
mod world_repo;

pub use act_repo::Neo4jActRepo;
pub use asset_repo::Neo4jAssetRepo;
pub use challenge_repo::Neo4jChallengeRepo;
pub use character_repo::Neo4jCharacterRepo;
pub use flag_repo::Neo4jFlagRepo;
pub use goal_repo::Neo4jGoalRepo;
pub use interaction_repo::Neo4jInteractionRepo;
pub use item_repo::Neo4jItemRepo;
pub use location_repo::Neo4jLocationRepo;
pub use location_state_repo::Neo4jLocationStateRepo;
pub use lore_repo::Neo4jLoreRepo;
pub use narrative_repo::Neo4jNarrativeRepo;
pub use observation_repo::Neo4jObservationRepo;
pub use player_character_repo::Neo4jPlayerCharacterRepo;
pub use region_state_repo::Neo4jRegionStateRepo;
pub use scene_repo::Neo4jSceneRepo;
pub use skill_repo::Neo4jSkillRepo;
pub use staging_repo::Neo4jStagingRepo;
pub use world_repo::Neo4jWorldRepo;

#[cfg(test)]
mod integration_tests;

/// Create all Neo4j repositories from a graph connection.
pub struct Neo4jRepositories {
    pub character: Arc<Neo4jCharacterRepo>,
    pub player_character: Arc<Neo4jPlayerCharacterRepo>,
    pub location: Arc<Neo4jLocationRepo>,
    pub scene: Arc<Neo4jSceneRepo>,
    pub act: Arc<Neo4jActRepo>,
    pub skill: Arc<Neo4jSkillRepo>,
    pub interaction: Arc<Neo4jInteractionRepo>,
    pub challenge: Arc<Neo4jChallengeRepo>,
    pub narrative: Arc<Neo4jNarrativeRepo>,
    pub staging: Arc<Neo4jStagingRepo>,
    pub observation: Arc<Neo4jObservationRepo>,
    pub item: Arc<Neo4jItemRepo>,
    pub world: Arc<Neo4jWorldRepo>,
    pub asset: Arc<Neo4jAssetRepo>,
    pub flag: Arc<Neo4jFlagRepo>,
    pub goal: Arc<Neo4jGoalRepo>,
    pub lore: Arc<Neo4jLoreRepo>,
    pub location_state: Arc<Neo4jLocationStateRepo>,
    pub region_state: Arc<Neo4jRegionStateRepo>,
}

impl Neo4jRepositories {
    pub fn new(graph: Neo4jGraph, clock: Arc<dyn ClockPort>) -> Self {
        Self {
            character: Arc::new(Neo4jCharacterRepo::new(graph.clone())),
            player_character: Arc::new(Neo4jPlayerCharacterRepo::new(graph.clone(), clock.clone())),
            location: Arc::new(Neo4jLocationRepo::new(graph.clone())),
            scene: Arc::new(Neo4jSceneRepo::new(graph.clone())),
            act: Arc::new(Neo4jActRepo::new(graph.clone())),
            skill: Arc::new(Neo4jSkillRepo::new(graph.clone())),
            interaction: Arc::new(Neo4jInteractionRepo::new(graph.clone())),
            challenge: Arc::new(Neo4jChallengeRepo::new(graph.clone())),
            narrative: Arc::new(Neo4jNarrativeRepo::new(graph.clone(), clock.clone())),
            staging: Arc::new(Neo4jStagingRepo::new(graph.clone(), clock.clone())),
            observation: Arc::new(Neo4jObservationRepo::new(graph.clone(), clock.clone())),
            item: Arc::new(Neo4jItemRepo::new(graph.clone())),
            world: Arc::new(Neo4jWorldRepo::new(graph.clone(), clock.clone())),
            asset: Arc::new(Neo4jAssetRepo::new(graph.clone())),
            flag: Arc::new(Neo4jFlagRepo::new(Arc::new(graph.clone()))),
            goal: Arc::new(Neo4jGoalRepo::new(graph.clone())),
            lore: Arc::new(Neo4jLoreRepo::new(graph.clone(), clock.clone())),
            location_state: Arc::new(Neo4jLocationStateRepo::new(graph.clone(), clock.clone())),
            region_state: Arc::new(Neo4jRegionStateRepo::new(graph, clock)),
        }
    }
}
