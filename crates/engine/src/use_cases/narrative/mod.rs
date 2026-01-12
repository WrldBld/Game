//! Narrative use cases - Event triggering and effect execution.

#[cfg(test)]
mod challenge_llm_tests;

#[cfg(test)]
mod llm_tool_tests;

pub mod chains;
pub mod decision;
pub mod execute_effects;
pub mod events;

pub use chains::{EventChainError, EventChainOps};
pub use decision::{NarrativeDecisionFlow, NarrativeDecisionOutcome, NarrativeTriggeredPayload};
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
    pub decision_flow: Arc<NarrativeDecisionFlow>,
}

impl NarrativeUseCases {
    pub fn new(
        execute_effects: Arc<ExecuteEffects>,
        events: Arc<NarrativeEventOps>,
        chains: Arc<EventChainOps>,
        decision_flow: Arc<NarrativeDecisionFlow>,
    ) -> Self {
        Self {
            execute_effects,
            events,
            chains,
            decision_flow,
        }
    }
}
