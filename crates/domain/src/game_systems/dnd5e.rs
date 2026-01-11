//! D&D 5th Edition game system implementation.
//!
//! Implements all calculation rules and spellcasting mechanics for D&D 5e.

use super::traits::{
    CalculationEngine, CasterType, CharacterSheetProvider, CharacterSheetSchema,
    CreationStep, DerivedField, DerivationType, FieldDefinition, FieldLayout,
    FieldValidation, GameSystem, ProficiencyLevel, ProficiencyOption, ResourceColor,
    SchemaFieldType, SchemaSection, SchemaSelectOption, SectionType, SpellcastingSystem,
};
use crate::entities::{StatBlock, StatModifier};
use std::collections::HashMap;

/// XP thresholds for each level in D&D 5e.
/// Index is level - 1 (so level 1 = index 0).
const XP_THRESHOLDS: [i32; 20] = [
    0,      // Level 1
    300,    // Level 2
    900,    // Level 3
    2700,   // Level 4
    6500,   // Level 5
    14000,  // Level 6
    23000,  // Level 7
    34000,  // Level 8
    48000,  // Level 9
    64000,  // Level 10
    85000,  // Level 11
    100000, // Level 12
    120000, // Level 13
    140000, // Level 14
    165000, // Level 15
    195000, // Level 16
    225000, // Level 17
    265000, // Level 18
    305000, // Level 19
    355000, // Level 20
];

/// Get XP required for a given level.
fn xp_for_level(level: u8) -> i32 {
    if level == 0 || level > 20 {
        return 0;
    }
    XP_THRESHOLDS[(level - 1) as usize]
}

/// Get XP required for the next level.
fn xp_for_next_level(current_level: u8) -> i32 {
    if current_level >= 20 {
        return XP_THRESHOLDS[19]; // Max level
    }
    XP_THRESHOLDS[current_level as usize]
}

/// Calculate level from current XP.
fn level_from_xp(xp: i32) -> u8 {
    for (i, &threshold) in XP_THRESHOLDS.iter().enumerate().rev() {
        if xp >= threshold {
            return (i + 1) as u8;
        }
    }
    1
}

/// D&D 5th Edition game system.
pub struct Dnd5eSystem;

impl Default for Dnd5eSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Dnd5eSystem {
    /// Create a new D&D 5e system instance.
    pub fn new() -> Self {
        Self
    }
}

impl GameSystem for Dnd5eSystem {
    fn system_id(&self) -> &str {
        "dnd5e"
    }

    fn display_name(&self) -> &str {
        "D&D 5th Edition"
    }

    fn calculation_engine(&self) -> &dyn CalculationEngine {
        self
    }

    fn spellcasting_system(&self) -> Option<&dyn SpellcastingSystem> {
        Some(self)
    }

    fn stat_names(&self) -> &[&str] {
        &["STR", "DEX", "CON", "INT", "WIS", "CHA"]
    }

    fn skill_names(&self) -> &[&str] {
        &[
            "Acrobatics",
            "Animal Handling",
            "Arcana",
            "Athletics",
            "Deception",
            "History",
            "Insight",
            "Intimidation",
            "Investigation",
            "Medicine",
            "Nature",
            "Perception",
            "Performance",
            "Persuasion",
            "Religion",
            "Sleight of Hand",
            "Stealth",
            "Survival",
        ]
    }
}

impl CalculationEngine for Dnd5eSystem {
    fn ability_modifier(&self, score: i32) -> i32 {
        // D&D uses floor division, Rust's / rounds toward zero
        // Use proper floor division: floor((score - 10) / 2)
        let diff = score - 10;
        if diff >= 0 {
            diff / 2
        } else {
            (diff - 1) / 2
        }
    }

    fn proficiency_bonus(&self, level: u8) -> i32 {
        ((level as i32 - 1) / 4) + 2
    }

    fn spell_save_dc(&self, stats: &StatBlock, casting_stat: &str) -> i32 {
        let stat_value = stats.get_stat(casting_stat).unwrap_or(10);
        let modifier = self.ability_modifier(stat_value);
        let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;
        let prof = self.proficiency_bonus(level);
        8 + modifier + prof
    }

    fn spell_attack_bonus(&self, stats: &StatBlock, casting_stat: &str) -> i32 {
        let stat_value = stats.get_stat(casting_stat).unwrap_or(10);
        let modifier = self.ability_modifier(stat_value);
        let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;
        let prof = self.proficiency_bonus(level);
        modifier + prof
    }

    fn attack_bonus(&self, stats: &StatBlock, attack_stat: &str, proficient: bool) -> i32 {
        let stat_value = stats.get_stat(attack_stat).unwrap_or(10);
        let modifier = self.ability_modifier(stat_value);
        if proficient {
            let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;
            modifier + self.proficiency_bonus(level)
        } else {
            modifier
        }
    }

    fn stack_modifiers(&self, modifiers: &[StatModifier]) -> i32 {
        // D&D 5e stacking rules:
        // - Same-named bonuses don't stack (take highest)
        // - Different-named bonuses do stack
        // - Penalties always stack
        // For simplicity, we take the highest active bonus
        modifiers
            .iter()
            .filter(|m| m.active)
            .map(|m| m.value)
            .max()
            .unwrap_or(0)
    }

    fn calculate_ac(
        &self,
        stats: &StatBlock,
        armor_ac: Option<i32>,
        shield_bonus: Option<i32>,
        allows_dex: bool,
        max_dex_bonus: Option<i32>,
    ) -> i32 {
        let dex = stats.get_stat("DEX").unwrap_or(10);
        let dex_mod = self.ability_modifier(dex);

        let base_ac = match armor_ac {
            Some(ac) => {
                // Armor provides a base AC
                if allows_dex {
                    let dex_bonus = match max_dex_bonus {
                        Some(max) => dex_mod.min(max),
                        None => dex_mod,
                    };
                    ac + dex_bonus
                } else {
                    ac
                }
            }
            None => 10 + dex_mod, // Unarmored: 10 + DEX (always applies)
        };

        base_ac + shield_bonus.unwrap_or(0)
    }

