use serde::{Deserialize, Serialize};

use super::{CreateInteractionData, UpdateInteractionData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InteractionRequest {
    ListInteractions {
        scene_id: String,
    },
    GetInteraction {
        interaction_id: String,
    },
    CreateInteraction {
        scene_id: String,
        data: CreateInteractionData,
    },
    UpdateInteraction {
        interaction_id: String,
        data: UpdateInteractionData,
    },
    DeleteInteraction {
        interaction_id: String,
    },
    SetInteractionAvailability {
        interaction_id: String,
        available: bool,
    },
}
