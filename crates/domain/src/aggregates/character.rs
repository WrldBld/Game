//! Character aggregate - NPCs with Campbell archetypes
//!
//! # Graph-First Design (Phase 0.C)
//!
//! The following relationships are stored as Neo4j edges, NOT embedded fields:
//! - Wants: `(Character)-[:HAS_WANT]->(Want)`
//! - Inventory: `(Character)-[:POSSESSES]->(Item)`
//! - Location relationships: `HOME_LOCATION`, `WORKS_AT`, `FREQUENTS`, `AVOIDS`
//! - Actantial views: `VIEWS_AS_HELPER`, `VIEWS_AS_OPPONENT`, etc.
//!
//! Archetype history remains as JSON (acceptable per ADR - complex nested non-relational)
//!
//! # Rustic DDD Design
//!
//! This aggregate follows Rustic DDD principles:
//! - **Private fields**: All fields are encapsulated
//! - **Newtypes**: `CharacterName` and `Description` for validated strings
//! - **State enum**: `CharacterState` replaces `is_alive`/`is_active` booleans
//! - **Domain events**: Mutations return outcome enums (`DamageOutcome`, etc.)
//! - **Valid by construction**: `new()` takes pre-validated types

use serde::{Deserialize, Serialize};

use crate::events::{
    ArchetypeShift, CharacterStateChange, CharacterUpdate, DamageOutcome, HealOutcome,
    ResurrectOutcome,
};
use crate::value_objects::{
    ArchetypeChange, AssetPath, CampbellArchetype, CharacterName, CharacterState, Description,
    DispositionLevel, ExpressionConfig, MoodState,
};
use wrldbldr_domain::{CharacterId, WorldId};

// Re-export from value_objects (StatBlock, StatModifier, StatValue)
pub use crate::value_objects::{StatBlock, StatModifier, StatValue};

/// A character (NPC) in the world
///
/// # Invariants
///
/// - `name` is always non-empty and <= 200 characters (enforced by `CharacterName`)
/// - `description` is always <= 5000 characters (enforced by `Description`)
/// - State is always one of `Active`, `Inactive`, or `Dead` (enforced by `CharacterState`)
///
/// # Example
///
/// ```
/// use wrldbldr_domain::{WorldId, CharacterId};
/// use wrldbldr_domain::aggregates::Character;
/// use wrldbldr_domain::value_objects::{CharacterName, CampbellArchetype, Description};
///
/// let world_id = WorldId::new();
/// let name = CharacterName::new("Gandalf").unwrap();
/// let character = Character::new(world_id, name, CampbellArchetype::Mentor);
///
/// assert_eq!(character.name().as_str(), "Gandalf");
/// assert!(character.is_alive());
/// assert!(character.is_active());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    // Identity
    id: CharacterId,
    world_id: WorldId,

    // Core attributes (newtypes)
    name: CharacterName,
    description: Description,

    // Assets
    sprite_asset: Option<AssetPath>,
    portrait_asset: Option<AssetPath>,

    // Campbell Archetype System (Layered)
    base_archetype: CampbellArchetype,
    current_archetype: CampbellArchetype,
    archetype_history: Vec<ArchetypeChange>,

    // Game Stats (stored as JSON - acceptable per ADR)
    stats: StatBlock,

    // Character state (enum, not booleans)
    state: CharacterState,

    // Default disposition for this NPC
    default_disposition: DispositionLevel,

    // Mood & Expression System (Three-Tier Model)
    default_mood: MoodState,
    expression_config: ExpressionConfig,
}

impl Character {
    // =========================================================================
    // Constructor
    // =========================================================================

