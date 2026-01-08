//! Visual State use cases - State resolution for locations and regions.
//!
//! This module handles determining which LocationState and RegionState
//! should be active based on current context and activation rules.

mod resolve_state;

pub use resolve_state::{
    ResolveVisualState, ResolvedStateInfo, SoftRuleContext, StateResolutionContext,
    StateResolutionResult,
};

use std::sync::Arc;

/// Container for visual state use cases.
pub struct VisualStateUseCases {
    pub resolve: Arc<ResolveVisualState>,
}

impl VisualStateUseCases {
    pub fn new(resolve: Arc<ResolveVisualState>) -> Self {
        Self { resolve }
    }
}
