use serde::{Deserialize, Serialize};

use super::{CreateLocationConnectionData, CreateLocationData, UpdateLocationData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LocationRequest {
    ListLocations {
        world_id: String,
    },
    GetLocation {
        location_id: String,
    },
    CreateLocation {
        world_id: String,
        data: CreateLocationData,
    },
    UpdateLocation {
        location_id: String,
        data: UpdateLocationData,
    },
    DeleteLocation {
        location_id: String,
    },
    GetLocationConnections {
        location_id: String,
    },
    CreateLocationConnection {
        data: CreateLocationConnectionData,
    },
    DeleteLocationConnection {
        from_id: String,
        to_id: String,
    },
}
