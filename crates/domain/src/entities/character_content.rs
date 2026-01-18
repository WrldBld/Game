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
    cantrips: Vec<String>,
    /// Spells the character knows (for known-spell casters)
    #[serde(default)]
    known: Vec<KnownSpell>,
    /// Currently prepared spells (spell IDs)
    #[serde(default)]
    prepared: Vec<String>,
    /// Spell slots by level (1-9)
    #[serde(default)]
    slots: HashMap<u8, SpellSlotPool>,
    /// Pact magic slots (for warlocks and similar)
    pact_slots: Option<SpellSlotPool>,
    /// Primary spellcasting ability (e.g., "INT", "WIS", "CHA")
    spellcasting_ability: Option<String>,
}

impl CharacterSpells {
    /// Create empty spellcasting data.
    pub fn new() -> Self {
        Self::default()
    }

    // Read-only accessors

    /// Get the cantrips the character knows.
    pub fn cantrips(&self) -> &[String] {
        &self.cantrips
    }

    /// Get the spells the character knows.
    pub fn known(&self) -> &[KnownSpell] {
        &self.known
    }

    /// Get the currently prepared spells.
    pub fn prepared(&self) -> &[String] {
        &self.prepared
    }

    /// Get the spell slots by level.
    pub fn slots(&self) -> &HashMap<u8, SpellSlotPool> {
        &self.slots
    }

    /// Get mutable access to spell slots.
    pub fn slots_mut(&mut self) -> &mut HashMap<u8, SpellSlotPool> {
        &mut self.slots
    }

    /// Get the pact magic slots.
    pub fn pact_slots(&self) -> Option<&SpellSlotPool> {
        self.pact_slots.as_ref()
    }

    /// Get the primary spellcasting ability.
    pub fn spellcasting_ability(&self) -> Option<&str> {
        self.spellcasting_ability.as_deref()
    }

    // Builder-style methods for optional fields

    /// Set the pact magic slots.
    pub fn with_pact_slots(mut self, pact_slots: SpellSlotPool) -> Self {
        self.pact_slots = Some(pact_slots);
        self
    }

    /// Set the spellcasting ability.
    pub fn with_spellcasting_ability(mut self, ability: impl Into<String>) -> Self {
        self.spellcasting_ability = Some(ability.into());
        self
    }

    // Mutation methods

    /// Add a cantrip.
    pub fn add_cantrip(&mut self, spell_id: impl Into<String>) {
        let id = spell_id.into();
        if !self.cantrips.contains(&id) {
            self.cantrips.push(id);
        }
    }

    /// Learn a spell.
    pub fn learn_spell(&mut self, spell_id: impl Into<String>, source: impl Into<String>) {
        let spell = KnownSpell::new(spell_id, source);
        if !self.known.iter().any(|s| s.spell_id() == spell.spell_id()) {
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
            if pool.current() > 0 {
                pool.use_slot();
                return true;
            }
        }
        false
    }

    /// Restore all spell slots (e.g., after a long rest).
    pub fn restore_all_slots(&mut self) {
        for pool in self.slots.values_mut() {
            pool.restore_all();
        }
        if let Some(pact) = &mut self.pact_slots {
            pact.restore_all();
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
                let can_restore = (pool.max() - pool.current()).min(remaining / level);
                pool.restore(can_restore);
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
    spell_id: String,
    /// How the spell was learned (e.g., "class", "feat", "item")
    source: String,
    /// Optional notes about this spell
    notes: Option<String>,
}

impl KnownSpell {
    /// Create a new known spell.
    pub fn new(spell_id: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            spell_id: spell_id.into(),
            source: source.into(),
            notes: None,
        }
    }

    /// Get the spell ID.
    pub fn spell_id(&self) -> &str {
        &self.spell_id
    }

    /// Get the source of how the spell was learned.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the optional notes.
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    /// Set the notes.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// A pool of spell slots at a given level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpellSlotPool {
    /// Currently available slots
    current: u8,
    /// Maximum slots
    max: u8,
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

    /// Get the current available slots.
    pub fn current(&self) -> u8 {
        self.current
    }

    /// Get the maximum slots.
    pub fn max(&self) -> u8 {
        self.max
    }

    /// Check if any slots are available.
    pub fn has_slots(&self) -> bool {
        self.current > 0
    }

    /// Use one slot.
    pub fn use_slot(&mut self) {
        if self.current > 0 {
            self.current -= 1;
        }
    }

    /// Restore all slots.
    pub fn restore_all(&mut self) {
        self.current = self.max;
    }

    /// Restore a specific number of slots.
    pub fn restore(&mut self, amount: u8) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// A character's acquired feats.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CharacterFeats {
    /// Feats the character has acquired
    #[serde(default)]
    feats: Vec<AcquiredFeat>,
}

impl CharacterFeats {
    /// Create empty feat collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the acquired feats.
    pub fn feats(&self) -> &[AcquiredFeat] {
        &self.feats
    }

