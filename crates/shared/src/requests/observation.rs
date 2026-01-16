use serde::{Deserialize, Serialize};

use super::CreateObservationData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ObservationRequest {
    ListObservations {
        pc_id: String,
    },
    CreateObservation {
        pc_id: String,
        data: CreateObservationData,
    },
    DeleteObservation {
        pc_id: String,
        npc_id: String,
    },
}
