//! Entity modules - Domain capability encapsulation.
//!
//! Each module wraps operations for a domain entity type.
//! They depend on repository ports and provide the building blocks for use cases.

pub mod act;
pub mod assets;
pub mod challenge;
pub mod flag;
pub mod goal;
pub mod interaction;
pub mod location_state;
pub mod lore;
pub mod observation;
pub mod player_character;
pub mod region_state;
pub mod settings;
pub mod skill;
pub mod world;

#[cfg(test)]
mod narrative_integration_tests;

pub use act::Act;
pub use assets::Assets;
pub use challenge::Challenge;
pub use flag::Flag;
pub use goal::Goal;
pub use interaction::Interaction;
pub use location_state::LocationStateEntity;
pub use lore::Lore;
pub use observation::Observation;
pub use player_character::PlayerCharacter;
pub use region_state::RegionStateEntity;
pub use settings::{Settings, SettingsError};
pub use skill::Skill;
pub use world::{World, WorldError};
