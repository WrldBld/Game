//! StatBlock, StatModifier, and StatValue - Character stat management
//!
//! This module contains value objects for managing character stats with support
//! for base values and temporary modifiers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::StatModifierId;

/// A temporary modifier to a stat (from equipment, spells, conditions, etc.)
///
/// This is an immutable value object. Use builder-style methods to create
/// modified copies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatModifier {
    /// Unique identifier for this modifier
    id: StatModifierId,
    /// Source of the modifier (e.g., "Sword of Strength", "Bless spell", "Exhausted condition")
    source: String,
    /// The value to add (positive) or subtract (negative)
    value: i32,
    /// Whether this modifier is currently active
    active: bool,
}

impl StatModifier {
    /// Create a new active modifier with the given source and value.
    pub fn new(source: impl Into<String>, value: i32) -> Self {
        Self {
            id: StatModifierId::new(),
            source: source.into(),
            value,
            active: true,
        }
    }

    /// Create an inactive modifier (for tracking but not applying)
    pub fn inactive(source: impl Into<String>, value: i32) -> Self {
        Self {
            id: StatModifierId::new(),
            source: source.into(),
            value,
            active: false,
        }
    }

    /// Reconstruct from storage (database hydration)
    pub fn from_storage(id: StatModifierId, source: String, value: i32, active: bool) -> Self {
        Self {
            id,
            source,
            value,
            active,
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Read accessors
    // ──────────────────────────────────────────────────────────────────────────

    /// Get the unique identifier for this modifier.
    pub fn id(&self) -> StatModifierId {
        self.id
    }

    /// Get the source of this modifier (e.g., "Sword of Strength").
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the modifier value (positive = bonus, negative = penalty).
    pub fn value(&self) -> i32 {
        self.value
    }

    /// Check if this modifier is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Builder-style methods (consume self, return new instance)
    // ──────────────────────────────────────────────────────────────────────────

    /// Create a copy with the active state changed.
    pub fn with_active(self, active: bool) -> Self {
        Self { active, ..self }
    }

    /// Create a copy with the active state toggled.
    pub fn toggled(self) -> Self {
        Self {
            active: !self.active,
            ..self
        }
    }
}

/// Character stats (system-agnostic)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatBlock {
    /// Map of stat name to base value
    stats: HashMap<String, i32>,
    /// Map of stat name to modifiers affecting that stat
    #[serde(default)]
    modifiers: HashMap<String, Vec<StatModifier>>,
    /// Current hit points (private - use accessors)
    current_hp: Option<i32>,
    /// Maximum hit points (private - use accessors)
    max_hp: Option<i32>,
}

impl StatBlock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all stats (immutable view).
    pub fn stats(&self) -> &HashMap<String, i32> {
        &self.stats
    }

    /// Get all modifiers (immutable view).
    pub fn modifiers(&self) -> &HashMap<String, Vec<StatModifier>> {
        &self.modifiers
    }

    /// Get a mutable reference to stats for controlled modification.
    pub fn stats_mut(&mut self) -> &mut HashMap<String, i32> {
        &mut self.stats
    }

    /// Get a mutable reference to modifiers for controlled modification.
    pub fn modifiers_mut(&mut self) -> &mut HashMap<String, Vec<StatModifier>> {
        &mut self.modifiers
    }

    pub fn with_stat(mut self, name: impl Into<String>, value: i32) -> Self {
        self.stats.insert(name.into(), value);
        self
    }

    pub fn with_hp(mut self, current: i32, max: i32) -> Self {
        self.current_hp = Some(current);
        self.max_hp = Some(max);
        self
    }

    /// Get the base value of a stat (without modifiers)
    pub fn get_base_stat(&self, name: &str) -> Option<i32> {
        self.stats.get(name).copied()
    }

    /// Get the effective value of a stat (base + active modifiers)
    pub fn get_stat(&self, name: &str) -> Option<i32> {
        self.stats.get(name).map(|base| {
            let modifier_total = self.get_modifier_total(name);
            base + modifier_total
        })
    }

    /// Get the total of all active modifiers for a stat
    pub fn get_modifier_total(&self, name: &str) -> i32 {
        self.modifiers
            .get(name)
            .map(|mods| {
                mods.iter()
                    .filter(|m| m.is_active())
                    .map(|m| m.value())
                    .sum()
            })
            .unwrap_or(0)
    }

