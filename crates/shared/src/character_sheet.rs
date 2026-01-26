//! Character Sheet Schema Types for Game System Rendering
//!
//! This module defines the schema types for dynamic character sheet rendering.
//! The engine sends these schemas to the client, which renders the appropriate
//! fields without needing system-specific knowledge.
//!
//! # Distinction from sheet_template.rs
//!
//! - `sheet_template.rs`: Stored templates for world-specific character sheets
//! - `character_sheet.rs` (this): Game system schemas for engine-driven rendering
//!
//! # Design Philosophy
//!
//! - **Engine-driven rendering**: The engine knows about game systems, the client just displays
//! - **Field-level granularity**: Each field has a type, validation, and display hints
//! - **Calculated fields**: Some fields derive from others (e.g., ability modifiers)
//! - **Sections**: Fields are grouped into logical sections for UI layout

use serde::{Deserialize, Serialize};
pub use wrldbldr_domain::types::character_sheet::{CharacterSheetValues, SheetValue};

// =============================================================================
// Character Sheet Schema
// =============================================================================

/// Complete schema for rendering a character sheet.
///
/// Sent by the engine to describe what fields/sections a character sheet
/// should display for a given game system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheetSchema {
    /// Game system ID (e.g., "dnd5e", "pf2e", "blades")
    pub system_id: String,
    /// Human-readable system name
    pub system_name: String,
    /// Ordered list of sections to display
    pub sections: Vec<SchemaSection>,
    /// Character creation steps (if applicable)
    #[serde(default)]
    pub creation_steps: Vec<CreationStep>,
}

/// A section of the character sheet (e.g., "Ability Scores", "Skills", "Combat").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaSection {
    /// Unique section identifier
    pub id: String,
    /// Display label for the section header
    pub label: String,
    /// Type of section (affects layout)
    pub section_type: SectionType,
    /// Fields within this section
    pub fields: Vec<FieldDefinition>,
    /// Whether this section is collapsible
    #[serde(default)]
    pub collapsible: bool,
    /// Whether collapsed by default
    #[serde(default)]
    pub collapsed_default: bool,
    /// Help text for the section
    #[serde(default)]
    pub description: Option<String>,
}

/// Type of section, affects rendering layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionType {
    /// Ability scores / attributes (grid layout)
    AbilityScores,
    /// Skills (list with checkboxes for proficiency)
    Skills,
    /// Combat stats (AC, HP, speed, etc.)
    Combat,
    /// Spellcasting section
    Spellcasting,
    /// Resources (stress, fate points, etc.)
    Resources,
    /// Inventory/equipment
    Inventory,
    /// Character info (name, background, etc.)
    Identity,
    /// Features and abilities
    Features,
    /// Progress clocks (Blades in the Dark)
    Clocks,
    /// Moves (PbtA)
    Moves,
    /// Active modifiers/conditions
    Modifiers,
    /// Experience/advancement tracking
    Advancement,
    /// Free-form section
    Custom,
    /// Unknown for forward compatibility
    #[serde(other)]
    Unknown,
}

// =============================================================================
// Field Definitions
// =============================================================================

