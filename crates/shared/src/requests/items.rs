use serde::{Deserialize, Serialize};

use super::CreateItemData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ItemsRequest {
    PlaceItemInRegion {
        region_id: String,
        item_id: String,
    },
    CreateAndPlaceItem {
        world_id: String,
        region_id: String,
        data: CreateItemData,
    },
}
