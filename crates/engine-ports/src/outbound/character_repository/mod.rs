//! Split Character repository ports following Interface Segregation Principle.
//!
//! # Architecture
//!
//! The original `CharacterRepositoryPort` (42 methods) is split into 6 focused traits:
//!
//! 1. `CharacterCrudPort` - Core CRUD operations (6 methods)
//! 2. `CharacterWantPort` - Want management (7 methods)
//! 3. `CharacterActantialPort` - Actantial view management (5 methods)
//! 4. `CharacterInventoryPort` - Inventory management (5 methods)
//! 5. `CharacterLocationPort` - Location relationships (13 methods)
//! 6. `CharacterDispositionPort` - NPC disposition tracking (6 methods)
//!
//! # Clean ISP Design
//!
//! Services should depend only on the traits they actually need:
//! - Services needing only CRUD operations depend on `CharacterCrudPort`
//! - Services managing wants depend on `CharacterWantPort`
//! - Services building actantial context depend on `CharacterActantialPort`
//! - Services handling inventory depend on `CharacterInventoryPort`
//! - Services managing location relationships depend on `CharacterLocationPort`
//! - Services tracking NPC dispositions depend on `CharacterDispositionPort`
//!
//! The composition root passes the same concrete repository instance to each service,
//! and Rust coerces to the needed trait interface.

mod actantial_port;
mod crud_port;
mod disposition_port;
mod inventory_port;
mod location_port;
mod want_port;

pub use actantial_port::CharacterActantialPort;
pub use crud_port::CharacterCrudPort;
pub use disposition_port::CharacterDispositionPort;
pub use inventory_port::CharacterInventoryPort;
pub use location_port::CharacterLocationPort;
pub use want_port::CharacterWantPort;

