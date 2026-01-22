//! NarrativeEvent mutation outcomes.

use crate::aggregates::narrative_event::{EventActivation, FavoriteStatus};
use crate::entities::{EventOutcome, NarrativeTrigger};
use crate::value_objects::NarrativeEventName;

/// Outcome of updating narrative event fields or state.
#[derive(Debug, Clone)]
pub enum NarrativeEventUpdate {
    NameChanged {
        from: NarrativeEventName,
        to: NarrativeEventName,
    },
    DescriptionChanged {
        from: String,
        to: String,
    },
    SceneDirectionChanged {
        from: String,
        to: String,
    },
    TriggerConditionsUpdated {
        from: Vec<NarrativeTrigger>,
        to: Vec<NarrativeTrigger>,
    },
    OutcomesUpdated {
        from: Vec<EventOutcome>,
        to: Vec<EventOutcome>,
    },
    ActivationChanged {
        from: EventActivation,
        to: EventActivation,
    },
    PriorityChanged {
        from: i32,
        to: i32,
    },
    FavoriteChanged {
        from: FavoriteStatus,
        to: FavoriteStatus,
    },
    Triggered {
        outcome: Option<String>,
        trigger_count: u32,
        active: bool,
    },
    Reset {
        trigger_count: u32,
    },
}
