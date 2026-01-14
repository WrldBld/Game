//! PlayerCharacter aggregate - PCs created by players, distinct from NPCs
//!
//! # Rustic DDD Design
//!
//! This aggregate follows Rustic DDD principles:
//! - **Private fields**: All fields are encapsulated
//! - **Newtypes**: `CharacterName` for validated name
//! - **Valid by construction**: `new()` takes pre-validated types
//! - **Builder pattern**: Fluent API for optional fields

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::character_sheet::CharacterSheetData;
use crate::value_objects::CharacterName;
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId, WorldId};

/// A player character (PC) - distinct from NPCs
///
/// # Invariants
///
/// - `name` is always non-empty and <= 200 characters (enforced by `CharacterName`)
///
/// # Example
///
/// ```
/// use chrono::Utc;
/// use wrldbldr_domain::{WorldId, LocationId, PlayerCharacterId};
/// use wrldbldr_domain::aggregates::player_character::PlayerCharacter;
/// use wrldbldr_domain::value_objects::CharacterName;
///
/// let world_id = WorldId::new();
/// let location_id = LocationId::new();
/// let name = CharacterName::new("Aragorn").unwrap();
/// let now = Utc::now();
/// let pc = PlayerCharacter::new("user123", world_id, name, location_id, now);
///
/// assert_eq!(pc.name().as_str(), "Aragorn");
/// ```
#[derive(Debug, Clone)]
pub struct PlayerCharacter {
    // Identity
    id: PlayerCharacterId,
    user_id: String, // Anonymous user ID from Player
    world_id: WorldId,

    // Character identity
    name: CharacterName,
    description: Option<String>,

    // Character sheet data (matches CharacterSheetData from Phase 14)
    sheet_data: Option<CharacterSheetData>,

    // Location tracking
    current_location_id: LocationId,
    /// The specific region within the location (for JRPG-style navigation)
    current_region_id: Option<RegionId>,
    starting_location_id: LocationId, // For reference/history

    // Visual assets (optional, can be generated later)
    sprite_asset: Option<String>,
    portrait_asset: Option<String>,

    // Status flags
    /// Whether the character is alive (false if killed/removed from play)
    is_alive: bool,
    /// Whether the character is currently active in the world
    is_active: bool,

    // Metadata
    created_at: DateTime<Utc>,
    last_active_at: DateTime<Utc>,
}

impl PlayerCharacter {
    // =========================================================================
    // Constructor
    // =========================================================================

    /// Create a new player character.
    ///
    /// The `name` parameter must be a pre-validated `CharacterName` - validation
    /// happens when creating the `CharacterName`, not here.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::Utc;
    /// use wrldbldr_domain::{WorldId, LocationId};
    /// use wrldbldr_domain::aggregates::player_character::PlayerCharacter;
    /// use wrldbldr_domain::value_objects::CharacterName;
    ///
    /// let world_id = WorldId::new();
    /// let location_id = LocationId::new();
    /// let name = CharacterName::new("Legolas").unwrap();
    /// let now = Utc::now();
    /// let pc = PlayerCharacter::new("user456", world_id, name, location_id, now);
    ///
    /// assert_eq!(pc.name().as_str(), "Legolas");
    /// assert!(pc.is_alive());
    /// assert!(pc.is_active());
    /// ```
    pub fn new(
        user_id: impl Into<String>,
        world_id: WorldId,
        name: CharacterName,
        starting_location_id: LocationId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: PlayerCharacterId::new(),
            user_id: user_id.into(),
            world_id,
            name,
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

    // =========================================================================
    // Identity Accessors (read-only)
    // =========================================================================

    /// Returns the player character's unique identifier.
    #[inline]
    pub fn id(&self) -> PlayerCharacterId {
        self.id
    }

    /// Returns the user ID (anonymous user from Player).
    #[inline]
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Returns the ID of the world this character belongs to.
    #[inline]
    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    /// Returns the character's name.
    #[inline]
    pub fn name(&self) -> &CharacterName {
        &self.name
    }

    /// Returns the character's description.
    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    // =========================================================================
    // Character Sheet Accessors
    // =========================================================================

    /// Returns the character's sheet data.
    #[inline]
    pub fn sheet_data(&self) -> Option<&CharacterSheetData> {
        self.sheet_data.as_ref()
    }

    /// Returns a mutable reference to the character's sheet data.
    #[inline]
    pub fn sheet_data_mut(&mut self) -> Option<&mut CharacterSheetData> {
        self.sheet_data.as_mut()
    }

    // =========================================================================
    // Location Accessors
    // =========================================================================

    /// Returns the character's current location ID.
    #[inline]
    pub fn current_location_id(&self) -> LocationId {
        self.current_location_id
    }

    /// Returns the character's current region ID, if any.
    #[inline]
    pub fn current_region_id(&self) -> Option<RegionId> {
        self.current_region_id
    }

    /// Returns the character's starting location ID.
    #[inline]
    pub fn starting_location_id(&self) -> LocationId {
        self.starting_location_id
    }

    // =========================================================================
    // Asset Accessors
    // =========================================================================

    /// Returns the path to the character's sprite asset, if any.
    #[inline]
    pub fn sprite_asset(&self) -> Option<&str> {
        self.sprite_asset.as_deref()
    }

    /// Returns the path to the character's portrait asset, if any.
    #[inline]
    pub fn portrait_asset(&self) -> Option<&str> {
        self.portrait_asset.as_deref()
    }

    // =========================================================================
    // Status Accessors
    // =========================================================================

    /// Returns true if the character is alive.
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.is_alive
    }

