use serde::{Deserialize, Serialize};

use super::{CreateDmMarkerData, UpdateStoryEventData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StoryEventRequest {
    ListStoryEvents {
        world_id: String,
        #[serde(default)]
        page: Option<u32>,
        #[serde(default)]
        page_size: Option<u32>,
    },
    GetStoryEvent {
        event_id: String,
    },
    CreateDmMarker {
        world_id: String,
        data: CreateDmMarkerData,
    },
    UpdateStoryEvent {
        event_id: String,
        data: UpdateStoryEventData,
    },
    SetStoryEventVisibility {
        event_id: String,
        visible: bool,
    },
}
