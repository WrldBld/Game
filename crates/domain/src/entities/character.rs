//! Character entity - NPCs with Campbell archetypes
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

use crate::value_objects::{
    ArchetypeChange, CampbellArchetype, DispositionLevel, ExpressionConfig, MoodState,
};
use serde::{Deserialize, Serialize};
use wrldbldr_domain::{CharacterId, WorldId};

/// A character (NPC) in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub id: CharacterId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    /// Path to sprite image asset
    pub sprite_asset: Option<String>,
    /// Path to portrait image asset
    pub portrait_asset: Option<String>,

    // Campbell Archetype System (Layered)
    /// The character's base archetype
    pub base_archetype: CampbellArchetype,
    /// Current archetype (may differ from base)
    pub current_archetype: CampbellArchetype,
    /// History of archetype changes (stored as JSON - acceptable per ADR)
    pub archetype_history: Vec<ArchetypeChange>,

    // Game Stats (stored as JSON - acceptable per ADR)
    pub stats: StatBlock,

    // Character state
    pub is_alive: bool,
    pub is_active: bool,

    /// Default disposition for this NPC (used when no PC-specific disposition is set)
    pub default_disposition: DispositionLevel,

    // Mood & Expression System (Three-Tier Model)
    /// Default mood for this NPC (Tier 2)
    /// Used when staging doesn't specify a mood; affects default expression and dialogue tone
    pub default_mood: MoodState,
    /// Expression configuration (Tier 3)
    /// Defines available expressions and actions for this character's sprite sheet
    pub expression_config: ExpressionConfig,
}

impl Character {
    pub fn new(world_id: WorldId, name: impl Into<String>, archetype: CampbellArchetype) -> Self {
        Self {
            id: CharacterId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            sprite_asset: None,
            portrait_asset: None,
            base_archetype: archetype,
            current_archetype: archetype,
            archetype_history: Vec::new(),
            stats: StatBlock::default(),
            is_alive: true,
            is_active: true,
            default_disposition: DispositionLevel::Neutral,
            default_mood: MoodState::default(),
            expression_config: ExpressionConfig::default(),
        }
    }

    pub fn with_default_disposition(mut self, disposition: DispositionLevel) -> Self {
        self.default_disposition = disposition;
        self
    }

    pub fn with_default_mood(mut self, mood: MoodState) -> Self {
        self.default_mood = mood;
        self
    }

    pub fn with_expression_config(mut self, config: ExpressionConfig) -> Self {
        self.expression_config = config;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_sprite(mut self, asset_path: impl Into<String>) -> Self {
        self.sprite_asset = Some(asset_path.into());
        self
    }

    pub fn with_portrait(mut self, asset_path: impl Into<String>) -> Self {
        self.portrait_asset = Some(asset_path.into());
        self
    }

    /// Change the character's current archetype
    pub fn change_archetype(
        &mut self,
        new_archetype: CampbellArchetype,
        reason: impl Into<String>,
        now: chrono::DateTime<chrono::Utc>,
    ) {
        let change = ArchetypeChange {
            from: self.current_archetype,
            to: new_archetype,
            reason: reason.into(),
            timestamp: now,
        };
        self.archetype_history.push(change);
        self.current_archetype = new_archetype;
    }

    /// Temporarily assume a different archetype (for a scene)
    pub fn assume_archetype(&mut self, archetype: CampbellArchetype) {
        // Only changes current, doesn't record in history (temporary)
        self.current_archetype = archetype;
    }

    /// Revert to base archetype
    pub fn revert_to_base(&mut self) {
        self.current_archetype = self.base_archetype;
    }
}

/// A temporary modifier to a stat (from equipment, spells, conditions, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatModifier {
    /// Unique identifier for this modifier
    pub id: uuid::Uuid,
    /// Source of the modifier (e.g., "Sword of Strength", "Bless spell", "Exhausted condition")
    pub source: String,
    /// The value to add (positive) or subtract (negative)
    pub value: i32,
    /// Whether this modifier is currently active
    pub active: bool,
}

impl StatModifier {
    pub fn new(source: impl Into<String>, value: i32) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            source: source.into(),
            value,
            active: true,
        }
    }

    /// Create an inactive modifier (for tracking but not applying)
    pub fn inactive(source: impl Into<String>, value: i32) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            source: source.into(),
            value,
            active: false,
        }
    }
}

/// Character stats (system-agnostic)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatBlock {
    /// Map of stat name to base value
    stats: std::collections::HashMap<String, i32>,
    /// Map of stat name to modifiers affecting that stat
    #[serde(default)]
    modifiers: std::collections::HashMap<String, Vec<StatModifier>>,
    /// Current hit points
    pub current_hp: Option<i32>,
    /// Maximum hit points
    pub max_hp: Option<i32>,
}

