//! Game system traits for TTRPG-specific mechanics.
//!
//! These traits define the interface for system-specific calculations
//! and mechanics, allowing different TTRPGs to implement their own rules
//! while sharing a common API.

use crate::entities::{ContentFilter, ContentItem, ContentType, StatBlock, StatModifier};
use std::collections::HashMap;
use thiserror::Error;

// Re-export character sheet schema types for game system implementations
pub use crate::character_sheet::{
    AllocationSystem, BoostSource, CharacterSheetSchema, ConditionLevel, CreationStep,
    DerivedField, DerivationType, DotPoolCategory, FieldDefinition, FieldLayout, FieldValidation,
    LadderLabel, PercentileCategory, PointCost, ProficiencyOption, ResourceColor, SchemaFieldType,
    SchemaSection, SchemaSelectOption, SectionType, StartingDot, StatArrayOption,
};

/// Core trait all game systems must implement.
///
/// This trait provides system identification and access to the calculation engine.
pub trait GameSystem: Send + Sync {
    /// Unique identifier for this game system (e.g., "dnd5e", "pf2e").
    fn system_id(&self) -> &str;

    /// Human-readable display name (e.g., "D&D 5th Edition").
    fn display_name(&self) -> &str;

    /// Get the calculation engine for this system.
    fn calculation_engine(&self) -> &dyn CalculationEngine;

    /// Optional: Get the spellcasting system if this system has spellcasting.
    fn spellcasting_system(&self) -> Option<&dyn SpellcastingSystem> {
        None
    }

    /// List of stat names used by this system.
    fn stat_names(&self) -> &[&str];

    /// List of skill names used by this system.
    fn skill_names(&self) -> &[&str];
}

/// Calculation rules that vary per game system.
///
/// Implements the mathematical formulas specific to each TTRPG.
pub trait CalculationEngine: Send + Sync {
    /// Calculate ability modifier from score.
    ///
    /// For D&D-like systems: floor((score - 10) / 2)
    /// For percentile systems: might return the score directly
    fn ability_modifier(&self, score: i32) -> i32;

    /// Calculate proficiency bonus from character level.
    ///
    /// For D&D 5e: ((level - 1) / 4) + 2
    /// For systems without proficiency: returns 0
    fn proficiency_bonus(&self, level: u8) -> i32;

    /// Calculate spell save DC.
    ///
    /// For D&D 5e: 8 + proficiency + casting stat modifier
    fn spell_save_dc(&self, stats: &StatBlock, casting_stat: &str) -> i32;

    /// Calculate spell attack bonus.
    ///
    /// For D&D 5e: proficiency + casting stat modifier
    fn spell_attack_bonus(&self, stats: &StatBlock, casting_stat: &str) -> i32;

    /// Calculate attack bonus for a weapon attack.
    fn attack_bonus(&self, stats: &StatBlock, attack_stat: &str, proficient: bool) -> i32;

    /// Stack multiple modifiers according to system rules.
    ///
    /// For D&D 5e: Most bonuses don't stack (take highest), untyped stack
    /// For PF2e: Stack by type
    fn stack_modifiers(&self, modifiers: &[StatModifier]) -> i32;

    /// Calculate Armor Class from stats and equipment.
    fn calculate_ac(
        &self,
        stats: &StatBlock,
        armor_ac: Option<i32>,
        shield_bonus: Option<i32>,
        allows_dex: bool,
        max_dex_bonus: Option<i32>,
    ) -> i32;

    /// Calculate skill check modifier.
    fn skill_modifier(
        &self,
        stats: &StatBlock,
        ability: &str,
        proficiency_level: ProficiencyLevel,
    ) -> i32;

    /// Calculate saving throw modifier.
    fn saving_throw_modifier(
        &self,
        stats: &StatBlock,
        ability: &str,
        proficient: bool,
    ) -> i32;

    /// Calculate passive perception (or equivalent).
    fn passive_perception(&self, stats: &StatBlock, proficiency_level: ProficiencyLevel) -> i32;

    /// Get the hit die size for a class.
    fn hit_die(&self, class_name: &str) -> u8;

    /// Calculate max HP for a character.
    fn calculate_max_hp(
        &self,
        level: u8,
        class_name: &str,
        constitution_modifier: i32,
        additional_hp: i32,
    ) -> i32;
}

