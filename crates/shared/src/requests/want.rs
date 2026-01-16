use serde::{Deserialize, Serialize};

use crate::messages::{CreateWantData, UpdateWantData, WantTargetTypeData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WantRequest {
    ListWants {
        character_id: String,
    },
    GetWant {
        want_id: String,
    },
    CreateWant {
        character_id: String,
        data: CreateWantData,
    },
    UpdateWant {
        want_id: String,
        data: UpdateWantData,
    },
    DeleteWant {
        want_id: String,
    },
    SetWantTarget {
        want_id: String,
        target_id: String,
        target_type: WantTargetTypeData,
    },
    RemoveWantTarget {
        want_id: String,
    },
}