impl StatBlock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all stats (immutable view).
    pub fn stats(&self) -> &std::collections::HashMap<String, i32> {
        &self.stats
    }

    /// Get all modifiers (immutable view).
    pub fn modifiers(&self) -> &std::collections::HashMap<String, Vec<StatModifier>> {
        &self.modifiers
    }

    /// Get a mutable reference to stats for controlled modification.
    pub fn stats_mut(&mut self) -> &mut std::collections::HashMap<String, i32> {
        &mut self.stats
    }

    /// Get a mutable reference to modifiers for controlled modification.
    pub fn modifiers_mut(&mut self) -> &mut std::collections::HashMap<String, Vec<StatModifier>> {
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
            .map(|mods| mods.iter().filter(|m| m.active).map(|m| m.value).sum())
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
    pub fn remove_modifier(&mut self, stat_name: &str, modifier_id: uuid::Uuid) -> bool {
        if let Some(mods) = self.modifiers.get_mut(stat_name) {
            let len_before = mods.len();
            mods.retain(|m| m.id != modifier_id);
            return mods.len() < len_before;
        }
        false
    }

    /// Toggle a modifier's active state
    pub fn toggle_modifier(&mut self, stat_name: &str, modifier_id: uuid::Uuid) -> bool {
        if let Some(mods) = self.modifiers.get_mut(stat_name) {
            if let Some(modifier) = mods.iter_mut().find(|m| m.id == modifier_id) {
                modifier.active = !modifier.active;
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
    pub fn get_all_stats(&self) -> std::collections::HashMap<String, StatValue> {
        self.stats
            .iter()
            .map(|(name, &base)| {
                let modifier_total = self.get_modifier_total(name);
                (
                    name.clone(),
                    StatValue {
                        base,
                        modifier_total,
                        effective: base + modifier_total,
                    },
                )
            })
            .collect()
    }
}

/// A stat value with base, modifiers, and effective total
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatValue {
    /// The base value (before modifiers)
    pub base: i32,
    /// Sum of all active modifiers
    pub modifier_total: i32,
    /// Effective value (base + modifier_total)
    pub effective: i32,
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
        let modifier_id = modifier.id;
        stats.add_modifier("STR", modifier);

        assert_eq!(stats.get_stat("STR"), Some(17));
        assert!(stats.remove_modifier("STR", modifier_id));
        assert_eq!(stats.get_stat("STR"), Some(15));
    }

    #[test]
    fn stat_block_remove_nonexistent_modifier_returns_false() {
        let mut stats = StatBlock::new().with_stat("STR", 15);
        let fake_id = uuid::Uuid::new_v4();
        assert!(!stats.remove_modifier("STR", fake_id));
        assert!(!stats.remove_modifier("DEX", fake_id));
    }

    #[test]
    fn stat_block_toggle_modifier_works() {
        let mut stats = StatBlock::new().with_stat("DEX", 14);
        let modifier = StatModifier::new("Haste", 2);
        let modifier_id = modifier.id;
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
        assert_eq!(str_value.base, 10);
        assert_eq!(str_value.modifier_total, 4);
        assert_eq!(str_value.effective, 14);

        let dex_value = all.get("DEX").unwrap();
        assert_eq!(dex_value.base, 14);
        assert_eq!(dex_value.modifier_total, 0);
        assert_eq!(dex_value.effective, 14);
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
        assert_eq!(modifiers[0].source, "Cloak");
        assert_eq!(modifiers[1].source, "Curse");
    }

    #[test]
    fn stat_modifier_new_creates_active_modifier() {
        let modifier = StatModifier::new("Test Source", 5);
        assert_eq!(modifier.source, "Test Source");
        assert_eq!(modifier.value, 5);
        assert!(modifier.active);
    }

    #[test]
    fn stat_modifier_inactive_creates_inactive_modifier() {
        let modifier = StatModifier::inactive("Test Source", -3);
        assert_eq!(modifier.source, "Test Source");
        assert_eq!(modifier.value, -3);
        assert!(!modifier.active);
    }

    #[test]
    fn stat_block_hp_tracking() {
        let stats = StatBlock::new().with_hp(45, 50);
        assert_eq!(stats.current_hp, Some(45));
        assert_eq!(stats.max_hp, Some(50));
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
        assert_eq!(modifiers[0].source, "Dormant Blessing");
        assert!(!modifiers[0].active);
    }

    #[test]
    fn stat_block_hp_toggle_modifier() {
        let mut stats = StatBlock::new().with_hp(30, 30);
        stats.add_hp_modifier("Rage Bonus", 10);

        let modifiers = stats.get_hp_modifiers();
        let modifier_id = modifiers[0].id;

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
        let v1 = StatValue {
            base: 10,
            modifier_total: 4,
            effective: 14,
        };
        let v2 = StatValue {
            base: 10,
            modifier_total: 4,
            effective: 14,
        };
        assert_eq!(v1, v2);
    }
}
