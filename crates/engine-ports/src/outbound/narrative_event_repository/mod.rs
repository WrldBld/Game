//! Split NarrativeEvent repository ports following Interface Segregation Principle.
//!
//! # Architecture
//!
//! The original `NarrativeEventRepositoryPort` (30 methods) is split into 4 focused traits:
//!
//! 1. `NarrativeEventCrudPort` - Core CRUD + state management (12 methods)
//! 2. `NarrativeEventTiePort` - Scene/Location/Act relationships (9 methods)
//! 3. `NarrativeEventNpcPort` - Featured NPC management (5 methods)
//! 4. `NarrativeEventQueryPort` - Query by relationships (4 methods)
//!
//! # Backward Compatibility
//!
//! `NarrativeEventRepositoryPort` is retained as a super-trait that extends all four.
//! Existing code using `Arc<dyn NarrativeEventRepositoryPort>` continues to work.
//!
//! # Migration Path
//!
//! 1. New code can depend on specific smaller traits (e.g., `NarrativeEventCrudPort`)
//! 2. When migrating, change `NarrativeEventRepositoryPort` to specific trait bounds
//! 3. Eventually, services can accept only the traits they need

mod crud_port;
mod npc_port;
mod query_port;
mod tie_port;

pub use crud_port::NarrativeEventCrudPort;
pub use npc_port::NarrativeEventNpcPort;
pub use query_port::NarrativeEventQueryPort;
pub use tie_port::NarrativeEventTiePort;

use async_trait::async_trait;

/// Backward-compatible super-trait combining all NarrativeEvent repository capabilities.
///
/// This trait exists for compatibility with code that depends on the full repository interface.
/// New code should prefer depending on specific smaller traits (e.g., `NarrativeEventCrudPort`).
#[async_trait]
pub trait NarrativeEventRepositoryPort:
    NarrativeEventCrudPort + NarrativeEventTiePort + NarrativeEventNpcPort + NarrativeEventQueryPort
{
}

// Blanket implementation: anything that implements all sub-traits is a NarrativeEventRepositoryPort
impl<T> NarrativeEventRepositoryPort for T where
    T: NarrativeEventCrudPort + NarrativeEventTiePort + NarrativeEventNpcPort + NarrativeEventQueryPort
{
}

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use mockall::mock;
    use wrldbldr_domain::{
        ActId, CharacterId, EventChainMembership, FeaturedNpc, LocationId, NarrativeEvent,
        NarrativeEventId, SceneId, WorldId,
    };

    mock! {
        /// Mock implementation of all NarrativeEvent repository traits for testing.
        pub NarrativeEventRepository {}

        #[async_trait]
        impl NarrativeEventCrudPort for NarrativeEventRepository {
            async fn create(&self, event: &NarrativeEvent) -> anyhow::Result<()>;
            async fn get(&self, id: NarrativeEventId) -> anyhow::Result<Option<NarrativeEvent>>;
            async fn update(&self, event: &NarrativeEvent) -> anyhow::Result<bool>;
            async fn list_by_world(&self, world_id: WorldId) -> anyhow::Result<Vec<NarrativeEvent>>;
            async fn list_active(&self, world_id: WorldId) -> anyhow::Result<Vec<NarrativeEvent>>;
            async fn list_favorites(&self, world_id: WorldId) -> anyhow::Result<Vec<NarrativeEvent>>;
            async fn list_pending(&self, world_id: WorldId) -> anyhow::Result<Vec<NarrativeEvent>>;
            async fn toggle_favorite(&self, id: NarrativeEventId) -> anyhow::Result<bool>;
            async fn set_active(&self, id: NarrativeEventId, is_active: bool) -> anyhow::Result<bool>;
            async fn mark_triggered(&self, id: NarrativeEventId, outcome_name: Option<String>) -> anyhow::Result<bool>;
            async fn reset_triggered(&self, id: NarrativeEventId) -> anyhow::Result<bool>;
            async fn delete(&self, id: NarrativeEventId) -> anyhow::Result<bool>;
        }

        #[async_trait]
        impl NarrativeEventTiePort for NarrativeEventRepository {
            async fn tie_to_scene(&self, event_id: NarrativeEventId, scene_id: SceneId) -> anyhow::Result<bool>;
            async fn get_tied_scene(&self, event_id: NarrativeEventId) -> anyhow::Result<Option<SceneId>>;
            async fn untie_from_scene(&self, event_id: NarrativeEventId) -> anyhow::Result<bool>;
            async fn tie_to_location(&self, event_id: NarrativeEventId, location_id: LocationId) -> anyhow::Result<bool>;
            async fn get_tied_location(&self, event_id: NarrativeEventId) -> anyhow::Result<Option<LocationId>>;
            async fn untie_from_location(&self, event_id: NarrativeEventId) -> anyhow::Result<bool>;
            async fn assign_to_act(&self, event_id: NarrativeEventId, act_id: ActId) -> anyhow::Result<bool>;
            async fn get_act(&self, event_id: NarrativeEventId) -> anyhow::Result<Option<ActId>>;
            async fn unassign_from_act(&self, event_id: NarrativeEventId) -> anyhow::Result<bool>;
        }

        #[async_trait]
        impl NarrativeEventNpcPort for NarrativeEventRepository {
            async fn add_featured_npc(&self, event_id: NarrativeEventId, featured_npc: FeaturedNpc) -> anyhow::Result<bool>;
            async fn get_featured_npcs(&self, event_id: NarrativeEventId) -> anyhow::Result<Vec<FeaturedNpc>>;
            async fn remove_featured_npc(&self, event_id: NarrativeEventId, character_id: CharacterId) -> anyhow::Result<bool>;
            async fn update_featured_npc_role(&self, event_id: NarrativeEventId, character_id: CharacterId, role: Option<String>) -> anyhow::Result<bool>;
            async fn get_chain_memberships(&self, event_id: NarrativeEventId) -> anyhow::Result<Vec<EventChainMembership>>;
        }

        #[async_trait]
        impl NarrativeEventQueryPort for NarrativeEventRepository {
            async fn list_by_scene(&self, scene_id: SceneId) -> anyhow::Result<Vec<NarrativeEvent>>;
            async fn list_by_location(&self, location_id: LocationId) -> anyhow::Result<Vec<NarrativeEvent>>;
            async fn list_by_act(&self, act_id: ActId) -> anyhow::Result<Vec<NarrativeEvent>>;
            async fn list_by_featured_npc(&self, character_id: CharacterId) -> anyhow::Result<Vec<NarrativeEvent>>;
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockNarrativeEventRepository;