    /// Get all modifiers for a stat
    pub fn get_modifiers(&self, name: &str) -> &[StatModifier] {
        self.modifiers
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Set the base value of a stat
    pub fn set_stat(&mut self, name: impl Into<String>, value: i32) {
        self.stats.insert(name.into(), value);
    }

    /// Add a modifier to a stat.
    ///
    /// Note: Modifiers can be added to stats that don't have a base value yet.
    /// In this case, `get_stat()` will return `None` and the modifier will have
    /// no effect until a base value is set via `set_stat()`. This allows pre-staging
    /// modifiers (e.g., from equipment) before character creation is complete.
    pub fn add_modifier(&mut self, stat_name: impl Into<String>, modifier: StatModifier) {
        self.modifiers
            .entry(stat_name.into())
            .or_default()
            .push(modifier);
    }

    /// Remove a modifier by its ID
    pub fn remove_modifier(&mut self, stat_name: &str, modifier_id: StatModifierId) -> bool {
        if let Some(mods) = self.modifiers.get_mut(stat_name) {
            let len_before = mods.len();
            mods.retain(|m| m.id() != modifier_id);
            return mods.len() < len_before;
        }
        false
    }

    /// Toggle a modifier's active state
    ///
    /// Note: This method mutates in-place because modifiers are stored in a Vec
    /// and need to be updated by reference. The StatModifier itself is immutable
    /// but we replace it with a toggled copy.
    pub fn toggle_modifier(&mut self, stat_name: &str, modifier_id: StatModifierId) -> bool {
        if let Some(mods) = self.modifiers.get_mut(stat_name) {
            if let Some(idx) = mods.iter().position(|m| m.id() == modifier_id) {
                let modifier = mods.remove(idx);
                mods.insert(idx, modifier.toggled());
                return true;
            }
        }
        false
    }

    /// Clear all modifiers for a stat
    pub fn clear_modifiers(&mut self, stat_name: &str) {
        self.modifiers.remove(stat_name);
    }

    /// Clear all modifiers from all stats
    pub fn clear_all_modifiers(&mut self) {
        self.modifiers.clear();
    }

    // =========================================================================
    // HP Methods (with modifier support)
    // =========================================================================

    /// Get effective current HP (base + modifiers).
    ///
    /// Returns `None` if no base HP has been set.
    /// Modifiers on "current_hp" are added to the base value.
    pub fn get_current_hp(&self) -> Option<i32> {
        self.current_hp.map(|base| {
            let modifier_total = self.get_modifier_total("current_hp");
            base + modifier_total
        })
    }

    /// Get effective max HP (base + modifiers).
    ///
    /// Returns `None` if no max HP has been set.
    /// Modifiers on "max_hp" are added to the base value.
    pub fn get_max_hp(&self) -> Option<i32> {
        self.max_hp.map(|base| {
            let modifier_total = self.get_modifier_total("max_hp");
            base + modifier_total
        })
    }

    /// Get base current HP (without modifiers).
    pub fn get_base_current_hp(&self) -> Option<i32> {
        self.current_hp
    }

    /// Get base max HP (without modifiers).
    pub fn get_base_max_hp(&self) -> Option<i32> {
        self.max_hp
    }

    /// Get raw current HP value (alias for get_base_current_hp).
    ///
    /// Returns the stored current HP without applying any modifiers.
    /// For HP with modifiers applied, use `get_current_hp()`.
    #[inline]
    pub fn current_hp(&self) -> Option<i32> {
        self.current_hp
    }

    /// Get raw max HP value (alias for get_base_max_hp).
    ///
    /// Returns the stored max HP without applying any modifiers.
    /// For HP with modifiers applied, use `get_max_hp()`.
    #[inline]
    pub fn max_hp(&self) -> Option<i32> {
        self.max_hp
    }

    /// Set current HP directly.
    ///
    /// This sets the base HP value. Modifiers are applied on top when
    /// reading via `get_current_hp()`.
    pub fn set_current_hp(&mut self, hp: Option<i32>) {
        self.current_hp = hp;
    }

    /// Set max HP directly.
    ///
    /// This sets the base max HP value. Modifiers are applied on top when
    /// reading via `get_max_hp()`.
    pub fn set_max_hp(&mut self, hp: Option<i32>) {
        self.max_hp = hp;
    }

    /// Set both current and max HP with validation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Either value is negative
    /// - Current HP exceeds max HP
    ///
    /// # Example
    ///
    /// ```
    /// use wrldbldr_domain::value_objects::StatBlock;
    ///
    /// let mut stats = StatBlock::new();
    /// stats.set_hp(50, 100).expect("valid HP values");
    /// assert_eq!(stats.current_hp(), Some(50));
    /// assert_eq!(stats.max_hp(), Some(100));
    /// ```
    pub fn set_hp(&mut self, current: i32, max: i32) -> Result<(), crate::DomainError> {
        if current < 0 {
            return Err(crate::DomainError::validation("HP cannot be negative"));
        }
        if max < 0 {
            return Err(crate::DomainError::validation("Max HP cannot be negative"));
        }
        if current > max {
            return Err(crate::DomainError::validation(
                "Current HP cannot exceed max HP",
            ));
        }
        self.current_hp = Some(current);
        self.max_hp = Some(max);
        Ok(())
    }

    /// Add a temporary HP modifier (e.g., from "Aid" spell, "Inspiring Leader" feat).
    ///
    /// The modifier affects `get_current_hp()` calculations but not the stored base value.
    pub fn add_hp_modifier(&mut self, source: impl Into<String>, value: i32) {
        self.add_modifier("current_hp", StatModifier::new(source, value));
    }

    /// Add an inactive temporary HP modifier (tracked but not applied until activated).
    pub fn add_hp_modifier_inactive(&mut self, source: impl Into<String>, value: i32) {
        self.add_modifier("current_hp", StatModifier::inactive(source, value));
    }

    /// Add a max HP modifier (e.g., from "Heroes' Feast", Constitution changes).
    ///
    /// The modifier affects `get_max_hp()` calculations but not the stored base value.
    pub fn add_max_hp_modifier(&mut self, source: impl Into<String>, value: i32) {
        self.add_modifier("max_hp", StatModifier::new(source, value));
    }

    /// Get all current HP modifiers.
    pub fn get_hp_modifiers(&self) -> &[StatModifier] {
        self.get_modifiers("current_hp")
    }

    /// Get all max HP modifiers.
    pub fn get_max_hp_modifiers(&self) -> &[StatModifier] {
        self.get_modifiers("max_hp")
    }

    /// Get a summary of all stats with their effective values
    pub fn get_all_stats(&self) -> HashMap<String, StatValue> {
        self.stats
            .iter()
            .map(|(name, &base)| {
                let modifier_total = self.get_modifier_total(name);
                (name.clone(), StatValue::new(base, modifier_total))
            })
            .collect()
    }
}

/// A stat value with base, modifiers, and effective total
///
/// This is an immutable value object representing a computed stat snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatValue {
    /// The base value (before modifiers)
    base: i32,
    /// Sum of all active modifiers
    modifier_total: i32,
    /// Effective value (base + modifier_total)
    effective: i32,
}