    /// Returns true if the character is active in the world.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    // =========================================================================
    // Timestamp Accessors
    // =========================================================================

    /// Returns when the character was created.
    #[inline]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns when the character was last active.
    #[inline]
    pub fn last_active_at(&self) -> DateTime<Utc> {
        self.last_active_at
    }

    // =========================================================================
    // Builder Methods (for construction)
    // =========================================================================

    /// Set the starting region (spawn point).
    pub fn with_starting_region(mut self, region_id: RegionId) -> Self {
        self.current_region_id = Some(region_id);
        self
    }

    /// Set the character description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the character sheet data.
    pub fn with_sheet_data(mut self, sheet_data: CharacterSheetData) -> Self {
        self.sheet_data = Some(sheet_data);
        self
    }

    /// Set the sprite asset.
    pub fn with_sprite(mut self, asset_path: impl Into<String>) -> Self {
        self.sprite_asset = Some(asset_path.into());
        self
    }

    /// Set the portrait asset.
    pub fn with_portrait(mut self, asset_path: impl Into<String>) -> Self {
        self.portrait_asset = Some(asset_path.into());
        self
    }

    /// Set the character's ID (used when loading from storage).
    pub fn with_id(mut self, id: PlayerCharacterId) -> Self {
        self.id = id;
        self
    }

    /// Set the alive status (used when loading from storage).
    pub fn with_alive(mut self, is_alive: bool) -> Self {
        self.is_alive = is_alive;
        self
    }

    /// Set the active status (used when loading from storage).
    pub fn with_active(mut self, is_active: bool) -> Self {
        self.is_active = is_active;
        self
    }

    /// Set the current location (used when loading from storage).
    pub fn with_current_location(mut self, location_id: LocationId) -> Self {
        self.current_location_id = location_id;
        self
    }

    /// Set the current region (used when loading from storage).
    pub fn with_current_region(mut self, region_id: Option<RegionId>) -> Self {
        self.current_region_id = region_id;
        self
    }

    /// Set the created_at timestamp (used when loading from storage).
    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
        self
    }

    /// Set the last_active_at timestamp (used when loading from storage).
    pub fn with_last_active_at(mut self, last_active_at: DateTime<Utc>) -> Self {
        self.last_active_at = last_active_at;
        self
    }

    // =========================================================================
    // Mutation Methods
    // =========================================================================

    /// Update the character's current location (clears region).
    pub fn update_location(&mut self, location_id: LocationId, now: DateTime<Utc>) {
        self.current_location_id = location_id;
        self.current_region_id = None; // Region needs to be set separately
        self.last_active_at = now;
    }

    /// Update the character's current region (within current location).
    pub fn update_region(&mut self, region_id: RegionId, now: DateTime<Utc>) {
        self.current_region_id = Some(region_id);
        self.last_active_at = now;
    }

    /// Update both location and region at once.
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

    /// Update the last active timestamp.
    pub fn touch(&mut self, now: DateTime<Utc>) {
        self.last_active_at = now;
    }

    /// Set the character's name.
    pub fn set_name(&mut self, name: CharacterName) {
        self.name = name;
    }

    /// Set the character's description.
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    /// Set the character sheet data.
    pub fn set_sheet_data(&mut self, sheet_data: Option<CharacterSheetData>) {
        self.sheet_data = sheet_data;
    }

    /// Set the sprite asset path.
    pub fn set_sprite(&mut self, path: Option<String>) {
        self.sprite_asset = path;
    }

    /// Set the portrait asset path.
    pub fn set_portrait(&mut self, path: Option<String>) {
        self.portrait_asset = path;
    }

    /// Kill the character.
    pub fn kill(&mut self) {
        self.is_alive = false;
        self.is_active = false;
    }

    /// Deactivate the character (still alive but not actively playing).
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Activate the character.
    pub fn activate(&mut self) {
        if self.is_alive {
            self.is_active = true;
        }
    }

    /// Resurrect the character.
    pub fn resurrect(&mut self) {
        self.is_alive = true;
        self.is_active = true;
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate that the character has required fields.
    /// Note: With newtypes, most validation happens at construction time.
    pub fn validate(&self) -> Result<(), String> {
        // CharacterName already validates non-empty at construction
        Ok(())
    }
}