    fn skill_modifier(
        &self,
        stats: &StatBlock,
        ability: &str,
        proficiency_level: ProficiencyLevel,
    ) -> i32 {
        let stat_value = stats.get_stat(ability).unwrap_or(10);
        let modifier = self.ability_modifier(stat_value);
        let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;
        let prof = self.proficiency_bonus(level);

        let prof_bonus = match proficiency_level {
            ProficiencyLevel::None => 0,
            ProficiencyLevel::Half => prof / 2,
            ProficiencyLevel::Proficient => prof,
            ProficiencyLevel::Expert => prof * 2,
        };

        modifier + prof_bonus
    }

    fn saving_throw_modifier(&self, stats: &StatBlock, ability: &str, proficient: bool) -> i32 {
        let stat_value = stats.get_stat(ability).unwrap_or(10);
        let modifier = self.ability_modifier(stat_value);

        if proficient {
            let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;
            modifier + self.proficiency_bonus(level)
        } else {
            modifier
        }
    }

    fn passive_perception(&self, stats: &StatBlock, proficiency_level: ProficiencyLevel) -> i32 {
        10 + self.skill_modifier(stats, "WIS", proficiency_level)
    }

    fn hit_die(&self, class_name: &str) -> u8 {
        match class_name.to_lowercase().as_str() {
            "barbarian" => 12,
            "fighter" | "paladin" | "ranger" => 10,
            "bard" | "cleric" | "druid" | "monk" | "rogue" | "warlock" => 8,
            "sorcerer" | "wizard" => 6,
            _ => 8, // Default to d8
        }
    }

    fn calculate_max_hp(
        &self,
        level: u8,
        class_name: &str,
        constitution_modifier: i32,
        additional_hp: i32,
    ) -> i32 {
        let hit_die = self.hit_die(class_name) as i32;
        // First level: max hit die + CON mod
        // Subsequent levels: average (ceil) + CON mod per level
        let first_level_hp = hit_die + constitution_modifier;
        let avg_roll = (hit_die / 2) + 1; // Average rounded up
        let subsequent_hp = (level as i32 - 1) * (avg_roll + constitution_modifier);

        (first_level_hp + subsequent_hp + additional_hp).max(1)
    }
}

impl SpellcastingSystem for Dnd5eSystem {
    fn caster_type(&self, class: &str) -> Option<CasterType> {
        match class.to_lowercase().as_str() {
            "wizard" | "cleric" | "druid" | "sorcerer" | "bard" => Some(CasterType::Full),
            "paladin" | "ranger" => Some(CasterType::Half),
            "warlock" => Some(CasterType::Pact),
            "eldritch knight" | "arcane trickster" => Some(CasterType::Third),
            _ => None,
        }
    }

    fn spellcasting_stat(&self, class: &str) -> Option<&str> {
        match class.to_lowercase().as_str() {
            "wizard" => Some("INT"),
            "cleric" | "druid" | "ranger" | "monk" => Some("WIS"),
            "sorcerer" | "bard" | "paladin" | "warlock" => Some("CHA"),
            "eldritch knight" | "arcane trickster" => Some("INT"),
            _ => None,
        }
    }

    fn uses_spell_preparation(&self, class: &str) -> bool {
        matches!(
            class.to_lowercase().as_str(),
            "wizard" | "cleric" | "druid" | "paladin"
        )
    }

    fn max_prepared_spells(&self, class: &str, level: u8, stat_mod: i32) -> u8 {
        match class.to_lowercase().as_str() {
            "wizard" | "cleric" | "druid" => (level as i32 + stat_mod).max(1) as u8,
            "paladin" => ((level as i32 / 2) + stat_mod).max(1) as u8,
            _ => 0,
        }
    }

    fn spell_slots(&self, class: &str, level: u8) -> HashMap<u8, u8> {
        let caster_type = self.caster_type(class);
        match caster_type {
            Some(CasterType::Full) => full_caster_slots(level),
            Some(CasterType::Half) => half_caster_slots(level),
            Some(CasterType::Third) => third_caster_slots(level),
            Some(CasterType::Pact) => warlock_slots(level),
            _ => HashMap::new(),
        }
    }

    fn cantrips_known(&self, class: &str, level: u8) -> u8 {
        match class.to_lowercase().as_str() {
            "wizard" => match level {
                1..=3 => 3,
                4..=9 => 4,
                _ => 5,
            },
            "sorcerer" => match level {
                1..=3 => 4,
                4..=9 => 5,
                _ => 6,
            },
            "bard" => match level {
                1..=3 => 2,
                4..=9 => 3,
                _ => 4,
            },
            "cleric" | "druid" => match level {
                1..=3 => 3,
                4..=9 => 4,
                _ => 5,
            },
            "warlock" => match level {
                1..=3 => 2,
                4..=9 => 3,
                _ => 4,
            },
            "eldritch knight" | "arcane trickster" => match level {
                1..=9 => 2,
                _ => 3,
            },
            _ => 0,
        }
    }

    fn spells_known(&self, class: &str, level: u8) -> Option<u8> {
        // Only some classes track spells known
        match class.to_lowercase().as_str() {
            "sorcerer" => Some(SORCERER_SPELLS_KNOWN.get(level as usize).copied().unwrap_or(15)),
            "bard" => Some(BARD_SPELLS_KNOWN.get(level as usize).copied().unwrap_or(22)),
            "ranger" => Some(RANGER_SPELLS_KNOWN.get(level as usize).copied().unwrap_or(11)),
            "warlock" => Some(WARLOCK_SPELLS_KNOWN.get(level as usize).copied().unwrap_or(15)),
            "eldritch knight" => Some(
                ELDRITCH_KNIGHT_SPELLS_KNOWN
                    .get(level as usize)
                    .copied()
                    .unwrap_or(13),
            ),
            "arcane trickster" => Some(
                ARCANE_TRICKSTER_SPELLS_KNOWN
                    .get(level as usize)
                    .copied()
                    .unwrap_or(13),
            ),
            _ => None, // Prepared casters don't have a limit
        }
    }
}