/// Definition of a single field in the character sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Unique field identifier (matches stat/property name)
    pub id: String,
    /// Display label
    pub label: String,
    /// Field data type and rendering hints
    pub field_type: SchemaFieldType,
    /// Whether this field can be edited by players
    #[serde(default = "default_true")]
    pub editable: bool,
    /// Whether this field is required
    #[serde(default)]
    pub required: bool,
    /// If this is a calculated field, the formula reference
    #[serde(default)]
    pub derived_from: Option<DerivedField>,
    /// Validation rules
    #[serde(default)]
    pub validation: Option<FieldValidation>,
    /// Layout hints
    #[serde(default)]
    pub layout: FieldLayout,
    /// Help text / tooltip
    #[serde(default)]
    pub description: Option<String>,
    /// Placeholder text for empty fields
    #[serde(default)]
    pub placeholder: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Type of field data and how to render it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SchemaFieldType {
    /// Plain text input
    Text {
        #[serde(default)]
        multiline: bool,
        #[serde(default)]
        max_length: Option<usize>,
    },
    /// Integer number
    Integer {
        #[serde(default)]
        min: Option<i32>,
        #[serde(default)]
        max: Option<i32>,
        /// If true, display as +/- modifier
        #[serde(default)]
        show_modifier: bool,
    },
    /// D&D-style ability score with modifier display
    AbilityScore {
        #[serde(default)]
        min: Option<i32>,
        #[serde(default)]
        max: Option<i32>,
    },
    /// Skill with proficiency level
    Skill {
        /// The ability this skill is based on
        ability: String,
        /// Available proficiency levels
        proficiency_levels: Vec<ProficiencyOption>,
    },
    /// Saving throw
    SavingThrow {
        /// The ability for this save
        ability: String,
    },
    /// Boolean checkbox
    Boolean {
        /// Label for checked state
        #[serde(default)]
        checked_label: Option<String>,
        /// Label for unchecked state
        #[serde(default)]
        unchecked_label: Option<String>,
    },
    /// Selection from options
    Select {
        options: Vec<SchemaSelectOption>,
        #[serde(default)]
        allow_custom: bool,
    },
    /// Multiple selection
    MultiSelect {
        options: Vec<SchemaSelectOption>,
        #[serde(default)]
        max_selections: Option<usize>,
    },
    /// HP / resource bar
    ResourceBar {
        /// ID of the max value field
        max_field: String,
        /// Color theme
        #[serde(default)]
        color: ResourceColor,
    },
    /// Dice pool (Blades, WoD)
    DicePool {
        /// Maximum dice in pool
        max_dice: u8,
        /// Die type (d6, d10, etc.)
        die_type: u8,
    },
    /// Ladder rating (FATE)
    LadderRating {
        /// Minimum rating value
        min: i32,
        /// Maximum rating value
        max: i32,
        /// Rating labels
        labels: Vec<LadderLabel>,
    },
    /// Percentile skill (CoC)
    PercentileSkill {
        /// Whether to show half/fifth values
        #[serde(default)]
        show_derived: bool,
    },
    /// Progress clock (Blades)
    Clock {
        /// Number of segments
        segments: u8,
    },
    /// Harm/condition track
    ConditionTrack {
        /// Levels of the track
        levels: Vec<ConditionLevel>,
    },
    /// Reference to another entity (class, race, etc.)
    EntityRef {
        /// Type of entity being referenced
        entity_type: EntityRefType,
    },
    /// Tags/keywords list
    Tags,
    /// XP progress bar showing current XP vs next level threshold
    XpProgress {
        /// Field ID for current XP
        current_field: String,
        /// Field ID for XP needed for next level (derived)
        next_level_field: String,
    },
    /// List of active stat modifiers (conditions, effects, etc.)
    ModifierList {
        /// Which stat this modifier list is for (None = all modifiers)
        #[serde(default)]
        filter_stat: Option<String>,
    },
    /// Unknown for forward compatibility
    #[serde(other)]
    Unknown,
}

/// Option for select fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaSelectOption {
    /// Internal value
    pub value: String,
    /// Display label
    pub label: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}

/// Proficiency option for skills.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProficiencyOption {
    /// Internal value
    pub value: String,
    /// Display label
    pub label: String,
    /// Multiplier for proficiency bonus
    pub multiplier: f32,
}

/// Label for ladder ratings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LadderLabel {
    pub value: i32,
    pub label: String,
}

/// Level in a condition/harm track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionLevel {
    pub level: u8,
    pub label: String,
    #[serde(default)]
    pub effect: Option<String>,
}

/// Color theme for resource bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ResourceColor {
    #[default]
    Red,
    Blue,
    Green,
    Purple,
    Orange,
    Gray,
}

/// Type of entity reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityRefType {
    Class,
    Race,
    Background,
    Playbook,
    Archetype,
    Occupation,
    Custom,
}

// =============================================================================
// Derived Fields
// =============================================================================

/// Specification for a calculated/derived field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedField {
    /// Type of derivation
    pub derivation_type: DerivationType,
    /// Fields this depends on
    pub dependencies: Vec<String>,
    /// Optional display format
    #[serde(default)]
    pub display_format: Option<String>,
}

/// How a field is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DerivationType {
    /// D&D-style ability modifier: floor((score - 10) / 2)
    AbilityModifier,
    /// Proficiency bonus from level
    ProficiencyBonus,
    /// Skill modifier (ability + proficiency)
    SkillModifier,
    /// Save modifier (ability + proficiency if proficient)
    SaveModifier,
    /// Spell save DC (8 + prof + stat mod)
    SpellSaveDc,
    /// Spell attack (prof + stat mod)
    SpellAttack,
    /// Sum of dependent fields
    Sum,
    /// Maximum of dependent fields
    Max,
    /// Half of dependent field (rounded down)
    HalfDown,
    /// Fifth of dependent field (CoC)
    Fifth,
    /// Custom formula (evaluated server-side)
    Custom,
}

