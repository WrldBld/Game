//! Domain entities

pub mod character;
pub mod location;
pub mod player_action;
pub mod scene;
pub mod world;

// Only re-export what is currently used outside the domain module.
pub use player_action::PlayerAction;
