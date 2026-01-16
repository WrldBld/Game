//! Character content - spells, feats, and features owned by a character.
//!
//! These structs represent the character's personal collection of abilities,
//! including tracking of uses, preparation states, and choices made.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A character's spellcasting data.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSpells {
    /// Cantrips the character knows
    #[serde(default)]
    pub cantrips: Vec<String>,
    /// Spells the character knows (for known-spell casters)
    #[serde(default)]
    pub known: Vec<KnownSpell>,
    /// Currently prepared spells (spell IDs)
    #[serde(default)]
    pub prepared: Vec<String>,
    /// Spell slots by level (1-9)
    #[serde(default)]
    pub slots: HashMap<u8, SpellSlotPool>,
    /// Pact magic slots (for warlocks and similar)
    pub pact_slots: Option<SpellSlotPool>,
    /// Primary spellcasting ability (e.g., "INT", "WIS", "CHA")
    pub spellcasting_ability: Option<String>,
}

impl CharacterSpells {
    /// Create empty spellcasting data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a cantrip.
    pub fn add_cantrip(&mut self, spell_id: impl Into<String>) {
        let id = spell_id.into();
        if !self.cantrips.contains(&id) {
            self.cantrips.push(id);
        }
    }

    /// Learn a spell.
    pub fn learn_spell(&mut self, spell_id: impl Into<String>, source: impl Into<String>) {
        let spell = KnownSpell {
            spell_id: spell_id.into(),
            source: source.into(),
            notes: None,
        };
        if !self.known.iter().any(|s| s.spell_id == spell.spell_id) {
            self.known.push(spell);
        }
    }

    /// Prepare a spell (must already be known for prepared casters).
    pub fn prepare_spell(&mut self, spell_id: impl Into<String>) {
        let id = spell_id.into();
        if !self.prepared.contains(&id) {
            self.prepared.push(id);
        }
    }

    /// Unprepare a spell.
    pub fn unprepare_spell(&mut self, spell_id: &str) {
        self.prepared.retain(|id| id != spell_id);
    }

    /// Use a spell slot of a given level.
    pub fn use_slot(&mut self, level: u8) -> bool {
        if let Some(pool) = self.slots.get_mut(&level) {
            if pool.current > 0 {
                pool.current -= 1;
                return true;
            }
        }
        false
    }

    /// Restore all spell slots (e.g., after a long rest).
    pub fn restore_all_slots(&mut self) {
        for pool in self.slots.values_mut() {
            pool.current = pool.max;
        }
        if let Some(pact) = &mut self.pact_slots {
            pact.current = pact.max;
        }
    }

    /// Restore slots up to a certain level (e.g., Arcane Recovery).
    pub fn restore_slots_up_to(&mut self, max_level: u8, total_levels: u8) {
        let mut remaining = total_levels;
        for level in (1..=max_level).rev() {
            if remaining == 0 {
                break;
            }
            if let Some(pool) = self.slots.get_mut(&level) {
                let can_restore = (pool.max - pool.current).min(remaining / level);
                pool.current += can_restore;
                remaining -= can_restore * level;
            }
        }
    }
}

/// A spell that the character knows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnownSpell {
    /// ID of the spell
    pub spell_id: String,
    /// How the spell was learned (e.g., "class", "feat", "item")
    pub source: String,
    /// Optional notes about this spell
    pub notes: Option<String>,
}

/// A pool of spell slots at a given level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpellSlotPool {
    /// Currently available slots
    pub current: u8,
    /// Maximum slots
    pub max: u8,
}

impl SpellSlotPool {
    /// Create a new spell slot pool.
    pub fn new(max: u8) -> Self {
        Self { current: max, max }
    }

    /// Create an empty pool.
    pub fn empty(max: u8) -> Self {
        Self { current: 0, max }
    }

    /// Check if any slots are available.
    pub fn has_slots(&self) -> bool {
        self.current > 0
    }
}

/// A character's acquired feats.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CharacterFeats {
    /// Feats the character has acquired
    #[serde(default)]
    pub feats: Vec<AcquiredFeat>,
}

impl CharacterFeats {
    /// Create empty feat collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Acquire a feat.
    pub fn acquire(&mut self, feat_id: impl Into<String>, level: Option<u8>) {
        let feat = AcquiredFeat {
            feat_id: feat_id.into(),
            acquired_at_level: level,
            choices: HashMap::new(),
            notes: None,
        };
        self.feats.push(feat);
    }

    /// Check if the character has a specific feat.
    pub fn has_feat(&self, feat_id: &str) -> bool {
        self.feats.iter().any(|f| f.feat_id == feat_id)
    }

    /// Get a mutable reference to an acquired feat.
    pub fn get_mut(&mut self, feat_id: &str) -> Option<&mut AcquiredFeat> {
        self.feats.iter_mut().find(|f| f.feat_id == feat_id)
    }
}

/// A feat that the character has acquired.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AcquiredFeat {
    /// ID of the feat
    pub feat_id: String,
    /// Level at which the feat was acquired
    pub acquired_at_level: Option<u8>,
    /// Choices made when acquiring the feat (e.g., stat to increase)
    #[serde(default)]
    pub choices: HashMap<String, String>,
    /// Optional notes about this feat
    pub notes: Option<String>,
}

/// A character's active class features.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CharacterFeatures {
    /// Active features with their current state
    #[serde(default)]
    pub features: Vec<ActiveFeature>,
}

impl CharacterFeatures {
    /// Create empty feature collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a feature.
    pub fn add(&mut self, feature_id: impl Into<String>) {
        let feature = ActiveFeature {
            feature_id: feature_id.into(),
            uses_remaining: None,
            uses_max: None,
            choices: HashMap::new(),
        };
        self.features.push(feature);
    }

