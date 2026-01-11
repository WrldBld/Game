//! FATE Core game system implementation.
//!
//! FATE uses 4dF (Fudge dice) + skill vs difficulty.
//! Key features:
//! - Ladder-based results (-2 to +8)
//! - Aspects as central mechanic
//! - Fate Points for narrative control
//! - Stress and Consequences instead of HP
//! - Four actions: Overcome, Create Advantage, Attack, Defend

use super::traits::{
    CalculationEngine, CharacterSheetProvider, CharacterSheetSchema, CreationStep, DerivedField,
    DerivationType, FieldDefinition, FieldLayout, FieldValidation, GameSystem, LadderLabel,
    ProficiencyLevel, ResourceColor, SchemaFieldType, SchemaSection, SectionType,
};
use crate::entities::{StatBlock, StatModifier};
use std::collections::HashMap;

/// FATE ladder value to descriptor mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LadderRating {
    Terrible = -2,
    Poor = -1,
    Mediocre = 0,
    Average = 1,
    Fair = 2,
    Good = 3,
    Great = 4,
    Superb = 5,
    Fantastic = 6,
    Epic = 7,
    Legendary = 8,
}

impl LadderRating {
    pub fn from_value(value: i32) -> Option<Self> {
        match value {
            -2 => Some(LadderRating::Terrible),
            -1 => Some(LadderRating::Poor),
            0 => Some(LadderRating::Mediocre),
            1 => Some(LadderRating::Average),
            2 => Some(LadderRating::Fair),
            3 => Some(LadderRating::Good),
            4 => Some(LadderRating::Great),
            5 => Some(LadderRating::Superb),
            6 => Some(LadderRating::Fantastic),
            7 => Some(LadderRating::Epic),
            8 => Some(LadderRating::Legendary),
            _ => None,
        }
    }

    pub fn descriptor(&self) -> &'static str {
        match self {
            LadderRating::Terrible => "Terrible",
            LadderRating::Poor => "Poor",
            LadderRating::Mediocre => "Mediocre",
            LadderRating::Average => "Average",
            LadderRating::Fair => "Fair",
            LadderRating::Good => "Good",
            LadderRating::Great => "Great",
            LadderRating::Superb => "Superb",
            LadderRating::Fantastic => "Fantastic",
            LadderRating::Epic => "Epic",
            LadderRating::Legendary => "Legendary",
        }
    }
}

/// FATE roll outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FateOutcome {
    /// Shifts >= 3
    SuccessWithStyle { shifts: i32 },
    /// Shifts 1-2
    Success { shifts: i32 },
    /// Shifts = 0
    Tie,
    /// Shifts < 0
    Failure { shifts: i32 },
}

impl FateOutcome {
    /// Determine outcome from roll total vs difficulty.
    pub fn determine(total: i32, difficulty: i32) -> Self {
        let shifts = total - difficulty;
        if shifts >= 3 {
            FateOutcome::SuccessWithStyle { shifts }
        } else if shifts >= 1 {
            FateOutcome::Success { shifts }
        } else if shifts == 0 {
            FateOutcome::Tie
        } else {
            FateOutcome::Failure { shifts }
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, FateOutcome::SuccessWithStyle { .. } | FateOutcome::Success { .. })
    }
}

/// Consequence severity in FATE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsequenceSeverity {
    /// Absorbs 2 shifts, clears after scene
    Mild,
    /// Absorbs 4 shifts, clears after session
    Moderate,
    /// Absorbs 6 shifts, clears after scenario
    Severe,
}

impl ConsequenceSeverity {
    pub fn shifts_absorbed(&self) -> i32 {
        match self {
            ConsequenceSeverity::Mild => 2,
            ConsequenceSeverity::Moderate => 4,
            ConsequenceSeverity::Severe => 6,
        }
    }
}

/// Type of aspect invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokeType {
    /// +2 to roll
    AddTwo,
    /// Reroll all 4dF
    Reroll,
}

/// FATE Core game system.
pub struct FateCoreSystem {
    stat_names: Vec<&'static str>,
    skill_names: Vec<&'static str>,
}

impl FateCoreSystem {
    pub fn new() -> Self {
        Self {
            // FATE Core uses skills, not stats
            // But we can map approaches for FATE Accelerated
            stat_names: vec![],
            skill_names: vec![
                "Athletics",
                "Burglary",
                "Contacts",
                "Crafts",
                "Deceive",
                "Drive",
                "Empathy",
                "Fight",
                "Investigate",
                "Lore",
                "Notice",
                "Physique",
                "Provoke",
                "Rapport",
                "Resources",
                "Shoot",
                "Stealth",
                "Will",
            ],
        }
    }

    /// Create a FATE Accelerated variant.
    pub fn accelerated() -> Self {
        Self {
            // FATE Accelerated uses approaches
            stat_names: vec!["Careful", "Clever", "Flashy", "Forceful", "Quick", "Sneaky"],
            skill_names: vec![],
        }
    }

