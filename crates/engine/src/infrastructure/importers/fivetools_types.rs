//! Type definitions for 5etools JSON data format.
//!
//! These types mirror the 5etools JSON schema for spells, feats, items, etc.
//! They are used for deserialization and then converted to our domain types.
//!
//! Note: Some fields are parsed but not yet used in the conversion to domain types.
//! They are kept for future expansion and to maintain compatibility with the JSON schema.

#![allow(dead_code)]

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
    ClassLevel {
        class: FiveToolsClassEntry,
        level: u8,
    },
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

// === Race Types ===

/// Root structure for the 5etools races.json file.
#[derive(Debug, Deserialize)]
pub struct FiveToolsRaceFile {
    #[serde(default)]
    pub race: Vec<FiveToolsRace>,
    #[serde(default)]
    pub subrace: Vec<FiveToolsSubrace>,
}

/// A race in 5etools format.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsRace {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub size: Vec<String>,
    #[serde(default)]
    pub speed: FiveToolsSpeed,
    #[serde(default)]
    pub ability: Vec<FiveToolsRaceAbility>,
    #[serde(default)]
    pub darkvision: Option<u32>,
    #[serde(default)]
    pub trait_tags: Option<Vec<String>>,
    #[serde(default)]
    pub language_proficiencies: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub skill_proficiencies: Vec<FiveToolsSkillProficiency>,
    #[serde(default)]
    pub resist: Option<Vec<String>>,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
    #[serde(default)]
    pub age: Option<FiveToolsAge>,
    #[serde(default)]
    pub lineage: Option<String>,
    #[serde(rename = "_copy", default)]
    pub copy: Option<FiveToolsCopy>,
}

/// A subrace in 5etools format.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsSubrace {
    pub name: String,
    pub source: String,
    pub race_name: String,
    pub race_source: String,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub ability: Vec<FiveToolsRaceAbility>,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
    #[serde(rename = "_copy", default)]
    pub copy: Option<FiveToolsCopy>,
}

/// Speed can be a simple number or an object with multiple movement types.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(untagged)]
pub enum FiveToolsSpeed {
    Simple(u32),
    Complex(FiveToolsSpeedComplex),
    #[default]
    None,
}

/// Complex speed with multiple movement types.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FiveToolsSpeedComplex {
    #[serde(default)]
    pub walk: Option<FiveToolsSpeedValue>,
    #[serde(default)]
    pub fly: Option<FiveToolsSpeedValue>,
    #[serde(default)]
    pub swim: Option<FiveToolsSpeedValue>,
    #[serde(default)]
    pub climb: Option<FiveToolsSpeedValue>,
    #[serde(default)]
    pub burrow: Option<FiveToolsSpeedValue>,
}

/// Speed value can be a number or a boolean (for fly = walking speed).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FiveToolsSpeedValue {
    Number(u32),
    Bool(bool),
    Conditional(FiveToolsConditionalSpeed),
}

/// Conditional speed (e.g., fly with condition).
#[derive(Debug, Clone, Deserialize)]
pub struct FiveToolsConditionalSpeed {
    pub number: u32,
    #[serde(default)]
    pub condition: Option<String>,
}

/// Ability bonus for a race.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FiveToolsRaceAbility {
    Fixed(HashMap<String, i32>),
    Choice(FiveToolsRaceAbilityChoice),
}

/// Choice-based ability bonus.
#[derive(Debug, Clone, Deserialize)]
pub struct FiveToolsRaceAbilityChoice {
    pub choose: FiveToolsAbilityChooseSpec,
}

/// Ability choice specification.
#[derive(Debug, Clone, Deserialize)]
pub struct FiveToolsAbilityChooseSpec {
    #[serde(default)]
    pub from: Vec<String>,
    #[serde(default)]
    pub count: Option<u8>,
    #[serde(default)]
    pub amount: Option<i32>,
    #[serde(default)]
    pub weighted: Option<FiveToolsWeightedChoice>,
}

/// Weighted choice for abilities.
#[derive(Debug, Clone, Deserialize)]
pub struct FiveToolsWeightedChoice {
    pub from: Vec<String>,
    pub weights: Vec<i32>,
}

/// Skill proficiency (can have choices).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FiveToolsSkillProficiency {
    Fixed(HashMap<String, bool>),
    Choice(FiveToolsSkillChoice),
}

/// Skill choice specification.
#[derive(Debug, Clone, Deserialize)]
pub struct FiveToolsSkillChoice {
    #[serde(default)]
    pub choose: Option<FiveToolsSkillChooseSpec>,
    #[serde(default)]
    pub any: Option<u8>,
}

/// Skill choice details.
#[derive(Debug, Clone, Deserialize)]
pub struct FiveToolsSkillChooseSpec {
    pub from: Vec<String>,
    #[serde(default = "default_one")]
    pub count: u8,
}

/// Age information for a race.
#[derive(Debug, Clone, Deserialize)]
pub struct FiveToolsAge {
    #[serde(default)]
    pub mature: Option<u32>,
    #[serde(default)]
    pub max: Option<u32>,
}

/// Copy directive for inherited races.
#[derive(Debug, Clone, Deserialize)]
pub struct FiveToolsCopy {
    pub name: String,
    pub source: String,
    #[serde(rename = "_mod", default)]
    pub modifications: Option<serde_json::Value>,
}

fn default_one() -> u8 {
    1
}

// === Class Types ===

/// Root structure for a 5etools class file.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsClassFile {
    #[serde(default)]
    pub class: Vec<FiveToolsClass>,
    #[serde(default)]
    pub subclass: Vec<FiveToolsSubclass>,
    #[serde(default)]
    pub class_feature: Vec<FiveToolsClassFeature>,
    #[serde(default)]
    pub subclass_feature: Vec<FiveToolsSubclassFeature>,
}

