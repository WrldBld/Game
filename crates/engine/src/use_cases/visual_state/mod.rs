// Visual State use cases - methods for future state resolution
#![allow(dead_code)]

//! Visual State use cases - State resolution for locations and regions.
//!
//! This module handles determining which LocationState and RegionState
//! should be active based on current context and activation rules.

mod catalog;
mod resolve_state;

#[cfg(test)]
mod llm_condition_tests;

pub use catalog::{CatalogData, CatalogError, GeneratedVisualState, VisualStateCatalog, VisualStateDetails};
pub use resolve_state::{ResolveVisualState, StateResolutionContext};

use std::sync::Arc;

/// Container for visual state use cases.
pub struct VisualStateUseCases {
    pub resolve: Arc<ResolveVisualState>,
    pub catalog: Arc<VisualStateCatalog>,
}

impl VisualStateUseCases {
    pub fn new(resolve: Arc<ResolveVisualState>, catalog: Arc<VisualStateCatalog>) -> Self {
        Self { resolve, catalog }
    }
}