    /// Create a new character with the given world, name, and archetype.
    ///
    /// The `name` parameter must be a pre-validated `CharacterName` - validation
    /// happens when creating the `CharacterName`, not here.
    ///
    /// # Example
    ///
    /// ```
    /// use wrldbldr_domain::{WorldId, CharacterId};
    /// use wrldbldr_domain::aggregates::Character;
    /// use wrldbldr_domain::value_objects::{CharacterName, CampbellArchetype};
    ///
    /// let world_id = WorldId::new();
    /// let name = CharacterName::new("Aragorn").unwrap();
    /// let character = Character::new(world_id, name, CampbellArchetype::Hero);
    ///
    /// assert_eq!(character.name().as_str(), "Aragorn");
    /// ```
    pub fn new(world_id: WorldId, name: CharacterName, archetype: CampbellArchetype) -> Self {
        Self {
            id: CharacterId::new(),
            world_id,
            name,
            description: Description::empty(),
            sprite_asset: None,
            portrait_asset: None,
            base_archetype: archetype,
            current_archetype: archetype,
            archetype_history: Vec::new(),
            stats: StatBlock::default(),
            state: CharacterState::Active,
            default_disposition: DispositionLevel::Neutral,
            default_mood: MoodState::default(),
            expression_config: ExpressionConfig::default(),
        }
    }

    // =========================================================================
    // Identity Accessors (read-only)
    // =========================================================================

    /// Returns the character's unique identifier.
    #[inline]
    pub fn id(&self) -> CharacterId {
        self.id
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
    pub fn description(&self) -> &Description {
        &self.description
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
    // Archetype Accessors
    // =========================================================================

    /// Returns the character's base archetype.
    #[inline]
    pub fn base_archetype(&self) -> CampbellArchetype {
        self.base_archetype
    }

    /// Returns the character's current archetype.
    #[inline]
    pub fn current_archetype(&self) -> CampbellArchetype {
        self.current_archetype
    }

    /// Returns the history of archetype changes.
    #[inline]
    pub fn archetype_history(&self) -> &[ArchetypeChange] {
        &self.archetype_history
    }

    // =========================================================================
    // Stats Accessors
    // =========================================================================

    /// Returns a reference to the character's stats.
    #[inline]
    pub fn stats(&self) -> &StatBlock {
        &self.stats
    }

    /// Replace the character's stats with a new StatBlock.
    ///
    /// This is used when you need to modify stats using the builder pattern:
    /// ```ignore
    /// character.set_stats(character.stats().clone().with_stat("STR", 18));
    /// ```
    #[inline]
    pub fn set_stats(&mut self, stats: StatBlock) {
        self.stats = stats;
    }

    // =========================================================================
    // State Accessors
    // =========================================================================

    /// Returns the character's current state.
    #[inline]
    pub fn state(&self) -> CharacterState {
        self.state
    }

    /// Returns true if the character is alive (Active or Inactive).
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.state.is_alive()
    }

    /// Returns true if the character is actively participating (Active state).
    #[inline]
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Returns true if the character is dead.
    #[inline]
    pub fn is_dead(&self) -> bool {
        self.state.is_dead()
    }

    // =========================================================================
    // Disposition/Mood Accessors
    // =========================================================================

    /// Returns the character's default disposition level.
    #[inline]
    pub fn default_disposition(&self) -> DispositionLevel {
        self.default_disposition
    }

    /// Returns the character's default mood state.
    #[inline]
    pub fn default_mood(&self) -> &MoodState {
        &self.default_mood
    }

    /// Returns the character's expression configuration.
    #[inline]
    pub fn expression_config(&self) -> &ExpressionConfig {
        &self.expression_config
    }

    // =========================================================================
    // Builder Methods (for construction)
    // =========================================================================

    /// Set the character's default disposition.
    pub fn with_default_disposition(mut self, disposition: DispositionLevel) -> Self {
        self.default_disposition = disposition;
        self
    }

    /// Set the character's default mood.
    pub fn with_default_mood(mut self, mood: MoodState) -> Self {
        self.default_mood = mood;
        self
    }

    /// Set the character's expression configuration.
    pub fn with_expression_config(mut self, config: ExpressionConfig) -> Self {
        self.expression_config = config;
        self
    }

    /// Set the character's description.
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = description;
        self
    }