    /// Calculate stress boxes from skill rating.
    pub fn stress_boxes_from_skill(skill_rating: i32) -> u8 {
        // Base: 2 boxes
        // +1 or +2 skill: 3 boxes
        // +3 or +4 skill: 4 boxes
        match skill_rating {
            ..=0 => 2,
            1..=2 => 3,
            _ => 4,
        }
    }

    /// Calculate refresh from stunt count.
    pub fn calculate_refresh(stunt_count: u8, base_refresh: u8) -> u8 {
        // Stunts beyond 3 reduce refresh
        if stunt_count <= 3 {
            base_refresh
        } else {
            base_refresh.saturating_sub(stunt_count - 3)
        }
    }

    /// Validate skill pyramid.
    pub fn validate_pyramid(skills: &[(String, i32)], max_rating: i32) -> Result<(), String> {
        // Count skills at each level
        let mut counts = std::collections::HashMap::new();
        for (_, rating) in skills {
            if *rating > 0 {
                *counts.entry(*rating).or_insert(0) += 1;
            }
        }

        // Verify pyramid: each level must have >= skills than level above
        for level in (2..=max_rating).rev() {
            let count_at_level = counts.get(&level).copied().unwrap_or(0);
            let count_below = counts.get(&(level - 1)).copied().unwrap_or(0);

            if count_below < count_at_level {
                return Err(format!(
                    "Invalid pyramid: {} skills at +{} but only {} at +{}",
                    count_at_level, level, count_below, level - 1
                ));
            }
        }

        Ok(())
    }
}

impl Default for FateCoreSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSystem for FateCoreSystem {
    fn system_id(&self) -> &str {
        "fate_core"
    }

    fn display_name(&self) -> &str {
        "FATE Core"
    }

    fn calculation_engine(&self) -> &dyn CalculationEngine {
        self
    }

    fn stat_names(&self) -> &[&str] {
        &self.stat_names
    }

    fn skill_names(&self) -> &[&str] {
        &self.skill_names
    }
}

impl CalculationEngine for FateCoreSystem {
    fn ability_modifier(&self, score: i32) -> i32 {
        // In FATE, skills ARE the modifier (ladder value)
        score
    }

    fn proficiency_bonus(&self, _level: u8) -> i32 {
        // FATE has no proficiency system
        0
    }

    fn spell_save_dc(&self, _stats: &StatBlock, _casting_stat: &str) -> i32 {
        // FATE doesn't use spell DCs - magic is handled narratively
        // or through Create Advantage actions
        0
    }

    fn spell_attack_bonus(&self, _stats: &StatBlock, _casting_stat: &str) -> i32 {
        // FATE magic typically uses a skill like Lore
        0
    }

    fn attack_bonus(&self, stats: &StatBlock, attack_skill: &str, _proficient: bool) -> i32 {
        // Return the skill rating directly
        stats.get_stat(attack_skill).unwrap_or(0)
    }

    fn stack_modifiers(&self, modifiers: &[StatModifier]) -> i32 {
        // In FATE, most bonuses don't stack
        // Aspect invocations are +2 each but usually limited
        // Take highest bonus, all penalties stack
        let max_bonus = modifiers
            .iter()
            .filter(|m| m.active && m.value > 0)
            .map(|m| m.value)
            .max()
            .unwrap_or(0);

        let total_penalties: i32 = modifiers
            .iter()
            .filter(|m| m.active && m.value < 0)
            .map(|m| m.value)
            .sum();

        max_bonus + total_penalties
    }

    fn calculate_ac(
        &self,
        stats: &StatBlock,
        _armor_ac: Option<i32>,
        _shield_bonus: Option<i32>,
        _allows_dex: bool,
        _max_dex_bonus: Option<i32>,
    ) -> i32 {
        // FATE uses Athletics or Fight for defense
        stats.get_stat("Athletics").unwrap_or(0)
    }

    fn skill_modifier(
        &self,
        stats: &StatBlock,
        skill: &str,
        _proficiency_level: ProficiencyLevel,
    ) -> i32 {
        // Return skill rating directly
        stats.get_stat(skill).unwrap_or(0)
    }

    fn saving_throw_modifier(
        &self,
        stats: &StatBlock,
        ability: &str,
        _proficient: bool,
    ) -> i32 {
        // FATE uses skills for defense
        // Physique for physical, Will for mental
        match ability {
            "STR" | "DEX" | "CON" => stats.get_stat("Physique").unwrap_or(0),
            "INT" | "WIS" | "CHA" => stats.get_stat("Will").unwrap_or(0),
            _ => stats.get_stat(ability).unwrap_or(0),
        }
    }

    fn passive_perception(&self, stats: &StatBlock, _proficiency_level: ProficiencyLevel) -> i32 {
        // FATE uses Notice skill
        stats.get_stat("Notice").unwrap_or(0)
    }

    fn hit_die(&self, _class_name: &str) -> u8 {
        // FATE doesn't use hit dice
        0
    }

