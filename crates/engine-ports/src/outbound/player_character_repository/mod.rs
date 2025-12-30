//! Split PlayerCharacter repository ports following Interface Segregation Principle.
//!
//! # Architecture
//!
//! The original `PlayerCharacterRepositoryPort` (16 methods) is split into 4 focused traits:
//!
//! 1. `PlayerCharacterCrudPort` - Core CRUD operations (5 methods)
//! 2. `PlayerCharacterQueryPort` - Query/lookup operations (4 methods)
//! 3. `PlayerCharacterPositionPort` - Position/movement operations (3 methods)
//! 4. `PlayerCharacterInventoryPort` - Inventory management (5 methods)
//!
//! # Clean ISP Design
//!
//! Services should depend only on the traits they actually need:
//! - Services needing only basic CRUD depend on `PlayerCharacterCrudPort`
//! - Services performing lookups depend on `PlayerCharacterQueryPort`
//! - Movement/navigation services depend on `PlayerCharacterPositionPort`
//! - Inventory management depends on `PlayerCharacterInventoryPort`
//!
//! The composition root passes the same concrete repository instance to each service,
//! and Rust coerces to the needed trait interface.

mod crud_port;
mod inventory_port;
mod position_port;
mod query_port;

pub use crud_port::PlayerCharacterCrudPort;
pub use inventory_port::PlayerCharacterInventoryPort;
pub use position_port::PlayerCharacterPositionPort;
pub use query_port::PlayerCharacterQueryPort;

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use wrldbldr_domain::{
        AcquisitionMethod, InventoryItem, ItemId, LocationId, PlayerCharacter, PlayerCharacterId,
        RegionId, WorldId,
    };

    mock! {
        /// Mock implementation of all PlayerCharacter repository traits for testing.
        pub PlayerCharacterRepository {}

        #[async_trait]
        impl PlayerCharacterCrudPort for PlayerCharacterRepository {
            async fn create(&self, pc: &PlayerCharacter) -> anyhow::Result<()>;
            async fn get(&self, id: PlayerCharacterId) -> anyhow::Result<Option<PlayerCharacter>>;
            async fn update(&self, pc: &PlayerCharacter) -> anyhow::Result<()>;
            async fn delete(&self, id: PlayerCharacterId) -> anyhow::Result<()>;
            async fn unbind_from_session(&self, id: PlayerCharacterId) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl PlayerCharacterQueryPort for PlayerCharacterRepository {
            async fn get_by_location(&self, location_id: LocationId) -> anyhow::Result<Vec<PlayerCharacter>>;
            async fn get_by_user_and_world(&self, user_id: &str, world_id: WorldId) -> anyhow::Result<Vec<PlayerCharacter>>;
            async fn get_all_by_world(&self, world_id: WorldId) -> anyhow::Result<Vec<PlayerCharacter>>;
            async fn get_unbound_by_user(&self, user_id: &str) -> anyhow::Result<Vec<PlayerCharacter>>;
        }

        #[async_trait]
        impl PlayerCharacterPositionPort for PlayerCharacterRepository {
            async fn update_location(&self, id: PlayerCharacterId, location_id: LocationId) -> anyhow::Result<()>;
            async fn update_region(&self, id: PlayerCharacterId, region_id: RegionId) -> anyhow::Result<()>;
            async fn update_position(&self, id: PlayerCharacterId, location_id: LocationId, region_id: Option<RegionId>) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl PlayerCharacterInventoryPort for PlayerCharacterRepository {
            async fn add_inventory_item(
                &self,
                pc_id: PlayerCharacterId,
                item_id: ItemId,
                quantity: u32,
                is_equipped: bool,
                acquisition_method: Option<AcquisitionMethod>,
            ) -> anyhow::Result<()>;
            async fn get_inventory(&self, pc_id: PlayerCharacterId) -> anyhow::Result<Vec<InventoryItem>>;
            async fn get_inventory_item(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> anyhow::Result<Option<InventoryItem>>;
            async fn update_inventory_item(&self, pc_id: PlayerCharacterId, item_id: ItemId, quantity: u32, is_equipped: bool) -> anyhow::Result<()>;
            async fn remove_inventory_item(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> anyhow::Result<()>;
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockPlayerCharacterRepository;
