use serde::{Deserialize, Serialize};

use super::CreateActData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActRequest {
    ListActs {
        world_id: String,
    },
    CreateAct {
        world_id: String,
        data: CreateActData,
    },
}
