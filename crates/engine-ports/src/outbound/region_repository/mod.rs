//! Split Region repository ports following Interface Segregation Principle.
//!
//! # Architecture
//!
//! The original `RegionRepositoryPort` (19 methods) is split into 5 focused traits:
//!
//! 1. `RegionCrudPort` - Core CRUD operations (5 methods)
//! 2. `RegionConnectionPort` - Region-to-region connections (4 methods)
//! 3. `RegionExitPort` - Region exits to other locations (3 methods)
//! 4. `RegionNpcPort` - NPC relationship queries (1 method)
//! 5. `RegionItemPort` - Item placement in regions (3 stub methods)
//!
//! # Clean ISP Design
//!
//! Services should depend only on the traits they actually need:
//! - Services needing only CRUD operations depend on `RegionCrudPort`
//! - Services managing intra-location navigation depend on `RegionConnectionPort`
//! - Services handling inter-location travel depend on `RegionExitPort`
//! - Services determining NPC presence depend on `RegionNpcPort`
//! - Services managing item placement depend on `RegionItemPort`
//!
//! The composition root passes the same concrete repository instance to each service,
//! and Rust coerces to the needed trait interface.

mod connection_port;
mod crud_port;
mod exit_port;
mod item_port;
mod npc_port;

pub use connection_port::RegionConnectionPort;
pub use crud_port::RegionCrudPort;
pub use exit_port::RegionExitPort;
pub use item_port::RegionItemPort;
pub use npc_port::RegionNpcPort;

#[cfg(any(test, feature = "testing"))]
pub use connection_port::MockRegionConnectionPort;
#[cfg(any(test, feature = "testing"))]
pub use crud_port::MockRegionCrudPort;
#[cfg(any(test, feature = "testing"))]
pub use exit_port::MockRegionExitPort;
#[cfg(any(test, feature = "testing"))]
pub use item_port::MockRegionItemPort;
#[cfg(any(test, feature = "testing"))]
pub use npc_port::MockRegionNpcPort;

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use wrldbldr_domain::ids::{ItemId, LocationId, RegionId, WorldId};
    use wrldbldr_domain::value_objects::RegionRelationshipType;
    use wrldbldr_domain::{Character, Item, Region, RegionConnection, RegionExit};

    mock! {
        /// Mock implementation of all Region repository traits for testing.
        ///
        /// This combined mock implements all 5 ISP traits, making it easy to use
        /// in tests where a single mock instance needs to satisfy multiple trait bounds.
        pub RegionRepository {}

        #[async_trait]
        impl RegionCrudPort for RegionRepository {
            async fn get(&self, id: RegionId) -> anyhow::Result<Option<Region>>;
            async fn update(&self, region: &Region) -> anyhow::Result<()>;
            async fn delete(&self, id: RegionId) -> anyhow::Result<()>;
            async fn list_by_location(&self, location_id: LocationId) -> anyhow::Result<Vec<Region>>;
            async fn list_spawn_points(&self, world_id: WorldId) -> anyhow::Result<Vec<Region>>;
        }

        #[async_trait]
        impl RegionConnectionPort for RegionRepository {
            async fn create_connection(&self, connection: &RegionConnection) -> anyhow::Result<()>;
            async fn get_connections(&self, region_id: RegionId) -> anyhow::Result<Vec<RegionConnection>>;
            async fn delete_connection(&self, from: RegionId, to: RegionId) -> anyhow::Result<()>;
            async fn unlock_connection(&self, from: RegionId, to: RegionId) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl RegionExitPort for RegionRepository {
            async fn create_exit(&self, exit: &RegionExit) -> anyhow::Result<()>;
            async fn get_exits(&self, region_id: RegionId) -> anyhow::Result<Vec<RegionExit>>;
            async fn delete_exit(&self, from_region: RegionId, to_location: LocationId) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl RegionNpcPort for RegionRepository {
            async fn get_npcs_related_to_region(
                &self,
                region_id: RegionId,
            ) -> anyhow::Result<Vec<(Character, RegionRelationshipType)>>;
        }

        #[async_trait]
        impl RegionItemPort for RegionRepository {
            async fn add_item_to_region(&self, region_id: RegionId, item_id: ItemId) -> anyhow::Result<()>;
            async fn get_region_items(&self, region_id: RegionId) -> anyhow::Result<Vec<Item>>;
            async fn remove_item_from_region(&self, region_id: RegionId, item_id: ItemId) -> anyhow::Result<()>;
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockRegionRepository;
