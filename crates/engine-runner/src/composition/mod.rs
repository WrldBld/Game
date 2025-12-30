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
//!
//! # Module Structure
//!
//! - `app_state`: Main AppState construction and wiring
//! - `factories`: Factory functions for creating port trait objects
//!   - `infrastructure`: Infrastructure services (clock, rng, Neo4j, Ollama, ComfyUI)
//!   - `repositories`: ISP-compliant repository ports
//!   - `event_infra`: Event bus and domain event infrastructure
//!   - `queue_services`: Queue backends and queue services
//!   - `core_services`: Core domain service ports
//!   - `asset_services`: Asset service ports
//!   - `game_services`: Game service ports (not yet wired)
//!   - `use_cases`: Use case construction with adapters
//! - `services`: Service container re-exports (deprecated)

pub mod app_state;
pub mod factories;
pub mod services;

pub use app_state::{new_adapter_state, AppStatePort};
