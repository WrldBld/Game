//! Split Location repository ports following Interface Segregation Principle.
//!
//! # Architecture
//!
//! The original `LocationRepositoryPort` (21 methods) is split into 4 focused traits:
//!
//! 1. `LocationCrudPort` - Core CRUD operations (5 methods)
//! 2. `LocationHierarchyPort` - Parent-child relationships (4 methods)
//! 3. `LocationConnectionPort` - Navigation connections (5 methods)
//! 4. `LocationMapPort` - Grid maps and regions (5 methods)
//!
//! # Clean ISP Design
//!
//! Services should depend only on the traits they actually need:
//! - Services needing only CRUD operations depend on `LocationCrudPort`
//! - Services managing location hierarchy depend on `LocationHierarchyPort`
//! - Services handling navigation depend on `LocationConnectionPort`
//! - Services working with maps and regions depend on `LocationMapPort`
//!
//! The composition root passes the same concrete repository instance to each service,
//! and Rust coerces to the needed trait interface.

mod connection_port;
mod crud_port;
mod hierarchy_port;
mod map_port;

pub use connection_port::LocationConnectionPort;
pub use crud_port::LocationCrudPort;
pub use hierarchy_port::LocationHierarchyPort;
pub use map_port::LocationMapPort;

#[cfg(any(test, feature = "testing"))]
pub use connection_port::MockLocationConnectionPort;
#[cfg(any(test, feature = "testing"))]
pub use crud_port::MockLocationCrudPort;
#[cfg(any(test, feature = "testing"))]
pub use hierarchy_port::MockLocationHierarchyPort;
#[cfg(any(test, feature = "testing"))]
pub use map_port::MockLocationMapPort;

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use wrldbldr_domain::entities::{Location, LocationConnection, Region};
    use wrldbldr_domain::{GridMapId, LocationId, WorldId};

    mock! {
        /// Mock implementation of all Location repository traits for testing.
        ///
        /// This combined mock implements all 4 ISP traits, making it easy to use
        /// in tests where a single mock instance needs to satisfy multiple trait bounds.
        pub LocationRepository {}

        #[async_trait]
        impl LocationCrudPort for LocationRepository {
            async fn create(&self, location: &Location) -> anyhow::Result<()>;
            async fn get(&self, id: LocationId) -> anyhow::Result<Option<Location>>;
            async fn list(&self, world_id: WorldId) -> anyhow::Result<Vec<Location>>;
            async fn update(&self, location: &Location) -> anyhow::Result<()>;
            async fn delete(&self, id: LocationId) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl LocationHierarchyPort for LocationRepository {
            async fn set_parent(&self, child_id: LocationId, parent_id: LocationId) -> anyhow::Result<()>;
            async fn remove_parent(&self, child_id: LocationId) -> anyhow::Result<()>;
            async fn get_parent(&self, location_id: LocationId) -> anyhow::Result<Option<Location>>;
            async fn get_children(&self, location_id: LocationId) -> anyhow::Result<Vec<Location>>;
        }

        #[async_trait]
        impl LocationConnectionPort for LocationRepository {
            async fn create_connection(&self, connection: &LocationConnection) -> anyhow::Result<()>;
            async fn get_connections(&self, location_id: LocationId) -> anyhow::Result<Vec<LocationConnection>>;
            async fn update_connection(&self, connection: &LocationConnection) -> anyhow::Result<()>;
            async fn delete_connection(&self, from: LocationId, to: LocationId) -> anyhow::Result<()>;
            async fn unlock_connection(&self, from: LocationId, to: LocationId) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl LocationMapPort for LocationRepository {
            async fn set_grid_map(&self, location_id: LocationId, grid_map_id: GridMapId) -> anyhow::Result<()>;
            async fn remove_grid_map(&self, location_id: LocationId) -> anyhow::Result<()>;
            async fn get_grid_map_id(&self, location_id: LocationId) -> anyhow::Result<Option<GridMapId>>;
            async fn create_region(&self, location_id: LocationId, region: &Region) -> anyhow::Result<()>;
            async fn get_regions(&self, location_id: LocationId) -> anyhow::Result<Vec<Region>>;
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockLocationRepository;
