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

// =============================================================================
// Character Sheet Schema
// =============================================================================

/// Complete schema for rendering a character sheet.
///
/// Sent by the engine to describe what fields/sections a character sheet
/// should display for a given game system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "snake_case", tag = "type")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct LadderLabel {
    pub value: i32,
    pub label: String,
}

/// Level in a condition/harm track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionLevel {
    pub level: u8,
    pub label: String,
    #[serde(default)]
    pub effect: Option<String>,
}

/// Color theme for resource bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "camelCase")]
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
    pub error_message: Option<String>,
}

// =============================================================================
// Layout
// =============================================================================

/// Layout hints for field rendering.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldLayout {
    /// Width of field (1-12 grid columns)
    #[serde(default)]
    pub width: Option<u8>,
    /// Whether to start a new row before this field
    #[serde(default)]
    pub new_row: bool,
    /// CSS class to apply
    #[serde(default)]
    pub css_class: Option<String>,
    /// Display order (lower = first)
    #[serde(default)]
    pub order: Option<i32>,
}

// =============================================================================
// Character Creation
// =============================================================================

/// A step in the character creation process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationStep {
    /// Step identifier
    pub id: String,
    /// Step display name
    pub label: String,
    /// Step description
    pub description: String,
    /// Sections included in this step
    pub section_ids: Vec<String>,
    /// Order of this step
    pub order: u8,
    /// Whether this step is required
    #[serde(default = "default_true")]
    pub required: bool,
    /// Allocation system for this step (if applicable)
    #[serde(default)]
    pub allocation: Option<AllocationSystem>,
}

// =============================================================================
// Allocation Systems
// =============================================================================

/// Allocation system for distributing points/values during character creation.
///
/// Different TTRPGs use different allocation mechanics for stats:
/// - D&D 5e: Point Buy, Standard Array, or Rolling
/// - PF2e: Ancestry/Background/Class boosts
/// - CoC 7e: Rolling or Point Pool
/// - FATE: Skill Pyramid
/// - Blades: Dot Pool
/// - PbtA: Stat Arrays per playbook
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AllocationSystem {
    /// Point buy system (D&D 5e style)
    /// Players spend points to buy stat values
    PointBuy {
        /// Total points available to spend
        total_points: u32,
        /// Minimum stat value allowed
        min_value: i32,
        /// Maximum stat value allowed (before racial bonuses)
        max_value: i32,
        /// Default starting value for each stat
        base_value: i32,
        /// Cost table: value -> point cost
        cost_table: Vec<PointCost>,
        /// Fields this applies to
        target_fields: Vec<String>,
    },

    /// Standard array selection (D&D 5e style)
    /// Players assign predetermined values to stats
    StandardArray {
        /// The array of values to assign
        values: Vec<i32>,
        /// Fields to assign values to
        target_fields: Vec<String>,
        /// Whether each value can only be used once
        #[serde(default = "default_true")]
        unique_assignment: bool,
    },

    /// Dice rolling for stats
    /// Players roll dice to determine stat values
    DiceRoll {
        /// Dice formula (e.g., "4d6kh3" for 4d6 keep highest 3)
        formula: String,
        /// Human-readable description
        description: String,
        /// Number of rolls to generate
        roll_count: u8,
        /// Fields to assign rolled values to
        target_fields: Vec<String>,
        /// Whether to allow rerolling
        #[serde(default)]
        allow_reroll: bool,
        /// Minimum total for all stats (reroll if below)
        #[serde(default)]
        minimum_total: Option<i32>,
    },

    /// Boost/Flaw system (PF2e style)
    /// Players apply boosts (+2) and flaws (-2) from various sources
    BoostFlaw {
        /// Boost sources (ancestry, background, class, free)
        boost_sources: Vec<BoostSource>,
        /// Whether flaws are optional
        #[serde(default)]
        optional_flaws: bool,
        /// Target fields for boosts
        target_fields: Vec<String>,
        /// Base value for all stats
        base_value: i32,
        /// Maximum value after boosts (at creation)
        max_value: i32,
    },

    /// Skill pyramid (FATE style)
    /// Skills must form a pyramid shape (more at lower tiers)
    Pyramid {
        /// Maximum skill level (apex of pyramid)
        apex: i32,
        /// Minimum skill level
        base: i32,
        /// Target skill fields
        target_fields: Vec<String>,
        /// Labels for each level
        level_labels: Vec<LadderLabel>,
    },

    /// Dot pool allocation (Blades in the Dark, World of Darkness)
    /// Players distribute dots among actions/skills
    DotPool {
        /// Total dots to distribute
        total_dots: u8,
        /// Maximum dots per field
        max_per_field: u8,
        /// Categories of fields with separate pools
        categories: Vec<DotPoolCategory>,
        /// Starting dots (e.g., from playbook)
        #[serde(default)]
        starting_dots: Vec<StartingDot>,
    },

    /// Fixed array selection per playbook/archetype (PbtA style)
    /// Players choose one of several predetermined stat arrays
    StatArray {
        /// Available arrays to choose from
        arrays: Vec<StatArrayOption>,
        /// Target stat fields
        target_fields: Vec<String>,
    },

    /// Percentile pool allocation (CoC 7e style)
    /// Players distribute a pool of percentage points
    PercentilePool {
        /// Total points in the pool
        total_points: u32,
        /// Minimum value per skill
        min_per_field: u8,
        /// Maximum value per skill (at creation)
        max_per_field: u8,
        /// Categories with separate pools
        categories: Vec<PercentileCategory>,
    },

    /// Free allocation with total constraint
    /// Players distribute points freely up to a total
    FreeAllocation {
        /// Total points to distribute
        total_points: u32,
        /// Minimum per field
        min_per_field: i32,
        /// Maximum per field
        max_per_field: i32,
        /// Target fields
        target_fields: Vec<String>,
    },
}

