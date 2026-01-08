use serde::{Deserialize, Serialize};

use super::{CreateWorldData, UpdateWorldData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorldRequest {
    ListWorlds,
    GetWorld {
        world_id: String,
    },
    CreateWorld {
        data: CreateWorldData,
    },
    UpdateWorld {
        world_id: String,
        data: UpdateWorldData,
    },
    DeleteWorld {
        world_id: String,
    },
    ExportWorld {
        world_id: String,
    },
    GetSheetTemplate {
        world_id: String,
    },
}