    fn calculate_max_hp(
        &self,
        _level: u8,
        _class_name: &str,
        _constitution_modifier: i32,
        _additional_hp: i32,
    ) -> i32 {
        // FATE uses stress boxes, not HP
        // Return physical stress boxes (typically 2-4)
        2
    }
}

impl CharacterSheetProvider for FateCoreSystem {
    fn character_sheet_schema(&self) -> CharacterSheetSchema {
        CharacterSheetSchema {
            system_id: "fate_core".to_string(),
            system_name: "FATE Core".to_string(),
            sections: vec![
                self.identity_section(),
                self.aspects_section(),
                self.skills_section(),
                self.stunts_section(),
                self.stress_section(),
                self.consequences_section(),
                self.resources_section(),
                self.modifiers_section(),
            ],
            creation_steps: vec![
                CreationStep {
                    id: "identity".to_string(),
                    label: "Identity".to_string(),
                    description: "Name your character and write a brief description.".to_string(),
                    section_ids: vec!["identity".to_string()],
                    order: 1,
                    required: true,
                },
                CreationStep {
                    id: "aspects".to_string(),
                    label: "Aspects".to_string(),
                    description: "Define your High Concept, Trouble, and three additional aspects."
                        .to_string(),
                    section_ids: vec!["aspects".to_string()],
                    order: 2,
                    required: true,
                },
                CreationStep {
                    id: "skills".to_string(),
                    label: "Skills".to_string(),
                    description: "Assign your skills using the skill pyramid.".to_string(),
                    section_ids: vec!["skills".to_string()],
                    order: 3,
                    required: true,
                },
                CreationStep {
                    id: "stunts".to_string(),
                    label: "Stunts & Refresh".to_string(),
                    description: "Choose up to 3 free stunts. Additional stunts reduce refresh."
                        .to_string(),
                    section_ids: vec!["stunts".to_string(), "resources".to_string()],
                    order: 4,
                    required: false,
                },
                CreationStep {
                    id: "stress".to_string(),
                    label: "Stress & Consequences".to_string(),
                    description: "Calculate stress boxes based on Physique and Will.".to_string(),
                    section_ids: vec!["stress".to_string(), "consequences".to_string()],
                    order: 5,
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

        // Calculate physical stress boxes based on Physique
        let physique = values
            .get("PHYSIQUE")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let physical_stress_boxes = Self::stress_boxes_from_skill(physique);
        derived.insert(
            "PHYSICAL_STRESS_BOXES".to_string(),
            serde_json::json!(physical_stress_boxes),
        );

        // Calculate mental stress boxes based on Will
        let will = values
            .get("WILL")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let mental_stress_boxes = Self::stress_boxes_from_skill(will);
        derived.insert(
            "MENTAL_STRESS_BOXES".to_string(),
            serde_json::json!(mental_stress_boxes),
        );

        // Calculate refresh based on stunt count
        let stunt_count = self.count_stunts(values);
        let base_refresh = values
            .get("BASE_REFRESH")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u8;
        let current_refresh = Self::calculate_refresh(stunt_count, base_refresh);
        derived.insert("REFRESH".to_string(), serde_json::json!(current_refresh));

        derived
    }

    fn validate_field(
        &self,
        field_id: &str,
        value: &serde_json::Value,
        _all_values: &HashMap<String, serde_json::Value>,
    ) -> Option<String> {
        // Validate skill ratings
        if self.skill_names().iter().any(|s| s.to_uppercase() == field_id) {
            if let Some(rating) = value.as_i64() {
                if rating < -2 || rating > 8 {
                    return Some("Skill rating must be between -2 (Terrible) and +8 (Legendary)".to_string());
                }
            } else {
                return Some("Skill rating must be a number".to_string());
            }
        }

        // Validate refresh
        if field_id == "REFRESH" || field_id == "BASE_REFRESH" {
            if let Some(refresh) = value.as_i64() {
                if refresh < 1 {
                    return Some("Refresh must be at least 1".to_string());
                }
            } else {
                return Some("Refresh must be a number".to_string());
            }
        }

        // Validate name
        if field_id == "NAME" {
            if let Some(name) = value.as_str() {
                if name.is_empty() {
                    return Some("Name is required".to_string());
                }
            } else {
                return Some("Name must be a string".to_string());
            }
        }

        // Validate high concept and trouble (required aspects)
        if field_id == "HIGH_CONCEPT" || field_id == "TROUBLE" {
            if let Some(aspect) = value.as_str() {
                if aspect.is_empty() {
                    return Some(format!("{} is required", field_id.replace('_', " ").to_lowercase()));
                }
            } else {
                return Some("Aspect must be a string".to_string());
            }
        }

        None
    }

    fn default_values(&self) -> HashMap<String, serde_json::Value> {
        let mut defaults = HashMap::new();

        // Identity
        defaults.insert("NAME".to_string(), serde_json::json!(""));
        defaults.insert("DESCRIPTION".to_string(), serde_json::json!(""));

        // Aspects
        defaults.insert("HIGH_CONCEPT".to_string(), serde_json::json!(""));
        defaults.insert("TROUBLE".to_string(), serde_json::json!(""));
        defaults.insert("ASPECT_1".to_string(), serde_json::json!(""));
        defaults.insert("ASPECT_2".to_string(), serde_json::json!(""));
        defaults.insert("ASPECT_3".to_string(), serde_json::json!(""));

        // Skills - default to Mediocre (0)
        for skill in self.skill_names() {
            defaults.insert(skill.to_uppercase(), serde_json::json!(0));
        }

        // Stunts
        defaults.insert("STUNT_1".to_string(), serde_json::json!(""));
        defaults.insert("STUNT_2".to_string(), serde_json::json!(""));
        defaults.insert("STUNT_3".to_string(), serde_json::json!(""));
        defaults.insert("STUNT_4".to_string(), serde_json::json!(""));
        defaults.insert("STUNT_5".to_string(), serde_json::json!(""));

        // Stress (current values, not max)
        defaults.insert("PHYSICAL_STRESS_1".to_string(), serde_json::json!(false));
        defaults.insert("PHYSICAL_STRESS_2".to_string(), serde_json::json!(false));
        defaults.insert("PHYSICAL_STRESS_3".to_string(), serde_json::json!(false));
        defaults.insert("PHYSICAL_STRESS_4".to_string(), serde_json::json!(false));
        defaults.insert("MENTAL_STRESS_1".to_string(), serde_json::json!(false));
        defaults.insert("MENTAL_STRESS_2".to_string(), serde_json::json!(false));
        defaults.insert("MENTAL_STRESS_3".to_string(), serde_json::json!(false));
        defaults.insert("MENTAL_STRESS_4".to_string(), serde_json::json!(false));

        // Consequences
        defaults.insert("CONSEQUENCE_MILD".to_string(), serde_json::json!(""));
        defaults.insert("CONSEQUENCE_MODERATE".to_string(), serde_json::json!(""));
        defaults.insert("CONSEQUENCE_SEVERE".to_string(), serde_json::json!(""));

        // Resources
        defaults.insert("BASE_REFRESH".to_string(), serde_json::json!(3));
        defaults.insert("CURRENT_FATE_POINTS".to_string(), serde_json::json!(3));

        defaults
    }
}

// Helper methods for building the schema
impl FateCoreSystem {
    /// Get the FATE ladder labels for skill ratings.
    fn fate_ladder_labels() -> Vec<LadderLabel> {
        vec![
            LadderLabel { value: -2, label: "Terrible (-2)".to_string() },
            LadderLabel { value: -1, label: "Poor (-1)".to_string() },
            LadderLabel { value: 0, label: "Mediocre (+0)".to_string() },
            LadderLabel { value: 1, label: "Average (+1)".to_string() },
            LadderLabel { value: 2, label: "Fair (+2)".to_string() },
            LadderLabel { value: 3, label: "Good (+3)".to_string() },
            LadderLabel { value: 4, label: "Great (+4)".to_string() },
            LadderLabel { value: 5, label: "Superb (+5)".to_string() },
            LadderLabel { value: 6, label: "Fantastic (+6)".to_string() },
            LadderLabel { value: 7, label: "Epic (+7)".to_string() },
            LadderLabel { value: 8, label: "Legendary (+8)".to_string() },
        ]
    }

    /// Count non-empty stunts.
    fn count_stunts(&self, values: &HashMap<String, serde_json::Value>) -> u8 {
        let mut count = 0;
        for i in 1..=5 {
            if let Some(stunt) = values.get(&format!("STUNT_{}", i)) {
                if let Some(text) = stunt.as_str() {
                    if !text.is_empty() {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    fn identity_section(&self) -> SchemaSection {
        SchemaSection {
            id: "identity".to_string(),
            label: "Identity".to_string(),
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
                    id: "DESCRIPTION".to_string(),
                    label: "Description".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: true,
                        max_length: Some(1000),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("A brief description of your character".to_string()),
                    placeholder: Some("Describe your character's appearance, background, or personality...".to_string()),
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: None,
        }
    }

    fn aspects_section(&self) -> SchemaSection {
        SchemaSection {
            id: "aspects".to_string(),
            label: "Aspects".to_string(),
            section_type: SectionType::Features,
            fields: vec![
                FieldDefinition {
                    id: "HIGH_CONCEPT".to_string(),
                    label: "High Concept".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        ..Default::default()
                    },
                    description: Some("A phrase that sums up what your character is about - who they are and what they do.".to_string()),
                    placeholder: Some("e.g., 'Hard-boiled Detective with a Heart of Gold'".to_string()),
                },
                FieldDefinition {
                    id: "TROUBLE".to_string(),
                    label: "Trouble".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Something that complicates your character's existence - a weakness, rival, or obligation.".to_string()),
                    placeholder: Some("e.g., 'The Mob Wants Me Dead'".to_string()),
                },
                FieldDefinition {
                    id: "ASPECT_1".to_string(),
                    label: "Aspect".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("An additional aspect describing your character.".to_string()),
                    placeholder: Some("Enter an aspect...".to_string()),
                },
                FieldDefinition {
                    id: "ASPECT_2".to_string(),
                    label: "Aspect".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        new_row: true,
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Enter an aspect...".to_string()),
                },
                FieldDefinition {
                    id: "ASPECT_3".to_string(),
                    label: "Aspect".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        new_row: true,
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Enter an aspect...".to_string()),
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: Some("Aspects are phrases that describe something unique or important about your character.".to_string()),
        }
    }

    fn skills_section(&self) -> SchemaSection {
        let ladder_labels = Self::fate_ladder_labels();

        let fields: Vec<FieldDefinition> = self
            .skill_names()
            .iter()
            .map(|skill| {
                FieldDefinition {
                    id: skill.to_uppercase(),
                    label: skill.to_string(),
                    field_type: SchemaFieldType::LadderRating {
                        min: -2,
                        max: 8,
                        labels: ladder_labels.clone(),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(-2),
                        max: Some(8),
                        pattern: None,
                        error_message: Some("Rating must be between -2 and +8".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(6),
                        ..Default::default()
                    },
                    description: None,
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
            description: Some("Rate your skills using the FATE ladder. Build a skill pyramid: 1 Great (+4), 2 Good (+3), 3 Fair (+2), 4 Average (+1).".to_string()),
        }
    }

    fn stunts_section(&self) -> SchemaSection {
        let mut fields = Vec::new();

        for i in 1..=5 {
            let required = i <= 3; // First 3 stunts are free
            fields.push(FieldDefinition {
                id: format!("STUNT_{}", i),
                label: format!("Stunt {}", i),
                field_type: SchemaFieldType::Text {
                    multiline: true,
                    max_length: Some(500),
                },
                editable: true,
                required: false,
                derived_from: None,
                validation: None,
                layout: FieldLayout {
                    width: Some(12),
                    new_row: true,
                    ..Default::default()
                },
                description: if required {
                    Some("Free stunt slot".to_string())
                } else {
                    Some("Additional stunt (costs 1 Refresh)".to_string())
                },
                placeholder: Some("Describe your stunt and its mechanical effect...".to_string()),
            });
        }

        SchemaSection {
            id: "stunts".to_string(),
            label: "Stunts".to_string(),
            section_type: SectionType::Features,
            fields,
            collapsible: true,
            collapsed_default: false,
            description: Some("Stunts are special abilities that give you a bonus in specific circumstances. You get 3 free stunts; additional stunts cost 1 Refresh each.".to_string()),
        }
    }

    fn stress_section(&self) -> SchemaSection {
        let mut fields = Vec::new();

        // Physical stress boxes
        fields.push(FieldDefinition {
            id: "PHYSICAL_STRESS_BOXES".to_string(),
            label: "Physical Stress Boxes".to_string(),
            field_type: SchemaFieldType::Integer {
                min: Some(2),
                max: Some(4),
                show_modifier: false,
            },
            editable: false,
            required: false,
            derived_from: Some(DerivedField {
                derivation_type: DerivationType::Custom,
                dependencies: vec!["PHYSIQUE".to_string()],
                display_format: None,
            }),
            validation: None,
            layout: FieldLayout {
                width: Some(3),
                ..Default::default()
            },
            description: Some("Based on Physique skill".to_string()),
            placeholder: None,
        });

        for i in 1..=4 {
            fields.push(FieldDefinition {
                id: format!("PHYSICAL_STRESS_{}", i),
                label: format!("[{}]", i),
                field_type: SchemaFieldType::Boolean {
                    checked_label: Some("Marked".to_string()),
                    unchecked_label: Some("Clear".to_string()),
                },
                editable: true,
                required: false,
                derived_from: None,
                validation: None,
                layout: FieldLayout {
                    width: Some(2),
                    ..Default::default()
                },
                description: Some(format!("{}-shift stress box", i)),
                placeholder: None,
            });
        }

        // Mental stress boxes
        fields.push(FieldDefinition {
            id: "MENTAL_STRESS_BOXES".to_string(),
            label: "Mental Stress Boxes".to_string(),
            field_type: SchemaFieldType::Integer {
                min: Some(2),
                max: Some(4),
                show_modifier: false,
            },
            editable: false,
            required: false,
            derived_from: Some(DerivedField {
                derivation_type: DerivationType::Custom,
                dependencies: vec!["WILL".to_string()],
                display_format: None,
            }),
            validation: None,
            layout: FieldLayout {
                width: Some(3),
                new_row: true,
                ..Default::default()
            },
            description: Some("Based on Will skill".to_string()),
            placeholder: None,
        });

        for i in 1..=4 {
            fields.push(FieldDefinition {
                id: format!("MENTAL_STRESS_{}", i),
                label: format!("[{}]", i),
                field_type: SchemaFieldType::Boolean {
                    checked_label: Some("Marked".to_string()),
                    unchecked_label: Some("Clear".to_string()),
                },
                editable: true,
                required: false,
                derived_from: None,
                validation: None,
                layout: FieldLayout {
                    width: Some(2),
                    ..Default::default()
                },
                description: Some(format!("{}-shift stress box", i)),
                placeholder: None,
            });
        }

        SchemaSection {
            id: "stress".to_string(),
            label: "Stress".to_string(),
            section_type: SectionType::Resources,
            fields,
            collapsible: false,
            collapsed_default: false,
            description: Some("Stress represents minor hits. Physical stress boxes are based on Physique, mental on Will. Mark boxes to absorb shifts of harm.".to_string()),
        }
    }

    fn consequences_section(&self) -> SchemaSection {
        SchemaSection {
            id: "consequences".to_string(),
            label: "Consequences".to_string(),
            section_type: SectionType::Resources,
            fields: vec![
                FieldDefinition {
                    id: "CONSEQUENCE_MILD".to_string(),
                    label: "Mild (2)".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        ..Default::default()
                    },
                    description: Some("Absorbs 2 shifts. Clears at end of scene with successful overcome action.".to_string()),
                    placeholder: Some("e.g., 'Bruised Ribs', 'Rattled'".to_string()),
                },
                FieldDefinition {
                    id: "CONSEQUENCE_MODERATE".to_string(),
                    label: "Moderate (4)".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Absorbs 4 shifts. Clears at end of session.".to_string()),
                    placeholder: Some("e.g., 'Deep Gash', 'Shaken to the Core'".to_string()),
                },
                FieldDefinition {
                    id: "CONSEQUENCE_SEVERE".to_string(),
                    label: "Severe (6)".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Absorbs 6 shifts. Clears at end of scenario.".to_string()),
                    placeholder: Some("e.g., 'Broken Leg', 'Complete Mental Breakdown'".to_string()),
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: Some("Consequences are lasting injuries or trauma. They absorb shifts but can be compelled as aspects.".to_string()),
        }
    }

    fn resources_section(&self) -> SchemaSection {
        SchemaSection {
            id: "resources".to_string(),
            label: "Refresh & Fate Points".to_string(),
            section_type: SectionType::Resources,
            fields: vec![
                FieldDefinition {
                    id: "BASE_REFRESH".to_string(),
                    label: "Base Refresh".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(1),
                        max: Some(10),
                        show_modifier: false,
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(1),
                        max: Some(10),
                        pattern: None,
                        error_message: Some("Base refresh must be at least 1".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(3),
                        ..Default::default()
                    },
                    description: Some("Starting refresh value (default 3)".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "REFRESH".to_string(),
                    label: "Current Refresh".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(1),
                        max: Some(10),
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["BASE_REFRESH".to_string(), "STUNT_1".to_string(), "STUNT_2".to_string(), "STUNT_3".to_string(), "STUNT_4".to_string(), "STUNT_5".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(3),
                        ..Default::default()
                    },
                    description: Some("Refresh after stunt costs (3 free, then -1 per additional)".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "CURRENT_FATE_POINTS".to_string(),
                    label: "Fate Points".to_string(),
                    field_type: SchemaFieldType::ResourceBar {
                        max_field: "REFRESH".to_string(),
                        color: ResourceColor::Blue,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(6),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Spend to invoke aspects or power stunts. Resets to refresh at session start.".to_string()),
                    placeholder: None,
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: Some("Fate points let you invoke aspects for bonuses or trigger certain stunts.".to_string()),
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
                    label: "Situational Aspects & Boosts".to_string(),
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
                        "Active situational aspects, boosts, and temporary effects affecting your rolls.".to_string(),
                    ),
                    placeholder: None,
                },
            ],
            collapsible: true,
            collapsed_default: false,
            description: Some("Aspects can be invoked for +2 or reroll. Boosts are free invokes that disappear after use. Consequences are negative aspects that can be compelled.".to_string()),
        }
    }
}

/// Four actions in FATE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FateAction {
    /// Get past obstacles
    Overcome,
    /// Create or discover aspects
    CreateAdvantage,
    /// Deal stress to opponents
    Attack,
    /// Prevent Attack or Create Advantage
    Defend,
}

/// Simulate rolling 4dF (4 Fudge dice).
/// Each die: -1, 0, or +1
/// Returns sum (-4 to +4)
pub fn roll_4df(results: &[i8; 4]) -> i32 {
    results.iter().map(|&d| d as i32).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_ratings() {
        assert_eq!(LadderRating::from_value(4), Some(LadderRating::Great));
        assert_eq!(LadderRating::Great.descriptor(), "Great");
        assert_eq!(LadderRating::from_value(-2), Some(LadderRating::Terrible));
    }

    #[test]
    fn fate_outcomes() {
        // Success with style (3+ shifts)
        assert!(matches!(
            FateOutcome::determine(7, 4),
            FateOutcome::SuccessWithStyle { shifts: 3 }
        ));

        // Success (1-2 shifts)
        assert!(matches!(
            FateOutcome::determine(5, 4),
            FateOutcome::Success { shifts: 1 }
        ));

        // Tie (0 shifts)
        assert_eq!(FateOutcome::determine(4, 4), FateOutcome::Tie);

        // Failure (negative shifts)
        assert!(matches!(
            FateOutcome::determine(2, 4),
            FateOutcome::Failure { shifts: -2 }
        ));
    }

    #[test]
    fn consequence_absorption() {
        assert_eq!(ConsequenceSeverity::Mild.shifts_absorbed(), 2);
        assert_eq!(ConsequenceSeverity::Moderate.shifts_absorbed(), 4);
        assert_eq!(ConsequenceSeverity::Severe.shifts_absorbed(), 6);
    }

    #[test]
    fn stress_boxes_calculation() {
        assert_eq!(FateCoreSystem::stress_boxes_from_skill(0), 2);
        assert_eq!(FateCoreSystem::stress_boxes_from_skill(2), 3);
        assert_eq!(FateCoreSystem::stress_boxes_from_skill(4), 4);
    }

    #[test]
    fn refresh_calculation() {
        assert_eq!(FateCoreSystem::calculate_refresh(3, 3), 3);
        assert_eq!(FateCoreSystem::calculate_refresh(4, 3), 2);
        assert_eq!(FateCoreSystem::calculate_refresh(5, 3), 1);
    }

    #[test]
    fn pyramid_validation() {
        // Valid pyramid: 1 at +4, 2 at +3, 3 at +2, 4 at +1
        let valid_skills = vec![
            ("Fight".to_string(), 4),
            ("Athletics".to_string(), 3),
            ("Will".to_string(), 3),
            ("Notice".to_string(), 2),
            ("Physique".to_string(), 2),
            ("Shoot".to_string(), 2),
            ("Investigate".to_string(), 1),
            ("Lore".to_string(), 1),
            ("Empathy".to_string(), 1),
            ("Rapport".to_string(), 1),
        ];
        assert!(FateCoreSystem::validate_pyramid(&valid_skills, 4).is_ok());

        // Invalid: 2 at +4, only 1 at +3
        let invalid_skills = vec![
            ("Fight".to_string(), 4),
            ("Athletics".to_string(), 4),
            ("Will".to_string(), 3),
        ];
        assert!(FateCoreSystem::validate_pyramid(&invalid_skills, 4).is_err());
    }

    #[test]
    fn roll_4df_sums() {
        assert_eq!(roll_4df(&[1, 1, 1, 1]), 4);
        assert_eq!(roll_4df(&[-1, -1, -1, -1]), -4);
        assert_eq!(roll_4df(&[1, -1, 0, 0]), 0);
        assert_eq!(roll_4df(&[1, 1, -1, 0]), 1);
    }

    #[test]
    fn system_identification() {
        let system = FateCoreSystem::new();
        assert_eq!(system.system_id(), "fate_core");
        assert_eq!(system.display_name(), "FATE Core");
    }

    #[test]
    fn character_sheet_schema_structure() {
        use super::CharacterSheetProvider;

        let system = FateCoreSystem::new();
        let schema = system.character_sheet_schema();

        assert_eq!(schema.system_id, "fate_core");
        assert_eq!(schema.system_name, "FATE Core");
        assert_eq!(schema.sections.len(), 7);

        // Verify section IDs
        let section_ids: Vec<&str> = schema.sections.iter().map(|s| s.id.as_str()).collect();
        assert!(section_ids.contains(&"identity"));
        assert!(section_ids.contains(&"aspects"));
        assert!(section_ids.contains(&"skills"));
        assert!(section_ids.contains(&"stunts"));
        assert!(section_ids.contains(&"stress"));
        assert!(section_ids.contains(&"consequences"));
        assert!(section_ids.contains(&"resources"));

        // Verify creation steps
        assert_eq!(schema.creation_steps.len(), 5);
    }

    #[test]
    fn character_sheet_skills_section() {
        use super::CharacterSheetProvider;

        let system = FateCoreSystem::new();
        let schema = system.character_sheet_schema();

        let skills_section = schema.sections.iter().find(|s| s.id == "skills").unwrap();
        assert_eq!(skills_section.fields.len(), 18); // 18 FATE Core skills

        // Verify skill field types are LadderRating
        for field in &skills_section.fields {
            match &field.field_type {
                super::SchemaFieldType::LadderRating { min, max, labels } => {
                    assert_eq!(*min, -2);
                    assert_eq!(*max, 8);
                    assert_eq!(labels.len(), 11); // -2 to +8
                }
                _ => panic!("Expected LadderRating field type for skill: {}", field.id),
            }
        }
    }

    #[test]
    fn character_sheet_aspects_section() {
        use super::CharacterSheetProvider;

        let system = FateCoreSystem::new();
        let schema = system.character_sheet_schema();

        let aspects_section = schema.sections.iter().find(|s| s.id == "aspects").unwrap();
        assert_eq!(aspects_section.fields.len(), 5); // High Concept, Trouble, 3 additional

        // High Concept and Trouble are required
        let high_concept = aspects_section.fields.iter().find(|f| f.id == "HIGH_CONCEPT").unwrap();
        assert!(high_concept.required);

        let trouble = aspects_section.fields.iter().find(|f| f.id == "TROUBLE").unwrap();
        assert!(trouble.required);
    }

    #[test]
    fn calculate_derived_stress_boxes() {
        use super::CharacterSheetProvider;

        let system = FateCoreSystem::new();
        let mut values = HashMap::new();

        // Test with Physique +0 (Mediocre) and Will +0 (Mediocre)
        values.insert("PHYSIQUE".to_string(), serde_json::json!(0));
        values.insert("WILL".to_string(), serde_json::json!(0));

        let derived = system.calculate_derived_values(&values);
        assert_eq!(derived.get("PHYSICAL_STRESS_BOXES"), Some(&serde_json::json!(2)));
        assert_eq!(derived.get("MENTAL_STRESS_BOXES"), Some(&serde_json::json!(2)));

        // Test with Physique +3 (Good) and Will +4 (Great)
        values.insert("PHYSIQUE".to_string(), serde_json::json!(3));
        values.insert("WILL".to_string(), serde_json::json!(4));

        let derived = system.calculate_derived_values(&values);
        assert_eq!(derived.get("PHYSICAL_STRESS_BOXES"), Some(&serde_json::json!(4)));
        assert_eq!(derived.get("MENTAL_STRESS_BOXES"), Some(&serde_json::json!(4)));
    }

    #[test]
    fn calculate_refresh_from_stunts() {
        use super::CharacterSheetProvider;

        let system = FateCoreSystem::new();
        let mut values = HashMap::new();

        // Base refresh 3, no stunts
        values.insert("BASE_REFRESH".to_string(), serde_json::json!(3));

        let derived = system.calculate_derived_values(&values);
        assert_eq!(derived.get("REFRESH"), Some(&serde_json::json!(3)));

        // Add 3 stunts (all free, no reduction)
        values.insert("STUNT_1".to_string(), serde_json::json!("Stunt One"));
        values.insert("STUNT_2".to_string(), serde_json::json!("Stunt Two"));
        values.insert("STUNT_3".to_string(), serde_json::json!("Stunt Three"));

        let derived = system.calculate_derived_values(&values);
        assert_eq!(derived.get("REFRESH"), Some(&serde_json::json!(3)));

        // Add a 4th stunt (costs 1 refresh)
        values.insert("STUNT_4".to_string(), serde_json::json!("Stunt Four"));

        let derived = system.calculate_derived_values(&values);
        assert_eq!(derived.get("REFRESH"), Some(&serde_json::json!(2)));

        // Add a 5th stunt (costs another refresh)
        values.insert("STUNT_5".to_string(), serde_json::json!("Stunt Five"));

        let derived = system.calculate_derived_values(&values);
        assert_eq!(derived.get("REFRESH"), Some(&serde_json::json!(1)));
    }

    #[test]
    fn validate_skill_ratings() {
        use super::CharacterSheetProvider;

        let system = FateCoreSystem::new();
        let values = HashMap::new();

        // Valid rating
        assert!(system.validate_field("ATHLETICS", &serde_json::json!(4), &values).is_none());
        assert!(system.validate_field("ATHLETICS", &serde_json::json!(-2), &values).is_none());

        // Invalid ratings
        assert!(system.validate_field("ATHLETICS", &serde_json::json!(9), &values).is_some());
        assert!(system.validate_field("ATHLETICS", &serde_json::json!(-3), &values).is_some());
    }

    #[test]
    fn default_values_structure() {
        use super::CharacterSheetProvider;

        let system = FateCoreSystem::new();
        let defaults = system.default_values();

        // Check identity defaults
        assert_eq!(defaults.get("NAME"), Some(&serde_json::json!("")));

        // Check aspect defaults
        assert_eq!(defaults.get("HIGH_CONCEPT"), Some(&serde_json::json!("")));
        assert_eq!(defaults.get("TROUBLE"), Some(&serde_json::json!("")));

        // Check skill defaults (should be 0 = Mediocre)
        assert_eq!(defaults.get("ATHLETICS"), Some(&serde_json::json!(0)));
        assert_eq!(defaults.get("WILL"), Some(&serde_json::json!(0)));

        // Check resource defaults
        assert_eq!(defaults.get("BASE_REFRESH"), Some(&serde_json::json!(3)));
        assert_eq!(defaults.get("CURRENT_FATE_POINTS"), Some(&serde_json::json!(3)));

        // Check stress box defaults
        assert_eq!(defaults.get("PHYSICAL_STRESS_1"), Some(&serde_json::json!(false)));
        assert_eq!(defaults.get("MENTAL_STRESS_1"), Some(&serde_json::json!(false)));

        // Check consequence defaults
        assert_eq!(defaults.get("CONSEQUENCE_MILD"), Some(&serde_json::json!("")));
    }
}