impl StatValue {
    /// Create a new stat value from base and modifier total.
    ///
    /// The effective value is automatically computed as `base + modifier_total`.
    pub fn new(base: i32, modifier_total: i32) -> Self {
        Self {
            base,
            modifier_total,
            effective: base + modifier_total,
        }
    }

    /// Get the base value (before modifiers).
    pub fn base(&self) -> i32 {
        self.base
    }

    /// Get the sum of all active modifiers.
    pub fn modifier_total(&self) -> i32 {
        self.modifier_total
    }

    /// Get the effective value (base + modifier_total).
    pub fn effective(&self) -> i32 {
        self.effective
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_block_get_base_stat_returns_base_value() {
        let stats = StatBlock::new().with_stat("STR", 15);
        assert_eq!(stats.get_base_stat("STR"), Some(15));
        assert_eq!(stats.get_base_stat("DEX"), None);
    }

    #[test]
    fn stat_block_get_stat_without_modifiers_returns_base() {
        let stats = StatBlock::new().with_stat("STR", 15);
        assert_eq!(stats.get_stat("STR"), Some(15));
    }

    #[test]
    fn stat_block_get_stat_with_active_modifier_adds_value() {
        let mut stats = StatBlock::new().with_stat("STR", 15);
        stats.add_modifier("STR", StatModifier::new("Gauntlets of Ogre Power", 4));
        assert_eq!(stats.get_stat("STR"), Some(19));
        assert_eq!(stats.get_base_stat("STR"), Some(15));
    }

    #[test]
    fn stat_block_get_stat_with_inactive_modifier_ignores_value() {
        let mut stats = StatBlock::new().with_stat("STR", 15);
        stats.add_modifier("STR", StatModifier::inactive("Exhausted", -2));
        assert_eq!(stats.get_stat("STR"), Some(15));
    }

    #[test]
    fn stat_block_multiple_modifiers_sum_correctly() {
        let mut stats = StatBlock::new().with_stat("STR", 10);
        stats.add_modifier("STR", StatModifier::new("Belt of Giant Strength", 4));
        stats.add_modifier("STR", StatModifier::new("Bull's Strength spell", 4));
        stats.add_modifier("STR", StatModifier::inactive("Curse", -2));
        // 10 + 4 + 4 = 18 (curse is inactive)
        assert_eq!(stats.get_stat("STR"), Some(18));
        assert_eq!(stats.get_modifier_total("STR"), 8);
    }

    #[test]
    fn stat_block_negative_modifiers_work() {
        let mut stats = StatBlock::new().with_stat("STR", 15);
        stats.add_modifier("STR", StatModifier::new("Weakened condition", -4));
        assert_eq!(stats.get_stat("STR"), Some(11));
    }

    #[test]
    fn stat_block_remove_modifier_works() {
        let mut stats = StatBlock::new().with_stat("STR", 15);
        let modifier = StatModifier::new("Temporary Buff", 2);
        let modifier_id = modifier.id();
        stats.add_modifier("STR", modifier);

        assert_eq!(stats.get_stat("STR"), Some(17));
        assert!(stats.remove_modifier("STR", modifier_id));
        assert_eq!(stats.get_stat("STR"), Some(15));
    }

    #[test]
    fn stat_block_remove_nonexistent_modifier_returns_false() {
        let mut stats = StatBlock::new().with_stat("STR", 15);
        let fake_id = crate::StatModifierId::new();
        assert!(!stats.remove_modifier("STR", fake_id));
        assert!(!stats.remove_modifier("DEX", fake_id));
    }

    #[test]
    fn stat_block_toggle_modifier_works() {
        let mut stats = StatBlock::new().with_stat("DEX", 14);
        let modifier = StatModifier::new("Haste", 2);
        let modifier_id = modifier.id();
        stats.add_modifier("DEX", modifier);

        // Initially active
        assert_eq!(stats.get_stat("DEX"), Some(16));

        // Toggle off
        assert!(stats.toggle_modifier("DEX", modifier_id));
        assert_eq!(stats.get_stat("DEX"), Some(14));

        // Toggle back on
        assert!(stats.toggle_modifier("DEX", modifier_id));
        assert_eq!(stats.get_stat("DEX"), Some(16));
    }

    #[test]
    fn stat_block_clear_modifiers_for_stat() {
        let mut stats = StatBlock::new().with_stat("INT", 12).with_stat("WIS", 14);
        stats.add_modifier("INT", StatModifier::new("Book", 2));
        stats.add_modifier("INT", StatModifier::new("Headband", 4));
        stats.add_modifier("WIS", StatModifier::new("Periapt", 2));

        stats.clear_modifiers("INT");

        assert_eq!(stats.get_stat("INT"), Some(12));
        assert_eq!(stats.get_stat("WIS"), Some(16)); // WIS modifiers intact
    }

    #[test]
    fn stat_block_clear_all_modifiers() {
        let mut stats = StatBlock::new().with_stat("INT", 12).with_stat("WIS", 14);
        stats.add_modifier("INT", StatModifier::new("Book", 2));
        stats.add_modifier("WIS", StatModifier::new("Periapt", 2));

        stats.clear_all_modifiers();

        assert_eq!(stats.get_stat("INT"), Some(12));
        assert_eq!(stats.get_stat("WIS"), Some(14));
    }

    #[test]
    fn stat_block_get_all_stats_includes_modifiers() {
        let mut stats = StatBlock::new().with_stat("STR", 10).with_stat("DEX", 14);
        stats.add_modifier("STR", StatModifier::new("Belt", 4));

        let all = stats.get_all_stats();

        let str_value = all.get("STR").unwrap();
        assert_eq!(str_value.base(), 10);
        assert_eq!(str_value.modifier_total(), 4);
        assert_eq!(str_value.effective(), 14);

        let dex_value = all.get("DEX").unwrap();
        assert_eq!(dex_value.base(), 14);
        assert_eq!(dex_value.modifier_total(), 0);
        assert_eq!(dex_value.effective(), 14);
    }

    #[test]
    fn stat_block_get_modifiers_returns_all() {
        let mut stats = StatBlock::new().with_stat("CHA", 16);
        let m1 = StatModifier::new("Cloak", 2);
        let m2 = StatModifier::inactive("Curse", -1);
        stats.add_modifier("CHA", m1.clone());
        stats.add_modifier("CHA", m2.clone());

        let modifiers = stats.get_modifiers("CHA");
        assert_eq!(modifiers.len(), 2);
        assert_eq!(modifiers[0].source(), "Cloak");
        assert_eq!(modifiers[1].source(), "Curse");
    }

    #[test]
    fn stat_modifier_new_creates_active_modifier() {
        let modifier = StatModifier::new("Test Source", 5);
        assert_eq!(modifier.source(), "Test Source");
        assert_eq!(modifier.value(), 5);
        assert!(modifier.is_active());
    }

    #[test]
    fn stat_modifier_inactive_creates_inactive_modifier() {
        let modifier = StatModifier::inactive("Test Source", -3);
        assert_eq!(modifier.source(), "Test Source");
        assert_eq!(modifier.value(), -3);
        assert!(!modifier.is_active());
    }

    #[test]
    fn stat_block_hp_tracking() {
        let stats = StatBlock::new().with_hp(45, 50);
        assert_eq!(stats.current_hp(), Some(45));
        assert_eq!(stats.max_hp(), Some(50));
    }

    #[test]
    fn stat_block_hp_with_modifiers() {
        let mut stats = StatBlock::new().with_hp(45, 50);

        // Base values should be accessible
        assert_eq!(stats.get_base_current_hp(), Some(45));
        assert_eq!(stats.get_base_max_hp(), Some(50));

        // Without modifiers, effective equals base
        assert_eq!(stats.get_current_hp(), Some(45));
        assert_eq!(stats.get_max_hp(), Some(50));

        // Add a temporary HP modifier
        stats.add_hp_modifier("Aid Spell", 10);
        assert_eq!(stats.get_base_current_hp(), Some(45)); // Base unchanged
        assert_eq!(stats.get_current_hp(), Some(55)); // Effective includes modifier

        // Add a max HP modifier
        stats.add_max_hp_modifier("Constitution Boost", 5);
        assert_eq!(stats.get_base_max_hp(), Some(50)); // Base unchanged
        assert_eq!(stats.get_max_hp(), Some(55)); // Effective includes modifier
    }

    #[test]
    fn stat_block_hp_modifiers_stack() {
        let mut stats = StatBlock::new().with_hp(30, 30);

        stats.add_hp_modifier("Heroism", 10);
        stats.add_hp_modifier("Aid", 5);
        stats.add_hp_modifier("Inspiring Leader", 8);

        assert_eq!(stats.get_base_current_hp(), Some(30));
        assert_eq!(stats.get_current_hp(), Some(53)); // 30 + 10 + 5 + 8
    }

    #[test]
    fn stat_block_hp_negative_modifiers() {
        let mut stats = StatBlock::new().with_hp(50, 50);

        stats.add_hp_modifier("Poison", -10);
        stats.add_max_hp_modifier("Exhaustion", -5);

        assert_eq!(stats.get_current_hp(), Some(40));
        assert_eq!(stats.get_max_hp(), Some(45));
    }

    #[test]
    fn stat_block_hp_inactive_modifiers() {
        let mut stats = StatBlock::new().with_hp(40, 40);

        stats.add_hp_modifier_inactive("Dormant Blessing", 15);

        // Inactive modifier shouldn't affect effective HP
        assert_eq!(stats.get_current_hp(), Some(40));

        // But should be retrievable
        let modifiers = stats.get_hp_modifiers();
        assert_eq!(modifiers.len(), 1);
        assert_eq!(modifiers[0].source(), "Dormant Blessing");
        assert!(!modifiers[0].is_active());
    }

    #[test]
    fn stat_block_hp_toggle_modifier() {
        let mut stats = StatBlock::new().with_hp(30, 30);
        stats.add_hp_modifier("Rage Bonus", 10);

        let modifiers = stats.get_hp_modifiers();
        let modifier_id = modifiers[0].id();

        assert_eq!(stats.get_current_hp(), Some(40));

        // Toggle to inactive
        stats.toggle_modifier("current_hp", modifier_id);
        assert_eq!(stats.get_current_hp(), Some(30));

        // Toggle back to active
        stats.toggle_modifier("current_hp", modifier_id);
        assert_eq!(stats.get_current_hp(), Some(40));
    }

    #[test]
    fn stat_block_hp_without_base_values() {
        let stats = StatBlock::new();

        // No HP set, should return None
        assert_eq!(stats.get_base_current_hp(), None);
        assert_eq!(stats.get_base_max_hp(), None);
        assert_eq!(stats.get_current_hp(), None);
        assert_eq!(stats.get_max_hp(), None);
    }

    #[test]
    fn stat_value_struct_equality() {
        let v1 = StatValue::new(10, 4);
        let v2 = StatValue::new(10, 4);
        assert_eq!(v1, v2);
        assert_eq!(v1.base(), 10);
        assert_eq!(v1.modifier_total(), 4);
        assert_eq!(v1.effective(), 14);
    }
}
