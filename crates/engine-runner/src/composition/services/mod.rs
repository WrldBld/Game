//! Service Composition Modules
//!
//! This module re-exports the service group types from engine-adapters.
//! The actual service group implementations live in engine-adapters/infrastructure/state.
//!
//! Note: In the future, these may be moved here to the composition root,
//! but for now they remain in engine-adapters since the handlers there
//! need access to AppState fields.

// Re-export service groups from engine-adapters
// Currently unused in this crate but kept for potential future use
#[allow(unused_imports)]
pub use wrldbldr_engine_adapters::infrastructure::state::{
    AssetServices, CoreServices, EventInfrastructure, GameServices, PlayerServices, QueueServices,
    UseCases,
};