// =============================================================================
// Validation
// =============================================================================

/// Validation rules for a field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldValidation {
    /// Minimum value (for numeric)
    #[serde(default)]
    pub min: Option<i32>,
    /// Maximum value (for numeric)
    #[serde(default)]
    pub max: Option<i32>,
    /// Regex pattern (for text)
    #[serde(default)]
    pub pattern: Option<String>,
    /// Error message for validation failure
    #[serde(default)]
    pub message: Option<String>,
}

/// Layout hints for fields in a section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FieldLayout {
    /// Grid column span (default = 1)
    #[serde(default = "default_one")]
    pub column_span: u8,
    /// Alignment within grid cell
    #[serde(default)]
    pub alignment: FieldAlignment,
}

fn default_one() -> u8 {
    1
}

/// Alignment of field within its grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FieldAlignment {
    #[default]
    Left,
    Center,
    Right,
}

// =============================================================================
// Character Creation Steps
// =============================================================================

/// A step in character creation flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreationStep {
    /// Unique identifier for the step
    pub id: String,
    /// Display label
    pub label: String,
    /// Description of what happens in this step
    pub description: Option<String>,
    /// Sections to show in this step
    pub sections: Vec<String>,
    /// Whether this step is optional
    #[serde(default)]
    pub optional: bool,
}

// =============================================================================
// Point Buy / Allocation Systems
// =============================================================================

/// Point allocation system (used in character creation)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AllocationSystem {
    /// Fixed array of stats
    StandardArray {
        /// Available arrays
        arrays: Vec<Vec<i32>>,
        target_fields: Vec<String>,
        unique_assignment: bool,
    },
    /// Point buy system
    PointBuy {
        /// Total points available
        points: i32,
        min_value: i32,
        max_value: i32,
        base_value: i32,
        /// Cost per stat value
        cost_table: Vec<PointCost>,
        target_fields: Vec<String>,
    },
    /// Roll dice for stats
    RollStats {
        /// Dice formula (e.g., "4d6k3")
        formula: String,
    },
    /// Manual entry
    Manual,
    /// Dot pool allocation (Blades)
    DotPool {
        total_dots: u8,
        max_per_field: u8,
        categories: Vec<DotPoolCategory>,
        starting_dots: Vec<StartingDot>,
    },
    /// Dice roll allocation (legacy)
    DiceRoll {
        formula: String,
        description: String,
        roll_count: u8,
        target_fields: Vec<String>,
        allow_reroll: bool,
        minimum_total: Option<i32>,
    },
    /// Percentile pool allocation (CoC)
    PercentilePool {
        total_points: i32,
        min_per_field: i32,
        max_per_field: i32,
        categories: Vec<PercentileCategory>,
    },
    /// Free allocation (CoC quick-fire)
    FreeAllocation {
        total_points: i32,
        min_per_field: i32,
        max_per_field: i32,
        target_fields: Vec<String>,
    },
    /// Stat array allocation
    StatArray {
        arrays: Vec<StatArrayOption>,
        target_fields: Vec<String>,
    },
    /// Boost/flaw allocation (PF2e)
    BoostFlaw {
        boost_sources: Vec<BoostSource>,
        optional_flaws: bool,
        base_value: i32,
        max_value: i32,
        target_fields: Vec<String>,
    },
    /// Pyramid allocation (Fate)
    Pyramid {
        apex: i32,
        base: i32,
        rows: Vec<Vec<i32>>,
        level_labels: Vec<LadderLabel>,
        target_fields: Vec<String>,
    },
    /// Unknown for forward compatibility
    #[serde(other)]
    Unknown,
}

/// Point cost for a stat value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointCost {
    pub value: i32,
    pub cost: i32,
}

/// Stat array option for standard array allocation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatArrayOption {
    pub id: String,
    pub description: Option<String>,
    pub values: Vec<i32>,
}

/// Input type for character creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputType {
    /// Free text input
    Text,
    /// Selection from options
    Select,
    /// Numeric input
    Number,
    /// Boolean toggle
    Boolean,
    /// Multi-select
    MultiSelect,
}

/// Field input default
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputDefault {
    /// Default value (if any)
    pub value: SheetValue,
}

/// Trait indicating a field can be derived
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DerivationTypeLocation {
    /// Derived from another field
    Derived,
    /// User-entered
    Direct,
}

/// Field definition for character creation flow
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreationField {
    /// Field identifier
    pub id: String,
    /// Display label
    pub label: String,
    /// Input type
    pub input_type: InputType,
    /// Whether required
    #[serde(default)]
    pub required: bool,
    /// Default value
    pub default_value: Option<InputDefault>,
    /// Additional metadata for UI
    #[serde(default)]
    pub metadata: Option<SheetValue>,
}