/// Proficiency level for skills and saves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProficiencyLevel {
    /// Not proficient
    None,
    /// Half proficiency (Jack of All Trades, etc.)
    Half,
    /// Standard proficiency
    Proficient,
    /// Expertise (double proficiency)
    Expert,
}

impl ProficiencyLevel {
    /// Get the multiplier for this proficiency level.
    pub fn multiplier(&self) -> f32 {
        match self {
            ProficiencyLevel::None => 0.0,
            ProficiencyLevel::Half => 0.5,
            ProficiencyLevel::Proficient => 1.0,
            ProficiencyLevel::Expert => 2.0,
        }
    }
}

/// For systems with spellcasting.
pub trait SpellcastingSystem: Send + Sync {
    /// Get the caster type for a class (if it has spellcasting).
    fn caster_type(&self, class: &str) -> Option<CasterType>;

    /// Get the spellcasting ability for a class.
    fn spellcasting_stat(&self, class: &str) -> Option<&str>;

    /// Whether this class uses spell preparation.
    fn uses_spell_preparation(&self, class: &str) -> bool;

    /// Calculate maximum prepared spells for a class.
    fn max_prepared_spells(&self, class: &str, level: u8, stat_mod: i32) -> u8;

    /// Get spell slots for a class at a given level.
    fn spell_slots(&self, class: &str, level: u8) -> HashMap<u8, u8>;

    /// Get cantrips known for a class at a given level.
    fn cantrips_known(&self, class: &str, level: u8) -> u8;

    /// Get spells known for a class at a given level (for known-spell casters).
    fn spells_known(&self, class: &str, level: u8) -> Option<u8>;
}

/// Type of spellcaster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasterType {
    /// Full caster (Wizard, Cleric, Druid, Sorcerer, Bard)
    Full,
    /// Half caster (Paladin, Ranger)
    Half,
    /// Third caster (Eldritch Knight, Arcane Trickster)
    Third,
    /// Pact magic (Warlock)
    Pact,
    /// Innate spellcasting (racial abilities)
    Innate,
}

impl CasterType {
    /// Get the caster level for multiclassing calculations.
    pub fn effective_caster_levels(&self, class_level: u8) -> u8 {
        match self {
            CasterType::Full => class_level,
            CasterType::Half => class_level / 2,
            CasterType::Third => class_level / 3,
            CasterType::Pact => 0, // Warlock doesn't contribute to multiclass slots
            CasterType::Innate => 0,
        }
    }
}

/// Rest type for resource recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestType {
    /// Short rest (typically 1 hour)
    Short,
    /// Long rest (typically 8 hours)
    Long,
}

/// Trait for generating character sheet schemas.
///
/// Game systems implement this to describe what fields their character
/// sheets need. The engine uses these schemas to drive client rendering.
pub trait CharacterSheetProvider: Send + Sync {
    /// Generate the character sheet schema for this game system.
    ///
    /// Returns a complete schema with all sections, fields, and creation steps.
    fn character_sheet_schema(&self) -> CharacterSheetSchema;

    /// Calculate derived field values.
    ///
    /// Given a map of field IDs to values, calculate all derived values.
    fn calculate_derived_values(
        &self,
        values: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value>;

    /// Validate a field value.
    ///
    /// Returns None if valid, or an error message if invalid.
    fn validate_field(
        &self,
        field_id: &str,
        value: &serde_json::Value,
        all_values: &HashMap<String, serde_json::Value>,
    ) -> Option<String>;

    /// Get default values for all fields.
    fn default_values(&self) -> HashMap<String, serde_json::Value>;
}

/// Error type for content loading operations.
#[derive(Debug, Error)]
pub enum ContentError {
    /// Failed to load content from a data source.
    #[error("Failed to load content: {0}")]
    LoadError(String),

    /// Content type not supported by this system.
    #[error("Content type '{0}' not supported by this system")]
    UnsupportedContentType(String),

    /// Content not found.
    #[error("Content not found: {0}")]
    NotFound(String),

    /// IO error during content loading.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON parsing error.
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Schema for filtering content of a specific type.
///
/// Describes what filter options are available for a content type.
#[derive(Debug, Clone, Default)]
pub struct FilterSchema {
    /// Available source books to filter by
    pub sources: Vec<String>,

    /// Available tags to filter by
    pub tags: Vec<String>,

    /// Whether text search is supported
    pub supports_search: bool,