    /// Set the character's sprite asset path.
    pub fn with_sprite(mut self, asset_path: AssetPath) -> Self {
        self.sprite_asset = Some(asset_path);
        self
    }

    /// Set the character's portrait asset path.
    pub fn with_portrait(mut self, asset_path: AssetPath) -> Self {
        self.portrait_asset = Some(asset_path);
        self
    }

    /// Set the character's stats.
    pub fn with_stats(mut self, stats: StatBlock) -> Self {
        self.stats = stats;
        self
    }

    /// Set the character's ID (used when loading from storage).
    pub fn with_id(mut self, id: CharacterId) -> Self {
        self.id = id;
        self
    }

    /// Set the character's state (used when loading from storage).
    pub fn with_state(mut self, state: CharacterState) -> Self {
        self.state = state;
        self
    }

    /// Set the character's archetype history (used when loading from storage).
    pub fn with_archetype_history(mut self, history: Vec<ArchetypeChange>) -> Self {
        self.archetype_history = history;
        self
    }

    /// Set the character's current archetype (used when loading from storage).
    pub fn with_current_archetype(mut self, archetype: CampbellArchetype) -> Self {
        self.current_archetype = archetype;
        self
    }

    // =========================================================================
    // Mutation Methods (return domain events)
    // =========================================================================

    /// Apply damage to the character.
    ///
    /// Returns a `DamageOutcome` indicating what happened:
    /// - `AlreadyDead` if the character was already dead
    /// - `NoHpTracking` if the character has no HP configured
    /// - `Wounded` if the character took damage but survived
    /// - `Killed` if this damage killed the character
    ///
    /// # Example
    ///
    /// ```
    /// use wrldbldr_domain::{WorldId, DamageOutcome};
    /// use wrldbldr_domain::aggregates::{Character, StatBlock};
    /// use wrldbldr_domain::value_objects::{CharacterName, CampbellArchetype};
    ///
    /// let world_id = WorldId::new();
    /// let name = CharacterName::new("Boromir").unwrap();
    /// let mut character = Character::new(world_id, name, CampbellArchetype::Hero)
    ///     .with_stats(StatBlock::new().with_hp(50, 50));
    ///
    /// match character.apply_damage(30) {
    ///     DamageOutcome::Wounded { damage_dealt, remaining_hp } => {
    ///         assert_eq!(damage_dealt, 30);
    ///         assert_eq!(remaining_hp, 20);
    ///     }
    ///     _ => panic!("Expected Wounded outcome"),
    /// }
    /// ```
    pub fn apply_damage(&mut self, amount: i32) -> DamageOutcome {
        // Can't damage the dead
        if self.state.is_dead() {
            return DamageOutcome::AlreadyDead;
        }

        // Check if HP tracking is enabled
        let (current_hp, max_hp): (i32, i32) = match (self.stats.current_hp(), self.stats.max_hp())
        {
            (Some(current), Some(max)) => (current, max),
            _ => return DamageOutcome::NoHpTracking,
        };
        let _ = max_hp; // Silence unused variable warning

        // Apply damage
        let new_hp = current_hp.saturating_sub(amount);
        self.stats = std::mem::take(&mut self.stats).with_current_hp(Some(new_hp));

        if new_hp <= 0 {
            self.state = CharacterState::Dead;
            DamageOutcome::Killed {
                damage_dealt: amount,
            }
        } else {
            DamageOutcome::Wounded {
                damage_dealt: amount,
                remaining_hp: new_hp,
            }
        }
    }

