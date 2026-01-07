use serde::{Deserialize, Serialize};

use super::{CreateNarrativeEventData, UpdateNarrativeEventData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NarrativeEventRequest {
    ListNarrativeEvents {
        world_id: String,
    },
    GetNarrativeEvent {
        event_id: String,
    },
    CreateNarrativeEvent {
        world_id: String,
        data: CreateNarrativeEventData,
    },
    UpdateNarrativeEvent {
        event_id: String,
        data: UpdateNarrativeEventData,
    },
    DeleteNarrativeEvent {
        event_id: String,
    },
    SetNarrativeEventActive {
        event_id: String,
        active: bool,
    },
    SetNarrativeEventFavorite {
        event_id: String,
        favorite: bool,
    },
    TriggerNarrativeEvent {
        event_id: String,
    },
    ResetNarrativeEvent {
        event_id: String,
    },
    GetTriggerSchema,
}