    /// Additional filter fields specific to this content type
    pub custom_fields: Vec<FilterField>,
}

/// A custom filter field for content queries.
#[derive(Debug, Clone)]
pub struct FilterField {
    /// Machine-readable field ID
    pub id: String,

    /// Human-readable label
    pub label: String,

    /// Type of filter
    pub field_type: FilterFieldType,
}

/// Type of filter field.
#[derive(Debug, Clone)]
pub enum FilterFieldType {
    /// Single select from options
    Select(Vec<String>),

    /// Multi-select from options
    MultiSelect(Vec<String>),

    /// Numeric range (min, max)
    Range(i32, i32),

    /// Boolean toggle
    Boolean,
}

/// Trait for systems that provide compendium content.
///
/// Game systems implement this trait to expose their content (races, classes,
/// spells, etc.) through a unified API. Each system can use its own data
/// sources (5etools JSON, Pf2eTools, manual entry) while providing a
/// consistent interface.
///
/// # Example
///
/// ```ignore
/// impl CompendiumProvider for Dnd5eSystem {
///     fn content_types(&self) -> Vec<ContentType> {
///         vec![
///             ContentType::CharacterOrigin,    // Races
///             ContentType::CharacterClass,
///             ContentType::CharacterBackground,
///             ContentType::Spell,
///         ]
///     }
///
///     fn load_content(
///         &self,
///         content_type: &ContentType,
///         filter: &ContentFilter,
///     ) -> Result<Vec<ContentItem>, ContentError> {
///         match content_type {
///             ContentType::CharacterOrigin => self.load_races(filter),
///             ContentType::CharacterClass => self.load_classes(filter),
///             _ => Err(ContentError::UnsupportedContentType(content_type.to_string())),
///         }
///     }
/// }
/// ```
pub trait CompendiumProvider: Send + Sync {
    /// Get the content types this system supports.
    ///
    /// Returns a list of all content types that can be queried from this system.
    fn content_types(&self) -> Vec<ContentType>;

    /// Check if this system supports a specific content type.
    fn supports_content_type(&self, content_type: &ContentType) -> bool {
        self.content_types().contains(content_type)
    }

    /// Load content of a specific type.
    ///
    /// Returns all items matching the filter, converted to the unified ContentItem format.
    fn load_content(
        &self,
        content_type: &ContentType,
        filter: &ContentFilter,
    ) -> Result<Vec<ContentItem>, ContentError>;

    /// Get a single content item by ID.
    ///
    /// The default implementation loads all content and filters by ID.
    /// Systems should override this for better performance.
    fn get_content_by_id(
        &self,
        content_type: &ContentType,
        id: &str,
    ) -> Result<Option<ContentItem>, ContentError> {
        let items = self.load_content(content_type, &ContentFilter::default())?;
        Ok(items.into_iter().find(|item| item.id == id))
    }

    /// Get the filter schema for a content type.
    ///
    /// Returns None if the content type is not supported.
    fn filter_schema(&self, content_type: &ContentType) -> Option<FilterSchema>;

    /// Count content items of a specific type without loading all data.
    ///
    /// The default implementation loads content and counts it. Providers
    /// can override this for better performance by counting from cached
    /// data without full conversion to ContentItem.
    fn count_content(&self, content_type: &ContentType) -> Result<usize, ContentError> {
        let items = self.load_content(content_type, &ContentFilter::default())?;
        Ok(items.len())
    }

    /// Get content statistics for this system.
    fn content_stats(&self) -> HashMap<ContentType, usize> {
        let mut stats = HashMap::new();
        for ct in self.content_types() {
            if let Ok(count) = self.count_content(&ct) {
                stats.insert(ct, count);
            }
        }
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proficiency_level_multipliers() {
        assert_eq!(ProficiencyLevel::None.multiplier(), 0.0);
        assert_eq!(ProficiencyLevel::Half.multiplier(), 0.5);
        assert_eq!(ProficiencyLevel::Proficient.multiplier(), 1.0);
        assert_eq!(ProficiencyLevel::Expert.multiplier(), 2.0);
    }

    #[test]
    fn caster_type_effective_levels() {
        assert_eq!(CasterType::Full.effective_caster_levels(5), 5);
        assert_eq!(CasterType::Half.effective_caster_levels(6), 3);
        assert_eq!(CasterType::Third.effective_caster_levels(9), 3);
        assert_eq!(CasterType::Pact.effective_caster_levels(10), 0);
    }
}
