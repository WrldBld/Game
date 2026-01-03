//! Neo4j narrative repository - stub implementation.

use async_trait::async_trait;
use neo4rs::Graph;
use wrldbldr_domain::*;
use crate::infrastructure::ports::{NarrativeRepo, RepoError};

pub struct Neo4jNarrativeRepo { #[allow(dead_code)] graph: Graph }
impl Neo4jNarrativeRepo { pub fn new(graph: Graph) -> Self { Self { graph } } }

#[async_trait]
impl NarrativeRepo for Neo4jNarrativeRepo {
    async fn get_event(&self, _id: NarrativeEventId) -> Result<Option<NarrativeEvent>, RepoError> { todo!() }
    async fn save_event(&self, _event: &NarrativeEvent) -> Result<(), RepoError> { todo!() }
    async fn list_events_for_world(&self, _world_id: WorldId) -> Result<Vec<NarrativeEvent>, RepoError> { todo!() }
    async fn get_chain(&self, _id: EventChainId) -> Result<Option<EventChain>, RepoError> { todo!() }
    async fn save_chain(&self, _chain: &EventChain) -> Result<(), RepoError> { todo!() }
    async fn get_story_event(&self, _id: StoryEventId) -> Result<Option<StoryEvent>, RepoError> { todo!() }
    async fn save_story_event(&self, _event: &StoryEvent) -> Result<(), RepoError> { todo!() }
    async fn list_story_events(&self, _world_id: WorldId, _limit: usize) -> Result<Vec<StoryEvent>, RepoError> { todo!() }
    async fn get_triggers_for_region(&self, _region_id: RegionId) -> Result<Vec<NarrativeEvent>, RepoError> { todo!() }
}
