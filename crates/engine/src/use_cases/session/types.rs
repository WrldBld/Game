//! Domain types for session use cases.
//!
//! These types represent session data in a structured way, avoiding raw JSON.
//! JSON serialization happens at the API boundary layer.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId, SceneId, WorldId};

/// Complete snapshot of a world for session initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub world: WorldSummary,
    pub locations: Vec<LocationSummary>,
    pub characters: Vec<CharacterSummary>,
    pub scenes: Vec<SceneSummary>,
    pub current_scene: Option<SceneSummary>,
}

/// Summary of world metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSummary {
    pub id: WorldId,
    pub name: String,
    pub description: String,
    pub rule_system: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Summary of a location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationSummary {
    pub id: LocationId,
    pub name: String,
    pub description: String,
    pub location_type: String,
    pub backdrop_asset: Option<String>,
    pub parent_id: Option<LocationId>,
}

/// Summary of a character.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub id: CharacterId,
    pub name: String,
    pub description: String,
    pub archetype: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_alive: bool,
    pub is_active: bool,
}

/// Summary of a scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSummary {
    pub id: SceneId,
    pub name: String,
    pub location_id: LocationId,
    pub time_context: String,
    pub backdrop_override: Option<String>,
    pub featured_characters: Vec<String>,
    pub directorial_notes: Option<String>,
}

/// Summary of a player character.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerCharacterSummary {
    pub id: PlayerCharacterId,
    pub name: String,
    pub description: Option<String>,
    pub portrait_asset: Option<String>,
    pub sprite_asset: Option<String>,
    pub current_location_id: LocationId,
    pub current_region_id: Option<RegionId>,
}