    /// Add a feature with limited uses.
    pub fn add_with_uses(&mut self, feature_id: impl Into<String>, max_uses: u8) {
        let feature = ActiveFeature {
            feature_id: feature_id.into(),
            uses_remaining: Some(max_uses),
            uses_max: Some(max_uses),
            choices: HashMap::new(),
        };
        self.features.push(feature);
    }

    /// Get a mutable reference to a feature.
    pub fn get_mut(&mut self, feature_id: &str) -> Option<&mut ActiveFeature> {
        self.features
            .iter_mut()
            .find(|f| f.feature_id == feature_id)
    }

    /// Use a feature (if it has limited uses).
    pub fn use_feature(&mut self, feature_id: &str) -> bool {
        if let Some(feature) = self.get_mut(feature_id) {
            if let Some(uses) = &mut feature.uses_remaining {
                if *uses > 0 {
                    *uses -= 1;
                    return true;
                }
                return false;
            }
            // No uses tracking = unlimited uses
            return true;
        }
        false
    }

    /// Restore all feature uses (e.g., after a rest).
    pub fn restore_all_uses(&mut self) {
        for feature in &mut self.features {
            if let (Some(remaining), Some(max)) = (&mut feature.uses_remaining, feature.uses_max) {
                *remaining = max;
            }
        }
    }
}

/// A class feature that the character has active.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveFeature {
    /// ID of the feature
    pub feature_id: String,
    /// Current uses remaining (if tracked)
    pub uses_remaining: Option<u8>,
    /// Maximum uses (if tracked)
    pub uses_max: Option<u8>,
    /// Choices made for this feature
    #[serde(default)]
    pub choices: HashMap<String, String>,
}

/// Character identity information (race, class, background).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CharacterIdentity {
    /// Race/ancestry ID
    pub race: Option<String>,
    /// Subrace/heritage ID
    pub subrace: Option<String>,
    /// Class levels
    #[serde(default)]
    pub classes: Vec<ClassLevel>,
    /// Background ID
    pub background: Option<String>,
    /// Alignment (for systems that use it)
    pub alignment: Option<String>,
    /// Total character level (sum of all class levels)
    #[serde(default)]
    pub total_level: u8,
}

impl CharacterIdentity {
    /// Create empty identity.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the race.
    pub fn with_race(mut self, race: impl Into<String>) -> Self {
        self.race = Some(race.into());
        self
    }

    /// Add a class level.
    pub fn add_class(&mut self, class_id: impl Into<String>, levels: u8) {
        let id = class_id.into();
        if let Some(existing) = self.classes.iter_mut().find(|c| c.class_id == id) {
            existing.level += levels;
        } else {
            self.classes.push(ClassLevel {
                class_id: id,
                subclass: None,
                level: levels,
            });
        }
        self.total_level = self.classes.iter().map(|c| c.level).sum();
    }

    /// Get level in a specific class.
    pub fn class_level(&self, class_id: &str) -> u8 {
        self.classes
            .iter()
            .find(|c| c.class_id == class_id)
            .map(|c| c.level)
            .unwrap_or(0)
    }

    /// Get the primary class (highest level).
    pub fn primary_class(&self) -> Option<&ClassLevel> {
        self.classes.iter().max_by_key(|c| c.level)
    }
}

/// A class level entry for multiclass characters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClassLevel {
    /// ID of the class
    pub class_id: String,
    /// ID of the subclass (if chosen)
    pub subclass: Option<String>,
    /// Number of levels in this class
    pub level: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_spells_basic() {
        let mut spells = CharacterSpells::new();
        spells.add_cantrip("fire_bolt");
        spells.learn_spell("magic_missile", "class");
        spells.prepare_spell("magic_missile");

        assert_eq!(spells.cantrips.len(), 1);
        assert_eq!(spells.known.len(), 1);
        assert_eq!(spells.prepared.len(), 1);
    }

    #[test]
    fn spell_slot_usage() {
        let mut spells = CharacterSpells::new();
        spells.slots.insert(1, SpellSlotPool::new(4));
        spells.slots.insert(2, SpellSlotPool::new(3));

        assert!(spells.use_slot(1));
        assert_eq!(spells.slots[&1].current, 3);

        spells.restore_all_slots();
        assert_eq!(spells.slots[&1].current, 4);
    }

    #[test]
    fn character_feats_basic() {
        let mut feats = CharacterFeats::new();
        feats.acquire("great_weapon_master", Some(4));

        assert!(feats.has_feat("great_weapon_master"));
        assert!(!feats.has_feat("sentinel"));
    }

    #[test]
    fn character_features_with_uses() {
        let mut features = CharacterFeatures::new();
        features.add_with_uses("second_wind", 1);

        assert!(features.use_feature("second_wind"));
        assert!(!features.use_feature("second_wind")); // No uses left

        features.restore_all_uses();
        assert!(features.use_feature("second_wind"));
    }

    #[test]
    fn character_identity_multiclass() {
        let mut identity = CharacterIdentity::new().with_race("human");
        identity.add_class("fighter", 5);
        identity.add_class("wizard", 2);

        assert_eq!(identity.total_level, 7);
        assert_eq!(identity.class_level("fighter"), 5);
        assert_eq!(identity.class_level("wizard"), 2);
        assert_eq!(identity.class_level("rogue"), 0);
        assert_eq!(identity.primary_class().unwrap().class_id, "fighter");
    }

    #[test]
    fn spell_pool_equality() {
        let mut spells = CharacterSpells::new();
        spells.add_cantrip("prestidigitation");
        spells.learn_spell("shield", "class");
        spells.slots.insert(1, SpellSlotPool::new(2));

        let other = spells.clone();
        assert_eq!(spells, other);
    }
}
