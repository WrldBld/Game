//! Narrative use cases - Event triggering and effect execution.

pub mod chains;
pub mod execute_effects;
pub mod events;

pub use chains::{EventChainError, EventChainOps};
pub use execute_effects::{
    EffectExecutionContext, EffectExecutionResult, EffectExecutionSummary, ExecuteEffects,
};
pub use events::{NarrativeEventError, NarrativeEventOps, TriggeredNarrativeEvent};

use std::sync::Arc;

/// Container for narrative-related use cases.
pub struct NarrativeUseCases {
    pub execute_effects: Arc<ExecuteEffects>,
    pub events: Arc<NarrativeEventOps>,
    pub chains: Arc<EventChainOps>,
}

impl NarrativeUseCases {
    pub fn new(
        execute_effects: Arc<ExecuteEffects>,
        events: Arc<NarrativeEventOps>,
        chains: Arc<EventChainOps>,
    ) -> Self {
        Self {
            execute_effects,
            events,
            chains,
        }
    }
}
