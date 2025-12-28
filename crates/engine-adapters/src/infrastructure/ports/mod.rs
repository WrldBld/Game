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
//!     ├── ConnectionManagerPort (trait)
//!     ├── SceneServicePort (trait)
//!     ├── WorldServicePort (trait)
//!     └── ObservationRepositoryPort / WorldMessagePort (traits)
//!
//! Adapter Layer (this module)
//!     │
//!     ├── StagingStateAdapter ─────────► WorldStateManager
//!     ├── StagingServiceAdapter ───────► StagingService
//!     ├── ConnectionManagerAdapter ────► WorldConnectionManager
//!     ├── SceneServiceAdapter ─────────► SceneService
//!     ├── InteractionServiceAdapter ───► InteractionService
//!     ├── WorldServiceAdapter ─────────► WorldService
//!     ├── PlayerCharacterServiceAdapter ► PlayerCharacterService
//!     ├── DirectorialContextAdapter ───► DirectorialContextRepositoryPort
//!     └── WorldMessageAdapter ──────────► WorldConnectionManager
//! ```
//!
//! # Usage
//!
//! These adapters are constructed in `UseCases::new()` and passed to use cases.
//! See `state/use_cases.rs` for wiring.
//!
//! # Implementation Status
//!
//! - [x] StagingStateAdapter - For MovementUseCase, StagingApprovalUseCase
//! - [x] StagingServiceAdapter - For MovementUseCase, StagingApprovalUseCase
//! - [x] ConnectionManagerAdapter - For ConnectionUseCase
//! - [x] PlayerActionAdapters - For PlayerActionUseCase
//! - [x] ObservationAdapters - For ObservationUseCase
//! - [x] ChallengeAdapters - For ChallengeUseCase (adapters created, wiring pending)
//! - [x] SceneAdapters - For SceneUseCase
//! - [x] ConnectionAdapters - For ConnectionUseCase (WorldServiceAdapter, PlayerCharacterServiceAdapter, etc.)

mod staging_state_adapter;
mod staging_service_adapter;
mod connection_manager_adapter;
mod player_action_adapters;
mod observation_adapters;
mod challenge_adapters;

mod scene_adapters;
mod connection_adapters;

pub use staging_state_adapter::*;
pub use staging_service_adapter::*;
pub use connection_manager_adapter::*;
pub use player_action_adapters::*;
pub use observation_adapters::*;
pub use challenge_adapters::*;
pub use scene_adapters::*;
pub use connection_adapters::*;
