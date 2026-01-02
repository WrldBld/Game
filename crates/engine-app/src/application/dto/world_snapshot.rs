//! World snapshot DTO for session management
//!
//! This DTO aggregates domain entities for session state. It lives in the
//! application layer because it's used for coordinating session state across
//! the application, not for infrastructure-specific concerns.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use wrldbldr_domain::entities::{Character, Location, Scene, World};

/// A snapshot of the current world state for session joining
///
/// Note: This struct does not derive Serialize/Deserialize because it contains
/// domain types. Use `to_json()` method for serialization instead.
#[derive(Debug, Clone)]
pub struct WorldSnapshot {
    pub world: World,
    pub locations: Vec<Location>,
    pub characters: Vec<Character>,
    pub scenes: Vec<Scene>,
    pub current_scene_id: Option<String>,
}

impl WorldSnapshot {
    /// Convert to a JSON value for transmission
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "world": {
                "id": self.world.id.to_string(),
                "name": &self.world.name,
                "description": &self.world.description
            },
            "locations": self.locations.iter().map(|l| serde_json::json!({
                "id": l.id.to_string(),
                "name": &l.name,
                "description": &l.description,
                "backdrop_asset": &l.backdrop_asset,
                "location_type": format!("{:?}", l.location_type)
            })).collect::<Vec<_>>(),
            "characters": self.characters.iter().map(|c| serde_json::json!({
                "id": c.id.to_string(),
                "name": &c.name,
                "description": &c.description,
                "sprite_asset": &c.sprite_asset,
                "portrait_asset": &c.portrait_asset,
                "archetype": format!("{:?}", c.current_archetype)
            })).collect::<Vec<_>>(),
            "scenes": self.scenes.iter().map(|s| serde_json::json!({
                "id": s.id.to_string(),
                "name": &s.name,
                "location_id": s.location_id.to_string(),
                "directorial_notes": &s.directorial_notes
            })).collect::<Vec<_>>(),
            "current_scene_id": &self.current_scene_id
        })
    }

    /// Wrap in Arc for shared ownership
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Default for WorldSnapshot {
    /// Create a minimal empty world snapshot with placeholder values.
    ///
    /// This is used as a fallback when JSON deserialization fails during
    /// session creation. In normal operation, proper world data should be
    /// provided, but this ensures the system remains functional with a
    /// basic empty world containing no locations, characters, or scenes.
    ///
    /// Uses Unix epoch (1970-01-01T00:00:00Z) as a sentinel value to indicate
    /// an uninitialized snapshot, avoiding impure `Utc::now()` calls.
    fn default() -> Self {
        // Use epoch as sentinel for "uninitialized" snapshot - avoids impure Utc::now()
        let epoch = DateTime::<Utc>::from_timestamp(0, 0).expect("epoch timestamp is always valid");
        Self {
            world: World::new("Empty World", "A placeholder world", epoch),
            locations: Vec::new(),
            characters: Vec::new(),
            scenes: Vec::new(),
            current_scene_id: None,
        }
    }
}
