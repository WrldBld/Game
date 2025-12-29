//! Service Container Re-exports
//!
//! This module re-exports service containers from engine-adapters.
//! The containers are defined in adapters because they hold concrete adapter types.
//!
//! # Why not in runner?
//!
//! We attempted to move these to the runner (composition root), but `AppState` and
//! service containers hold concrete adapter types (`OllamaClient`, `ComfyUIClient`,
//! `Neo4jRepository`), which would create circular dependencies if defined separately.
//!
//! # Architecture Decision
//!
//! Service containers stay in engine-adapters (with concrete types), while the
//! construction logic (`new_app_state()`) is in engine-runner.

pub use wrldbldr_engine_adapters::infrastructure::state::{
    AssetServices, CoreServices, EventInfrastructure, GameServices, PlayerServices, QueueServices,
    UseCases,
};
