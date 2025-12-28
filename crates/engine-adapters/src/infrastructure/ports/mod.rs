//! Port Adapters - Infrastructure implementations of use case ports
//!
//! This module contains adapters that implement the port traits defined in use cases.
//! Each adapter wraps an existing infrastructure component (WorldStateManager,
//! StagingService, etc.) and implements the port interface expected by use cases.
//!
//! # Architecture
//!
//! ```text
//! Use Case Layer (engine-app)
//!     │
//!     ├── StagingStatePort (trait)
//!     ├── StagingServicePort (trait)
//!     └── ConnectionManagerPort (trait)
//!
//! Adapter Layer (this module)
//!     │
//!     ├── StagingStateAdapter ─────────► WorldStateManager
//!     ├── StagingServiceAdapter ───────► StagingService
//!     └── ConnectionManagerAdapter ────► WorldConnectionManager
//! ```
//!
//! # Usage
//!
//! These adapters are constructed in `UseCases::new()` and passed to use cases.
//! See `state/use_cases.rs` for wiring.
//!
//! # Implementation Status
//!
//! - [x] StagingStateAdapter - For MovementUseCase
//! - [x] StagingServiceAdapter - For MovementUseCase
//! - [x] ConnectionManagerAdapter - For ConnectionUseCase (not yet wired)
//! - [ ] ChallengeAdapters - TODO: Fix generic type bounds
//! - [ ] SceneAdapters - TODO: Fix missing types
//! - [ ] ObservationAdapters - TODO: Fix missing types
//! - [ ] PlayerActionAdapters - TODO: Fix generic type bounds

mod staging_state_adapter;
mod staging_service_adapter;
mod connection_manager_adapter;

// TODO: These adapters need generic type fixes before they compile.
// Uncomment and fix when wiring the corresponding use cases.
// mod challenge_adapters;
// mod scene_adapters;
// mod observation_adapters;
mod player_action_adapters;

pub use staging_state_adapter::*;
pub use staging_service_adapter::*;
pub use connection_manager_adapter::*;

// pub use challenge_adapters::*;
// pub use scene_adapters::*;
// pub use observation_adapters::*;
pub use player_action_adapters::*;
