//! Port Adapters - Infrastructure implementations of use case ports
//!
//! This module contains adapters that implement the port traits defined in use cases.
//! Each adapter wraps an existing infrastructure component and implements the port
//! interface expected by use cases.
//!
//! # Architecture
//!
//! After the hexagonal refactor, most wrapper adapters have been eliminated:
//! - StagingService now directly implements StagingQueryPort
//! - Use cases depend on repository ports and domain types
//!
//! Remaining adapters:
//! - DirectorialContextAdapter - Implements DirectorialContextDtoRepositoryPort
//! - SceneDmActionQueueAdapter - Implements SceneDmActionQueuePort via DmActionEnqueuePort
//!
//! Direct implementations (on infrastructure types):
//! - ConnectionManagerPort - on WorldConnectionManager
//! - StagingStatePort - on WorldStateManager
//!
//! # Usage
//!
//! These adapters are constructed in `UseCases::new()` and passed to use cases.
//! See `engine-runner/composition/factories/use_cases.rs` for wiring.

#[path = "scene_adapters.rs"]
mod scene_adapters_mod;

// Re-export scene adapters
pub use scene_adapters_mod::{DirectorialContextAdapter, SceneDmActionQueueAdapter};