#[cfg(any(test, feature = "testing"))]
pub use actantial_port::MockCharacterActantialPort;
#[cfg(any(test, feature = "testing"))]
pub use crud_port::MockCharacterCrudPort;
#[cfg(any(test, feature = "testing"))]
pub use disposition_port::MockCharacterDispositionPort;
#[cfg(any(test, feature = "testing"))]
pub use inventory_port::MockCharacterInventoryPort;
#[cfg(any(test, feature = "testing"))]
pub use location_port::MockCharacterLocationPort;
#[cfg(any(test, feature = "testing"))]
pub use want_port::MockCharacterWantPort;

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use wrldbldr_domain::value_objects::{
        ActantialTarget, DispositionLevel, NpcDispositionState, RegionRelationship, RegionShift,
        WantTarget,
    };
    use wrldbldr_domain::{
        AcquisitionMethod, ActantialRole, ActantialView, Character, CharacterId, CharacterWant,
        FrequencyLevel, InventoryItem, ItemId, LocationId, PlayerCharacterId, RegionId, SceneId,
        Want, WantId, WorldId,
    };

    mock! {
        /// Mock implementation of all Character repository traits for testing.
        ///
        /// This combined mock implements all 6 ISP traits, making it easy to use
        /// in tests where a single mock instance needs to satisfy multiple trait bounds.
        pub CharacterRepository {}

        #[async_trait]
        impl CharacterCrudPort for CharacterRepository {
            async fn create(&self, character: &Character) -> anyhow::Result<()>;
            async fn get(&self, id: CharacterId) -> anyhow::Result<Option<Character>>;
            async fn list(&self, world_id: WorldId) -> anyhow::Result<Vec<Character>>;
            async fn update(&self, character: &Character) -> anyhow::Result<()>;
            async fn delete(&self, id: CharacterId) -> anyhow::Result<()>;
            async fn get_by_scene(&self, scene_id: SceneId) -> anyhow::Result<Vec<Character>>;
        }

        #[async_trait]
        impl CharacterWantPort for CharacterRepository {
            async fn create_want(&self, character_id: CharacterId, want: &Want, priority: u32) -> anyhow::Result<()>;
            async fn get_wants(&self, character_id: CharacterId) -> anyhow::Result<Vec<CharacterWant>>;
            async fn update_want(&self, want: &Want) -> anyhow::Result<()>;
            async fn delete_want(&self, want_id: WantId) -> anyhow::Result<()>;
            async fn set_want_target(&self, want_id: WantId, target_id: String, target_type: String) -> anyhow::Result<()>;
            async fn remove_want_target(&self, want_id: WantId) -> anyhow::Result<()>;
            async fn get_want_target(&self, want_id: WantId) -> anyhow::Result<Option<WantTarget>>;
        }

        #[async_trait]
        impl CharacterActantialPort for CharacterRepository {
            async fn add_actantial_view(&self, subject_id: CharacterId, role: ActantialRole, target_id: CharacterId, view: &ActantialView) -> anyhow::Result<()>;
            async fn add_actantial_view_to_pc(&self, subject_id: CharacterId, role: ActantialRole, target_id: PlayerCharacterId, view: &ActantialView) -> anyhow::Result<()>;
            async fn get_actantial_views(&self, character_id: CharacterId) -> anyhow::Result<Vec<(ActantialRole, ActantialTarget, ActantialView)>>;
            async fn remove_actantial_view(&self, subject_id: CharacterId, role: ActantialRole, target_id: CharacterId, want_id: WantId) -> anyhow::Result<()>;
            async fn remove_actantial_view_to_pc(&self, subject_id: CharacterId, role: ActantialRole, target_id: PlayerCharacterId, want_id: WantId) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl CharacterInventoryPort for CharacterRepository {
            async fn add_inventory_item(&self, character_id: CharacterId, item_id: ItemId, quantity: u32, equipped: bool, acquisition_method: Option<AcquisitionMethod>) -> anyhow::Result<()>;
            async fn get_inventory(&self, character_id: CharacterId) -> anyhow::Result<Vec<InventoryItem>>;
            async fn get_inventory_item(&self, character_id: CharacterId, item_id: ItemId) -> anyhow::Result<Option<InventoryItem>>;
            async fn update_inventory_item(&self, character_id: CharacterId, item_id: ItemId, quantity: u32, equipped: bool) -> anyhow::Result<()>;
            async fn remove_inventory_item(&self, character_id: CharacterId, item_id: ItemId) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl CharacterLocationPort for CharacterRepository {
            async fn set_home_location(&self, character_id: CharacterId, location_id: LocationId, description: Option<String>) -> anyhow::Result<()>;
            async fn remove_home_location(&self, character_id: CharacterId) -> anyhow::Result<()>;
            async fn set_work_location(&self, character_id: CharacterId, location_id: LocationId, role: String, schedule: Option<String>) -> anyhow::Result<()>;
            async fn remove_work_location(&self, character_id: CharacterId) -> anyhow::Result<()>;
            async fn add_frequented_location(&self, character_id: CharacterId, location_id: LocationId, frequency: FrequencyLevel, time_of_day: String, day_of_week: Option<String>, reason: Option<String>) -> anyhow::Result<()>;
            async fn remove_frequented_location(&self, character_id: CharacterId, location_id: LocationId) -> anyhow::Result<()>;
            async fn add_avoided_location(&self, character_id: CharacterId, location_id: LocationId, reason: String) -> anyhow::Result<()>;
            async fn remove_avoided_location(&self, character_id: CharacterId, location_id: LocationId) -> anyhow::Result<()>;
            async fn get_npcs_at_location(&self, location_id: LocationId, time_of_day: Option<String>) -> anyhow::Result<Vec<Character>>;
            async fn get_region_relationships(&self, character_id: CharacterId) -> anyhow::Result<Vec<RegionRelationship>>;
            async fn set_home_region(&self, character_id: CharacterId, region_id: RegionId) -> anyhow::Result<()>;
            async fn set_work_region(&self, character_id: CharacterId, region_id: RegionId, shift: RegionShift) -> anyhow::Result<()>;
            async fn remove_region_relationship(&self, character_id: CharacterId, region_id: RegionId, relationship_type: String) -> anyhow::Result<()>;
        }

        #[async_trait]
        impl CharacterDispositionPort for CharacterRepository {
            async fn get_disposition_toward_pc(&self, npc_id: CharacterId, pc_id: PlayerCharacterId) -> anyhow::Result<Option<NpcDispositionState>>;
            async fn set_disposition_toward_pc(&self, disposition_state: &NpcDispositionState) -> anyhow::Result<()>;
            async fn get_scene_dispositions(&self, npc_ids: &[CharacterId], pc_id: PlayerCharacterId) -> anyhow::Result<Vec<NpcDispositionState>>;
            async fn get_all_npc_dispositions_for_pc(&self, pc_id: PlayerCharacterId) -> anyhow::Result<Vec<NpcDispositionState>>;
            async fn get_default_disposition(&self, npc_id: CharacterId) -> anyhow::Result<DispositionLevel>;
            async fn set_default_disposition(&self, npc_id: CharacterId, disposition: DispositionLevel) -> anyhow::Result<()>;
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockCharacterRepository;
