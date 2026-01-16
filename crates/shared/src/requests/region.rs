use serde::{Deserialize, Serialize};

use super::{CreateRegionConnectionData, CreateRegionData, UpdateRegionData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegionRequest {
    ListRegions {
        location_id: String,
    },
    GetRegion {
        region_id: String,
    },
    CreateRegion {
        location_id: String,
        data: CreateRegionData,
    },
    UpdateRegion {
        region_id: String,
        data: UpdateRegionData,
    },
    DeleteRegion {
        region_id: String,
    },

    GetRegionConnections {
        region_id: String,
    },
    CreateRegionConnection {
        from_id: String,
        to_id: String,
        data: CreateRegionConnectionData,
    },
    DeleteRegionConnection {
        from_id: String,
        to_id: String,
    },
    UnlockRegionConnection {
        from_id: String,
        to_id: String,
    },

    GetRegionExits {
        region_id: String,
    },
    CreateRegionExit {
        region_id: String,
        location_id: String,
        arrival_region_id: String,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        bidirectional: Option<bool>,
    },
    DeleteRegionExit {
        region_id: String,
        location_id: String,
    },

    ListSpawnPoints {
        world_id: String,
    },
}