/// A class in 5etools format.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsClass {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub srd: Option<bool>,
    #[serde(default)]
    pub basic_rules: Option<bool>,
    #[serde(default)]
    pub edition: Option<String>,
    pub hd: FiveToolsHitDice,
    #[serde(default)]
    pub proficiency: Vec<String>,
    #[serde(default)]
    pub starting_proficiencies: FiveToolsStartingProficiencies,
    #[serde(default)]
    pub starting_equipment: Option<FiveToolsStartingEquipment>,
    #[serde(default)]
    pub multiclassing: Option<FiveToolsMulticlassing>,
    #[serde(default)]
    pub class_features: Vec<serde_json::Value>,
    #[serde(default)]
    pub subclass_title: Option<String>,
    #[serde(default)]
    pub spellcasting_ability: Option<String>,
    #[serde(default)]
    pub caster_progression: Option<String>,
    #[serde(default)]
    pub cantrip_progression: Option<Vec<u8>>,
    #[serde(default)]
    pub spells_known_progression: Option<Vec<u8>>,
    #[serde(default)]
    pub prepared_spells: Option<String>,
    #[serde(default)]
    pub primary_ability: Option<Vec<HashMap<String, bool>>>,
}

/// Hit dice specification.
#[derive(Debug, Clone, Deserialize)]
pub struct FiveToolsHitDice {
    #[serde(default = "default_one_u32")]
    pub number: u32,
    pub faces: u8,
}

fn default_one_u32() -> u32 {
    1
}

/// Starting proficiencies for a class.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FiveToolsStartingProficiencies {
    #[serde(default)]
    pub armor: Vec<String>,
    #[serde(default)]
    pub weapons: Vec<String>,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    #[serde(default)]
    pub skills: Vec<FiveToolsSkillProficiency>,
}

/// Starting equipment for a class.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsStartingEquipment {
    #[serde(default)]
    pub additional_from_background: bool,
    #[serde(default)]
    pub default: Vec<String>,
    #[serde(default)]
    pub gold_alternative: Option<String>,
}

/// Multiclassing requirements and proficiencies.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsMulticlassing {
    #[serde(default)]
    pub requirements: Option<FiveToolsMulticlassRequirements>,
    #[serde(default)]
    pub proficiencies_gained: Option<FiveToolsStartingProficiencies>,
}

/// Multiclassing requirements.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FiveToolsMulticlassRequirements {
    #[serde(default)]
    pub or: Option<Vec<HashMap<String, i32>>>,
    #[serde(flatten)]
    pub fixed: HashMap<String, i32>,
}

/// A subclass in 5etools format.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsSubclass {
    pub name: String,
    #[serde(default)]
    pub short_name: String,
    pub source: String,
    pub class_name: String,
    pub class_source: String,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub subclass_features: Vec<serde_json::Value>,
    #[serde(default)]
    pub spellcasting_ability: Option<String>,
    #[serde(default)]
    pub caster_progression: Option<String>,
}

// === Background Types ===

/// Root structure for the 5etools backgrounds.json file.
#[derive(Debug, Deserialize)]
pub struct FiveToolsBackgroundFile {
    #[serde(default)]
    pub background: Vec<FiveToolsBackground>,
}

/// A background in 5etools format.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsBackground {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub edition: Option<String>,
    #[serde(default)]
    pub skill_proficiencies: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub tool_proficiencies: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub language_proficiencies: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub starting_equipment: Vec<serde_json::Value>,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
    #[serde(default)]
    pub feats: Option<Vec<HashMap<String, bool>>>,
    #[serde(default)]
    pub ability: Option<Vec<FiveToolsRaceAbility>>,
    #[serde(rename = "_copy", default)]
    pub copy: Option<FiveToolsCopy>,
}

// === Optional Feature Types ===

/// Root structure for the 5etools optionalfeatures.json file.
#[derive(Debug, Deserialize)]
pub struct FiveToolsOptionalFeatureFile {
    #[serde(default)]
    pub optionalfeature: Vec<FiveToolsOptionalFeature>,
}

/// An optional feature in 5etools format.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsOptionalFeature {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub feature_type: Vec<String>,
    #[serde(default)]
    pub prerequisite: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
}

// === Item Types ===

/// Root structure for the 5etools items.json file.
#[derive(Debug, Deserialize)]
pub struct FiveToolsItemFile {
    #[serde(default)]
    pub item: Vec<FiveToolsItem>,
}

/// An item in 5etools format.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsItem {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(rename = "type", default)]
    pub item_type: Option<String>,
    #[serde(default)]
    pub rarity: Option<String>,
    #[serde(default)]
    pub weight: Option<f32>,
    #[serde(default)]
    pub value: Option<i32>,
    #[serde(default)]
    pub req_attune: Option<serde_json::Value>,
    #[serde(default)]
    pub req_attune_tags: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub wondrous: Option<bool>,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Root structure for the 5etools items-base.json file.
#[derive(Debug, Deserialize)]
pub struct FiveToolsBaseItemFile {
    #[serde(default)]
    pub baseitem: Vec<FiveToolsBaseItem>,
}

/// A base item in 5etools format (weapons, armor, gear).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiveToolsBaseItem {
    pub name: String,
    pub source: String,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(rename = "type", default)]
    pub item_type: Option<String>,
    #[serde(default)]
    pub rarity: Option<String>,
    #[serde(default)]
    pub weight: Option<f32>,
    #[serde(default)]
    pub value: Option<i32>,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
    #[serde(default)]
    pub weapon: Option<bool>,
    #[serde(default)]
    pub armor: Option<bool>,
    #[serde(default)]
    pub weapon_category: Option<String>,
    #[serde(default)]
    pub armor_category: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
