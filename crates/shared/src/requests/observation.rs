use serde::{Deserialize, Serialize};

use super::CreateObservationData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
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