// =============================================================================
// Field Validation
// =============================================================================

/// Validation for character creation fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldValidationRule {
    /// Field to validate
    pub field_id: String,
    /// Validation rule type
    pub rule_type: ValidationRuleType,
    /// Error message
    pub message: String,
}

/// Type of validation rule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationRuleType {
    /// Required field
    Required,
    /// Numeric range
    Range { min: i32, max: i32 },
    /// String length
    Length { min: usize, max: usize },
    /// Regex pattern
    Pattern { regex: String },
    /// Custom validation (handled in engine)
    Custom,
}

// =============================================================================
// Field Layout Definitions
// =============================================================================

/// Layout for character sheet sections
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionLayout {
    /// Section type
    pub section_type: SectionType,
    /// Layout columns
    pub columns: u8,
    /// Optional section-specific metadata
    #[serde(default)]
    pub metadata: Option<SheetValue>,
}

/// Definition of how a field should be laid out
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldLayoutDefinition {
    pub field_id: String,
    pub layout: FieldLayout,
}

// =============================================================================
// Character Sheet Values
// =============================================================================
/// Field change payload
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheetFieldChange {
    pub field_id: String,
    pub new_value: SheetValue,
    pub actor_id: Option<String>,
}

/// Apply multiple field changes to a character sheet
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheetUpdate {
    pub character_id: String,
    pub changes: Vec<CharacterSheetFieldChange>,
}

// =============================================================================
// Derived Field Definitions
// =============================================================================

/// Derived field definition for schema generation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedFieldDefinition {
    pub id: String,
    pub derivation_type: DerivationType,
    pub dependencies: Vec<String>,
}

/// Resource allocation configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub system: AllocationSystem,
    pub fields: Vec<String>,
}

/// Stats array definition for standard arrays
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatArray {
    pub name: String,
    pub values: Vec<i32>,
}

/// User-configurable schema variant
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaVariantConfig {
    pub variant_id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub base_schema_id: Option<String>,
    pub overrides: Option<SheetValue>,
}

/// Mapping between schema field IDs and domain stats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaStatMapping {
    pub field_id: String,
    pub stat_name: String,
}

/// Field visibility rules
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldVisibilityRule {
    pub field_id: String,
    pub rule: VisibilityRule,
}

/// Visibility rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VisibilityRule {
    /// Always visible
    Always,
    /// Visible when a field has a specific value
    FieldValue { field_id: String, value: SheetValue },
    /// Visible when any of multiple values match
    FieldValueAny {
        field_id: String,
        values: Vec<SheetValue>,
    },
    /// Visible when field is non-empty
    FieldValuePresent { field_id: String },
    /// Visible when a derived value meets a condition
    DerivedValue {
        field_id: String,
        comparison: ComparisonOperator,
        value: SheetValue,
    },
}

/// Comparison operator for derived value checks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
}

// =============================================================================
// Utility Types
// =============================================================================

/// Source of boosts in character creation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoostSource {
    pub source_type: String,
    pub boosts: Vec<String>,
}

/// Boost definition for point buy systems
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoostDefinition {
    pub id: String,
    pub label: String,
    pub boosts: Vec<String>,
}

/// Dot pool category for Blades
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DotPoolCategory {
    pub id: String,
    pub label: String,
    pub max: u8,
    pub dots: u8,
    pub fields: Vec<String>,
}

/// Definition of derived fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedFieldDefinitionLegacy {
    pub id: String,
    pub derivation_type: DerivationType,
    pub dependencies: Vec<String>,
}

/// Definition for derived values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedValueDefinition {
    pub id: String,
    pub derivation_type: DerivationType,
    pub dependencies: Vec<String>,
}

/// Percentile allocation category (CoC)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PercentileCategory {
    pub id: String,
    pub label: String,
    pub points: i32,
    pub fields: Vec<String>,
    #[serde(default)]
    pub formula: Option<String>,
}

/// Starting dot allocation (Blades)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartingDot {
    pub field: String,
    pub dots: u8,
    pub source: String,
}

/// Definition for character sheet asset prompts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheetAssetPrompt {
    pub field_id: String,
    pub prompt: String,
}

/// Rule system configuration (legacy)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleSystemConfigLegacy {
    pub name: String,
    pub description: Option<String>,
    pub variant: String,
    pub metadata: Option<SheetValue>,
}
