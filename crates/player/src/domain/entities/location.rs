//! Location domain entity
//!
//! Represents a place in the game world that characters can visit.

use crate::domain::value_objects::LocationId;

/// A location in the game world
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    /// Unique identifier
    pub id: LocationId,
    /// Location name
    pub name: String,
    /// Description of the location
    pub description: Option<String>,
    /// Optional backdrop asset URL
    pub backdrop_asset: Option<String>,
    /// IDs of connected locations
    pub connected_to: Vec<LocationId>,
}

impl Location {
    /// Create a new location
    pub fn new(id: impl Into<LocationId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            backdrop_asset: None,
            connected_to: Vec::new(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the backdrop asset
    pub fn with_backdrop(mut self, url: impl Into<String>) -> Self {
        self.backdrop_asset = Some(url.into());
        self
    }

    /// Add a connection to another location
    pub fn connect_to(mut self, location_id: impl Into<LocationId>) -> Self {
        self.connected_to.push(location_id.into());
        self
    }
}

/// Summary of a location for list views
#[derive(Debug, Clone, PartialEq)]
pub struct LocationSummary {
    /// Location ID
    pub id: LocationId,
    /// Location name
    pub name: String,
    /// Description preview
    pub description: Option<String>,
    /// Number of connected locations
    pub connection_count: usize,
}

impl LocationSummary {
    /// Create from a Location entity
    pub fn from_location(location: &Location) -> Self {
        Self {
            id: location.id.clone(),
            name: location.name.clone(),
            description: location.description.clone(),
            connection_count: location.connected_to.len(),
        }
    }
}