    /// Acquire a feat.
    pub fn acquire(&mut self, feat_id: impl Into<String>, level: Option<u8>) {
        let feat = AcquiredFeat::new(feat_id, level);
        self.feats.push(feat);
    }

    /// Check if the character has a specific feat.
    pub fn has_feat(&self, feat_id: &str) -> bool {
        self.feats.iter().any(|f| f.feat_id() == feat_id)
    }

    /// Get a mutable reference to an acquired feat.
    pub fn get_mut(&mut self, feat_id: &str) -> Option<&mut AcquiredFeat> {
        self.feats.iter_mut().find(|f| f.feat_id() == feat_id)
    }
}

/// A feat that the character has acquired.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AcquiredFeat {
    /// ID of the feat
    feat_id: String,
    /// Level at which the feat was acquired
    acquired_at_level: Option<u8>,
    /// Choices made when acquiring the feat (e.g., stat to increase)
    #[serde(default)]
    choices: HashMap<String, String>,
    /// Optional notes about this feat
    notes: Option<String>,
}

impl AcquiredFeat {
    /// Create a new acquired feat.
    pub fn new(feat_id: impl Into<String>, acquired_at_level: Option<u8>) -> Self {
        Self {
            feat_id: feat_id.into(),
            acquired_at_level,
            choices: HashMap::new(),
            notes: None,
        }
    }

    /// Get the feat ID.
    pub fn feat_id(&self) -> &str {
        &self.feat_id
    }

    /// Get the level at which the feat was acquired.
    pub fn acquired_at_level(&self) -> Option<u8> {
        self.acquired_at_level
    }

    /// Get the choices made for this feat.
    pub fn choices(&self) -> &HashMap<String, String> {
        &self.choices
    }

    /// Get mutable access to choices.
    pub fn choices_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.choices
    }

    /// Get the optional notes.
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    /// Set the notes.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Set a choice.
    pub fn with_choice(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.choices.insert(key.into(), value.into());
        self
    }
}

/// A character's active class features.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CharacterFeatures {
    /// Active features with their current state
    #[serde(default)]
    features: Vec<ActiveFeature>,
}

impl CharacterFeatures {
    /// Create empty feature collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the active features.
    pub fn features(&self) -> &[ActiveFeature] {
        &self.features
    }

    /// Add a feature.
    pub fn add(&mut self, feature_id: impl Into<String>) {
        let feature = ActiveFeature::new(feature_id);
        self.features.push(feature);
    }

    /// Add a feature with limited uses.
    pub fn add_with_uses(&mut self, feature_id: impl Into<String>, max_uses: u8) {
        let feature = ActiveFeature::new(feature_id).with_uses(max_uses, max_uses);
        self.features.push(feature);
    }

    /// Get a mutable reference to a feature.
    pub fn get_mut(&mut self, feature_id: &str) -> Option<&mut ActiveFeature> {
        self.features
            .iter_mut()
            .find(|f| f.feature_id() == feature_id)
    }

    /// Use a feature (if it has limited uses).
    pub fn use_feature(&mut self, feature_id: &str) -> bool {
        if let Some(feature) = self.get_mut(feature_id) {
            return feature.use_once();
        }
        false
    }

    /// Restore all feature uses (e.g., after a rest).
    pub fn restore_all_uses(&mut self) {
        for feature in &mut self.features {
            feature.restore_uses();
        }
    }
}

/// A class feature that the character has active.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveFeature {
    /// ID of the feature
    feature_id: String,
    /// Current uses remaining (if tracked)
    uses_remaining: Option<u8>,
    /// Maximum uses (if tracked)
    uses_max: Option<u8>,
    /// Choices made for this feature
    #[serde(default)]
    choices: HashMap<String, String>,
}

