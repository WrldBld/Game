//! Entity modules - Domain capability encapsulation.
//!
//! Each module wraps operations for a domain entity type.
//! They depend on repository ports and provide the building blocks for use cases.

pub mod character;
pub mod player_character;
pub mod location;
pub mod scene;
pub mod challenge;
pub mod narrative;
pub mod staging;
pub mod observation;
pub mod inventory;
pub mod assets;
pub mod world;
pub mod flag;
pub mod lore;
pub mod location_state;
pub mod region_state;

pub use character::Character;
pub use player_character::PlayerCharacter;
pub use location::Location;
pub use scene::{Scene, SceneResolutionContext, SceneResolutionResult};
pub use challenge::Challenge;
pub use narrative::Narrative;
pub use staging::Staging;
pub use observation::Observation;
pub use inventory::Inventory;
pub use assets::Assets;
pub use world::World;
pub use flag::Flag;
pub use lore::Lore;
pub use location_state::LocationStateEntity;
pub use region_state::RegionStateEntity;
