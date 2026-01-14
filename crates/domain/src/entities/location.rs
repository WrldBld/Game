//! Location entity - Physical or conceptual places in the world
//!
//! Locations form a hierarchy via CONTAINS_LOCATION edges in Neo4j.
//! Connections between locations use CONNECTED_TO edges.
//! Regions are separate nodes with HAS_REGION edges (see region.rs).

use serde::{Deserialize, Serialize};
use wrldbldr_domain::LocationId;

/// The type of location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocationType {
    /// Indoor location (tavern, dungeon room, etc.)
    Interior,
    /// Outdoor location (forest, city street, etc.)
    Exterior,
    /// Abstract or metaphysical location (dreamscape, etc.)
    Abstract,
    /// Unknown type for forward compatibility
    #[serde(other)]
    Unknown,
}

/// Type of connection between locations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ConnectionType {
    /// A door or doorway
    Door,
    /// A path, road, or trail
    Path,
    /// Stairs or ladder
    Stairs,
    /// Magical or supernatural portal
    Portal,
    /// Hidden or secret passage
    Hidden,
    /// Other/custom connection type (for forward compatibility)
    #[default]
    #[serde(other)]
    Other,
}

impl ConnectionType {
    /// Get a display-friendly name for this connection type
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Door => "Door",
            Self::Path => "Path",
            Self::Stairs => "Stairs",
            Self::Portal => "Portal",
            Self::Hidden => "Hidden",
            Self::Other => "Connection",
        }
    }

    /// Get the string representation for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Door => "Door",
            Self::Path => "Path",
            Self::Stairs => "Stairs",
            Self::Portal => "Portal",
            Self::Hidden => "Hidden",
            Self::Other => "Connection",
        }
    }

    /// Parse a connection type from a string (case-insensitive)
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "door" => Self::Door,
            "path" => Self::Path,
            "stairs" => Self::Stairs,
            "portal" => Self::Portal,
            "hidden" => Self::Hidden,
            _ => Self::Other,
        }
    }
}

/// A connection between two locations
///
/// Stored as a `CONNECTED_TO` edge in Neo4j with properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationConnection {
    pub from_location: LocationId,
    pub to_location: LocationId,
    /// Type of connection (Door, Path, Stairs, Portal, Hidden, or Other)
    pub connection_type: ConnectionType,
    /// Description of the path/transition
    pub description: Option<String>,
    /// Whether this connection works both ways
    pub bidirectional: bool,
    /// Travel time in game-time units (0 = instant)
    pub travel_time: u32,
    /// Whether this connection is currently locked
    pub is_locked: bool,
    /// Description of what's needed to unlock (if locked)
    pub lock_description: Option<String>,
}

impl LocationConnection {
    pub fn new(from: LocationId, to: LocationId, connection_type: ConnectionType) -> Self {
        Self {
            from_location: from,
            to_location: to,
            connection_type,
            description: None,
            bidirectional: true,
            travel_time: 0,
            is_locked: false,
            lock_description: None,
        }
    }

    /// Create a door connection
    pub fn door(from: LocationId, to: LocationId) -> Self {
        Self::new(from, to, ConnectionType::Door)
    }

    /// Create a path/road connection
    pub fn path(from: LocationId, to: LocationId) -> Self {
        Self::new(from, to, ConnectionType::Path)
    }

    /// Create a stairs connection
    pub fn stairs(from: LocationId, to: LocationId) -> Self {
        Self::new(from, to, ConnectionType::Stairs)
    }

    /// Create a portal/magical connection
    pub fn portal(from: LocationId, to: LocationId) -> Self {
        Self::new(from, to, ConnectionType::Portal)
    }

    /// Create a hidden/secret passage connection
    pub fn hidden(from: LocationId, to: LocationId) -> Self {
        Self::new(from, to, ConnectionType::Hidden)
    }

    pub fn one_way(mut self) -> Self {
        self.bidirectional = false;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_travel_time(mut self, time: u32) -> Self {
        self.travel_time = time;
        self
    }

    pub fn locked(mut self, description: impl Into<String>) -> Self {
        self.is_locked = true;
        self.lock_description = Some(description.into());
        self
    }
}
