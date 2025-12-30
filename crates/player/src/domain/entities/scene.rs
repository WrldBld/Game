//! Scene domain entity
//!
//! Represents a scene in the game.

/// Scene data from the game
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Unique identifier
    pub id: String,
    /// Scene name
    pub name: String,
    /// Location ID where this scene takes place
    pub location_id: String,
    /// Location name
    pub location_name: String,
    /// Optional backdrop asset URL
    pub backdrop_asset: Option<String>,
    /// Time context description (e.g., "Morning", "Night")
    pub time_context: String,
    /// Directorial notes for the scene
    pub directorial_notes: String,
}

impl Scene {
    /// Create a new scene
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        location_id: impl Into<String>,
        location_name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            location_id: location_id.into(),
            location_name: location_name.into(),
            backdrop_asset: None,
            time_context: String::new(),
            directorial_notes: String::new(),
        }
    }

    /// Set the backdrop asset
    pub fn with_backdrop(mut self, url: impl Into<String>) -> Self {
        self.backdrop_asset = Some(url.into());
        self
    }

    /// Set the time context
    pub fn with_time_context(mut self, context: impl Into<String>) -> Self {
        self.time_context = context.into();
        self
    }

    /// Set directorial notes
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.directorial_notes = notes.into();
        self
    }
}