    /// Heal the character.
    ///
    /// Returns a `HealOutcome` indicating what happened:
    /// - `Dead` if the character is dead (use `resurrect` instead)
    /// - `NoHpTracking` if the character has no HP configured
    /// - `AlreadyFull` if the character is already at max HP
    /// - `Healed` with the actual amount healed and new HP
    ///
    /// # Example
    ///
    /// ```
    /// use wrldbldr_domain::{WorldId, HealOutcome};
    /// use wrldbldr_domain::aggregates::{Character, StatBlock};
    /// use wrldbldr_domain::value_objects::{CharacterName, CampbellArchetype};
    ///
    /// let world_id = WorldId::new();
    /// let name = CharacterName::new("Frodo").unwrap();
    /// let mut character = Character::new(world_id, name, CampbellArchetype::Hero)
    ///     .with_stats(StatBlock::new().with_hp(20, 50));
    ///
    /// match character.heal(15) {
    ///     HealOutcome::Healed { amount_healed, new_hp } => {
    ///         assert_eq!(amount_healed, 15);
    ///         assert_eq!(new_hp, 35);
    ///     }
    ///     _ => panic!("Expected Healed outcome"),
    /// }
    /// ```
    pub fn heal(&mut self, amount: i32) -> HealOutcome {
        // Validate healing amount must be positive
        if amount <= 0 {
            return HealOutcome::InvalidAmount;
        }

        // Can't heal the dead
        if self.state.is_dead() {
            return HealOutcome::Dead;
        }

        // Check if HP tracking is enabled
        let (current_hp, max_hp): (i32, i32) = match (self.stats.current_hp(), self.stats.max_hp())
        {
            (Some(current), Some(max)) => (current, max),
            _ => return HealOutcome::NoHpTracking,
        };

        // Check if already at max
        if current_hp >= max_hp {
            return HealOutcome::AlreadyFull;
        }

        // Apply healing, capped at max HP
        // Use saturating_add to prevent overflow, then cap at max_hp
        let new_hp: i32 = current_hp.saturating_add(amount).min(max_hp);
        let actual_healed = new_hp - current_hp;
        self.stats = std::mem::take(&mut self.stats).with_current_hp(Some(new_hp));

        HealOutcome::Healed {
            amount_healed: actual_healed,
            new_hp,
        }
    }

    /// Resurrect a dead character.
    ///
    /// Returns a `ResurrectOutcome` indicating what happened:
    /// - `NotDead` if the character was not dead
    /// - `Resurrected` with the HP restored to
    ///
    /// The character is restored to Active state with HP set to half max (minimum 1).
    /// If no HP tracking is configured, HP is set to 1.
    ///
    /// # Example
    ///
    /// ```
    /// use wrldbldr_domain::{WorldId, ResurrectOutcome, DamageOutcome};
    /// use wrldbldr_domain::aggregates::{Character, StatBlock};
    /// use wrldbldr_domain::value_objects::{CharacterName, CampbellArchetype};
    ///
    /// let world_id = WorldId::new();
    /// let name = CharacterName::new("Gandalf").unwrap();
    /// let mut character = Character::new(world_id, name, CampbellArchetype::Mentor)
    ///     .with_stats(StatBlock::new().with_hp(100, 100));
    ///
    /// // Kill the character
    /// character.apply_damage(200);
    /// assert!(character.is_dead());
    ///
    /// // Resurrect
    /// match character.resurrect() {
    ///     ResurrectOutcome::Resurrected { hp_restored_to } => {
    ///         assert_eq!(hp_restored_to, 50); // Half of max
    ///         assert!(character.is_alive());
    ///     }
    ///     _ => panic!("Expected Resurrected outcome"),
    /// }
    /// ```
    pub fn resurrect(&mut self) -> ResurrectOutcome {
        // Can only resurrect the dead
        if !self.state.is_dead() {
            return ResurrectOutcome::NotDead;
        }

        // Calculate HP to restore to
        let hp_restored_to: i32 = match self.stats.max_hp() {
            Some(max) => (max / 2).max(1),
            None => 1,
        };

        self.stats = std::mem::take(&mut self.stats).with_current_hp(Some(hp_restored_to));
        self.state = CharacterState::Active;

        ResurrectOutcome::Resurrected { hp_restored_to }
    }

