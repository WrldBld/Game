//! Repository modules - Data access wrappers around port traits.
//!
//! Most repositories have been eliminated in favor of direct port trait injection
//! (see ADR-009). The remaining modules either:
//! - Add real business logic beyond delegation (staging, flag, narrative)
//! - Provide deprecated facades still referenced (inventory, world, lore)
//! - Wrap infrastructure services (assets, settings)
//!
//! Naming convention:
//! - `*Repository` - Data access wrappers for domain entities
//! - `*Store` - In-memory state storage (see crate::stores)

pub mod assets;
pub mod flag;
pub mod inventory;
pub mod lore;
pub mod narrative;
pub mod settings;
pub mod staging;
pub mod world;

#[cfg(test)]
mod narrative_integration_tests;

// Repositories (entity data access)
pub use assets::AssetsRepository;
pub use flag::FlagRepository;
pub use inventory::InventoryRepository;
pub use lore::LoreRepository;
pub use settings::SettingsRepository;
pub use staging::StagingRepository;
pub use world::{WorldError, WorldRepository};