impl CharacterSheetProvider for Dnd5eSystem {
    fn character_sheet_schema(&self) -> CharacterSheetSchema {
        CharacterSheetSchema {
            system_id: "dnd5e".to_string(),
            system_name: "D&D 5th Edition".to_string(),
            sections: vec![
                self.identity_section(),
                self.ability_scores_section(),
                self.combat_section(),
                self.skills_section(),
                self.saving_throws_section(),
                self.features_section(),
                self.modifiers_section(),
            ],
            creation_steps: vec![
                CreationStep {
                    id: "identity".to_string(),
                    label: "Basic Info".to_string(),
                    description: "Choose your character's name, race, class, and background."
                        .to_string(),
                    section_ids: vec!["identity".to_string()],
                    order: 1,
                    required: true,
                },
                CreationStep {
                    id: "abilities".to_string(),
                    label: "Ability Scores".to_string(),
                    description: "Set your ability scores using point buy, standard array, or rolling."
                        .to_string(),
                    section_ids: vec!["ability_scores".to_string()],
                    order: 2,
                    required: true,
                },
                CreationStep {
                    id: "proficiencies".to_string(),
                    label: "Skills & Proficiencies".to_string(),
                    description: "Choose your skill proficiencies and saving throw proficiencies."
                        .to_string(),
                    section_ids: vec!["skills".to_string(), "saving_throws".to_string()],
                    order: 3,
                    required: true,
                },
                CreationStep {
                    id: "equipment".to_string(),
                    label: "Equipment".to_string(),
                    description: "Select starting equipment or roll for gold.".to_string(),
                    section_ids: vec!["combat".to_string()],
                    order: 4,
                    required: false,
                },
            ],
        }
    }