// ============================================================================
// Serde Implementation
// ============================================================================

/// Intermediate format for serialization that matches the wire format
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerCharacterWireFormat {
    id: PlayerCharacterId,
    user_id: String,
    world_id: WorldId,
    name: CharacterName,
    description: Option<String>,
    sheet_data: Option<CharacterSheetData>,
    current_location_id: LocationId,
    current_region_id: Option<RegionId>,
    starting_location_id: LocationId,
    sprite_asset: Option<String>,
    portrait_asset: Option<String>,
    #[serde(default = "default_true")]
    is_alive: bool,
    #[serde(default = "default_true")]
    is_active: bool,
    created_at: DateTime<Utc>,
    last_active_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

impl Serialize for PlayerCharacter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire = PlayerCharacterWireFormat {
            id: self.id,
            user_id: self.user_id.clone(),
            world_id: self.world_id,
            name: self.name.clone(),
            description: self.description.clone(),
            sheet_data: self.sheet_data.clone(),
            current_location_id: self.current_location_id,
            current_region_id: self.current_region_id,
            starting_location_id: self.starting_location_id,
            sprite_asset: self.sprite_asset.clone(),
            portrait_asset: self.portrait_asset.clone(),
            is_alive: self.is_alive,
            is_active: self.is_active,
            created_at: self.created_at,
            last_active_at: self.last_active_at,
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PlayerCharacter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PlayerCharacterWireFormat::deserialize(deserializer)?;

        Ok(PlayerCharacter {
            id: wire.id,
            user_id: wire.user_id,
            world_id: wire.world_id,
            name: wire.name,
            description: wire.description,
            sheet_data: wire.sheet_data,
            current_location_id: wire.current_location_id,
            current_region_id: wire.current_region_id,
            starting_location_id: wire.starting_location_id,
            sprite_asset: wire.sprite_asset,
            portrait_asset: wire.portrait_asset,
            is_alive: wire.is_alive,
            is_active: wire.is_active,
            created_at: wire.created_at,
            last_active_at: wire.last_active_at,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pc() -> PlayerCharacter {
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let name = CharacterName::new("Test Hero").unwrap();
        let now = Utc::now();
        PlayerCharacter::new("user123", world_id, name, location_id, now)
    }

    mod constructor {
        use super::*;

        #[test]
        fn new_creates_pc_with_correct_defaults() {
            let world_id = WorldId::new();
            let location_id = LocationId::new();
            let name = CharacterName::new("Frodo").unwrap();
            let now = Utc::now();
            let pc = PlayerCharacter::new("user123", world_id, name, location_id, now);

            assert_eq!(pc.name().as_str(), "Frodo");
            assert_eq!(pc.user_id(), "user123");
            assert_eq!(pc.world_id(), world_id);
            assert_eq!(pc.current_location_id(), location_id);
            assert_eq!(pc.starting_location_id(), location_id);
            assert!(pc.current_region_id().is_none());
            assert!(pc.description().is_none());
            assert!(pc.sheet_data().is_none());
            assert!(pc.sprite_asset().is_none());
            assert!(pc.portrait_asset().is_none());
            assert!(pc.is_alive());
            assert!(pc.is_active());
        }

        #[test]
        fn builder_methods_work() {
            let world_id = WorldId::new();
            let location_id = LocationId::new();
            let region_id = RegionId::new();
            let name = CharacterName::new("Samwise").unwrap();
            let now = Utc::now();

            let pc = PlayerCharacter::new("user456", world_id, name, location_id, now)
                .with_description("A loyal gardener")
                .with_starting_region(region_id)
                .with_sprite("sprites/sam.png")
                .with_portrait("portraits/sam.png");

            assert_eq!(pc.description(), Some("A loyal gardener"));
            assert_eq!(pc.current_region_id(), Some(region_id));
            assert_eq!(pc.sprite_asset(), Some("sprites/sam.png"));
            assert_eq!(pc.portrait_asset(), Some("portraits/sam.png"));
        }
    }

    mod mutation {
        use super::*;

        #[test]
        fn update_location_clears_region() {
            let mut pc = create_test_pc();
            let region_id = RegionId::new();
            let new_location = LocationId::new();
            let now = Utc::now();

            // Set a region first
            pc.update_region(region_id, now);
            assert_eq!(pc.current_region_id(), Some(region_id));

            // Update location should clear region
            pc.update_location(new_location, now);
            assert_eq!(pc.current_location_id(), new_location);
            assert!(pc.current_region_id().is_none());
        }

        #[test]
        fn update_region_works() {
            let mut pc = create_test_pc();
            let region_id = RegionId::new();
            let now = Utc::now();

            pc.update_region(region_id, now);
            assert_eq!(pc.current_region_id(), Some(region_id));
        }

        #[test]
        fn update_position_works() {
            let mut pc = create_test_pc();
            let new_location = LocationId::new();
            let region_id = RegionId::new();
            let now = Utc::now();

            pc.update_position(new_location, Some(region_id), now);
            assert_eq!(pc.current_location_id(), new_location);
            assert_eq!(pc.current_region_id(), Some(region_id));
        }

        #[test]
        fn kill_sets_both_flags() {
            let mut pc = create_test_pc();
            pc.kill();
            assert!(!pc.is_alive());
            assert!(!pc.is_active());
        }

        #[test]
        fn deactivate_only_sets_active() {
            let mut pc = create_test_pc();
            pc.deactivate();
            assert!(pc.is_alive());
            assert!(!pc.is_active());
        }

        #[test]
        fn activate_works_only_if_alive() {
            let mut pc = create_test_pc();

            // Deactivate then reactivate
            pc.deactivate();
            pc.activate();
            assert!(pc.is_active());

            // Kill then try to activate - should not work
            pc.kill();
            pc.activate();
            assert!(!pc.is_active());
        }

        #[test]
        fn resurrect_sets_both_flags() {
            let mut pc = create_test_pc();
            pc.kill();
            pc.resurrect();
            assert!(pc.is_alive());
            assert!(pc.is_active());
        }
    }

    mod validation {
        use super::*;

        #[test]
        fn validate_passes_for_valid_pc() {
            let pc = create_test_pc();
            assert!(pc.validate().is_ok());
        }
    }

    mod serde {
        use super::*;

        #[test]
        fn serialize_deserialize_roundtrip() {
            let world_id = WorldId::new();
            let location_id = LocationId::new();
            let name = CharacterName::new("Bilbo").unwrap();
            let now = Utc::now();

            let pc = PlayerCharacter::new("user789", world_id, name, location_id, now)
                .with_description("A hobbit adventurer")
                .with_sprite("sprites/bilbo.png");

            let json = serde_json::to_string(&pc).unwrap();
            let deserialized: PlayerCharacter = serde_json::from_str(&json).unwrap();

            assert_eq!(deserialized.id(), pc.id());
            assert_eq!(deserialized.user_id(), "user789");
            assert_eq!(deserialized.name().as_str(), "Bilbo");
            assert_eq!(deserialized.description(), Some("A hobbit adventurer"));
            assert_eq!(deserialized.sprite_asset(), Some("sprites/bilbo.png"));
            assert!(deserialized.is_alive());
            assert!(deserialized.is_active());
        }

        #[test]
        fn serialize_produces_camel_case() {
            let pc = create_test_pc();
            let json = serde_json::to_string(&pc).unwrap();

            assert!(json.contains("userId"));
            assert!(json.contains("worldId"));
            assert!(json.contains("sheetData"));
            assert!(json.contains("currentLocationId"));
            assert!(json.contains("currentRegionId"));
            assert!(json.contains("startingLocationId"));
            assert!(json.contains("spriteAsset"));
            assert!(json.contains("portraitAsset"));
            assert!(json.contains("isAlive"));
            assert!(json.contains("isActive"));
            assert!(json.contains("createdAt"));
            assert!(json.contains("lastActiveAt"));
        }
    }
}
