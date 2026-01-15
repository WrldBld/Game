//! Repository modules - Data access wrappers around port traits.
//!
//! Each repository wraps a port trait and provides the interface
//! for use cases to access persisted aggregates.

pub mod act;
pub mod assets;
pub mod challenge;
pub mod character;
pub mod clock;
pub mod content;
pub mod directorial;
pub mod flag;
pub mod goal;
pub mod interaction;
pub mod inventory;
pub mod llm;
pub mod location;
pub mod location_state;
pub mod lore;
pub mod narrative;
pub mod observation;
pub mod pending_staging;
pub mod player_character;
pub mod queue;
pub mod random;
pub mod region_state;
pub mod scene;
pub mod session;
pub mod settings;
pub mod staging;
pub mod time_suggestion;
pub mod world;

#[cfg(test)]
mod narrative_integration_tests;

pub use act::Act;
pub use assets::Assets;
pub use challenge::Challenge;
pub use character::Character;
pub use clock::Clock;
pub use content::Content;
pub use directorial::DirectorialContextStore;
pub use flag::Flag;
pub use goal::Goal;
pub use interaction::Interaction;
pub use inventory::Inventory;
pub use inventory::InventoryActionResult;
pub use inventory::InventoryError;
pub use llm::Llm;
pub use location::Location;
pub use location::RegionExit;
pub use location::RegionExitsResult;
pub use location::SkippedExit;
pub use location_state::LocationStateEntity;
pub use lore::Lore;
pub use narrative::Narrative;
pub use observation::Observation;
pub use pending_staging::PendingStaging;
pub use player_character::PlayerCharacter;
pub use queue::Queue;
pub use random::Random;
pub use region_state::RegionStateEntity;
pub use scene::Scene;
pub use scene::SceneConsideration;
pub use scene::SceneResolutionContext;
pub use scene::SceneResolutionResult;
pub use session::WorldSession;
pub use settings::{Settings, SettingsError};
pub use staging::Staging;
pub use time_suggestion::TimeSuggestionStore;
pub use world::{World, WorldError};
