//! Split StoryEvent repository ports following Interface Segregation Principle.
//!
//! # Architecture
//!
//! The original `StoryEventRepositoryPort` (34 methods) is split into 4 focused traits:
//!
//! 1. `StoryEventCrudPort` - Core CRUD + state management (7 methods)
//! 2. `StoryEventEdgePort` - Edge relationship management (15 methods)
//! 3. `StoryEventQueryPort` - Query operations (10 methods)
//! 4. `StoryEventDialoguePort` - Dialogue-specific operations (2 methods)
//!
//! # Clean ISP Design
//!
//! Services should depend only on the traits they actually need:
//! - Services needing only CRUD operations depend on `StoryEventCrudPort`
//! - Services managing edges depend on `StoryEventEdgePort`
//! - Services performing queries depend on `StoryEventQueryPort`
//! - Services handling dialogues depend on `StoryEventDialoguePort`
//!
//! The composition root passes the same concrete repository instance to each service,
//! and Rust coerces to the needed trait interface.

mod crud_port;
mod dialogue_port;
mod edge_port;
mod query_port;

pub use crud_port::StoryEventCrudPort;
pub use dialogue_port::StoryEventDialoguePort;
pub use edge_port::StoryEventEdgePort;
pub use query_port::StoryEventQueryPort;

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use wrldbldr_domain::{
        ChallengeId, CharacterId, InvolvedCharacter, LocationId, NarrativeEventId,
        PlayerCharacterId, SceneId, StoryEvent, StoryEventId, WorldId,
    };

    mock! {
        /// Mock implementation of all StoryEvent repository traits for testing.
        pub StoryEventRepository {}

        #[async_trait]
        impl StoryEventCrudPort for StoryEventRepository {
            async fn create(&self, event: &StoryEvent) -> anyhow::Result<()>;
            async fn get(&self, id: StoryEventId) -> anyhow::Result<Option<StoryEvent>>;
            async fn update_summary(&self, id: StoryEventId, summary: &str) -> anyhow::Result<bool>;
            async fn set_hidden(&self, id: StoryEventId, is_hidden: bool) -> anyhow::Result<bool>;
            async fn update_tags(&self, id: StoryEventId, tags: Vec<String>) -> anyhow::Result<bool>;
            async fn delete(&self, id: StoryEventId) -> anyhow::Result<bool>;
            async fn count_by_world(&self, world_id: WorldId) -> anyhow::Result<u64>;
        }

        #[async_trait]
        impl StoryEventEdgePort for StoryEventRepository {
            async fn set_location(&self, event_id: StoryEventId, location_id: LocationId) -> anyhow::Result<bool>;
            async fn get_location(&self, event_id: StoryEventId) -> anyhow::Result<Option<LocationId>>;
            async fn remove_location(&self, event_id: StoryEventId) -> anyhow::Result<bool>;
            async fn set_scene(&self, event_id: StoryEventId, scene_id: SceneId) -> anyhow::Result<bool>;
            async fn get_scene(&self, event_id: StoryEventId) -> anyhow::Result<Option<SceneId>>;
            async fn remove_scene(&self, event_id: StoryEventId) -> anyhow::Result<bool>;
            async fn add_involved_character(&self, event_id: StoryEventId, involved: InvolvedCharacter) -> anyhow::Result<bool>;
            async fn get_involved_characters(&self, event_id: StoryEventId) -> anyhow::Result<Vec<InvolvedCharacter>>;
            async fn remove_involved_character(&self, event_id: StoryEventId, character_id: CharacterId) -> anyhow::Result<bool>;
            async fn set_triggered_by(&self, event_id: StoryEventId, narrative_event_id: NarrativeEventId) -> anyhow::Result<bool>;
            async fn get_triggered_by(&self, event_id: StoryEventId) -> anyhow::Result<Option<NarrativeEventId>>;
            async fn remove_triggered_by(&self, event_id: StoryEventId) -> anyhow::Result<bool>;
            async fn set_recorded_challenge(&self, event_id: StoryEventId, challenge_id: ChallengeId) -> anyhow::Result<bool>;
            async fn get_recorded_challenge(&self, event_id: StoryEventId) -> anyhow::Result<Option<ChallengeId>>;
            async fn remove_recorded_challenge(&self, event_id: StoryEventId) -> anyhow::Result<bool>;
        }

        #[async_trait]
        impl StoryEventQueryPort for StoryEventRepository {
            async fn list_by_world(&self, world_id: WorldId) -> anyhow::Result<Vec<StoryEvent>>;
            async fn list_by_world_paginated(&self, world_id: WorldId, limit: u32, offset: u32) -> anyhow::Result<Vec<StoryEvent>>;
            async fn list_visible(&self, world_id: WorldId, limit: u32) -> anyhow::Result<Vec<StoryEvent>>;
            async fn search_by_tags(&self, world_id: WorldId, tags: Vec<String>) -> anyhow::Result<Vec<StoryEvent>>;
            async fn search_by_text(&self, world_id: WorldId, search_text: &str) -> anyhow::Result<Vec<StoryEvent>>;
            async fn list_by_character(&self, character_id: CharacterId) -> anyhow::Result<Vec<StoryEvent>>;
            async fn list_by_location(&self, location_id: LocationId) -> anyhow::Result<Vec<StoryEvent>>;
            async fn list_by_narrative_event(&self, narrative_event_id: NarrativeEventId) -> anyhow::Result<Vec<StoryEvent>>;
            async fn list_by_challenge(&self, challenge_id: ChallengeId) -> anyhow::Result<Vec<StoryEvent>>;
            async fn list_by_scene(&self, scene_id: SceneId) -> anyhow::Result<Vec<StoryEvent>>;
        }

        #[async_trait]
        impl StoryEventDialoguePort for StoryEventRepository {
            async fn get_dialogues_with_npc(&self, world_id: WorldId, npc_id: CharacterId, limit: u32) -> anyhow::Result<Vec<StoryEvent>>;
            async fn update_spoke_to_edge(&self, pc_id: PlayerCharacterId, npc_id: CharacterId, topic: Option<String>) -> anyhow::Result<()>;
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockStoryEventRepository;