impl ActiveFeature {
    /// Create a new active feature.
    pub fn new(feature_id: impl Into<String>) -> Self {
        Self {
            feature_id: feature_id.into(),
            uses_remaining: None,
            uses_max: None,
            choices: HashMap::new(),
        }
    }

    /// Get the feature ID.
    pub fn feature_id(&self) -> &str {
        &self.feature_id
    }

    /// Get the uses remaining.
    pub fn uses_remaining(&self) -> Option<u8> {
        self.uses_remaining
    }

    /// Get the maximum uses.
    pub fn uses_max(&self) -> Option<u8> {
        self.uses_max
    }

    /// Get the choices made for this feature.
    pub fn choices(&self) -> &HashMap<String, String> {
        &self.choices
    }

    /// Get mutable access to choices.
    pub fn choices_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.choices
    }

    /// Set the uses tracking.
    pub fn with_uses(mut self, remaining: u8, max: u8) -> Self {
        self.uses_remaining = Some(remaining);
        self.uses_max = Some(max);
        self
    }

    /// Set a choice.
    pub fn with_choice(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.choices.insert(key.into(), value.into());
        self
    }

    /// Use the feature once.
    pub fn use_once(&mut self) -> bool {
        if let Some(uses) = &mut self.uses_remaining {
            if *uses > 0 {
                *uses -= 1;
                return true;
            }
            return false;
        }
        // No uses tracking = unlimited uses
        true
    }

    /// Restore all uses.
    pub fn restore_uses(&mut self) {
        if let (Some(remaining), Some(max)) = (&mut self.uses_remaining, self.uses_max) {
            *remaining = max;
        }
    }
}

/// Character identity information (race, class, background).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CharacterIdentity {
    /// Race/ancestry ID
    race: Option<String>,
    /// Subrace/heritage ID
    subrace: Option<String>,
    /// Class levels
    #[serde(default)]
    classes: Vec<ClassLevel>,
    /// Background ID
    background: Option<String>,
    /// Alignment (for systems that use it)
    alignment: Option<String>,
    /// Total character level (sum of all class levels)
    #[serde(default)]
    total_level: u8,
}

impl CharacterIdentity {
    /// Create empty identity.
    pub fn new() -> Self {
        Self::default()
    }

    // Read-only accessors

    /// Get the race/ancestry ID.
    pub fn race(&self) -> Option<&str> {
        self.race.as_deref()
    }

    /// Get the subrace/heritage ID.
    pub fn subrace(&self) -> Option<&str> {
        self.subrace.as_deref()
    }

    /// Get the class levels.
    pub fn classes(&self) -> &[ClassLevel] {
        &self.classes
    }

    /// Get the background ID.
    pub fn background(&self) -> Option<&str> {
        self.background.as_deref()
    }

    /// Get the alignment.
    pub fn alignment(&self) -> Option<&str> {
        self.alignment.as_deref()
    }

    /// Get the total character level.
    pub fn total_level(&self) -> u8 {
        self.total_level
    }

    // Builder-style methods

    /// Set the race.
    pub fn with_race(mut self, race: impl Into<String>) -> Self {
        self.race = Some(race.into());
        self
    }

    /// Set the subrace.
    pub fn with_subrace(mut self, subrace: impl Into<String>) -> Self {
        self.subrace = Some(subrace.into());
        self
    }

    /// Set the background.
    pub fn with_background(mut self, background: impl Into<String>) -> Self {
        self.background = Some(background.into());
        self
    }

    /// Set the alignment.
    pub fn with_alignment(mut self, alignment: impl Into<String>) -> Self {
        self.alignment = Some(alignment.into());
        self
    }

    // Mutation methods

    /// Add a class level.
    pub fn add_class(&mut self, class_id: impl Into<String>, levels: u8) {
        let id = class_id.into();
        if let Some(existing) = self.classes.iter_mut().find(|c| c.class_id() == id) {
            existing.add_levels(levels);
        } else {
            self.classes.push(ClassLevel::new(id, levels));
        }
        self.total_level = self.classes.iter().map(|c| c.level()).sum();
    }

