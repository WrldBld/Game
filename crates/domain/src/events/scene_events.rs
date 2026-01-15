//! Scene mutation outcomes.

use crate::entities::{SceneCondition, TimeContext};
use crate::value_objects::SceneName;
use crate::{CharacterId, LocationId};

/// Outcome of updating scene fields or state.
#[derive(Debug, Clone)]
pub enum SceneUpdate {
    NameChanged { from: SceneName, to: SceneName },
    LocationChanged { from: LocationId, to: LocationId },
    TimeContextChanged { from: TimeContext, to: TimeContext },
    BackdropOverrideChanged {
        from: Option<String>,
        to: Option<String>,
    },
    DirectorialNotesChanged { from: String, to: String },
    OrderChanged { from: u32, to: u32 },
    FeaturedCharacterAdded { character_id: CharacterId },
    FeaturedCharacterAlreadyPresent { character_id: CharacterId },
    FeaturedCharacterRemoved { character_id: CharacterId },
    FeaturedCharacterNotPresent { character_id: CharacterId },
    EntryConditionAdded { condition: SceneCondition },
    EntryConditionsCleared { previous_count: usize },
}
