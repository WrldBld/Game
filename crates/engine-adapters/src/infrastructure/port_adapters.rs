//! Port Adapters - Infrastructure implementations of use case ports
//!
//! This module contains adapters that implement the port traits defined in use cases.
//! Each adapter wraps an existing infrastructure component and implements the port
//! interface expected by use cases.
//!
//! # Architecture
//!
//! After the hexagonal refactor, most wrapper adapters have been eliminated.
//! Use cases now depend directly on internal service ports from engine-app.
//!
//! Remaining adapters:
//! - StagingServiceAdapter - Implements StagingUseCaseServicePort/ExtPort
//! - DirectorialContextAdapter - Implements DirectorialContextDtoRepositoryPort
//! - DmActionQueuePlaceholder - Implements SceneDmActionQueuePort
//!
//! Direct implementations (on infrastructure types):
//! - ConnectionManagerPort - on WorldConnectionManager
//! - StagingStatePort - on WorldStateManager
//!
//! # Usage
//!
//! These adapters are constructed in `UseCases::new()` and passed to use cases.
//! See `engine-runner/composition/factories/use_cases.rs` for wiring.

#[path = "staging_port_adapters.rs"]
mod staging_service_adapter;

#[path = "scene_adapters.rs"]
mod scene_adapters_mod;

// Re-export staging adapter
pub use staging_service_adapter::StagingServiceAdapter;

// Re-export scene adapters
pub use scene_adapters_mod::{DirectorialContextAdapter, DmActionQueuePlaceholder};
