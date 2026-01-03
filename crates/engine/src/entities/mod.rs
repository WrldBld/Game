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

pub use character::Character;
pub use player_character::PlayerCharacter;
pub use location::Location;
pub use scene::Scene;
pub use challenge::Challenge;
pub use narrative::Narrative;
pub use staging::Staging;
pub use observation::Observation;
pub use inventory::Inventory;
pub use assets::Assets;
pub use world::World;
