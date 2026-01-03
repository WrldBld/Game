//! Narrative use cases - Event triggering and effect execution.

pub mod execute_effects;

pub use execute_effects::{
    EffectExecutionContext, EffectExecutionResult, EffectExecutionSummary, ExecuteEffects,
};

use std::sync::Arc;

/// Container for narrative-related use cases.
pub struct NarrativeUseCases {
    pub execute_effects: Arc<ExecuteEffects>,
}

impl NarrativeUseCases {
    pub fn new(execute_effects: Arc<ExecuteEffects>) -> Self {
        Self { execute_effects }
    }
}
