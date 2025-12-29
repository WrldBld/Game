//! Composition Root Module
//!
//! This module contains the dependency injection and service composition logic
//! for the engine. It is responsible for:
//! - Creating and configuring all adapters
//! - Wiring adapters to ports
//! - Building the AppState with all required services
//!
//! The composition root follows the hexagonal architecture pattern where
//! all dependencies flow inward and are assembled here at the application boundary.

pub mod app_state;
pub mod services;

pub use app_state::{new_adapter_state, AdapterState, WorkerServices};
// Re-export composition-layer AppState for external consumers
#[allow(unused_imports)]
pub use wrldbldr_engine_composition::AppState;
