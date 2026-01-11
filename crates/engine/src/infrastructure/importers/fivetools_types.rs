//! Type definitions for 5etools JSON data format.
//!
//! These types mirror the 5etools JSON schema for spells, feats, items, etc.
//! They are used for deserialization and then converted to our domain types.

use serde::Deserialize;
use std::collections::HashMap;

/// Root structure for a 5etools spell file.
#[derive(Debug, Deserialize)]
pub struct FiveToolsSpellFile {
    #[serde(default)]
    pub spell: Vec<FiveToolsSpell>,
}

/// A spell in 5etools format.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsSpell {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
    pub level: u8,
    pub school: String,
    #[serde(default)]
    pub time: Vec<FiveToolsTime>,
    #[serde(default)]
    pub range: Option<FiveToolsRange>,
    #[serde(default)]
    pub components: Option<FiveToolsComponents>,
    #[serde(default)]
    pub duration: Vec<FiveToolsDuration>,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
    #[serde(rename = "entriesHigherLevel", default)]
    pub entries_higher_level: Option<Vec<serde_json::Value>>,
    #[serde(rename = "miscTags", default)]
    pub misc_tags: Option<Vec<String>>,
    #[serde(default)]
    pub meta: Option<FiveToolsSpellMeta>,
    #[serde(default)]
    pub classes: Option<FiveToolsClasses>,
    #[serde(default, rename = "damageInflict")]
    pub damage_inflict: Option<Vec<String>>,
    #[serde(default, rename = "conditionInflict")]
    pub condition_inflict: Option<Vec<String>>,
    #[serde(default, rename = "savingThrow")]
    pub saving_throw: Option<Vec<String>>,
    #[serde(default, rename = "spellAttack")]
    pub spell_attack: Option<Vec<String>>,
}

/// Spell metadata (concentration, ritual).
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsSpellMeta {
    #[serde(default)]
    pub ritual: bool,
}

/// Classes that can cast a spell.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsClasses {
    #[serde(default)]
    pub from_class_list: Option<Vec<FiveToolsClassEntry>>,
    #[serde(default)]
    pub from_subclass: Option<Vec<FiveToolsSubclassEntry>>,
}

/// A class entry.
#[derive(Debug, Deserialize)]
pub struct FiveToolsClassEntry {
    pub name: String,
    pub source: String,
}

/// A subclass entry.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsSubclassEntry {
    pub class: FiveToolsClassEntry,
    pub subclass: FiveToolsClassEntry,
}

/// Casting time for a spell.
#[derive(Debug, Deserialize)]
pub struct FiveToolsTime {
    #[serde(default)]
    pub number: Option<u32>,
    pub unit: String,
    #[serde(default)]
    pub condition: Option<String>,
}

/// Range of a spell.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsRange {
    #[serde(rename = "type")]
    pub range_type: String,
    #[serde(default)]
    pub distance: Option<FiveToolsDistance>,
}

/// Distance specification.
#[derive(Debug, Deserialize)]
pub struct FiveToolsDistance {
    #[serde(rename = "type")]
    pub distance_type: String,
    #[serde(default)]
    pub amount: Option<u32>,
}

/// Spell components.
#[derive(Debug, Deserialize)]
pub struct FiveToolsComponents {
    #[serde(default)]
    pub v: bool,
    #[serde(default)]
    pub s: bool,
    #[serde(default)]
    pub m: Option<FiveToolsMaterial>,
}

/// Material component - can be a string or object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FiveToolsMaterial {
    Simple(String),
    Detailed(FiveToolsMaterialDetailed),
}

/// Detailed material component.
#[derive(Debug, Deserialize)]
pub struct FiveToolsMaterialDetailed {
    pub text: String,
    #[serde(default)]
    pub cost: Option<u32>,
    #[serde(default)]
    pub consume: Option<FiveToolsConsume>,
}

/// Consume specification for material.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FiveToolsConsume {
    Bool(bool),
    String(String),
}

/// Duration of a spell.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsDuration {
    #[serde(rename = "type")]
    pub duration_type: String,
    #[serde(default)]
    pub duration: Option<FiveToolsDurationAmount>,
    #[serde(default)]
    pub concentration: bool,
    #[serde(default)]
    pub ends: Option<Vec<String>>,
}

/// Duration amount.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsDurationAmount {
    #[serde(rename = "type")]
    pub duration_type: String,
    #[serde(default)]
    pub amount: Option<u32>,
    #[serde(default)]
    pub up_to: bool,
}

// === Feat Types ===

/// Root structure for a 5etools feat file.
#[derive(Debug, Deserialize)]
pub struct FiveToolsFeatFile {
    #[serde(default)]
    pub feat: Vec<FiveToolsFeat>,
}

/// A feat in 5etools format.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsFeat {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub prerequisite: Option<Vec<FiveToolsPrerequisite>>,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
    #[serde(default)]
    pub ability: Option<Vec<FiveToolsAbilityBonus>>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub additional_sources: Option<Vec<FiveToolsAdditionalSource>>,
}

/// Prerequisite for a feat.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsPrerequisite {
    #[serde(default)]
    pub level: Option<FiveToolsLevelPrereq>,
    #[serde(default)]
    pub race: Option<Vec<FiveToolsRacePrereq>>,
    #[serde(default)]
    pub ability: Option<Vec<HashMap<String, i32>>>,
    #[serde(default)]
    pub spellcasting: Option<bool>,
    #[serde(default)]
    pub spellcasting2020: Option<bool>,
    #[serde(default)]
    pub proficiency: Option<Vec<HashMap<String, String>>>,
    #[serde(default)]
    pub other: Option<String>,
}

/// Level prerequisite.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FiveToolsLevelPrereq {
    Simple(u8),
    ClassLevel { class: FiveToolsClassEntry, level: u8 },
}

/// Race prerequisite.
#[derive(Debug, Deserialize)]
pub struct FiveToolsRacePrereq {
    pub name: String,
    #[serde(default)]
    pub subrace: Option<String>,
}

/// Ability bonus from a feat.
#[derive(Debug, Deserialize)]
pub struct FiveToolsAbilityBonus {
    #[serde(flatten)]
    pub bonuses: HashMap<String, i32>,
    #[serde(default)]
    pub choose: Option<FiveToolsAbilityChoice>,
}

/// Choice of ability increase.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsAbilityChoice {
    #[serde(default)]
    pub from: Vec<String>,
    #[serde(default)]
    pub count: Option<u8>,
    #[serde(default)]
    pub amount: Option<i32>,
}

/// Additional source reference.
#[derive(Debug, Deserialize)]
pub struct FiveToolsAdditionalSource {
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
}

// === Class Feature Types ===

/// Root structure for a 5etools class features file.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsClassFeaturesFile {
    #[serde(default)]
    pub class_feature: Vec<FiveToolsClassFeature>,
    #[serde(default)]
    pub subclass_feature: Vec<FiveToolsSubclassFeature>,
}

/// A class feature in 5etools format.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsClassFeature {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
    pub class_name: String,
    pub class_source: String,
    pub level: u8,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
    #[serde(default)]
    pub is_class_feature_variant: bool,
}

/// A subclass feature in 5etools format.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsSubclassFeature {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
    pub class_name: String,
    pub class_source: String,
    pub subclass_short_name: String,
    pub subclass_source: String,
    pub level: u8,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
}

// === Index Types ===

/// Index file mapping sources to filenames.
pub type FiveToolsIndex = HashMap<String, String>;
