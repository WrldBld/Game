//! PlayerCharacter aggregate - PCs created by players, distinct from NPCs
//!
//! # Rustic DDD Design
//!
//! This aggregate follows Rustic DDD principles:
//! - **Private fields**: All fields are encapsulated
//! - **Newtypes**: `CharacterName` for validated name
//! - **Valid by construction**: `new()` takes pre-validated types
//! - **Builder pattern**: Fluent API for optional fields
//! - **State enum**: `CharacterState` instead of boolean blindness
//! - **Domain events**: Mutations return `PlayerCharacterStateChange` enum

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::types::character_sheet::CharacterSheetValues;
use crate::value_objects::{AssetPath, CharacterName, CharacterState, Description};
use crate::{LocationId, PlayerCharacterId, RegionId, UserId, WorldId};

// ============================================================================
// Domain Events
// ============================================================================

/// Domain event returned from state-changing mutations.
///
/// Following Rustic DDD principles, mutations return domain events instead of `()`.
/// This allows the caller to know exactly what happened and react accordingly.
///
/// # Examples
///
/// ```
/// use wrldbldr_domain::aggregates::player_character::{PlayerCharacter, PlayerCharacterStateChange};
/// use wrldbldr_domain::value_objects::CharacterName;
/// use wrldbldr_domain::{WorldId, LocationId, UserId};
/// use chrono::TimeZone;
///
/// let name = CharacterName::new("Hero").unwrap();
/// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
/// let user_id = UserId::new("user1").unwrap();
/// let mut pc = PlayerCharacter::new(user_id, WorldId::new(), name, LocationId::new(), now);
///
/// // First kill returns Killed
/// assert_eq!(pc.kill(), PlayerCharacterStateChange::Killed);
///
/// // Second kill returns AlreadyDead
/// assert_eq!(pc.kill(), PlayerCharacterStateChange::AlreadyDead);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerCharacterStateChange {
    /// Character was killed (transitioned to Dead state)
    Killed,
    /// Kill attempted but character was already dead
    AlreadyDead,
    /// Character was deactivated (transitioned to Inactive state)
    Deactivated,
    /// Deactivation attempted but character was already inactive (or dead)
    AlreadyInactive,
    /// Character was activated (transitioned to Active state)
    Activated,
    /// Activation attempted but character was already active (or dead)
    AlreadyActive,
    /// Character was resurrected (transitioned from Dead to Active)
    Resurrected,
    /// Resurrection attempted but character was already alive
    AlreadyAlive,
}

