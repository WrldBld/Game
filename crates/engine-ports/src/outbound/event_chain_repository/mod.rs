//! Split EventChain repository ports following Interface Segregation Principle.
//!
//! # Architecture
//!
//! The original `EventChainRepositoryPort` (17 methods) is split into 4 focused traits:
//!
//! 1. `EventChainCrudPort` - Core CRUD operations (4 methods)
//! 2. `EventChainQueryPort` - Query/lookup operations (4 methods)
//! 3. `EventChainMembershipPort` - Event membership management (3 methods)
//! 4. `EventChainStatePort` - Status and state management (6 methods)
//!
//! # Clean ISP Design
//!
//! Services should depend only on the traits they actually need:
//! - Services needing basic CRUD depend on `EventChainCrudPort`
//! - Services performing lookups depend on `EventChainQueryPort`
//! - Services managing event membership depend on `EventChainMembershipPort`
//! - Services managing chain state depend on `EventChainStatePort`
//!
//! The composition root passes the same concrete repository instance to each service,
//! and Rust coerces to the needed trait interface.

mod crud_port;
mod membership_port;
mod query_port;
mod state_port;

pub use crud_port::EventChainCrudPort;
pub use membership_port::EventChainMembershipPort;
pub use query_port::EventChainQueryPort;
pub use state_port::EventChainStatePort;

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use wrldbldr_domain::{ChainStatus, EventChain, EventChainId, NarrativeEventId, WorldId};

    mock! {
        /// Mock implementation of all EventChain repository traits for testing.
        pub EventChainRepository {}

        #[async_trait]
        impl EventChainCrudPort for EventChainRepository {
            async fn create(&self, chain: &EventChain) -> anyhow::Result<()>;
            async fn get(&self, id: EventChainId) -> anyhow::Result<Option<EventChain>>;
            async fn update(&self, chain: &EventChain) -> anyhow::Result<bool>;
            async fn delete(&self, id: EventChainId) -> anyhow::Result<bool>;
        }

        #[async_trait]
        impl EventChainQueryPort for EventChainRepository {
            async fn list_by_world(&self, world_id: WorldId) -> anyhow::Result<Vec<EventChain>>;
            async fn list_active(&self, world_id: WorldId) -> anyhow::Result<Vec<EventChain>>;
            async fn list_favorites(&self, world_id: WorldId) -> anyhow::Result<Vec<EventChain>>;
            async fn get_chains_for_event(&self, event_id: NarrativeEventId) -> anyhow::Result<Vec<EventChain>>;
        }

        #[async_trait]
        impl EventChainMembershipPort for EventChainRepository {
            async fn add_event_to_chain(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> anyhow::Result<bool>;
            async fn remove_event_from_chain(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> anyhow::Result<bool>;
            async fn complete_event(&self, chain_id: EventChainId, event_id: NarrativeEventId) -> anyhow::Result<bool>;
        }

        #[async_trait]
        impl EventChainStatePort for EventChainRepository {
            async fn toggle_favorite(&self, id: EventChainId) -> anyhow::Result<bool>;
            async fn set_active(&self, id: EventChainId, is_active: bool) -> anyhow::Result<bool>;
            async fn reset(&self, id: EventChainId) -> anyhow::Result<bool>;
            async fn get_status(&self, id: EventChainId) -> anyhow::Result<Option<ChainStatus>>;
            async fn list_statuses(&self, world_id: WorldId) -> anyhow::Result<Vec<ChainStatus>>;
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockEventChainRepository;
