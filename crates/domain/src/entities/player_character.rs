//! Player Character entity - PCs created by players, distinct from NPCs

use crate::entities::sheet_template::CharacterSheetData;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId, WorldId};

/// A player character (PC) - distinct from NPCs
///
/// PCs are created by players when joining a world, have character sheets,
/// and track their current location/region for scene resolution.
///
/// Connection to the world is managed by WorldConnectionManager, not stored here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerCharacter {
    pub id: PlayerCharacterId,
    pub user_id: String, // Anonymous user ID from Player
    pub world_id: WorldId,

    // Character identity
    pub name: String,
    pub description: Option<String>,

    // Character sheet data (matches CharacterSheetData from Phase 14)
    pub sheet_data: Option<CharacterSheetData>,

    // Location tracking
    pub current_location_id: LocationId,
    /// The specific region within the location (for JRPG-style navigation)
    pub current_region_id: Option<RegionId>,
    pub starting_location_id: LocationId, // For reference/history

    // Visual assets (optional, can be generated later)
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,

    // Status flags
    /// Whether the character is alive (false if killed/removed from play)
    #[serde(default = "default_true")]
    pub is_alive: bool,
    /// Whether the character is currently active in the world
    #[serde(default = "default_true")]
    pub is_active: bool,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

impl PlayerCharacter {
    /// Create a new player character
    pub fn new(
        user_id: impl Into<String>,
        world_id: WorldId,
        name: impl Into<String>,
        starting_location_id: LocationId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: PlayerCharacterId::new(),
            user_id: user_id.into(),
            world_id,
            name: name.into(),
            description: None,
            sheet_data: None,
            current_location_id: starting_location_id,
            current_region_id: None,
            starting_location_id,
            sprite_asset: None,
            portrait_asset: None,
            is_alive: true,
            is_active: true,
            created_at: now,
            last_active_at: now,
        }
    }

    /// Set the starting region (spawn point)
    pub fn with_starting_region(mut self, region_id: RegionId) -> Self {
        self.current_region_id = Some(region_id);
        self
    }

    /// Set the character description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the character sheet data
    pub fn with_sheet_data(mut self, sheet_data: CharacterSheetData) -> Self {
        self.sheet_data = Some(sheet_data);
        self
    }

    /// Set the sprite asset
    pub fn with_sprite(mut self, asset_path: impl Into<String>) -> Self {
        self.sprite_asset = Some(asset_path.into());
        self
    }

    /// Set the portrait asset
    pub fn with_portrait(mut self, asset_path: impl Into<String>) -> Self {
        self.portrait_asset = Some(asset_path.into());
        self
    }

    /// Update the character's current location (clears region)
    pub fn update_location(&mut self, location_id: LocationId, now: DateTime<Utc>) {
        self.current_location_id = location_id;
        self.current_region_id = None; // Region needs to be set separately
        self.last_active_at = now;
    }

    /// Update the character's current region (within current location)
    pub fn update_region(&mut self, region_id: RegionId, now: DateTime<Utc>) {
        self.current_region_id = Some(region_id);
        self.last_active_at = now;
    }

    /// Update both location and region at once
    pub fn update_position(
        &mut self,
        location_id: LocationId,
        region_id: Option<RegionId>,
        now: DateTime<Utc>,
    ) {
        self.current_location_id = location_id;
        self.current_region_id = region_id;
        self.last_active_at = now;
    }

    /// Update the last active timestamp
    pub fn touch(&mut self, now: DateTime<Utc>) {
        self.last_active_at = now;
    }

    /// Validate that the character has required fields
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Character name cannot be empty".to_string());
        }
        Ok(())
    }
}
