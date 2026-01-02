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
//!     └── ObservationRepositoryPort (traits)
//!
//! Adapter Layer (this module)
//!     │
//!     ├── (direct impl) StagingStatePort ─► WorldStateManager
//!     ├── StagingServiceAdapter ───────► StagingService
//!     ├── (direct impl) ConnectionManagerPort ─► WorldConnectionManager
//!     ├── SceneServiceAdapter ─────────► SceneService
//!     ├── InteractionServiceAdapter ───► InteractionService
//!     ├── WorldServiceAdapter ─────────► WorldService
//!     ├── PlayerCharacterServiceAdapter ► PlayerCharacterService
//!     └── DirectorialContextAdapter ───► DirectorialContextRepositoryPort
//! ```
//!
//! # Usage
//!
//! These adapters are constructed in `UseCases::new()` and passed to use cases.
//! See `state/use_cases.rs` for wiring.
//!
//! # Implementation Status
//!
//! - [x] (direct impl) StagingStatePort - For MovementUseCase, StagingApprovalUseCase
//! - [x] StagingServiceAdapter - For MovementUseCase, StagingApprovalUseCase
//! - [x] (direct impl) ConnectionManagerPort - For ConnectionUseCase
//! - [x] PlayerActionAdapters - For PlayerActionUseCase
//! - [x] ObservationAdapters - For ObservationUseCase
//! - [x] ChallengeAdapters - For ChallengeUseCase (adapters created, wiring pending)
//! - [x] SceneAdapters - For SceneUseCase
//! - [x] ConnectionAdapters - For ConnectionUseCase (WorldServiceAdapter, PlayerCharacterServiceAdapter, etc.)

#[path = "challenge_port_adapters.rs"]
mod challenge_adapters;

#[path = "player_action_port_adapters.rs"]
mod player_action_adapters;

#[path = "staging_port_adapters.rs"]
mod staging_service_adapter;

#[path = "connection_port_adapters.rs"]
mod connection_adapters;

#[path = "scene_port_adapters.rs"]
mod scene_adapters;

// Explicit exports (no glob re-exports)
pub use challenge_adapters::{
    ChallengeDmApprovalQueueAdapter, ChallengeOutcomeApprovalAdapter, ChallengeResolutionAdapter,
};
pub use connection_adapters::{
    ConnectionDirectorialContextAdapter, PlayerCharacterServiceAdapter, WorldServiceAdapter,
};
pub use player_action_adapters::PlayerActionQueueAdapter;
pub use scene_adapters::{
    DirectorialContextAdapter, DmActionQueuePlaceholder, InteractionServiceAdapter,
    SceneServiceAdapter,
};
pub use staging_service_adapter::StagingServiceAdapter;