    fn calculate_derived_values(
        &self,
        values: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut derived = HashMap::new();

        // Get level (default to 1)
        let level = values
            .get("LEVEL")
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as u8;

        // Calculate proficiency bonus
        let prof_bonus = self.proficiency_bonus(level);
        derived.insert("PROF_BONUS".to_string(), serde_json::json!(prof_bonus));

        // Calculate XP thresholds
        let xp_next = xp_for_next_level(level);
        derived.insert("XP_NEXT_LEVEL".to_string(), serde_json::json!(xp_next));

        // Also calculate level from XP if XP_CURRENT is provided
        if let Some(xp_current) = values.get("XP_CURRENT").and_then(|v| v.as_i64()) {
            let calculated_level = level_from_xp(xp_current as i32);
            derived.insert("LEVEL_FROM_XP".to_string(), serde_json::json!(calculated_level));
        }

        // Calculate ability modifiers
        for ability in &["STR", "DEX", "CON", "INT", "WIS", "CHA"] {
            if let Some(score) = values.get(*ability).and_then(|v| v.as_i64()) {
                let modifier = self.ability_modifier(score as i32);
                derived.insert(format!("{}_MOD", ability), serde_json::json!(modifier));
            }
        }

        // Calculate skill modifiers
        for skill in self.skill_names() {
            if let Some(ability) = skill_ability(skill) {
                let ability_mod = derived
                    .get(&format!("{}_MOD", ability))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;

                let proficiency = values
                    .get(&format!("{}_PROF", skill.to_uppercase().replace(' ', "_")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("none");

                let prof_mult = match proficiency {
                    "expert" => 2.0,
                    "proficient" => 1.0,
                    "half" => 0.5,
                    _ => 0.0,
                };

                let skill_mod = ability_mod + (prof_bonus as f64 * prof_mult) as i32;
                derived.insert(
                    format!("{}_MOD", skill.to_uppercase().replace(' ', "_")),
                    serde_json::json!(skill_mod),
                );
            }
        }

        // Calculate saving throw modifiers
        for ability in &["STR", "DEX", "CON", "INT", "WIS", "CHA"] {
            let ability_mod = derived
                .get(&format!("{}_MOD", ability))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let proficient = values
                .get(&format!("{}_SAVE_PROF", ability))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let save_mod = if proficient {
                ability_mod + prof_bonus
            } else {
                ability_mod
            };
            derived.insert(format!("{}_SAVE", ability), serde_json::json!(save_mod));
        }

        // Calculate passive perception
        let wis_mod = derived
            .get("WIS_MOD")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let perception_prof = values
            .get("PERCEPTION_PROF")
            .and_then(|v| v.as_str())
            .unwrap_or("none");
        let perception_bonus = match perception_prof {
            "expert" => prof_bonus * 2,
            "proficient" => prof_bonus,
            "half" => prof_bonus / 2,
            _ => 0,
        };
        let passive_perception = 10 + wis_mod + perception_bonus;
        derived.insert(
            "PASSIVE_PERCEPTION".to_string(),
            serde_json::json!(passive_perception),
        );

        // Calculate initiative
        let dex_mod = derived
            .get("DEX_MOD")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        derived.insert("INITIATIVE".to_string(), serde_json::json!(dex_mod));

        // Calculate max HP if class and CON are set
        if let (Some(class), Some(con_mod)) = (
            values.get("CLASS").and_then(|v| v.as_str()),
            derived.get("CON_MOD").and_then(|v| v.as_i64()),
        ) {
            let max_hp = self.calculate_max_hp(level, class, con_mod as i32, 0);
            derived.insert("MAX_HP".to_string(), serde_json::json!(max_hp));
        }

        derived
    }

    fn validate_field(
        &self,
        field_id: &str,
        value: &serde_json::Value,
        _all_values: &HashMap<String, serde_json::Value>,
    ) -> Option<String> {
        match field_id {
            "STR" | "DEX" | "CON" | "INT" | "WIS" | "CHA" => {
                if let Some(score) = value.as_i64() {
                    if score < 1 || score > 30 {
                        return Some("Ability scores must be between 1 and 30".to_string());
                    }
                } else {
                    return Some("Ability score must be a number".to_string());
                }
            }
            "LEVEL" => {
                if let Some(level) = value.as_i64() {
                    if level < 1 || level > 20 {
                        return Some("Level must be between 1 and 20".to_string());
                    }
                } else {
                    return Some("Level must be a number".to_string());
                }
            }
            "NAME" => {
                if let Some(name) = value.as_str() {
                    if name.is_empty() {
                        return Some("Name is required".to_string());
                    }
                } else {
                    return Some("Name must be a string".to_string());
                }
            }
            _ => {}
        }
        None
    }

    fn default_values(&self) -> HashMap<String, serde_json::Value> {
        let mut defaults = HashMap::new();
        defaults.insert("LEVEL".to_string(), serde_json::json!(1));
        defaults.insert("XP_CURRENT".to_string(), serde_json::json!(0));
        defaults.insert("STR".to_string(), serde_json::json!(10));
        defaults.insert("DEX".to_string(), serde_json::json!(10));
        defaults.insert("CON".to_string(), serde_json::json!(10));
        defaults.insert("INT".to_string(), serde_json::json!(10));
        defaults.insert("WIS".to_string(), serde_json::json!(10));
        defaults.insert("CHA".to_string(), serde_json::json!(10));
        defaults.insert("CURRENT_HP".to_string(), serde_json::json!(0));
        defaults
    }
}

// Helper methods for building the schema
impl Dnd5eSystem {
    fn identity_section(&self) -> SchemaSection {
        SchemaSection {
            id: "identity".to_string(),
            label: "Character Identity".to_string(),
            section_type: SectionType::Identity,
            fields: vec![
                FieldDefinition {
                    id: "NAME".to_string(),
                    label: "Character Name".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(100),
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(6),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Enter character name".to_string()),
                },
                FieldDefinition {
                    id: "LEVEL".to_string(),
                    label: "Level".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(1),
                        max: Some(20),
                        show_modifier: false,
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(1),
                        max: Some(20),
                        pattern: None,
                        error_message: Some("Level must be 1-20".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: None,
                },
                FieldDefinition {
                    id: "PROF_BONUS".to_string(),
                    label: "Proficiency Bonus".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(2),
                        max: Some(6),
                        show_modifier: true,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::ProficiencyBonus,
                        dependencies: vec!["LEVEL".to_string()],
                        display_format: Some("+{}".to_string()),
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Based on character level".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "XP_CURRENT".to_string(),
                    label: "Experience Points".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: None,
                        show_modifier: false,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(0),
                        max: None,
                        pattern: None,
                        error_message: Some("XP cannot be negative".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(3),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Current experience points".to_string()),
                    placeholder: Some("0".to_string()),
                },
                FieldDefinition {
                    id: "XP_NEXT_LEVEL".to_string(),
                    label: "XP for Next Level".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: None,
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["LEVEL".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(3),
                        ..Default::default()
                    },
                    description: Some("XP needed to reach next level".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "CLASS".to_string(),
                    label: "Class".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "barbarian".to_string(),
                                label: "Barbarian".to_string(),
                                description: Some("A fierce warrior of primitive background".to_string()),
                            },
                            SchemaSelectOption {
                                value: "bard".to_string(),
                                label: "Bard".to_string(),
                                description: Some("An inspiring magician".to_string()),
                            },
                            SchemaSelectOption {
                                value: "cleric".to_string(),
                                label: "Cleric".to_string(),
                                description: Some("A priestly champion".to_string()),
                            },
                            SchemaSelectOption {
                                value: "druid".to_string(),
                                label: "Druid".to_string(),
                                description: Some("A priest of the Old Faith".to_string()),
                            },
                            SchemaSelectOption {
                                value: "fighter".to_string(),
                                label: "Fighter".to_string(),
                                description: Some("A master of martial combat".to_string()),
                            },
                            SchemaSelectOption {
                                value: "monk".to_string(),
                                label: "Monk".to_string(),
                                description: Some("A master of martial arts".to_string()),
                            },
                            SchemaSelectOption {
                                value: "paladin".to_string(),
                                label: "Paladin".to_string(),
                                description: Some("A holy warrior".to_string()),
                            },
                            SchemaSelectOption {
                                value: "ranger".to_string(),
                                label: "Ranger".to_string(),
                                description: Some("A warrior of the wilderness".to_string()),
                            },
                            SchemaSelectOption {
                                value: "rogue".to_string(),
                                label: "Rogue".to_string(),
                                description: Some("A scoundrel with stealth".to_string()),
                            },
                            SchemaSelectOption {
                                value: "sorcerer".to_string(),
                                label: "Sorcerer".to_string(),
                                description: Some("A spellcaster with innate magic".to_string()),
                            },
                            SchemaSelectOption {
                                value: "warlock".to_string(),
                                label: "Warlock".to_string(),
                                description: Some("A wielder of pact magic".to_string()),
                            },
                            SchemaSelectOption {
                                value: "wizard".to_string(),
                                label: "Wizard".to_string(),
                                description: Some("A scholarly magic-user".to_string()),
                            },
                        ],
                        allow_custom: false,
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(4),
                        new_row: true,
                        ..Default::default()
                    },
                    description: None,
                    placeholder: None,
                },
                FieldDefinition {
                    id: "RACE".to_string(),
                    label: "Race".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "human".to_string(),
                                label: "Human".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "elf".to_string(),
                                label: "Elf".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "dwarf".to_string(),
                                label: "Dwarf".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "halfling".to_string(),
                                label: "Halfling".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "dragonborn".to_string(),
                                label: "Dragonborn".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "gnome".to_string(),
                                label: "Gnome".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "half-elf".to_string(),
                                label: "Half-Elf".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "half-orc".to_string(),
                                label: "Half-Orc".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "tiefling".to_string(),
                                label: "Tiefling".to_string(),
                                description: None,
                            },
                        ],
                        allow_custom: true,
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: None,
                },
                FieldDefinition {
                    id: "BACKGROUND".to_string(),
                    label: "Background".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "acolyte".to_string(),
                                label: "Acolyte".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "charlatan".to_string(),
                                label: "Charlatan".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "criminal".to_string(),
                                label: "Criminal".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "entertainer".to_string(),
                                label: "Entertainer".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "folk_hero".to_string(),
                                label: "Folk Hero".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "guild_artisan".to_string(),
                                label: "Guild Artisan".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "hermit".to_string(),
                                label: "Hermit".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "noble".to_string(),
                                label: "Noble".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "outlander".to_string(),
                                label: "Outlander".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "sage".to_string(),
                                label: "Sage".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "sailor".to_string(),
                                label: "Sailor".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "soldier".to_string(),
                                label: "Soldier".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "urchin".to_string(),
                                label: "Urchin".to_string(),
                                description: None,
                            },
                        ],
                        allow_custom: true,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: None,
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: None,
        }
    }

