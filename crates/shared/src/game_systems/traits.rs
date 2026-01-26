//! Core traits and types for game system implementations.

use std::collections::HashMap;

pub use crate::character_sheet::{
    AllocationSystem, BoostSource, CharacterSheetSchema, ConditionLevel, CreationStep,
    DerivationType, DerivedField, DotPoolCategory, FieldDefinition, FieldLayout, FieldValidation,
    LadderLabel, PointCost, ProficiencyOption, ResourceColor, SchemaFieldType, SchemaSection,
    SchemaSelectOption, SectionType, SheetValue, StartingDot, StatArrayOption,
};
pub use crate::game_systems::content::{
    ContentError, ContentFilter, ContentItem, ContentType, FilterSchema,
};

use wrldbldr_domain::value_objects::{StatBlock, StatModifier};

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
    fn ability_modifier(&self, score: i32) -> i32;

    /// Calculate proficiency bonus from character level.
    fn proficiency_bonus(&self, level: u8) -> i32;

    /// Calculate spell save DC.
    fn spell_save_dc(&self, stats: &StatBlock, casting_stat: &str) -> i32;

    /// Calculate spell attack bonus.
    fn spell_attack_bonus(&self, stats: &StatBlock, casting_stat: &str) -> i32;

    /// Calculate attack bonus for a weapon attack.
    fn attack_bonus(&self, stats: &StatBlock, attack_stat: &str, proficient: bool) -> i32;

    /// Stack multiple modifiers according to system rules.
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
    fn saving_throw_modifier(&self, stats: &StatBlock, ability: &str, proficient: bool) -> i32;

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
            CasterType::Pact => 0,
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
    fn character_sheet_schema(&self) -> CharacterSheetSchema;

    /// Calculate derived field values.
    fn calculate_derived_values(
        &self,
        values: &HashMap<String, SheetValue>,
    ) -> HashMap<String, SheetValue>;

    /// Validate a field value.
    fn validate_field(
        &self,
        field_id: &str,
        value: &SheetValue,
        all_values: &HashMap<String, SheetValue>,
    ) -> Option<String>;

    /// Get default values for all fields.
    fn default_values(&self) -> HashMap<String, SheetValue>;
}

/// Trait for systems that provide compendium content.
///
/// Game systems implement this trait to expose their content (races, classes,
/// spells, etc.) through a unified API.
pub trait CompendiumProvider: Send + Sync {
    /// Get the content types this system supports.
    fn content_types(&self) -> Vec<ContentType>;

    /// Check if this system supports a specific content type.
    fn supports_content_type(&self, content_type: &ContentType) -> bool {
        self.content_types().contains(content_type)
    }

    /// Load content of a specific type.
    fn load_content(
        &self,
        content_type: &ContentType,
        filter: &ContentFilter,
    ) -> Result<Vec<ContentItem>, ContentError>;

    /// Get a single content item by ID.
    fn get_content_by_id(
        &self,
        content_type: &ContentType,
        id: &str,
    ) -> Result<Option<ContentItem>, ContentError> {
        let filter = ContentFilter::default();
        let items = self.load_content(content_type, &filter)?;
        Ok(items.into_iter().find(|item| item.id == id))
    }

    /// Count content items of a specific type.
    fn count_content(&self, content_type: &ContentType) -> Result<usize, ContentError> {
        let filter = ContentFilter::default();
        Ok(self.load_content(content_type, &filter)?.len())
    }

    /// Provide filter metadata for content types.
    fn filter_schema(&self, _content_type: &ContentType) -> Option<FilterSchema> {
        None
    }
}
