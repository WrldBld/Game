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

mod challenge_adapters;
mod connection_manager_adapter;
mod observation_adapters;
mod player_action_adapters;
mod staging_service_adapter;
mod staging_state_adapter;

mod connection_adapters;
mod scene_adapters;

// Explicit exports (no glob re-exports)
pub use challenge_adapters::{
    ChallengeDmApprovalQueueAdapter, ChallengeOutcomeApprovalAdapter, ChallengeResolutionAdapter,
};
pub use connection_adapters::{
    ConnectionDirectorialContextAdapter, ConnectionWorldStateAdapter,
    PlayerCharacterServiceAdapter, WorldServiceAdapter,
};
pub use connection_manager_adapter::ConnectionManagerAdapter;
pub use observation_adapters::WorldMessageAdapter;
pub use player_action_adapters::{DmNotificationAdapter, PlayerActionQueueAdapter};
pub use scene_adapters::{
    DirectorialContextAdapter, DmActionQueuePlaceholder, InteractionServiceAdapter,
    SceneServiceAdapter, SceneWorldStateAdapter,
};
pub use staging_service_adapter::StagingServiceAdapter;
pub use staging_state_adapter::StagingStateAdapter;
