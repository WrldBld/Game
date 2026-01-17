//! Repository modules - Data access wrappers around port traits.
//!
//! Per ADR-009, most repositories have been eliminated in favor of direct port trait injection.
//! The remaining modules:
//! - `assets.rs` - Coordinates 2 ports (AssetRepo + ImageGenPort) with real logic
//! - `settings.rs` - Has caching logic
//!
//! All other repository wrappers (flag, inventory, lore, narrative, staging, world)
//! have been deleted. Use the port traits directly from `infrastructure::ports`.
//!
//! Naming convention:
//! - `*Repository` - Data access wrappers for domain entities
//! - `*Store` - In-memory state storage (see crate::stores)

pub mod assets;
pub mod settings;

// Repositories (entity data access)
pub use assets::AssetsRepository;
pub use settings::SettingsRepository;