    /// Get level in a specific class.
    pub fn class_level(&self, class_id: &str) -> u8 {
        self.classes
            .iter()
            .find(|c| c.class_id() == class_id)
            .map(|c| c.level())
            .unwrap_or(0)
    }

    /// Get the primary class (highest level).
    pub fn primary_class(&self) -> Option<&ClassLevel> {
        self.classes.iter().max_by_key(|c| c.level())
    }
}

/// A class level entry for multiclass characters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClassLevel {
    /// ID of the class
    class_id: String,
    /// ID of the subclass (if chosen)
    subclass: Option<String>,
    /// Number of levels in this class
    level: u8,
}

impl ClassLevel {
    /// Create a new class level entry.
    pub fn new(class_id: impl Into<String>, level: u8) -> Self {
        Self {
            class_id: class_id.into(),
            subclass: None,
            level,
        }
    }

    /// Get the class ID.
    pub fn class_id(&self) -> &str {
        &self.class_id
    }

    /// Get the subclass ID.
    pub fn subclass(&self) -> Option<&str> {
        self.subclass.as_deref()
    }

    /// Get the number of levels.
    pub fn level(&self) -> u8 {
        self.level
    }

    /// Set the subclass.
    pub fn with_subclass(mut self, subclass: impl Into<String>) -> Self {
        self.subclass = Some(subclass.into());
        self
    }

    /// Add levels to this class.
    pub fn add_levels(&mut self, levels: u8) {
        self.level += levels;
    }
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

        assert_eq!(spells.cantrips().len(), 1);
        assert_eq!(spells.known().len(), 1);
        assert_eq!(spells.prepared().len(), 1);
    }

    #[test]
    fn spell_slot_usage() {
        let mut spells = CharacterSpells::new();
        spells.slots_mut().insert(1, SpellSlotPool::new(4));
        spells.slots_mut().insert(2, SpellSlotPool::new(3));

        assert!(spells.use_slot(1));
        assert_eq!(spells.slots()[&1].current(), 3);

        spells.restore_all_slots();
        assert_eq!(spells.slots()[&1].current(), 4);
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

        assert_eq!(identity.total_level(), 7);
        assert_eq!(identity.class_level("fighter"), 5);
        assert_eq!(identity.class_level("wizard"), 2);
        assert_eq!(identity.class_level("rogue"), 0);
        assert_eq!(identity.primary_class().unwrap().class_id(), "fighter");
    }

    #[test]
    fn spell_pool_equality() {
        let mut spells = CharacterSpells::new();
        spells.add_cantrip("prestidigitation");
        spells.learn_spell("shield", "class");
        spells.slots_mut().insert(1, SpellSlotPool::new(2));

        let other = spells.clone();
        assert_eq!(spells, other);
    }

    #[test]
    fn known_spell_accessors() {
        let spell = KnownSpell::new("fireball", "class").with_notes("Favorite spell");

        assert_eq!(spell.spell_id(), "fireball");
        assert_eq!(spell.source(), "class");
        assert_eq!(spell.notes(), Some("Favorite spell"));
    }

    #[test]
    fn acquired_feat_accessors() {
        let feat = AcquiredFeat::new("alert", Some(4))
            .with_notes("Always go first")
            .with_choice("stat", "DEX");

        assert_eq!(feat.feat_id(), "alert");
        assert_eq!(feat.acquired_at_level(), Some(4));
        assert_eq!(feat.notes(), Some("Always go first"));
        assert_eq!(feat.choices().get("stat"), Some(&"DEX".to_string()));
    }

    #[test]
    fn active_feature_accessors() {
        let feature = ActiveFeature::new("action_surge")
            .with_uses(1, 1)
            .with_choice("target", "self");

        assert_eq!(feature.feature_id(), "action_surge");
        assert_eq!(feature.uses_remaining(), Some(1));
        assert_eq!(feature.uses_max(), Some(1));
        assert_eq!(feature.choices().get("target"), Some(&"self".to_string()));
    }

    #[test]
    fn class_level_accessors() {
        let class = ClassLevel::new("wizard", 5).with_subclass("evocation");

        assert_eq!(class.class_id(), "wizard");
        assert_eq!(class.level(), 5);
        assert_eq!(class.subclass(), Some("evocation"));
    }
}