    fn ability_scores_section(&self) -> SchemaSection {
        let abilities = [
            ("STR", "Strength", "Physical power, athletics, melee attacks"),
            ("DEX", "Dexterity", "Agility, reflexes, ranged attacks"),
            ("CON", "Constitution", "Endurance, health, stamina"),
            ("INT", "Intelligence", "Reasoning, memory, knowledge"),
            ("WIS", "Wisdom", "Perception, intuition, insight"),
            ("CHA", "Charisma", "Force of personality, leadership"),
        ];

        let mut fields: Vec<FieldDefinition> = Vec::new();

        for (id, label, description) in &abilities {
            // Score field
            fields.push(FieldDefinition {
                id: id.to_string(),
                label: label.to_string(),
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
                    error_message: Some("Ability scores must be 1-30".to_string()),
                }),
                layout: FieldLayout {
                    width: Some(2),
                    ..Default::default()
                },
                description: Some(description.to_string()),
                placeholder: None,
            });
        }

        SchemaSection {
            id: "ability_scores".to_string(),
            label: "Ability Scores".to_string(),
            section_type: SectionType::AbilityScores,
            fields,
            collapsible: false,
            collapsed_default: false,
            description: Some(
                "Your character's six core abilities. Each has a score and derived modifier."
                    .to_string(),
            ),
        }
    }

    fn combat_section(&self) -> SchemaSection {
        SchemaSection {
            id: "combat".to_string(),
            label: "Combat".to_string(),
            section_type: SectionType::Combat,
            fields: vec![
                FieldDefinition {
                    id: "CURRENT_HP".to_string(),
                    label: "Current HP".to_string(),
                    field_type: SchemaFieldType::ResourceBar {
                        max_field: "MAX_HP".to_string(),
                        color: ResourceColor::Red,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: None,
                },
                FieldDefinition {
                    id: "MAX_HP".to_string(),
                    label: "Max HP".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(1),
                        max: None,
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["LEVEL".to_string(), "CLASS".to_string(), "CON".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Calculated from class and Constitution".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "TEMP_HP".to_string(),
                    label: "Temp HP".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: None,
                        show_modifier: false,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: None,
                },
                FieldDefinition {
                    id: "AC".to_string(),
                    label: "Armor Class".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(1),
                        max: None,
                        show_modifier: false,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Depends on armor and Dexterity".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "INITIATIVE".to_string(),
                    label: "Initiative".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: None,
                        max: None,
                        show_modifier: true,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::AbilityModifier,
                        dependencies: vec!["DEX".to_string()],
                        display_format: Some("+{}".to_string()),
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Based on Dexterity modifier".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "SPEED".to_string(),
                    label: "Speed".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: None,
                        show_modifier: false,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Movement speed in feet".to_string()),
                    placeholder: Some("30".to_string()),
                },
                FieldDefinition {
                    id: "PASSIVE_PERCEPTION".to_string(),
                    label: "Passive Perception".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: None,
                        max: None,
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["WIS".to_string(), "PERCEPTION_PROF".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("10 + Perception modifier".to_string()),
                    placeholder: None,
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: None,
        }
    }

    fn skills_section(&self) -> SchemaSection {
        let skill_abilities: Vec<(&str, &str)> = vec![
            ("Acrobatics", "DEX"),
            ("Animal Handling", "WIS"),
            ("Arcana", "INT"),
            ("Athletics", "STR"),
            ("Deception", "CHA"),
            ("History", "INT"),
            ("Insight", "WIS"),
            ("Intimidation", "CHA"),
            ("Investigation", "INT"),
            ("Medicine", "WIS"),
            ("Nature", "INT"),
            ("Perception", "WIS"),
            ("Performance", "CHA"),
            ("Persuasion", "CHA"),
            ("Religion", "INT"),
            ("Sleight of Hand", "DEX"),
            ("Stealth", "DEX"),
            ("Survival", "WIS"),
        ];

        let proficiency_options = vec![
            ProficiencyOption {
                value: "none".to_string(),
                label: "Not Proficient".to_string(),
                multiplier: 0.0,
            },
            ProficiencyOption {
                value: "half".to_string(),
                label: "Half (Jack of All Trades)".to_string(),
                multiplier: 0.5,
            },
            ProficiencyOption {
                value: "proficient".to_string(),
                label: "Proficient".to_string(),
                multiplier: 1.0,
            },
            ProficiencyOption {
                value: "expert".to_string(),
                label: "Expertise".to_string(),
                multiplier: 2.0,
            },
        ];

        let fields: Vec<FieldDefinition> = skill_abilities
            .iter()
            .map(|(skill, ability)| {
                let skill_id = skill.to_uppercase().replace(' ', "_");
                FieldDefinition {
                    id: format!("{}_PROF", skill_id),
                    label: skill.to_string(),
                    field_type: SchemaFieldType::Skill {
                        ability: ability.to_string(),
                        proficiency_levels: proficiency_options.clone(),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(6),
                        ..Default::default()
                    },
                    description: Some(format!("Based on {}", ability)),
                    placeholder: None,
                }
            })
            .collect();

        SchemaSection {
            id: "skills".to_string(),
            label: "Skills".to_string(),
            section_type: SectionType::Skills,
            fields,
            collapsible: true,
            collapsed_default: false,
            description: Some("Choose your skill proficiencies".to_string()),
        }
    }

    fn saving_throws_section(&self) -> SchemaSection {
        let abilities = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];
        let ability_names = [
            "Strength",
            "Dexterity",
            "Constitution",
            "Intelligence",
            "Wisdom",
            "Charisma",
        ];

        let fields: Vec<FieldDefinition> = abilities
            .iter()
            .zip(ability_names.iter())
            .map(|(id, name)| FieldDefinition {
                id: format!("{}_SAVE_PROF", id),
                label: format!("{} Save", name),
                field_type: SchemaFieldType::SavingThrow {
                    ability: id.to_string(),
                },
                editable: true,
                required: false,
                derived_from: None,
                validation: None,
                layout: FieldLayout {
                    width: Some(4),
                    ..Default::default()
                },
                description: None,
                placeholder: None,
            })
            .collect();

        SchemaSection {
            id: "saving_throws".to_string(),
            label: "Saving Throws".to_string(),
            section_type: SectionType::Combat,
            fields,
            collapsible: true,
            collapsed_default: true,
            description: Some("Mark proficient saves from your class".to_string()),
        }
    }

    fn features_section(&self) -> SchemaSection {
        SchemaSection {
            id: "features".to_string(),
            label: "Features & Traits".to_string(),
            section_type: SectionType::Features,
            fields: vec![
                FieldDefinition {
                    id: "FEATURES".to_string(),
                    label: "Features & Traits".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: true,
                        max_length: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        ..Default::default()
                    },
                    description: Some("Class features, racial traits, feats, etc.".to_string()),
                    placeholder: Some("Enter your features and traits...".to_string()),
                },
            ],
            collapsible: true,
            collapsed_default: true,
            description: None,
        }
    }

    fn modifiers_section(&self) -> SchemaSection {
        SchemaSection {
            id: "modifiers".to_string(),
            label: "Active Effects".to_string(),
            section_type: SectionType::Modifiers,
            fields: vec![
                FieldDefinition {
                    id: "ACTIVE_MODIFIERS".to_string(),
                    label: "Conditions & Effects".to_string(),
                    field_type: SchemaFieldType::ModifierList { filter_stat: None },
                    editable: false,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        ..Default::default()
                    },
                    description: Some(
                        "Active conditions, spells, and effects modifying your stats".to_string(),
                    ),
                    placeholder: None,
                },
            ],
            collapsible: true,
            collapsed_default: false,
            description: Some("View and manage active conditions and effects".to_string()),
        }
    }
}

// Spell slot progression tables

fn full_caster_slots(level: u8) -> HashMap<u8, u8> {
    let slots: &[(u8, &[u8])] = &[
        (1, &[2]),
        (2, &[3]),
        (3, &[4, 2]),
        (4, &[4, 3]),
        (5, &[4, 3, 2]),
        (6, &[4, 3, 3]),
        (7, &[4, 3, 3, 1]),
        (8, &[4, 3, 3, 2]),
        (9, &[4, 3, 3, 3, 1]),
        (10, &[4, 3, 3, 3, 2]),
        (11, &[4, 3, 3, 3, 2, 1]),
        (12, &[4, 3, 3, 3, 2, 1]),
        (13, &[4, 3, 3, 3, 2, 1, 1]),
        (14, &[4, 3, 3, 3, 2, 1, 1]),
        (15, &[4, 3, 3, 3, 2, 1, 1, 1]),
        (16, &[4, 3, 3, 3, 2, 1, 1, 1]),
        (17, &[4, 3, 3, 3, 2, 1, 1, 1, 1]),
        (18, &[4, 3, 3, 3, 3, 1, 1, 1, 1]),
        (19, &[4, 3, 3, 3, 3, 2, 1, 1, 1]),
        (20, &[4, 3, 3, 3, 3, 2, 2, 1, 1]),
    ];

    slots
        .iter()
        .find(|(l, _)| *l == level)
        .map(|(_, s)| {
            s.iter()
                .enumerate()
                .map(|(i, &count)| ((i + 1) as u8, count))
                .collect()
        })
        .unwrap_or_default()
}

fn half_caster_slots(level: u8) -> HashMap<u8, u8> {
    // Half casters get slots at half rate (starting at level 2)
    let slots: &[(u8, &[u8])] = &[
        (2, &[2]),
        (3, &[3]),
        (4, &[3]),
        (5, &[4, 2]),
        (6, &[4, 2]),
        (7, &[4, 3]),
        (8, &[4, 3]),
        (9, &[4, 3, 2]),
        (10, &[4, 3, 2]),
        (11, &[4, 3, 3]),
        (12, &[4, 3, 3]),
        (13, &[4, 3, 3, 1]),
        (14, &[4, 3, 3, 1]),
        (15, &[4, 3, 3, 2]),
        (16, &[4, 3, 3, 2]),
        (17, &[4, 3, 3, 3, 1]),
        (18, &[4, 3, 3, 3, 1]),
        (19, &[4, 3, 3, 3, 2]),
        (20, &[4, 3, 3, 3, 2]),
    ];

    slots
        .iter()
        .find(|(l, _)| *l == level)
        .map(|(_, s)| {
            s.iter()
                .enumerate()
                .map(|(i, &count)| ((i + 1) as u8, count))
                .collect()
        })
        .unwrap_or_default()
}

fn third_caster_slots(level: u8) -> HashMap<u8, u8> {
    // Third casters (Eldritch Knight, Arcane Trickster)
    let slots: &[(u8, &[u8])] = &[
        (3, &[2]),
        (4, &[3]),
        (5, &[3]),
        (6, &[3]),
        (7, &[4, 2]),
        (8, &[4, 2]),
        (9, &[4, 2]),
        (10, &[4, 3]),
        (11, &[4, 3]),
        (12, &[4, 3]),
        (13, &[4, 3, 2]),
        (14, &[4, 3, 2]),
        (15, &[4, 3, 2]),
        (16, &[4, 3, 3]),
        (17, &[4, 3, 3]),
        (18, &[4, 3, 3]),
        (19, &[4, 3, 3, 1]),
        (20, &[4, 3, 3, 1]),
    ];

    slots
        .iter()
        .find(|(l, _)| *l == level)
        .map(|(_, s)| {
            s.iter()
                .enumerate()
                .map(|(i, &count)| ((i + 1) as u8, count))
                .collect()
        })
        .unwrap_or_default()
}

fn warlock_slots(level: u8) -> HashMap<u8, u8> {
    // Warlock pact magic - fewer slots but higher level
    let (count, slot_level) = match level {
        1 => (1, 1),
        2 => (2, 1),
        3..=4 => (2, 2),
        5..=6 => (2, 3),
        7..=8 => (2, 4),
        9..=10 => (2, 5),
        11..=16 => (3, 5),
        17..=20 => (4, 5),
        _ => (0, 0),
    };

    if count > 0 {
        let mut slots = HashMap::new();
        slots.insert(slot_level, count);
        slots
    } else {
        HashMap::new()
    }
}

// Spells known tables (0-indexed, level 1 = index 1)
const SORCERER_SPELLS_KNOWN: &[u8] = &[
    0, // level 0 (unused)
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 12, 13, 13, 14, 14, 15, 15, 15, 15,
];

const BARD_SPELLS_KNOWN: &[u8] = &[
    0, // level 0
    4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 15, 16, 18, 19, 19, 20, 22, 22, 22,
];

const RANGER_SPELLS_KNOWN: &[u8] = &[
    0, // level 0
    0, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11,
];

const WARLOCK_SPELLS_KNOWN: &[u8] = &[
    0, // level 0
    2, 3, 4, 5, 6, 7, 8, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14, 15, 15,
];

const ELDRITCH_KNIGHT_SPELLS_KNOWN: &[u8] = &[
    0, // level 0
    0, 0, 3, 4, 4, 4, 5, 6, 6, 7, 8, 8, 9, 10, 10, 11, 11, 11, 12, 13,
];

const ARCANE_TRICKSTER_SPELLS_KNOWN: &[u8] = &[
    0, // level 0
    0, 0, 3, 4, 4, 4, 5, 6, 6, 7, 8, 8, 9, 10, 10, 11, 11, 11, 12, 13,
];

/// Get the skill's associated ability for D&D 5e.
pub fn skill_ability(skill: &str) -> Option<&'static str> {
    match skill.to_lowercase().as_str() {
        "athletics" => Some("STR"),
        "acrobatics" | "sleight of hand" | "stealth" => Some("DEX"),
        "arcana" | "history" | "investigation" | "nature" | "religion" => Some("INT"),
        "animal handling" | "insight" | "medicine" | "perception" | "survival" => Some("WIS"),
        "deception" | "intimidation" | "performance" | "persuasion" => Some("CHA"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_stats() -> StatBlock {
        let mut stats = StatBlock::default();
        stats.set_stat("STR", 16);
        stats.set_stat("DEX", 14);
        stats.set_stat("CON", 14);
        stats.set_stat("INT", 10);
        stats.set_stat("WIS", 12);
        stats.set_stat("CHA", 8);
        stats.set_stat("LEVEL", 5);
        stats
    }

    #[test]
    fn ability_modifier_calculation() {
        let system = Dnd5eSystem::new();
        assert_eq!(system.ability_modifier(1), -5);
        assert_eq!(system.ability_modifier(8), -1);
        assert_eq!(system.ability_modifier(10), 0);
        assert_eq!(system.ability_modifier(11), 0);
        assert_eq!(system.ability_modifier(12), 1);
        assert_eq!(system.ability_modifier(14), 2);
        assert_eq!(system.ability_modifier(16), 3);
        assert_eq!(system.ability_modifier(18), 4);
        assert_eq!(system.ability_modifier(20), 5);
    }

    #[test]
    fn proficiency_bonus_progression() {
        let system = Dnd5eSystem::new();
        assert_eq!(system.proficiency_bonus(1), 2);
        assert_eq!(system.proficiency_bonus(4), 2);
        assert_eq!(system.proficiency_bonus(5), 3);
        assert_eq!(system.proficiency_bonus(8), 3);
        assert_eq!(system.proficiency_bonus(9), 4);
        assert_eq!(system.proficiency_bonus(12), 4);
        assert_eq!(system.proficiency_bonus(13), 5);
        assert_eq!(system.proficiency_bonus(16), 5);
        assert_eq!(system.proficiency_bonus(17), 6);
        assert_eq!(system.proficiency_bonus(20), 6);
    }

    #[test]
    fn spell_save_dc_calculation() {
        let system = Dnd5eSystem::new();
        let stats = create_test_stats(); // Level 5, WIS 12

        // DC = 8 + proficiency (3) + WIS mod (1) = 12
        assert_eq!(system.spell_save_dc(&stats, "WIS"), 12);

        // DC with INT = 8 + 3 + 0 = 11
        assert_eq!(system.spell_save_dc(&stats, "INT"), 11);
    }

    #[test]
    fn spell_attack_bonus_calculation() {
        let system = Dnd5eSystem::new();
        let stats = create_test_stats();

        // Attack = proficiency (3) + WIS mod (1) = 4
        assert_eq!(system.spell_attack_bonus(&stats, "WIS"), 4);
    }

    #[test]
    fn ac_calculation_unarmored() {
        let system = Dnd5eSystem::new();
        let stats = create_test_stats(); // DEX 14

        // Unarmored: 10 + DEX mod (2) = 12
        assert_eq!(system.calculate_ac(&stats, None, None, true, None), 12);

        // With shield (+2): 12 + 2 = 14
        assert_eq!(system.calculate_ac(&stats, None, Some(2), true, None), 14);
    }

    #[test]
    fn ac_calculation_armored() {
        let system = Dnd5eSystem::new();
        let stats = create_test_stats(); // DEX 14

        // Chain mail (AC 16, no DEX): 16
        assert_eq!(system.calculate_ac(&stats, Some(16), None, false, None), 16);

        // Half plate (AC 15, max DEX +2): 15 + 2 = 17
        assert_eq!(
            system.calculate_ac(&stats, Some(15), None, true, Some(2)),
            17
        );

        // Leather (AC 11 + DEX): 11 + 2 = 13
        assert_eq!(system.calculate_ac(&stats, Some(11), None, true, None), 13);
    }

    #[test]
    fn skill_modifier_with_proficiency() {
        let system = Dnd5eSystem::new();
        let stats = create_test_stats(); // Level 5, DEX 14

        // No proficiency: just DEX mod (2)
        assert_eq!(
            system.skill_modifier(&stats, "DEX", ProficiencyLevel::None),
            2
        );

        // Proficient: DEX mod (2) + proficiency (3) = 5
        assert_eq!(
            system.skill_modifier(&stats, "DEX", ProficiencyLevel::Proficient),
            5
        );

        // Expertise: DEX mod (2) + double proficiency (6) = 8
        assert_eq!(
            system.skill_modifier(&stats, "DEX", ProficiencyLevel::Expert),
            8
        );

        // Jack of All Trades: DEX mod (2) + half proficiency (1) = 3
        assert_eq!(
            system.skill_modifier(&stats, "DEX", ProficiencyLevel::Half),
            3
        );
    }

    #[test]
    fn passive_perception() {
        let system = Dnd5eSystem::new();
        let stats = create_test_stats(); // WIS 12

        // 10 + WIS mod (1) = 11
        assert_eq!(
            system.passive_perception(&stats, ProficiencyLevel::None),
            11
        );

        // 10 + WIS mod (1) + proficiency (3) = 14
        assert_eq!(
            system.passive_perception(&stats, ProficiencyLevel::Proficient),
            14
        );
    }

    #[test]
    fn hit_die_by_class() {
        let system = Dnd5eSystem::new();
        assert_eq!(system.hit_die("barbarian"), 12);
        assert_eq!(system.hit_die("fighter"), 10);
        assert_eq!(system.hit_die("cleric"), 8);
        assert_eq!(system.hit_die("wizard"), 6);
    }

    #[test]
    fn max_hp_calculation() {
        let system = Dnd5eSystem::new();

        // Level 1 Fighter, +2 CON: 10 + 2 = 12
        assert_eq!(system.calculate_max_hp(1, "fighter", 2, 0), 12);

        // Level 5 Fighter, +2 CON:
        // Level 1: 10 + 2 = 12
        // Levels 2-5: 4 levels * (6 + 2) = 32
        // Total: 44
        assert_eq!(system.calculate_max_hp(5, "fighter", 2, 0), 44);

        // With Tough feat (+10 at level 5)
        assert_eq!(system.calculate_max_hp(5, "fighter", 2, 10), 54);
    }

    #[test]
    fn spellcasting_stat_by_class() {
        let system = Dnd5eSystem::new();
        assert_eq!(system.spellcasting_stat("wizard"), Some("INT"));
        assert_eq!(system.spellcasting_stat("cleric"), Some("WIS"));
        assert_eq!(system.spellcasting_stat("sorcerer"), Some("CHA"));
        assert_eq!(system.spellcasting_stat("paladin"), Some("CHA"));
        assert_eq!(system.spellcasting_stat("fighter"), None);
    }

    #[test]
    fn full_caster_spell_slots() {
        let system = Dnd5eSystem::new();

        let level1 = system.spell_slots("wizard", 1);
        assert_eq!(level1.get(&1), Some(&2));
        assert_eq!(level1.get(&2), None);

        let level5 = system.spell_slots("wizard", 5);
        assert_eq!(level5.get(&1), Some(&4));
        assert_eq!(level5.get(&2), Some(&3));
        assert_eq!(level5.get(&3), Some(&2));

        let level20 = system.spell_slots("wizard", 20);
        assert_eq!(level20.get(&9), Some(&1));
    }

    #[test]
    fn half_caster_spell_slots() {
        let system = Dnd5eSystem::new();

        // Paladins don't get slots until level 2
        let level1 = system.spell_slots("paladin", 1);
        assert!(level1.is_empty());

        let level2 = system.spell_slots("paladin", 2);
        assert_eq!(level2.get(&1), Some(&2));

        let level5 = system.spell_slots("paladin", 5);
        assert_eq!(level5.get(&1), Some(&4));
        assert_eq!(level5.get(&2), Some(&2));
    }

    #[test]
    fn warlock_pact_slots() {
        let system = Dnd5eSystem::new();

        let level1 = system.spell_slots("warlock", 1);
        assert_eq!(level1.get(&1), Some(&1));

        let level5 = system.spell_slots("warlock", 5);
        assert_eq!(level5.get(&3), Some(&2)); // 2 third-level slots

        let level11 = system.spell_slots("warlock", 11);
        assert_eq!(level11.get(&5), Some(&3)); // 3 fifth-level slots
    }

    #[test]
    fn cantrips_known() {
        let system = Dnd5eSystem::new();
        assert_eq!(system.cantrips_known("wizard", 1), 3);
        assert_eq!(system.cantrips_known("wizard", 4), 4);
        assert_eq!(system.cantrips_known("wizard", 10), 5);
    }

    #[test]
    fn spells_known_by_class() {
        let system = Dnd5eSystem::new();
        assert_eq!(system.spells_known("sorcerer", 1), Some(2));
        assert_eq!(system.spells_known("sorcerer", 5), Some(6));
        assert_eq!(system.spells_known("wizard", 1), None); // Prepared caster
    }

    #[test]
    fn max_prepared_spells() {
        let system = Dnd5eSystem::new();
        // Wizard level 5, INT 16 (+3): 5 + 3 = 8
        assert_eq!(system.max_prepared_spells("wizard", 5, 3), 8);

        // Paladin level 6, CHA 14 (+2): 3 + 2 = 5
        assert_eq!(system.max_prepared_spells("paladin", 6, 2), 5);

        // Minimum is 1
        assert_eq!(system.max_prepared_spells("wizard", 1, -3), 1);
    }

    #[test]
    fn skill_ability_mapping() {
        assert_eq!(skill_ability("Athletics"), Some("STR"));
        assert_eq!(skill_ability("Stealth"), Some("DEX"));
        assert_eq!(skill_ability("Arcana"), Some("INT"));
        assert_eq!(skill_ability("Perception"), Some("WIS"));
        assert_eq!(skill_ability("Persuasion"), Some("CHA"));
    }

    #[test]
    fn caster_type_identification() {
        let system = Dnd5eSystem::new();
        assert_eq!(system.caster_type("wizard"), Some(CasterType::Full));
        assert_eq!(system.caster_type("paladin"), Some(CasterType::Half));
        assert_eq!(system.caster_type("warlock"), Some(CasterType::Pact));
        assert_eq!(
            system.caster_type("eldritch knight"),
            Some(CasterType::Third)
        );
        assert_eq!(system.caster_type("fighter"), None);
    }

    #[test]
    fn xp_thresholds() {
        // Level 1 starts at 0 XP
        assert_eq!(xp_for_level(1), 0);
        // Level 2 requires 300 XP
        assert_eq!(xp_for_level(2), 300);
        // Level 5 requires 6500 XP
        assert_eq!(xp_for_level(5), 6500);
        // Level 20 requires 355000 XP
        assert_eq!(xp_for_level(20), 355000);
    }

    #[test]
    fn xp_for_next_level_calculation() {
        // At level 1, next level requires 300 XP
        assert_eq!(xp_for_next_level(1), 300);
        // At level 5, next level requires 14000 XP
        assert_eq!(xp_for_next_level(5), 14000);
        // At level 20, returns max (355000)
        assert_eq!(xp_for_next_level(20), 355000);
    }

    #[test]
    fn level_from_xp_calculation() {
        assert_eq!(level_from_xp(0), 1);
        assert_eq!(level_from_xp(299), 1);
        assert_eq!(level_from_xp(300), 2);
        assert_eq!(level_from_xp(6499), 4);
        assert_eq!(level_from_xp(6500), 5);
        assert_eq!(level_from_xp(355000), 20);
        assert_eq!(level_from_xp(500000), 20);
    }

    #[test]
    fn derived_values_include_xp_next_level() {
        let system = Dnd5eSystem::new();
        let mut values = HashMap::new();
        values.insert("LEVEL".to_string(), serde_json::json!(5));
        values.insert("XP_CURRENT".to_string(), serde_json::json!(8000));

        let derived = system.calculate_derived_values(&values);

        // XP for next level (6) should be 14000
        assert_eq!(derived.get("XP_NEXT_LEVEL").unwrap().as_i64().unwrap(), 14000);
        // Level from XP (8000) should be 5
        assert_eq!(derived.get("LEVEL_FROM_XP").unwrap().as_i64().unwrap(), 5);
    }
}