    /// Set the character to inactive state.
    ///
    /// Has no effect if the character is dead.
    pub fn deactivate(&mut self) -> CharacterStateChange {
        let previous = self.state;
        if self.state.is_alive() && !matches!(self.state, CharacterState::Inactive) {
            self.state = CharacterState::Inactive;
            return CharacterStateChange::StateChanged {
                from: previous,
                to: self.state,
            };
        }
        CharacterStateChange::Unchanged { state: self.state }
    }

    /// Set the character to active state.
    ///
    /// Has no effect if the character is dead.
    pub fn activate(&mut self) -> CharacterStateChange {
        let previous = self.state;
        if self.state.is_alive() && !matches!(self.state, CharacterState::Active) {
            self.state = CharacterState::Active;
            return CharacterStateChange::StateChanged {
                from: previous,
                to: self.state,
            };
        }
        CharacterStateChange::Unchanged { state: self.state }
    }

    /// Change the character's current archetype with a recorded reason.
    pub fn change_archetype(
        &mut self,
        new_archetype: CampbellArchetype,
        reason: impl Into<String>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> ArchetypeShift {
        let previous = self.current_archetype;
        let reason = reason.into();
        let change =
            ArchetypeChange::new(self.current_archetype, new_archetype, reason.clone(), now);
        self.archetype_history.push(change);
        self.current_archetype = new_archetype;

        ArchetypeShift {
            from: previous,
            to: self.current_archetype,
            reason,
        }
    }

    /// Temporarily assume a different archetype (for a scene).
    ///
    /// This only changes the current archetype, not the base, and doesn't
    /// record in history (it's temporary).
    pub fn assume_archetype(&mut self, archetype: CampbellArchetype) -> ArchetypeShift {
        let previous = self.current_archetype;
        self.current_archetype = archetype;
        ArchetypeShift {
            from: previous,
            to: self.current_archetype,
            reason: "assumed archetype".to_string(),
        }
    }

    /// Revert to base archetype.
    pub fn revert_to_base(&mut self) -> ArchetypeShift {
        let previous = self.current_archetype;
        self.current_archetype = self.base_archetype;
        ArchetypeShift {
            from: previous,
            to: self.current_archetype,
            reason: "reverted to base archetype".to_string(),
        }
    }

    /// Set the character's name.
    pub fn set_name(&mut self, name: CharacterName) -> CharacterUpdate {
        let previous = std::mem::replace(&mut self.name, name);
        CharacterUpdate::NameChanged {
            from: previous,
            to: self.name.clone(),
        }
    }

    /// Set the character's description.
    pub fn set_description(&mut self, description: Description) -> CharacterUpdate {
        let previous = std::mem::replace(&mut self.description, description);
        CharacterUpdate::DescriptionChanged {
            from: previous,
            to: self.description.clone(),
        }
    }

    /// Set the character's sprite asset path.
    pub fn set_sprite(&mut self, path: Option<AssetPath>) -> CharacterUpdate {
        let previous = std::mem::replace(&mut self.sprite_asset, path);
        CharacterUpdate::SpriteChanged {
            from: previous,
            to: self.sprite_asset.clone(),
        }
    }

    /// Set the character's portrait asset path.
    pub fn set_portrait(&mut self, path: Option<AssetPath>) -> CharacterUpdate {
        let previous = std::mem::replace(&mut self.portrait_asset, path);
        CharacterUpdate::PortraitChanged {
            from: previous,
            to: self.portrait_asset.clone(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};

    fn fixed_time() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    fn create_test_character() -> Character {
        let world_id = WorldId::new();
        let name = CharacterName::new("Test Hero").unwrap();
        Character::new(world_id, name, CampbellArchetype::Hero)
    }

    mod constructor {
        use super::*;

        #[test]
        fn new_creates_character_with_correct_defaults() {
            let world_id = WorldId::new();
            let name = CharacterName::new("Gandalf").unwrap();
            let character = Character::new(world_id, name, CampbellArchetype::Mentor);

            assert_eq!(character.name().as_str(), "Gandalf");
            assert_eq!(character.world_id(), world_id);
            assert_eq!(character.base_archetype(), CampbellArchetype::Mentor);
            assert_eq!(character.current_archetype(), CampbellArchetype::Mentor);
            assert!(character.is_alive());
            assert!(character.is_active());
            assert!(!character.is_dead());
            assert!(character.description().is_empty());
            assert!(character.sprite_asset().is_none());
            assert!(character.portrait_asset().is_none());
        }

        #[test]
        fn builder_methods_work() {
            let world_id = WorldId::new();
            let name = CharacterName::new("Frodo").unwrap();
            let desc = Description::new("A hobbit").unwrap();
            let sprite = AssetPath::new("sprites/frodo.png").unwrap();
            let portrait = AssetPath::new("portraits/frodo.png").unwrap();
            let character = Character::new(world_id, name, CampbellArchetype::Hero)
                .with_description(desc)
                .with_sprite(sprite)
                .with_portrait(portrait)
                .with_default_disposition(DispositionLevel::Friendly);

            assert_eq!(character.description().as_str(), "A hobbit");
            assert_eq!(
                character.sprite_asset().map(|p| p.as_str()),
                Some("sprites/frodo.png")
            );
            assert_eq!(
                character.portrait_asset().map(|p| p.as_str()),
                Some("portraits/frodo.png")
            );
            assert_eq!(character.default_disposition(), DispositionLevel::Friendly);
        }
    }

    mod damage {
        use super::*;

        #[test]
        fn apply_damage_without_hp_tracking_returns_no_hp_tracking() {
            let mut character = create_test_character();
            let outcome = character.apply_damage(10);
            assert_eq!(outcome, DamageOutcome::NoHpTracking);
        }

        #[test]
        fn apply_damage_wounds_character() {
            let mut character =
                create_test_character().with_stats(StatBlock::new().with_hp(50, 50));

            let outcome = character.apply_damage(20);
            assert_eq!(
                outcome,
                DamageOutcome::Wounded {
                    damage_dealt: 20,
                    remaining_hp: 30
                }
            );
            assert!(character.is_alive());
            assert_eq!(character.stats().current_hp(), Some(30));
        }

        #[test]
        fn apply_damage_kills_character() {
            let mut character =
                create_test_character().with_stats(StatBlock::new().with_hp(20, 50));

            let outcome = character.apply_damage(30);
            assert_eq!(outcome, DamageOutcome::Killed { damage_dealt: 30 });
            assert!(character.is_dead());
        }

        #[test]
        fn apply_damage_to_dead_character_returns_already_dead() {
            let mut character =
                create_test_character().with_stats(StatBlock::new().with_hp(10, 50));

            character.apply_damage(100); // Kill
            let outcome = character.apply_damage(10); // Try again

            assert_eq!(outcome, DamageOutcome::AlreadyDead);
        }
    }

    mod healing {
        use super::*;

        #[test]
        fn heal_without_hp_tracking_returns_no_hp_tracking() {
            let mut character = create_test_character();
            let outcome = character.heal(10);
            assert_eq!(outcome, HealOutcome::NoHpTracking);
        }

        #[test]
        fn heal_heals_character() {
            let mut character =
                create_test_character().with_stats(StatBlock::new().with_hp(20, 50));

            let outcome = character.heal(15);
            assert_eq!(
                outcome,
                HealOutcome::Healed {
                    amount_healed: 15,
                    new_hp: 35
                }
            );
            assert_eq!(character.stats().current_hp(), Some(35));
        }

        #[test]
        fn heal_caps_at_max_hp() {
            let mut character =
                create_test_character().with_stats(StatBlock::new().with_hp(45, 50));

            let outcome = character.heal(20);
            assert_eq!(
                outcome,
                HealOutcome::Healed {
                    amount_healed: 5,
                    new_hp: 50
                }
            );
        }

        #[test]
        fn heal_at_max_hp_returns_already_full() {
            let mut character =
                create_test_character().with_stats(StatBlock::new().with_hp(50, 50));

            let outcome = character.heal(10);
            assert_eq!(outcome, HealOutcome::AlreadyFull);
        }

        #[test]
        fn heal_dead_character_returns_dead() {
            let mut character =
                create_test_character().with_stats(StatBlock::new().with_hp(10, 50));

            character.apply_damage(100); // Kill
            let outcome = character.heal(10);

            assert_eq!(outcome, HealOutcome::Dead);
        }
    }

    mod resurrection {
        use super::*;

        #[test]
        fn resurrect_alive_character_returns_not_dead() {
            let mut character = create_test_character();
            let outcome = character.resurrect();
            assert_eq!(outcome, ResurrectOutcome::NotDead);
        }

        #[test]
        fn resurrect_dead_character_restores_to_half_hp() {
            let mut character =
                create_test_character().with_stats(StatBlock::new().with_hp(100, 100));

            character.apply_damage(200); // Kill
            assert!(character.is_dead());

            let outcome = character.resurrect();
            assert_eq!(
                outcome,
                ResurrectOutcome::Resurrected { hp_restored_to: 50 }
            );
            assert!(character.is_alive());
            assert!(character.is_active());
            assert_eq!(character.stats().current_hp(), Some(50));
        }

        #[test]
        fn resurrect_without_hp_tracking_sets_hp_to_1() {
            let mut character = create_test_character().with_state(CharacterState::Dead);

            let outcome = character.resurrect();
            assert_eq!(outcome, ResurrectOutcome::Resurrected { hp_restored_to: 1 });
            assert_eq!(character.stats().current_hp(), Some(1));
        }
    }

    mod state_transitions {
        use super::*;

        #[test]
        fn deactivate_sets_inactive() {
            let mut character = create_test_character();
            assert!(character.is_active());

            character.deactivate();
            assert!(!character.is_active());
            assert!(character.is_alive());
            assert_eq!(character.state(), CharacterState::Inactive);
        }

        #[test]
        fn activate_sets_active() {
            let mut character = create_test_character();
            character.deactivate();
            assert!(!character.is_active());

            character.activate();
            assert!(character.is_active());
        }

        #[test]
        fn deactivate_dead_character_has_no_effect() {
            let mut character = create_test_character().with_state(CharacterState::Dead);

            character.deactivate();
            assert!(character.is_dead());
        }

        #[test]
        fn activate_dead_character_has_no_effect() {
            let mut character = create_test_character().with_state(CharacterState::Dead);

            character.activate();
            assert!(character.is_dead());
        }
    }

    mod archetype {
        use super::*;

        #[test]
        fn assume_archetype_changes_current_only() {
            let mut character = create_test_character();
            assert_eq!(character.current_archetype(), CampbellArchetype::Hero);
            assert_eq!(character.base_archetype(), CampbellArchetype::Hero);

            character.assume_archetype(CampbellArchetype::Shadow);
            assert_eq!(character.current_archetype(), CampbellArchetype::Shadow);
            assert_eq!(character.base_archetype(), CampbellArchetype::Hero);
            assert!(character.archetype_history().is_empty());
        }

        #[test]
        fn revert_to_base_restores_base_archetype() {
            let mut character = create_test_character();
            character.assume_archetype(CampbellArchetype::Shadow);
            character.revert_to_base();
            assert_eq!(character.current_archetype(), CampbellArchetype::Hero);
        }

        #[test]
        fn change_archetype_records_history() {
            let mut character = create_test_character();
            let now = fixed_time();

            character.change_archetype(CampbellArchetype::Mentor, "Character growth", now);

            assert_eq!(character.current_archetype(), CampbellArchetype::Mentor);
            assert_eq!(character.archetype_history().len(), 1);
            let change = &character.archetype_history()[0];
            assert_eq!(change.from(), CampbellArchetype::Hero);
            assert_eq!(change.to(), CampbellArchetype::Mentor);
            assert_eq!(change.reason(), "Character growth");
        }
    }
}