/// A player character (PC) - distinct from NPCs
///
/// # Invariants
///
/// - `name` is always non-empty and <= 200 characters (enforced by `CharacterName`)
///
/// # Example
///
/// ```
/// use chrono::TimeZone;
/// use wrldbldr_domain::{WorldId, LocationId, PlayerCharacterId, UserId};
/// use wrldbldr_domain::aggregates::player_character::PlayerCharacter;
/// use wrldbldr_domain::value_objects::CharacterName;
///
/// let world_id = WorldId::new();
/// let location_id = LocationId::new();
/// let name = CharacterName::new("Aragorn").unwrap();
/// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
/// let user_id = UserId::new("user123").unwrap();
/// let pc = PlayerCharacter::new(user_id, world_id, name, location_id, now);
///
/// assert_eq!(pc.name().as_str(), "Aragorn");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerCharacter {
    // Identity
    id: PlayerCharacterId,
    user_id: UserId, // Typed wrapper for anonymous user ID from Player
    world_id: WorldId,

    // Character identity
    name: CharacterName,
    description: Option<Description>,

    // Character sheet data (wire format values + timestamp)
    sheet_data: Option<CharacterSheetValues>,

    // Location tracking
    current_location_id: LocationId,
    /// The specific region within the location (for JRPG-style navigation)
    current_region_id: Option<RegionId>,
    starting_location_id: LocationId, // For reference/history

    // Visual assets (optional, can be generated later)
    sprite_asset: Option<AssetPath>,
    portrait_asset: Option<AssetPath>,

    // Lifecycle state (replaces is_alive/is_active boolean blindness)
    /// The character's lifecycle state (Active, Inactive, or Dead)
    state: CharacterState,

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
    /// use chrono::TimeZone;
    /// use wrldbldr_domain::{WorldId, LocationId, UserId};
    /// use wrldbldr_domain::aggregates::player_character::PlayerCharacter;
    /// use wrldbldr_domain::value_objects::CharacterName;
    ///
    /// let world_id = WorldId::new();
    /// let location_id = LocationId::new();
    /// let name = CharacterName::new("Legolas").unwrap();
    /// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    /// let user_id = UserId::new("user456").unwrap();
    /// let pc = PlayerCharacter::new(user_id, world_id, name, location_id, now);
    ///
    /// assert_eq!(pc.name().as_str(), "Legolas");
    /// assert!(pc.is_alive());
    /// assert!(pc.is_active());
    /// ```
    pub fn new(
        user_id: UserId,
        world_id: WorldId,
        name: CharacterName,
        starting_location_id: LocationId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: PlayerCharacterId::new(),
            user_id,
            world_id,
            name,
            description: None,
            sheet_data: None,
            current_location_id: starting_location_id,
            current_region_id: None,
            starting_location_id,
            sprite_asset: None,
            portrait_asset: None,
            state: CharacterState::Active,
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
        self.user_id.as_str()
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
        self.description.as_ref().map(|d| d.as_str())
    }

    // =========================================================================
    // Character Sheet Accessors
    // =========================================================================

    /// Returns the character's sheet data.
    #[inline]
    pub fn sheet_data(&self) -> Option<&CharacterSheetValues> {
        self.sheet_data.as_ref()
    }

    /// Get a mutable reference to the character sheet data
    pub fn sheet_data_mut(&mut self) -> Option<&mut CharacterSheetValues> {
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
    pub fn sprite_asset(&self) -> Option<&AssetPath> {
        self.sprite_asset.as_ref()
    }

    /// Returns the path to the character's portrait asset, if any.
    #[inline]
    pub fn portrait_asset(&self) -> Option<&AssetPath> {
        self.portrait_asset.as_ref()
    }

    // =========================================================================
    // Status Accessors
    // =========================================================================

    /// Returns the character's current lifecycle state.
    #[inline]
    pub fn state(&self) -> CharacterState {
        self.state
    }

    /// Returns true if the character is alive (Active or Inactive state).
    ///
    /// Delegates to `CharacterState::is_alive()`.
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.state.is_alive()
    }

    /// Returns true if the character is active in the world.
    ///
    /// Delegates to `CharacterState::is_active()`.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Returns true if the character is dead.
    ///
    /// Delegates to `CharacterState::is_dead()`.
    #[inline]
    pub fn is_dead(&self) -> bool {
        self.state.is_dead()
    }

    /// Returns true if the character is inactive (alive but not actively playing).
    ///
    /// Delegates to `CharacterState::is_inactive()`.
    #[inline]
    pub fn is_inactive(&self) -> bool {
        self.state.is_inactive()
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
        self.description = Description::new(description).ok();
        self
    }

    /// Set the character sheet data.
    pub fn with_sheet_data(mut self, sheet_data: CharacterSheetValues) -> Self {
        self.sheet_data = Some(sheet_data);
        self
    }

    /// Set the sprite asset.
    pub fn with_sprite(mut self, asset_path: AssetPath) -> Self {
        self.sprite_asset = Some(asset_path);
        self
    }

    /// Set the portrait asset.
    pub fn with_portrait(mut self, asset_path: AssetPath) -> Self {
        self.portrait_asset = Some(asset_path);
        self
    }

    /// Set the character's ID (used when loading from storage).
    pub fn with_id(mut self, id: PlayerCharacterId) -> Self {
        self.id = id;
        self
    }

    /// Set the character's lifecycle state (used when loading from storage).
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::aggregates::player_character::PlayerCharacter;
    /// use wrldbldr_domain::value_objects::{CharacterName, CharacterState};
    /// use wrldbldr_domain::{WorldId, LocationId, UserId};
    /// use chrono::TimeZone;
    ///
    /// let name = CharacterName::new("Gandalf").unwrap();
    /// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    /// let user_id = UserId::new("user1").unwrap();
    /// let pc = PlayerCharacter::new(user_id, WorldId::new(), name, LocationId::new(), now)
    ///     .with_state(CharacterState::Inactive);
    ///
    /// assert!(pc.is_inactive());
    /// assert!(pc.is_alive());
    /// assert!(!pc.is_active());
    /// ```
    pub fn with_state(mut self, state: CharacterState) -> Self {
        self.state = state;
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
    pub fn set_description(&mut self, description: Option<impl Into<String>>) {
        self.description = description.and_then(|d| Description::new(d).ok());
    }

    /// Set the character sheet data.
    pub fn set_sheet_data(&mut self, sheet_data: Option<CharacterSheetValues>) {
        self.sheet_data = sheet_data;
    }

    /// Set the sprite asset path.
    pub fn set_sprite(&mut self, path: Option<AssetPath>) {
        self.sprite_asset = path;
    }

    /// Set the portrait asset path.
    pub fn set_portrait(&mut self, path: Option<AssetPath>) {
        self.portrait_asset = path;
    }

    /// Kill the character.
    ///
    /// Transitions the character to `Dead` state and returns a domain event
    /// indicating what happened.
    ///
    /// # Returns
    ///
    /// - `PlayerCharacterStateChange::Killed` if the character was alive and is now dead
    /// - `PlayerCharacterStateChange::AlreadyDead` if the character was already dead
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::aggregates::player_character::{PlayerCharacter, PlayerCharacterStateChange};
    /// use wrldbldr_domain::value_objects::CharacterName;
    /// use wrldbldr_domain::{WorldId, LocationId, UserId};
    /// use chrono::TimeZone;
    ///
    /// let name = CharacterName::new("Hero").unwrap();
    /// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    /// let user_id = UserId::new("user1").unwrap();
    /// let mut pc = PlayerCharacter::new(user_id, WorldId::new(), name, LocationId::new(), now);
    ///
    /// assert_eq!(pc.kill(), PlayerCharacterStateChange::Killed);
    /// assert!(pc.is_dead());
    ///
    /// // Killing again returns AlreadyDead
    /// assert_eq!(pc.kill(), PlayerCharacterStateChange::AlreadyDead);
    /// ```
    pub fn kill(&mut self) -> PlayerCharacterStateChange {
        if self.state.is_dead() {
            PlayerCharacterStateChange::AlreadyDead
        } else {
            self.state = CharacterState::Dead;
            PlayerCharacterStateChange::Killed
        }
    }

    /// Deactivate the character (still alive but not actively playing).
    ///
    /// Transitions the character from `Active` to `Inactive` state.
    /// Dead characters cannot be deactivated.
    ///
    /// # Returns
    ///
    /// - `PlayerCharacterStateChange::Deactivated` if the character was active and is now inactive
    /// - `PlayerCharacterStateChange::AlreadyInactive` if the character was already inactive or dead
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::aggregates::player_character::{PlayerCharacter, PlayerCharacterStateChange};
    /// use wrldbldr_domain::value_objects::CharacterName;
    /// use wrldbldr_domain::{WorldId, LocationId, UserId};
    /// use chrono::TimeZone;
    ///
    /// let name = CharacterName::new("Hero").unwrap();
    /// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    /// let user_id = UserId::new("user1").unwrap();
    /// let mut pc = PlayerCharacter::new(user_id, WorldId::new(), name, LocationId::new(), now);
    ///
    /// assert_eq!(pc.deactivate(), PlayerCharacterStateChange::Deactivated);
    /// assert!(pc.is_inactive());
    ///
    /// // Deactivating again returns AlreadyInactive
    /// assert_eq!(pc.deactivate(), PlayerCharacterStateChange::AlreadyInactive);
    /// ```
    pub fn deactivate(&mut self) -> PlayerCharacterStateChange {
        if self.state.is_active() {
            self.state = CharacterState::Inactive;
            PlayerCharacterStateChange::Deactivated
        } else {
            PlayerCharacterStateChange::AlreadyInactive
        }
    }

    /// Activate the character.
    ///
    /// Transitions the character from `Inactive` to `Active` state.
    /// Dead characters cannot be activated (use `resurrect()` instead).
    ///
    /// # Returns
    ///
    /// - `PlayerCharacterStateChange::Activated` if the character was inactive and is now active
    /// - `PlayerCharacterStateChange::AlreadyActive` if the character was already active or dead
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::aggregates::player_character::{PlayerCharacter, PlayerCharacterStateChange};
    /// use wrldbldr_domain::value_objects::CharacterName;
    /// use wrldbldr_domain::{WorldId, LocationId, UserId};
    /// use chrono::TimeZone;
    ///
    /// let name = CharacterName::new("Hero").unwrap();
    /// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    /// let user_id = UserId::new("user1").unwrap();
    /// let mut pc = PlayerCharacter::new(user_id, WorldId::new(), name, LocationId::new(), now);
    ///
    /// pc.deactivate();
    /// assert_eq!(pc.activate(), PlayerCharacterStateChange::Activated);
    /// assert!(pc.is_active());
    ///
    /// // Activating again returns AlreadyActive
    /// assert_eq!(pc.activate(), PlayerCharacterStateChange::AlreadyActive);
    /// ```
    pub fn activate(&mut self) -> PlayerCharacterStateChange {
        if self.state.is_inactive() {
            self.state = CharacterState::Active;
            PlayerCharacterStateChange::Activated
        } else {
            PlayerCharacterStateChange::AlreadyActive
        }
    }

    /// Resurrect the character.
    ///
    /// Transitions the character from `Dead` to `Active` state.
    ///
    /// # Returns
    ///
    /// - `PlayerCharacterStateChange::Resurrected` if the character was dead and is now active
    /// - `PlayerCharacterStateChange::AlreadyAlive` if the character was already alive
    ///
    /// # Examples
    ///
    /// ```
    /// use wrldbldr_domain::aggregates::player_character::{PlayerCharacter, PlayerCharacterStateChange};
    /// use wrldbldr_domain::value_objects::CharacterName;
    /// use wrldbldr_domain::{WorldId, LocationId, UserId};
    /// use chrono::TimeZone;
    ///
    /// let name = CharacterName::new("Hero").unwrap();
    /// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    /// let user_id = UserId::new("user1").unwrap();
    /// let mut pc = PlayerCharacter::new(user_id, WorldId::new(), name, LocationId::new(), now);
    ///
    /// pc.kill();
    /// assert_eq!(pc.resurrect(), PlayerCharacterStateChange::Resurrected);
    /// assert!(pc.is_alive());
    /// assert!(pc.is_active());
    ///
    /// // Resurrecting again returns AlreadyAlive
    /// assert_eq!(pc.resurrect(), PlayerCharacterStateChange::AlreadyAlive);
    /// ```
    pub fn resurrect(&mut self) -> PlayerCharacterStateChange {
        if self.state.is_dead() {
            self.state = CharacterState::Active;
            PlayerCharacterStateChange::Resurrected
        } else {
            PlayerCharacterStateChange::AlreadyAlive
        }
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate that the character has required fields.
    /// Note: With newtypes, most validation happens at construction time.
    pub fn validate(&self) -> Result<(), DomainError> {
        // CharacterName already validates non-empty at construction
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_time() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    fn create_test_pc() -> PlayerCharacter {
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let name = CharacterName::new("Test Hero").unwrap();
        let now = fixed_time();
        let user_id = UserId::new("user123").unwrap();
        PlayerCharacter::new(user_id, world_id, name, location_id, now)
    }

    mod constructor {
        use super::*;

        #[test]
        fn new_creates_pc_with_correct_defaults() {
            let world_id = WorldId::new();
            let location_id = LocationId::new();
            let name = CharacterName::new("Frodo").unwrap();
            let now = fixed_time();
            let user_id = UserId::new("user123").unwrap();
            let pc = PlayerCharacter::new(user_id, world_id, name, location_id, now);

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
            let now = fixed_time();
            let user_id = UserId::new("user456").unwrap();

            let pc = PlayerCharacter::new(user_id, world_id, name, location_id, now)
                .with_description("A loyal gardener")
                .with_starting_region(region_id)
                .with_sprite(AssetPath::new("sprites/sam.png").unwrap())
                .with_portrait(AssetPath::new("portraits/sam.png").unwrap());

            assert_eq!(pc.description(), Some("A loyal gardener"));
            assert_eq!(pc.current_region_id(), Some(region_id));
            assert_eq!(
                pc.sprite_asset().map(AssetPath::as_str),
                Some("sprites/sam.png")
            );
            assert_eq!(
                pc.portrait_asset().map(AssetPath::as_str),
                Some("portraits/sam.png")
            );
        }
    }

    mod mutation {
        use super::*;

        #[test]
        fn update_location_clears_region() {
            let mut pc = create_test_pc();
            let region_id = RegionId::new();
            let new_location = LocationId::new();
            let now = fixed_time();

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
            let now = fixed_time();

            pc.update_region(region_id, now);
            assert_eq!(pc.current_region_id(), Some(region_id));
        }

        #[test]
        fn update_position_works() {
            let mut pc = create_test_pc();
            let new_location = LocationId::new();
            let region_id = RegionId::new();
            let now = fixed_time();

            pc.update_position(new_location, Some(region_id), now);
            assert_eq!(pc.current_location_id(), new_location);
            assert_eq!(pc.current_region_id(), Some(region_id));
        }

        #[test]
        fn kill_sets_state_to_dead_and_returns_event() {
            let mut pc = create_test_pc();
            assert_eq!(pc.kill(), PlayerCharacterStateChange::Killed);
            assert!(pc.is_dead());
            assert!(!pc.is_alive());
            assert!(!pc.is_active());

            // Killing again returns AlreadyDead
            assert_eq!(pc.kill(), PlayerCharacterStateChange::AlreadyDead);
        }

        #[test]
        fn deactivate_sets_state_to_inactive_and_returns_event() {
            let mut pc = create_test_pc();
            assert_eq!(pc.deactivate(), PlayerCharacterStateChange::Deactivated);
            assert!(pc.is_inactive());
            assert!(pc.is_alive());
            assert!(!pc.is_active());

            // Deactivating again returns AlreadyInactive
            assert_eq!(pc.deactivate(), PlayerCharacterStateChange::AlreadyInactive);
        }

        #[test]
        fn activate_works_only_if_inactive_and_returns_event() {
            let mut pc = create_test_pc();

            // Already active
            assert_eq!(pc.activate(), PlayerCharacterStateChange::AlreadyActive);

            // Deactivate then reactivate
            pc.deactivate();
            assert_eq!(pc.activate(), PlayerCharacterStateChange::Activated);
            assert!(pc.is_active());

            // Kill then try to activate - should return AlreadyActive (dead chars can't activate)
            pc.kill();
            assert_eq!(pc.activate(), PlayerCharacterStateChange::AlreadyActive);
            assert!(!pc.is_active());
        }

        #[test]
        fn resurrect_sets_state_to_active_and_returns_event() {
            let mut pc = create_test_pc();
            pc.kill();
            assert_eq!(pc.resurrect(), PlayerCharacterStateChange::Resurrected);
            assert!(pc.is_alive());
            assert!(pc.is_active());
            assert_eq!(pc.state(), CharacterState::Active);

            // Resurrecting again returns AlreadyAlive
            assert_eq!(pc.resurrect(), PlayerCharacterStateChange::AlreadyAlive);
        }

        #[test]
        fn state_accessor_returns_correct_values() {
            let mut pc = create_test_pc();
            assert_eq!(pc.state(), CharacterState::Active);

            pc.deactivate();
            assert_eq!(pc.state(), CharacterState::Inactive);

            pc.kill();
            assert_eq!(pc.state(), CharacterState::Dead);

            pc.resurrect();
            assert_eq!(pc.state(), CharacterState::Active);
        }

        #[test]
        fn with_state_builder_works() {
            let world_id = WorldId::new();
            let location_id = LocationId::new();
            let name = CharacterName::new("Gimli").unwrap();
            let now = fixed_time();
            let user_id = UserId::new("user789").unwrap();

            let pc = PlayerCharacter::new(user_id.clone(), world_id, name, location_id, now)
                .with_state(CharacterState::Inactive);

            assert_eq!(pc.state(), CharacterState::Inactive);
            assert!(pc.is_inactive());
            assert!(pc.is_alive());
            assert!(!pc.is_active());

            let dead_pc = PlayerCharacter::new(
                user_id,
                world_id,
                CharacterName::new("Ghost").unwrap(),
                location_id,
                now,
            )
            .with_state(CharacterState::Dead);

            assert_eq!(dead_pc.state(), CharacterState::Dead);
            assert!(dead_pc.is_dead());
            assert!(!dead_pc.is_alive());
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
}
