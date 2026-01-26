// Narrative use cases - fields for future event features
#![allow(dead_code)]

//! Narrative use cases - Event triggering and effect execution.

#[cfg(test)]
mod challenge_llm_tests;

#[cfg(test)]
mod llm_tool_tests;

#[cfg(test)]
mod trigger_integration_tests;

pub mod chains;
pub mod decision;
pub mod events;
pub mod execute_effects;

pub use chains::{CreateEventChainInput, EventChainError, EventChainOps, UpdateEventChainInput};
pub use decision::NarrativeDecisionFlow;
pub use events::{NarrativeEventError, NarrativeEventOps};
pub use execute_effects::{EffectExecutionContext, EffectExecutionSummary, ExecuteEffects};

use std::sync::Arc;

/// Container for narrative-related use cases.
#[derive(Clone)]
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