/// Point cost entry for point buy systems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointCost {
    /// The stat value
    pub value: i32,
    /// Point cost to achieve this value
    pub cost: u32,
}

/// Source of ability boosts for PF2e-style systems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoostSource {
    /// Source identifier (e.g., "ancestry", "background", "class")
    pub id: String,
    /// Display name
    pub label: String,
    /// Number of free boosts (any stat)
    pub free_boosts: u8,
    /// Fixed boosts (specific stats)
    #[serde(default)]
    pub fixed_boosts: Vec<String>,
    /// Number of flaws to apply
    #[serde(default)]
    pub flaws: u8,
    /// Fixed flaws (specific stats)
    #[serde(default)]
    pub fixed_flaws: Vec<String>,
    /// Whether this source has been applied
    #[serde(default)]
    pub applied: bool,
}

/// Category for dot pool allocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DotPoolCategory {
    /// Category identifier
    pub id: String,
    /// Display name
    pub label: String,
    /// Dots available for this category
    pub dots: u8,
    /// Fields in this category
    pub fields: Vec<String>,
}

/// Starting dots from playbook or archetype.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartingDot {
    /// Field to receive starting dots
    pub field: String,
    /// Number of starting dots
    pub dots: u8,
    /// Source (playbook name, etc.)
    pub source: String,
}

/// Stat array option for PbtA-style selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatArrayOption {
    /// Option identifier
    pub id: String,
    /// Description (e.g., "Tough but not very bright")
    #[serde(default)]
    pub description: Option<String>,
    /// Values for each target field (in order)
    pub values: Vec<i32>,
}

/// Category for percentile pool allocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PercentileCategory {
    /// Category identifier (e.g., "occupation", "personal")
    pub id: String,
    /// Display name
    pub label: String,
    /// Points available in this pool
    pub points: u32,
    /// Fields this pool can be applied to
    pub fields: Vec<String>,
    /// Formula for calculating points (e.g., "EDU * 4")
    #[serde(default)]
    pub formula: Option<String>,
}

// =============================================================================
// Character Data Exchange
// =============================================================================

/// Character sheet response with schema and values for rendering.
///
/// Combines the schema with the character's current values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSheetResponse {
    /// Character ID
    pub character_id: String,
    /// Character name
    pub name: String,
    /// The schema to use for rendering
    pub schema: CharacterSheetSchema,
    /// Current field values (field_id -> value)
    pub values: std::collections::HashMap<String, serde_json::Value>,
    /// Calculated/derived values (field_id -> calculated value)
    pub calculated: std::collections::HashMap<String, serde_json::Value>,
}

/// Update to a character sheet field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldUpdate {
    /// Field ID being updated
    pub field_id: String,
    /// New value
    pub value: serde_json::Value,
}

