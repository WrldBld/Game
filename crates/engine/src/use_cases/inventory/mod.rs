// Inventory use cases - methods for future item management
#![allow(dead_code)]

//! Inventory use cases.
//!
//! Handles item management operations including equipping, dropping, picking up,
//! and DM-level item placement.

mod create_and_place_item;
mod drop_item;
mod equip_item;
mod error;
mod give_item;
mod pickup_item;
mod place_item;
mod types;
mod unequip_item;

pub use create_and_place_item::CreateAndPlaceItem;
pub use drop_item::DropItem;
pub use equip_item::EquipItem;
pub use error::InventoryError;
pub use give_item::GiveItem;
pub use pickup_item::PickupItem;
pub use place_item::PlaceItemInRegion;
pub use types::InventoryActionResult;
pub use unequip_item::UnequipItem;

use std::sync::Arc;

use crate::infrastructure::ports::{ItemRepo, PlayerCharacterRepo};

/// Container for inventory use cases.
pub struct InventoryUseCases {
    pub equip_item: Arc<EquipItem>,
    pub unequip_item: Arc<UnequipItem>,
    pub drop_item: Arc<DropItem>,
    pub pickup_item: Arc<PickupItem>,
    pub give_item: Arc<GiveItem>,
    pub place_item_in_region: Arc<PlaceItemInRegion>,
    pub create_and_place_item: Arc<CreateAndPlaceItem>,
}

impl InventoryUseCases {
    pub fn new(item_repo: Arc<dyn ItemRepo>, pc_repo: Arc<dyn PlayerCharacterRepo>) -> Self {
        Self {
            equip_item: Arc::new(EquipItem::new(item_repo.clone(), pc_repo.clone())),
            unequip_item: Arc::new(UnequipItem::new(item_repo.clone(), pc_repo.clone())),
            drop_item: Arc::new(DropItem::new(item_repo.clone(), pc_repo.clone())),
            pickup_item: Arc::new(PickupItem::new(item_repo.clone(), pc_repo.clone())),
            give_item: Arc::new(GiveItem::new(item_repo.clone(), pc_repo.clone())),
            place_item_in_region: Arc::new(PlaceItemInRegion::new(item_repo.clone())),
            create_and_place_item: Arc::new(CreateAndPlaceItem::new(item_repo)),
        }
    }
}
