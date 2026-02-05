// Visual State use cases - methods for future state resolution
#![allow(dead_code)]

//! Visual State use cases - State resolution for locations and regions.
//!
//! This module handles determining which LocationState and RegionState
//! should be active based on current context and activation rules.

mod catalog;
mod generation;
mod resolve_state;
mod validation;

#[cfg(test)]
mod llm_condition_tests;

pub use catalog::{CatalogError, VisualStateCatalog, VisualStateDetails};
pub use generation::{GenerateVisualState, GenerateVisualStateInput, GenerateVisualStateTarget, GenerationError};
pub use resolve_state::{
    ResolveVisualState, StateResolutionContext,
};
pub use validation::{
    validate_asset_path, validate_generation_prompt, validate_negative_prompt,
    validate_state_description, validate_state_name, validate_state_name_required,
    validate_workflow_id, ValidationResult,
};

// Re-export MAX_STATE_NAME as a pub const
pub const MAX_STATE_NAME: usize = validation::MAX_STATE_NAME;

use std::sync::Arc;

/// Container for visual state use cases.
pub struct VisualStateUseCases {
    pub resolve: Arc<ResolveVisualState>,
    pub catalog: Arc<VisualStateCatalog>,
    pub generation: Arc<GenerateVisualState>,
}

impl VisualStateUseCases {
    pub fn new(
        resolve: Arc<ResolveVisualState>,
        catalog: Arc<VisualStateCatalog>,
        generation: Arc<GenerateVisualState>,
    ) -> Self {
        Self {
            resolve,
            catalog,
            generation,
        }
    }
}
