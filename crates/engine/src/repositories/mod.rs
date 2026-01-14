//! Repository modules - Data access wrappers around port traits.
//!
//! Each repository wraps a port trait and provides the interface
//! for use cases to access persisted aggregates.

pub mod act;
pub mod assets;
pub mod challenge;
pub mod character;
pub mod flag;
pub mod goal;
pub mod interaction;
pub mod inventory;
pub mod location;
pub mod location_state;
pub mod lore;
pub mod narrative;
pub mod observation;
pub mod player_character;
pub mod region_state;
pub mod scene;
pub mod settings;
pub mod skill;
pub mod staging;
pub mod world;

#[cfg(test)]
mod narrative_integration_tests;

pub use act::Act;
pub use assets::Assets;
pub use challenge::Challenge;
pub use character::Character;
pub use flag::Flag;
pub use goal::Goal;
pub use interaction::Interaction;
pub use inventory::Inventory;
pub use inventory::InventoryActionResult;
pub use inventory::InventoryError;
pub use location::Location;
pub use location::RegionExit;
pub use location::RegionExitsResult;
pub use location::SkippedExit;
pub use location_state::LocationStateEntity;
pub use lore::Lore;
pub use narrative::Narrative;
pub use observation::Observation;
pub use player_character::PlayerCharacter;
pub use region_state::RegionStateEntity;
pub use scene::Scene;
pub use scene::SceneConsideration;
pub use scene::SceneResolutionContext;
pub use scene::SceneResolutionResult;
pub use settings::{Settings, SettingsError};
pub use skill::Skill;
pub use staging::Staging;
pub use world::{World, WorldError};
