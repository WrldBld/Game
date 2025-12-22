//! Visual novel components - Backdrop, dialogue box, sprites
//!
//! Components for the visual novel-style gameplay interface.

pub mod backdrop;
pub mod character_sprite;
pub mod choice_menu;
pub mod dialogue_box;

pub use backdrop::Backdrop;
pub use character_sprite::CharacterLayer;
pub use dialogue_box::{DialogueBox, EmptyDialogueBox};
