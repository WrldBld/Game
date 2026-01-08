//! Entity modules - Domain capability encapsulation.
//!
//! Each module wraps operations for a domain entity type.
//! They depend on repository ports and provide the building blocks for use cases.

pub mod assets;
pub mod challenge;
pub mod character;
pub mod flag;
pub mod goal;
pub mod inventory;
pub mod location;
pub mod location_state;
pub mod lore;
pub mod narrative;
pub mod observation;
pub mod player_character;
pub mod region_state;
pub mod scene;
pub mod staging;
pub mod world;

pub use assets::Assets;
pub use challenge::Challenge;
pub use character::Character;
pub use flag::Flag;
pub use goal::Goal;
pub use inventory::Inventory;
pub use location::Location;
pub use location_state::LocationStateEntity;
pub use lore::Lore;
pub use narrative::Narrative;
pub use observation::Observation;
pub use player_character::PlayerCharacter;
pub use region_state::RegionStateEntity;
pub use scene::{Scene, SceneResolutionContext, SceneResolutionResult};
pub use staging::Staging;
pub use world::{World, WorldError};