/// Response after updating a field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldUpdateResponse {
    /// Whether the update was successful
    pub success: bool,
    /// Updated calculated values (if any derived fields changed)
    #[serde(default)]
    pub updated_calculated: std::collections::HashMap<String, serde_json::Value>,
    /// Validation errors (if any)
    #[serde(default)]
    pub errors: Vec<ValidationError>,
}

/// Validation error for a field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    /// Field ID with the error
    pub field_id: String,
    /// Error message
    pub message: String,
}

// =============================================================================
// Character Sheet Data Storage
// =============================================================================

/// Character sheet data storage using JSON values for flexibility.
///
/// This struct stores the actual field values for a character, using JSON
/// for flexible type handling across different game systems.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSheetData {
    /// Field values keyed by field ID
    #[serde(default)]
    pub values: std::collections::HashMap<String, serde_json::Value>,
}

impl CharacterSheetData {
    /// Create a new empty character sheet data
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a field value
    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.values.insert(key.into(), value);
    }

    /// Get a field value
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.values.get(key)
    }

    /// Get a field value as an integer (i64)
    pub fn get_number(&self, key: &str) -> Option<i64> {
        self.values.get(key).and_then(|v| v.as_i64())
    }

    /// Get a field value as an i32 (for stat/modifier values)
    ///
    /// This is the unified numeric extraction method that handles all field types:
    /// - Number: direct value
    /// - SkillEntry: bonus value
    /// - DicePool: dice count (for Blades)
    /// - Percentile: skill value (for CoC 7e)
    /// - LadderRating: ladder position (for FATE)
    pub fn get_numeric_value(&self, key: &str) -> Option<i32> {
        self.values.get(key).and_then(|v| {
            // Handle various JSON representations
            if let Some(n) = v.as_i64() {
                return Some(n as i32);
            }
            if let Some(n) = v.as_f64() {
                return Some(n as i32);
            }
            // Handle object types like SkillEntry with a "bonus" or "value" field
            if let Some(obj) = v.as_object() {
                if let Some(bonus) = obj.get("bonus").and_then(|b| b.as_i64()) {
                    return Some(bonus as i32);
                }
                if let Some(value) = obj.get("value").and_then(|b| b.as_i64()) {
                    return Some(value as i32);
                }
                // For dice pool, use dice count
                if let Some(dice) = obj.get("dice").and_then(|d| d.as_i64()) {
                    return Some(dice as i32);
                }
            }
            None
        })
    }

    /// Get a field value as a float
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.values.get(key).and_then(|v| v.as_f64())
    }

    /// Get a field value as a string
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.values.get(key).and_then(|v| v.as_str())
    }

    /// Get a field value as a boolean
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.values.get(key).and_then(|v| v.as_bool())
    }

    /// Check if a field exists
    pub fn has(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Remove a field value
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.values.remove(key)
    }

    /// Get all field keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    /// Check if the sheet data is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type_serialization() {
        let field = SchemaFieldType::AbilityScore {
            min: Some(1),
            max: Some(30),
        };
        let json = serde_json::to_string(&field).unwrap();
        assert!(json.contains("ability_score"));

        let parsed: SchemaFieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, field);
    }

    #[test]
    fn test_section_type_serialization() {
        let section = SectionType::AbilityScores;
        let json = serde_json::to_string(&section).unwrap();
        assert_eq!(json, "\"ability_scores\"");
    }

    #[test]
    fn test_character_sheet_schema() {
        let schema = CharacterSheetSchema {
            system_id: "dnd5e".to_string(),
            system_name: "D&D 5th Edition".to_string(),
            sections: vec![SchemaSection {
                id: "abilities".to_string(),
                label: "Ability Scores".to_string(),
                section_type: SectionType::AbilityScores,
                fields: vec![FieldDefinition {
                    id: "STR".to_string(),
                    label: "Strength".to_string(),
                    field_type: SchemaFieldType::AbilityScore {
                        min: Some(1),
                        max: Some(30),
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(1),
                        max: Some(30),
                        pattern: None,
                        error_message: Some("Must be between 1 and 30".to_string()),
                    }),
                    layout: FieldLayout::default(),
                    description: Some("Physical power and carrying capacity".to_string()),
                    placeholder: None,
                }],
                collapsible: false,
                collapsed_default: false,
                description: None,
            }],
            creation_steps: vec![],
        };

        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("dnd5e"));
        assert!(json.contains("Strength"));
    }
}
