//! Character domain entity
//!
//! Represents a character in the game world.

/// Character position on screen (for visual novel display)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharacterPosition {
    Left,
    Center,
    Right,
    #[default]
    OffScreen,
}

/// Character data for display in scenes
#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    /// Unique identifier
    pub id: String,
    /// Character name
    pub name: String,
    /// URL to sprite asset (if any)
    pub sprite_asset: Option<String>,
    /// URL to portrait asset (if any)
    pub portrait_asset: Option<String>,
    /// Current position on screen
    pub position: CharacterPosition,
    /// Whether this character is currently speaking
    pub is_speaking: bool,
}

impl Character {
    /// Create a new character
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            sprite_asset: None,
            portrait_asset: None,
            position: CharacterPosition::OffScreen,
            is_speaking: false,
        }
    }

    /// Set the sprite asset
    pub fn with_sprite(mut self, url: impl Into<String>) -> Self {
        self.sprite_asset = Some(url.into());
        self
    }

    /// Set the portrait asset
    pub fn with_portrait(mut self, url: impl Into<String>) -> Self {
        self.portrait_asset = Some(url.into());
        self
    }

    /// Set the position
    pub fn with_position(mut self, position: CharacterPosition) -> Self {
        self.position = position;
        self
    }
}
