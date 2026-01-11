//! Blades in the Dark game system implementation.
//!
//! Blades uses a d6 dice pool system with position/effect mechanics.
//! Key features:
//! - Roll dice pool, take highest
//! - Position (Controlled/Risky/Desperate) determines consequence severity
//! - Effect (Zero/Limited/Standard/Great) determines success magnitude
//! - Stress as a spendable resource
//! - Clocks for progress tracking

use super::traits::{
    AllocationSystem, CalculationEngine, CharacterSheetProvider, CharacterSheetSchema,
    ConditionLevel, CreationStep, DerivedField, DerivationType, DotPoolCategory, FieldDefinition,
    FieldLayout, FieldValidation, GameSystem, ProficiencyLevel, ResourceColor, SchemaFieldType,
    SchemaSection, SchemaSelectOption, SectionType, StartingDot,
};
use crate::entities::{StatBlock, StatModifier};
use std::collections::HashMap;

/// Blades action roll outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BladesOutcome {
    /// Multiple 6s - success with increased effect
    Critical,
    /// Highest die is 6 - clean success
    Success,
    /// Highest die is 4-5 - success with complication
    PartialSuccess,
    /// Highest die is 1-3 - failure with consequence
    Failure,
}

impl BladesOutcome {
    /// Determine outcome from dice results.
    pub fn from_dice(dice: &[u8]) -> Self {
        if dice.is_empty() {
            return BladesOutcome::Failure;
        }

        let highest = *dice.iter().max().unwrap();
        let sixes = dice.iter().filter(|&&d| d == 6).count();

        if sixes >= 2 {
            BladesOutcome::Critical
        } else if highest == 6 {
            BladesOutcome::Success
        } else if highest >= 4 {
            BladesOutcome::PartialSuccess
        } else {
            BladesOutcome::Failure
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(
            self,
            BladesOutcome::Critical | BladesOutcome::Success | BladesOutcome::PartialSuccess
        )
    }
}

/// Position determines consequence severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Safe, dominant advantage - minor consequences
    Controlled,
    /// Standard risk - moderate consequences
    Risky,
    /// Serious trouble - severe consequences
    Desperate,
}

impl Position {
    /// XP is gained for desperate actions.
    pub fn grants_xp(&self) -> bool {
        matches!(self, Position::Desperate)
    }
}

/// Effect level determines success magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EffectLevel {
    /// No meaningful progress
    Zero,
    /// Partial/weak effect (1 clock tick)
    Limited,
    /// Normal effect (2 clock ticks)
    Standard,
    /// Better than usual (3 clock ticks)
    Great,
    /// Extraordinary (4 clock ticks, from critical)
    Extreme,
}

impl EffectLevel {
    /// Clock segments filled by this effect level.
    pub fn clock_ticks(&self) -> u8 {
        match self {
            EffectLevel::Zero => 0,
            EffectLevel::Limited => 1,
            EffectLevel::Standard => 2,
            EffectLevel::Great => 3,
            EffectLevel::Extreme => 4,
        }
    }

    /// Increase effect by one level (e.g., from critical).
    pub fn increase(self) -> Self {
        match self {
            EffectLevel::Zero => EffectLevel::Limited,
            EffectLevel::Limited => EffectLevel::Standard,
            EffectLevel::Standard => EffectLevel::Great,
            EffectLevel::Great => EffectLevel::Extreme,
            EffectLevel::Extreme => EffectLevel::Extreme,
        }
    }

    /// Decrease effect by one level.
    pub fn decrease(self) -> Self {
        match self {
            EffectLevel::Zero => EffectLevel::Zero,
            EffectLevel::Limited => EffectLevel::Zero,
            EffectLevel::Standard => EffectLevel::Limited,
            EffectLevel::Great => EffectLevel::Standard,
            EffectLevel::Extreme => EffectLevel::Great,
        }
    }
}

/// Harm levels in Blades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarmLevel {
    /// Lesser harm - narrative only
    Level1,
    /// Moderate harm - -1d to related actions
    Level2,
    /// Severe harm - -1d (stacks)
    Level3,
    /// Fatal - dying/dead
    Level4,
}

impl HarmLevel {
    /// Dice penalty from this harm level.
    pub fn dice_penalty(&self) -> u8 {
        match self {
            HarmLevel::Level1 => 0,
            HarmLevel::Level2 => 1,
            HarmLevel::Level3 => 1,
            HarmLevel::Level4 => 0, // You're dying, penalties don't matter
        }
    }
}

/// Trauma conditions in Blades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraumaCondition {
    Cold,
    Haunted,
    Obsessed,
    Paranoid,
    Reckless,
    Soft,
    Unstable,
    Vicious,
}

/// Load levels for equipment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadLevel {
    /// 3 items, no penalty
    Light,
    /// 5 items, no penalty
    Normal,
    /// 6 items, -1d to Prowess actions
    Heavy,
}

impl LoadLevel {
    pub fn max_items(&self) -> u8 {
        match self {
            LoadLevel::Light => 3,
            LoadLevel::Normal => 5,
            LoadLevel::Heavy => 6,
        }
    }

    pub fn prowess_penalty(&self) -> u8 {
        match self {
            LoadLevel::Light | LoadLevel::Normal => 0,
            LoadLevel::Heavy => 1,
        }
    }
}

/// Blades in the Dark game system.
pub struct BladesSystem {
    action_names: Vec<&'static str>,
}

impl BladesSystem {
    pub fn new() -> Self {
        Self {
            action_names: vec![
                // Insight
                "Hunt",
                "Study",
                "Survey",
                "Tinker",
                // Prowess
                "Finesse",
                "Prowl",
                "Skirmish",
                "Wreck",
                // Resolve
                "Attune",
                "Command",
                "Consort",
                "Sway",
            ],
        }
    }

    /// Calculate attribute rating from action dots.
    pub fn insight_rating(hunt: u8, study: u8, survey: u8, tinker: u8) -> u8 {
        hunt + study + survey + tinker
    }

    pub fn prowess_rating(finesse: u8, prowl: u8, skirmish: u8, wreck: u8) -> u8 {
        finesse + prowl + skirmish + wreck
    }

    pub fn resolve_rating(attune: u8, command: u8, consort: u8, sway: u8) -> u8 {
        attune + command + consort + sway
    }

    /// Calculate resistance roll stress cost.
    pub fn resistance_roll_cost(sixes_rolled: u8) -> u8 {
        6_u8.saturating_sub(sixes_rolled)
    }
}

impl Default for BladesSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSystem for BladesSystem {
    fn system_id(&self) -> &str {
        "blades"
    }

    fn display_name(&self) -> &str {
        "Blades in the Dark"
    }

    fn calculation_engine(&self) -> &dyn CalculationEngine {
        self
    }

    fn stat_names(&self) -> &[&str] {
        // Blades uses attributes (Insight, Prowess, Resolve)
        // but these are derived from actions
        &["Insight", "Prowess", "Resolve"]
    }

    fn skill_names(&self) -> &[&str] {
        // Actions are like skills
        &self.action_names
    }
}

impl CalculationEngine for BladesSystem {
    fn ability_modifier(&self, score: i32) -> i32 {
        // In Blades, action ratings (0-4) ARE the dice pool
        score
    }

    fn proficiency_bonus(&self, _level: u8) -> i32 {
        // Blades has no proficiency system
        0
    }

    fn spell_save_dc(&self, _stats: &StatBlock, _casting_stat: &str) -> i32 {
        // Blades doesn't use spell DCs
        0
    }

    fn spell_attack_bonus(&self, _stats: &StatBlock, _casting_stat: &str) -> i32 {
        // Attune is used for supernatural
        0
    }

    fn attack_bonus(&self, stats: &StatBlock, attack_action: &str, _proficient: bool) -> i32 {
        // Return action rating as dice pool
        stats.get_stat(attack_action).unwrap_or(0)
    }

    fn stack_modifiers(&self, modifiers: &[StatModifier]) -> i32 {
        // In Blades, modifiers add/remove dice from pool
        modifiers
            .iter()
            .filter(|m| m.active)
            .map(|m| m.value)
            .sum()
    }

    fn calculate_ac(
        &self,
        _stats: &StatBlock,
        _armor_ac: Option<i32>,
        _shield_bonus: Option<i32>,
        _allows_dex: bool,
        _max_dex_bonus: Option<i32>,
    ) -> i32 {
        // Blades doesn't have AC
        0
    }

    fn skill_modifier(
        &self,
        stats: &StatBlock,
        action: &str,
        _proficiency_level: ProficiencyLevel,
    ) -> i32 {
        // Return action rating
        stats.get_stat(action).unwrap_or(0)
    }

    fn saving_throw_modifier(
        &self,
        stats: &StatBlock,
        attribute: &str,
        _proficient: bool,
    ) -> i32 {
        // Resistance rolls use attribute ratings
        stats.get_stat(attribute).unwrap_or(0)
    }

    fn passive_perception(&self, stats: &StatBlock, _proficiency_level: ProficiencyLevel) -> i32 {
        // Survey is closest to perception
        stats.get_stat("Survey").unwrap_or(0)
    }

    fn hit_die(&self, _class_name: &str) -> u8 {
        // Blades uses stress, not HP
        0
    }

    fn calculate_max_hp(
        &self,
        _level: u8,
        _class_name: &str,
        _constitution_modifier: i32,
        _additional_hp: i32,
    ) -> i32 {
        // Max stress is 9
        9
    }
}

impl CharacterSheetProvider for BladesSystem {
    fn character_sheet_schema(&self) -> CharacterSheetSchema {
        CharacterSheetSchema {
            system_id: "blades".to_string(),
            system_name: "Blades in the Dark".to_string(),
            sections: vec![
                self.identity_section(),
                self.attributes_actions_section(),
                self.harm_section(),
                self.stress_trauma_section(),
                self.load_armor_section(),
                self.special_abilities_section(),
                self.modifiers_section(),
            ],
            creation_steps: vec![
                CreationStep {
                    id: "identity".to_string(),
                    label: "Identity".to_string(),
                    description: "Choose your character's name, alias, playbook, heritage, background, and vice.".to_string(),
                    section_ids: vec!["identity".to_string()],
                    order: 1,
                    required: true,
                    allocation: None,
                },
                CreationStep {
                    id: "actions".to_string(),
                    label: "Action Ratings".to_string(),
                    description: "Assign action dots based on your playbook. You get 4 action dots to assign, plus your playbook's starting action at rating 2.".to_string(),
                    section_ids: vec!["attributes_actions".to_string()],
                    order: 2,
                    required: true,
                    allocation: Some(Self::dot_pool_allocation()),
                },
                CreationStep {
                    id: "abilities".to_string(),
                    label: "Special Abilities".to_string(),
                    description: "Choose your starting special ability from your playbook.".to_string(),
                    section_ids: vec!["special_abilities".to_string()],
                    order: 3,
                    required: true,
                    allocation: None,
                },
                CreationStep {
                    id: "load".to_string(),
                    label: "Load & Equipment".to_string(),
                    description: "Choose your load level and equipment for the score.".to_string(),
                    section_ids: vec!["load_armor".to_string()],
                    order: 4,
                    required: false,
                    allocation: None,
                },
            ],
        }
    }

    fn calculate_derived_values(
        &self,
        values: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut derived = HashMap::new();

        // Helper to get action rating
        let get_action = |name: &str| -> u8 {
            values
                .get(name)
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u8
        };

        // Calculate Insight attribute (number of actions with 1+ dots)
        let insight_actions = ["HUNT", "STUDY", "SURVEY", "TINKER"];
        let insight_rating: u8 = insight_actions
            .iter()
            .filter(|a| get_action(a) > 0)
            .count() as u8;
        derived.insert("INSIGHT".to_string(), serde_json::json!(insight_rating));

        // Calculate Prowess attribute
        let prowess_actions = ["FINESSE", "PROWL", "SKIRMISH", "WRECK"];
        let prowess_rating: u8 = prowess_actions
            .iter()
            .filter(|a| get_action(a) > 0)
            .count() as u8;
        derived.insert("PROWESS".to_string(), serde_json::json!(prowess_rating));

        // Calculate Resolve attribute
        let resolve_actions = ["ATTUNE", "COMMAND", "CONSORT", "SWAY"];
        let resolve_rating: u8 = resolve_actions
            .iter()
            .filter(|a| get_action(a) > 0)
            .count() as u8;
        derived.insert("RESOLVE".to_string(), serde_json::json!(resolve_rating));

        // Calculate trauma count
        let trauma_conditions = [
            "TRAUMA_COLD",
            "TRAUMA_HAUNTED",
            "TRAUMA_OBSESSED",
            "TRAUMA_PARANOID",
            "TRAUMA_RECKLESS",
            "TRAUMA_SOFT",
            "TRAUMA_UNSTABLE",
            "TRAUMA_VICIOUS",
        ];
        let trauma_count: u8 = trauma_conditions
            .iter()
            .filter(|t| values.get(&t.to_string()).and_then(|v| v.as_bool()).unwrap_or(false))
            .count() as u8;
        derived.insert("TRAUMA_COUNT".to_string(), serde_json::json!(trauma_count));

        // Calculate load used
        // This would sum up equipped items' load values
        // For now, just track the load level's max
        if let Some(load_level) = values.get("LOAD_LEVEL").and_then(|v| v.as_str()) {
            let max_load = match load_level {
                "light" => 3,
                "normal" => 5,
                "heavy" => 6,
                _ => 5,
            };
            derived.insert("MAX_LOAD".to_string(), serde_json::json!(max_load));
        }

        // Calculate XP trigger based on playbook
        if let Some(playbook) = values.get("PLAYBOOK").and_then(|v| v.as_str()) {
            let xp_trigger = match playbook {
                "cutter" => "Address challenges with violence or coercion",
                "hound" => "Address challenges with tracking or violence",
                "leech" => "Address challenges with technical skill or mayhem",
                "lurk" => "Address challenges with stealth or evasion",
                "slide" => "Address challenges with deception or influence",
                "spider" => "Address challenges with calculation or conspiracy",
                "whisper" => "Address challenges with knowledge or arcane power",
                _ => "",
            };
            derived.insert("XP_TRIGGER".to_string(), serde_json::json!(xp_trigger));
        }

        derived
    }

    fn validate_field(
        &self,
        field_id: &str,
        value: &serde_json::Value,
        all_values: &HashMap<String, serde_json::Value>,
    ) -> Option<String> {
        match field_id {
            // Validate action ratings (0-4)
            "HUNT" | "STUDY" | "SURVEY" | "TINKER" | "FINESSE" | "PROWL" | "SKIRMISH"
            | "WRECK" | "ATTUNE" | "COMMAND" | "CONSORT" | "SWAY" => {
                if let Some(rating) = value.as_u64() {
                    if rating > 4 {
                        return Some("Action ratings must be between 0 and 4".to_string());
                    }
                } else {
                    return Some("Action rating must be a number".to_string());
                }
            }

            // Validate stress (0-9)
            "STRESS" => {
                if let Some(stress) = value.as_u64() {
                    if stress > 9 {
                        return Some("Stress must be between 0 and 9".to_string());
                    }
                } else {
                    return Some("Stress must be a number".to_string());
                }
            }

            // Validate trauma count
            "TRAUMA_COLD" | "TRAUMA_HAUNTED" | "TRAUMA_OBSESSED" | "TRAUMA_PARANOID"
            | "TRAUMA_RECKLESS" | "TRAUMA_SOFT" | "TRAUMA_UNSTABLE" | "TRAUMA_VICIOUS" => {
                // Count current traumas
                let trauma_conditions = [
                    "TRAUMA_COLD",
                    "TRAUMA_HAUNTED",
                    "TRAUMA_OBSESSED",
                    "TRAUMA_PARANOID",
                    "TRAUMA_RECKLESS",
                    "TRAUMA_SOFT",
                    "TRAUMA_UNSTABLE",
                    "TRAUMA_VICIOUS",
                ];

                let current_count: usize = trauma_conditions
                    .iter()
                    .filter(|t| {
                        if **t == field_id {
                            value.as_bool().unwrap_or(false)
                        } else {
                            all_values
                                .get(&t.to_string())
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false)
                        }
                    })
                    .count();

                if current_count > 4 {
                    return Some("Maximum 4 traumas allowed".to_string());
                }
            }

            // Validate name
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

        // Actions default to 0
        for action in &[
            "HUNT", "STUDY", "SURVEY", "TINKER", "FINESSE", "PROWL", "SKIRMISH", "WRECK",
            "ATTUNE", "COMMAND", "CONSORT", "SWAY",
        ] {
            defaults.insert(action.to_string(), serde_json::json!(0));
        }

        // Stress defaults to 0
        defaults.insert("STRESS".to_string(), serde_json::json!(0));

        // Traumas default to false
        for trauma in &[
            "TRAUMA_COLD",
            "TRAUMA_HAUNTED",
            "TRAUMA_OBSESSED",
            "TRAUMA_PARANOID",
            "TRAUMA_RECKLESS",
            "TRAUMA_SOFT",
            "TRAUMA_UNSTABLE",
            "TRAUMA_VICIOUS",
        ] {
            defaults.insert(trauma.to_string(), serde_json::json!(false));
        }

        // Harm defaults to empty
        defaults.insert("HARM_LEVEL1_1".to_string(), serde_json::json!(""));
        defaults.insert("HARM_LEVEL1_2".to_string(), serde_json::json!(""));
        defaults.insert("HARM_LEVEL2_1".to_string(), serde_json::json!(""));
        defaults.insert("HARM_LEVEL2_2".to_string(), serde_json::json!(""));
        defaults.insert("HARM_LEVEL3".to_string(), serde_json::json!(""));

        // Armor defaults to false
        defaults.insert("ARMOR_STANDARD".to_string(), serde_json::json!(false));
        defaults.insert("ARMOR_HEAVY".to_string(), serde_json::json!(false));
        defaults.insert("ARMOR_SPECIAL".to_string(), serde_json::json!(false));

        // Load defaults to normal
        defaults.insert("LOAD_LEVEL".to_string(), serde_json::json!("normal"));

        // Healing clock defaults to 0
        defaults.insert("HEALING_CLOCK".to_string(), serde_json::json!(0));

        defaults
    }
}

// Helper methods for building the schema
impl BladesSystem {
    /// Create the standard Blades dot pool allocation for action ratings.
    ///
    /// Players distribute 4 dots across 12 actions, organized into 3 attributes.
    /// Each playbook also gives a starting action at rating 2.
    pub fn dot_pool_allocation() -> AllocationSystem {
        AllocationSystem::DotPool {
            total_dots: 4,
            max_per_field: 2, // During creation, max 2 dots in any action
            categories: vec![
                DotPoolCategory {
                    id: "insight".to_string(),
                    label: "Insight (information and understanding)".to_string(),
                    dots: 0, // No per-category limit, dots can go anywhere
                    fields: vec![
                        "HUNT".to_string(),
                        "STUDY".to_string(),
                        "SURVEY".to_string(),
                        "TINKER".to_string(),
                    ],
                },
                DotPoolCategory {
                    id: "prowess".to_string(),
                    label: "Prowess (physical capability and violence)".to_string(),
                    dots: 0,
                    fields: vec![
                        "FINESSE".to_string(),
                        "PROWL".to_string(),
                        "SKIRMISH".to_string(),
                        "WRECK".to_string(),
                    ],
                },
                DotPoolCategory {
                    id: "resolve".to_string(),
                    label: "Resolve (willpower and social interaction)".to_string(),
                    dots: 0,
                    fields: vec![
                        "ATTUNE".to_string(),
                        "COMMAND".to_string(),
                        "CONSORT".to_string(),
                        "SWAY".to_string(),
                    ],
                },
            ],
            starting_dots: vec![
                // Playbook starting actions - player picks playbook, gets one action at 2
                // UI filters these based on PLAYBOOK field value matching the source
                StartingDot {
                    field: "SKIRMISH".to_string(),
                    dots: 2,
                    source: "cutter".to_string(),
                },
                StartingDot {
                    field: "HUNT".to_string(),
                    dots: 2,
                    source: "hound".to_string(),
                },
                StartingDot {
                    field: "TINKER".to_string(),
                    dots: 2,
                    source: "leech".to_string(),
                },
                StartingDot {
                    field: "PROWL".to_string(),
                    dots: 2,
                    source: "lurk".to_string(),
                },
                StartingDot {
                    field: "SWAY".to_string(),
                    dots: 2,
                    source: "slide".to_string(),
                },
                StartingDot {
                    field: "STUDY".to_string(),
                    dots: 2,
                    source: "spider".to_string(),
                },
                StartingDot {
                    field: "ATTUNE".to_string(),
                    dots: 2,
                    source: "whisper".to_string(),
                },
            ],
        }
    }

    fn identity_section(&self) -> SchemaSection {
        SchemaSection {
            id: "identity".to_string(),
            label: "Identity".to_string(),
            section_type: SectionType::Identity,
            fields: vec![
                FieldDefinition {
                    id: "NAME".to_string(),
                    label: "Name".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(100),
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
                    placeholder: Some("Character name".to_string()),
                },
                FieldDefinition {
                    id: "ALIAS".to_string(),
                    label: "Alias".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(100),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: Some("Your street name or nickname".to_string()),
                    placeholder: Some("Alias".to_string()),
                },
                FieldDefinition {
                    id: "PLAYBOOK".to_string(),
                    label: "Playbook".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "cutter".to_string(),
                                label: "Cutter".to_string(),
                                description: Some("A dangerous and intimidating fighter".to_string()),
                            },
                            SchemaSelectOption {
                                value: "hound".to_string(),
                                label: "Hound".to_string(),
                                description: Some("A deadly sharpshooter and tracker".to_string()),
                            },
                            SchemaSelectOption {
                                value: "leech".to_string(),
                                label: "Leech".to_string(),
                                description: Some("A saboteur and technician".to_string()),
                            },
                            SchemaSelectOption {
                                value: "lurk".to_string(),
                                label: "Lurk".to_string(),
                                description: Some("A stealthy infiltrator and burglar".to_string()),
                            },
                            SchemaSelectOption {
                                value: "slide".to_string(),
                                label: "Slide".to_string(),
                                description: Some("A subtle manipulator and spy".to_string()),
                            },
                            SchemaSelectOption {
                                value: "spider".to_string(),
                                label: "Spider".to_string(),
                                description: Some("A devious mastermind".to_string()),
                            },
                            SchemaSelectOption {
                                value: "whisper".to_string(),
                                label: "Whisper".to_string(),
                                description: Some("An arcane adept and channeler".to_string()),
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
                        ..Default::default()
                    },
                    description: None,
                    placeholder: None,
                },
                FieldDefinition {
                    id: "HERITAGE".to_string(),
                    label: "Heritage".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "akoros".to_string(),
                                label: "Akoros".to_string(),
                                description: Some("Educated and industrialized".to_string()),
                            },
                            SchemaSelectOption {
                                value: "dagger_isles".to_string(),
                                label: "Dagger Isles".to_string(),
                                description: Some("Fierce and independent".to_string()),
                            },
                            SchemaSelectOption {
                                value: "iruvia".to_string(),
                                label: "Iruvia".to_string(),
                                description: Some("Rich, proud, and tradition-bound".to_string()),
                            },
                            SchemaSelectOption {
                                value: "severos".to_string(),
                                label: "Severos".to_string(),
                                description: Some("Hardened and rugged".to_string()),
                            },
                            SchemaSelectOption {
                                value: "skovlan".to_string(),
                                label: "Skovlan".to_string(),
                                description: Some("Tenacious and hardworking".to_string()),
                            },
                            SchemaSelectOption {
                                value: "tycheros".to_string(),
                                label: "Tycheros".to_string(),
                                description: Some("Alien and unfamiliar".to_string()),
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
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Where your family line is from".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "BACKGROUND".to_string(),
                    label: "Background".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "academic".to_string(),
                                label: "Academic".to_string(),
                                description: Some("A scholar, scientist, or professor".to_string()),
                            },
                            SchemaSelectOption {
                                value: "labor".to_string(),
                                label: "Labor".to_string(),
                                description: Some("A worker, tradesperson, or servant".to_string()),
                            },
                            SchemaSelectOption {
                                value: "law".to_string(),
                                label: "Law".to_string(),
                                description: Some("A lawyer, judge, or inspector".to_string()),
                            },
                            SchemaSelectOption {
                                value: "trade".to_string(),
                                label: "Trade".to_string(),
                                description: Some("A merchant, shopkeeper, or broker".to_string()),
                            },
                            SchemaSelectOption {
                                value: "military".to_string(),
                                label: "Military".to_string(),
                                description: Some("A soldier, mercenary, or officer".to_string()),
                            },
                            SchemaSelectOption {
                                value: "noble".to_string(),
                                label: "Noble".to_string(),
                                description: Some("An aristocrat, courtier, or heir".to_string()),
                            },
                            SchemaSelectOption {
                                value: "underworld".to_string(),
                                label: "Underworld".to_string(),
                                description: Some("A criminal, gang member, or fence".to_string()),
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
                    description: Some("Your previous life before becoming a scoundrel".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "VICE".to_string(),
                    label: "Vice".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "faith".to_string(),
                                label: "Faith".to_string(),
                                description: Some("Worship of a deity or forgotten god".to_string()),
                            },
                            SchemaSelectOption {
                                value: "gambling".to_string(),
                                label: "Gambling".to_string(),
                                description: Some("Games of chance and risk".to_string()),
                            },
                            SchemaSelectOption {
                                value: "luxury".to_string(),
                                label: "Luxury".to_string(),
                                description: Some("Expensive or extravagant pleasures".to_string()),
                            },
                            SchemaSelectOption {
                                value: "obligation".to_string(),
                                label: "Obligation".to_string(),
                                description: Some("Family, friends, or causes you support".to_string()),
                            },
                            SchemaSelectOption {
                                value: "pleasure".to_string(),
                                label: "Pleasure".to_string(),
                                description: Some("Hedonistic gratification".to_string()),
                            },
                            SchemaSelectOption {
                                value: "stupor".to_string(),
                                label: "Stupor".to_string(),
                                description: Some("Drugs, alcohol, or other intoxicants".to_string()),
                            },
                            SchemaSelectOption {
                                value: "weird".to_string(),
                                label: "Weird".to_string(),
                                description: Some("Occult or strange practices".to_string()),
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
                    description: Some("How you relieve stress".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "XP_TRIGGER".to_string(),
                    label: "XP Trigger".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["PLAYBOOK".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Earn XP when you...".to_string()),
                    placeholder: None,
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: None,
        }
    }

    fn attributes_actions_section(&self) -> SchemaSection {
        let action_field = |id: &str, label: &str, description: &str| FieldDefinition {
            id: id.to_string(),
            label: label.to_string(),
            field_type: SchemaFieldType::DicePool {
                max_dice: 4,
                die_type: 6,
            },
            editable: true,
            required: false,
            derived_from: None,
            validation: Some(FieldValidation {
                min: Some(0),
                max: Some(4),
                pattern: None,
                error_message: Some("Action ratings must be 0-4".to_string()),
            }),
            layout: FieldLayout {
                width: Some(3),
                ..Default::default()
            },
            description: Some(description.to_string()),
            placeholder: None,
        };

        let attribute_field = |id: &str, label: &str, deps: Vec<&str>| FieldDefinition {
            id: id.to_string(),
            label: label.to_string(),
            field_type: SchemaFieldType::Integer {
                min: Some(0),
                max: Some(4),
                show_modifier: false,
            },
            editable: false,
            required: false,
            derived_from: Some(DerivedField {
                derivation_type: DerivationType::Custom,
                dependencies: deps.into_iter().map(String::from).collect(),
                display_format: None,
            }),
            validation: None,
            layout: FieldLayout {
                width: Some(12),
                new_row: true,
                ..Default::default()
            },
            description: Some("Number of actions with 1+ dots in this attribute".to_string()),
            placeholder: None,
        };

        SchemaSection {
            id: "attributes_actions".to_string(),
            label: "Attributes & Actions".to_string(),
            section_type: SectionType::Skills,
            fields: vec![
                // Insight attribute header
                attribute_field("INSIGHT", "Insight", vec!["HUNT", "STUDY", "SURVEY", "TINKER"]),
                // Insight actions
                action_field("HUNT", "Hunt", "Track, ambush, attack from a distance"),
                action_field("STUDY", "Study", "Scrutinize, research, analyze"),
                action_field("SURVEY", "Survey", "Observe, search, gather information"),
                action_field("TINKER", "Tinker", "Build, repair, disable mechanisms"),
                // Prowess attribute header
                attribute_field("PROWESS", "Prowess", vec!["FINESSE", "PROWL", "SKIRMISH", "WRECK"]),
                // Prowess actions
                action_field("FINESSE", "Finesse", "Precise movement, sleight of hand"),
                action_field("PROWL", "Prowl", "Move quietly, hide, sneak"),
                action_field("SKIRMISH", "Skirmish", "Fight in close combat"),
                action_field("WRECK", "Wreck", "Destroy, smash, break things"),
                // Resolve attribute header
                attribute_field("RESOLVE", "Resolve", vec!["ATTUNE", "COMMAND", "CONSORT", "SWAY"]),
                // Resolve actions
                action_field("ATTUNE", "Attune", "Open mind to the ghost field, use arcane powers"),
                action_field("COMMAND", "Command", "Compel with authority, intimidate"),
                action_field("CONSORT", "Consort", "Socialize, make connections"),
                action_field("SWAY", "Sway", "Influence with guile, charm, argue"),
            ],
            collapsible: false,
            collapsed_default: false,
            description: Some("Your action ratings determine your dice pool. Attribute ratings are the count of actions with at least 1 dot.".to_string()),
        }
    }

    fn harm_section(&self) -> SchemaSection {
        SchemaSection {
            id: "harm".to_string(),
            label: "Harm".to_string(),
            section_type: SectionType::Combat,
            fields: vec![
                FieldDefinition {
                    id: "HARM_TRACK".to_string(),
                    label: "Harm".to_string(),
                    field_type: SchemaFieldType::ConditionTrack {
                        levels: vec![
                            ConditionLevel {
                                level: 1,
                                label: "Less Serious (2 slots)".to_string(),
                                effect: None,
                            },
                            ConditionLevel {
                                level: 2,
                                label: "Serious (2 slots)".to_string(),
                                effect: Some("-1d to related actions".to_string()),
                            },
                            ConditionLevel {
                                level: 3,
                                label: "Severe/Fatal (1 slot)".to_string(),
                                effect: Some("Need help to act, -1d stacks with level 2".to_string()),
                            },
                        ],
                    },
                    editable: false,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(12),
                        ..Default::default()
                    },
                    description: Some("Injuries reduce your effectiveness".to_string()),
                    placeholder: None,
                },
                // Level 3 harm (1 slot)
                FieldDefinition {
                    id: "HARM_LEVEL3".to_string(),
                    label: "Severe/Fatal".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(50),
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
                    description: Some("Level 3 harm - Need help to act".to_string()),
                    placeholder: Some("Broken leg, impaled, shot in the chest...".to_string()),
                },
                // Level 2 harm (2 slots)
                FieldDefinition {
                    id: "HARM_LEVEL2_1".to_string(),
                    label: "Serious".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(50),
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
                    description: Some("Level 2 harm - Reduces effect by 1".to_string()),
                    placeholder: Some("Slashed arm, burned hand...".to_string()),
                },
                FieldDefinition {
                    id: "HARM_LEVEL2_2".to_string(),
                    label: "Serious".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(50),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(6),
                        ..Default::default()
                    },
                    description: Some("Level 2 harm - Reduces effect by 1".to_string()),
                    placeholder: Some("Exhausted, concussed...".to_string()),
                },
                // Level 1 harm (2 slots)
                FieldDefinition {
                    id: "HARM_LEVEL1_1".to_string(),
                    label: "Less Serious".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(50),
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
                    description: Some("Level 1 harm - No mechanical effect".to_string()),
                    placeholder: Some("Bruised, tired...".to_string()),
                },
                FieldDefinition {
                    id: "HARM_LEVEL1_2".to_string(),
                    label: "Less Serious".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(50),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(6),
                        ..Default::default()
                    },
                    description: Some("Level 1 harm - No mechanical effect".to_string()),
                    placeholder: Some("Shaken, winded...".to_string()),
                },
                // Healing clock
                FieldDefinition {
                    id: "HEALING_CLOCK".to_string(),
                    label: "Healing Clock".to_string(),
                    field_type: SchemaFieldType::Clock {
                        segments: 4,
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
                    description: Some("When filled, reduce harm by one level".to_string()),
                    placeholder: None,
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: None,
        }
    }

    fn stress_trauma_section(&self) -> SchemaSection {
        SchemaSection {
            id: "stress_trauma".to_string(),
            label: "Stress & Trauma".to_string(),
            section_type: SectionType::Resources,
            fields: vec![
                FieldDefinition {
                    id: "STRESS".to_string(),
                    label: "Stress".to_string(),
                    field_type: SchemaFieldType::ResourceBar {
                        max_field: "MAX_STRESS".to_string(),
                        color: ResourceColor::Purple,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(0),
                        max: Some(9),
                        pattern: None,
                        error_message: Some("Stress must be 0-9".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(6),
                        ..Default::default()
                    },
                    description: Some("Spend stress to push yourself, assist, or resist consequences. At 9 stress, you trauma out.".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "MAX_STRESS".to_string(),
                    label: "Max Stress".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(9),
                        max: Some(9),
                        show_modifier: false,
                    },
                    editable: false,
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
                    id: "TRAUMA_COUNT".to_string(),
                    label: "Trauma".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: Some(4),
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec![
                            "TRAUMA_COLD".to_string(),
                            "TRAUMA_HAUNTED".to_string(),
                            "TRAUMA_OBSESSED".to_string(),
                            "TRAUMA_PARANOID".to_string(),
                            "TRAUMA_RECKLESS".to_string(),
                            "TRAUMA_SOFT".to_string(),
                            "TRAUMA_UNSTABLE".to_string(),
                            "TRAUMA_VICIOUS".to_string(),
                        ],
                        display_format: Some("{}/4".to_string()),
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: Some("At 4 trauma, your character retires".to_string()),
                    placeholder: None,
                },
                // Trauma conditions as checkboxes
                FieldDefinition {
                    id: "TRAUMA_COLD".to_string(),
                    label: "Cold".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Cold".to_string()),
                        unchecked_label: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(3),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("You're not moved by emotional appeals or danger".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "TRAUMA_HAUNTED".to_string(),
                    label: "Haunted".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Haunted".to_string()),
                        unchecked_label: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(3),
                        ..Default::default()
                    },
                    description: Some("You're often lost in reverie, reliving past horrors".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "TRAUMA_OBSESSED".to_string(),
                    label: "Obsessed".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Obsessed".to_string()),
                        unchecked_label: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(3),
                        ..Default::default()
                    },
                    description: Some("You're fixated on a particular goal or idea".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "TRAUMA_PARANOID".to_string(),
                    label: "Paranoid".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Paranoid".to_string()),
                        unchecked_label: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(3),
                        ..Default::default()
                    },
                    description: Some("You imagine threats everywhere and can't trust anyone".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "TRAUMA_RECKLESS".to_string(),
                    label: "Reckless".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Reckless".to_string()),
                        unchecked_label: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(3),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("You have little regard for your own safety or best interests".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "TRAUMA_SOFT".to_string(),
                    label: "Soft".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Soft".to_string()),
                        unchecked_label: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(3),
                        ..Default::default()
                    },
                    description: Some("You lose your edge, become sentimental and easily manipulated".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "TRAUMA_UNSTABLE".to_string(),
                    label: "Unstable".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Unstable".to_string()),
                        unchecked_label: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(3),
                        ..Default::default()
                    },
                    description: Some("Your moods and actions are erratic and unpredictable".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "TRAUMA_VICIOUS".to_string(),
                    label: "Vicious".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Vicious".to_string()),
                        unchecked_label: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(3),
                        ..Default::default()
                    },
                    description: Some("You seek revenge and enjoy inflicting pain and misery".to_string()),
                    placeholder: None,
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: None,
        }
    }

    fn load_armor_section(&self) -> SchemaSection {
        SchemaSection {
            id: "load_armor".to_string(),
            label: "Load & Armor".to_string(),
            section_type: SectionType::Inventory,
            fields: vec![
                FieldDefinition {
                    id: "LOAD_LEVEL".to_string(),
                    label: "Load".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "light".to_string(),
                                label: "Light (3)".to_string(),
                                description: Some("Quick and quiet, 3 load".to_string()),
                            },
                            SchemaSelectOption {
                                value: "normal".to_string(),
                                label: "Normal (5)".to_string(),
                                description: Some("Standard load, 5 load".to_string()),
                            },
                            SchemaSelectOption {
                                value: "heavy".to_string(),
                                label: "Heavy (6)".to_string(),
                                description: Some("Slow and noisy, 6 load, -1d to Prowess".to_string()),
                            },
                        ],
                        allow_custom: false,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: Some("Determines how much gear you can carry".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "MAX_LOAD".to_string(),
                    label: "Max Load".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(3),
                        max: Some(6),
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["LOAD_LEVEL".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: None,
                },
                FieldDefinition {
                    id: "ARMOR_STANDARD".to_string(),
                    label: "Armor".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Armor (worn)".to_string()),
                        unchecked_label: Some("Armor (available)".to_string()),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Negate harm from an attack (1 load)".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "ARMOR_HEAVY".to_string(),
                    label: "Heavy".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Heavy (used)".to_string()),
                        unchecked_label: Some("Heavy (available)".to_string()),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Additional armor use (+1 load)".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "ARMOR_SPECIAL".to_string(),
                    label: "Special".to_string(),
                    field_type: SchemaFieldType::Boolean {
                        checked_label: Some("Special (used)".to_string()),
                        unchecked_label: Some("Special (available)".to_string()),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("From playbook abilities".to_string()),
                    placeholder: None,
                },
            ],
            collapsible: true,
            collapsed_default: false,
            description: Some("Choose your load before each score. Mark armor when used.".to_string()),
        }
    }

    fn special_abilities_section(&self) -> SchemaSection {
        SchemaSection {
            id: "special_abilities".to_string(),
            label: "Special Abilities".to_string(),
            section_type: SectionType::Features,
            fields: vec![
                FieldDefinition {
                    id: "SPECIAL_ABILITIES".to_string(),
                    label: "Special Abilities".to_string(),
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
                    description: Some("Your playbook special abilities".to_string()),
                    placeholder: Some("Enter your special abilities...".to_string()),
                },
                FieldDefinition {
                    id: "ITEMS".to_string(),
                    label: "Items".to_string(),
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
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Your carried items and their load cost".to_string()),
                    placeholder: Some("Fine pistol (1), blade (1), throwing knives (1)...".to_string()),
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
                        "Active effects modifying your rolls (Harm penalties, Devil's Bargains, etc.)".to_string(),
                    ),
                    placeholder: None,
                },
            ],
            collapsible: true,
            collapsed_default: false,
            description: Some("Harm penalties: Level 1 = narrative only, Level 2 = -1d to related actions, Level 3 = -1d stacking and need help to act.".to_string()),
        }
    }
}

/// Progress clock structure.
#[derive(Debug, Clone)]
pub struct ProgressClock {
    pub name: String,
    pub segments: u8,
    pub filled: u8,
}

impl ProgressClock {
    pub fn new(name: impl Into<String>, segments: u8) -> Self {
        Self {
            name: name.into(),
            segments,
            filled: 0,
        }
    }

    pub fn tick(&mut self, amount: u8) {
        self.filled = (self.filled + amount).min(self.segments);
    }

    pub fn is_complete(&self) -> bool {
        self.filled >= self.segments
    }

    pub fn remaining(&self) -> u8 {
        self.segments.saturating_sub(self.filled)
    }
}

/// Playbook types in Blades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Playbook {
    Cutter,    // Fighter
    Hound,     // Sharpshooter
    Leech,     // Technician
    Lurk,      // Infiltrator
    Slide,     // Manipulator
    Spider,    // Mastermind
    Whisper,   // Channeler
}

impl Playbook {
    pub fn starting_action(&self) -> (&'static str, u8) {
        match self {
            Playbook::Cutter => ("Skirmish", 2),
            Playbook::Hound => ("Hunt", 2),
            Playbook::Leech => ("Wreck", 2),
            Playbook::Lurk => ("Prowl", 2),
            Playbook::Slide => ("Sway", 2),
            Playbook::Spider => ("Study", 2),
            Playbook::Whisper => ("Attune", 2),
        }
    }

    pub fn xp_trigger(&self) -> &'static str {
        match self {
            Playbook::Cutter => "Address challenges with violence or coercion",
            Playbook::Hound => "Address challenges with tracking or violence",
            Playbook::Leech => "Address challenges with technical skill or mayhem",
            Playbook::Lurk => "Address challenges with stealth or evasion",
            Playbook::Slide => "Address challenges with deception or influence",
            Playbook::Spider => "Address challenges with calculation or conspiracy",
            Playbook::Whisper => "Address challenges with knowledge or arcane power",
        }
    }
}

/// Crew types in Blades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrewType {
    Assassins,
    Bravos,
    Cult,
    Hawkers,
    Shadows,
    Smugglers,
}

impl CrewType {
    pub fn hunting_grounds(&self) -> &'static str {
        match self {
            CrewType::Assassins => "Killings",
            CrewType::Bravos => "Extortion, sabotage",
            CrewType::Cult => "Occult operations",
            CrewType::Hawkers => "Product sales",
            CrewType::Shadows => "Burglary, espionage",
            CrewType::Smugglers => "Smuggling routes",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_from_dice() {
        // Critical (multiple 6s)
        assert_eq!(BladesOutcome::from_dice(&[6, 6, 3]), BladesOutcome::Critical);

        // Success (single 6)
        assert_eq!(BladesOutcome::from_dice(&[6, 3, 2]), BladesOutcome::Success);

        // Partial (4-5)
        assert_eq!(BladesOutcome::from_dice(&[5, 3, 2]), BladesOutcome::PartialSuccess);
        assert_eq!(BladesOutcome::from_dice(&[4, 2, 1]), BladesOutcome::PartialSuccess);

        // Failure (1-3)
        assert_eq!(BladesOutcome::from_dice(&[3, 2, 1]), BladesOutcome::Failure);
    }

    #[test]
    fn effect_level_clock_ticks() {
        assert_eq!(EffectLevel::Zero.clock_ticks(), 0);
        assert_eq!(EffectLevel::Limited.clock_ticks(), 1);
        assert_eq!(EffectLevel::Standard.clock_ticks(), 2);
        assert_eq!(EffectLevel::Great.clock_ticks(), 3);
        assert_eq!(EffectLevel::Extreme.clock_ticks(), 4);
    }

    #[test]
    fn effect_level_changes() {
        assert_eq!(EffectLevel::Standard.increase(), EffectLevel::Great);
        assert_eq!(EffectLevel::Standard.decrease(), EffectLevel::Limited);
        assert_eq!(EffectLevel::Extreme.increase(), EffectLevel::Extreme);
        assert_eq!(EffectLevel::Zero.decrease(), EffectLevel::Zero);
    }

    #[test]
    fn attribute_ratings() {
        assert_eq!(BladesSystem::insight_rating(1, 2, 1, 0), 4);
        assert_eq!(BladesSystem::prowess_rating(2, 3, 1, 0), 6);
        assert_eq!(BladesSystem::resolve_rating(1, 0, 1, 2), 4);
    }

    #[test]
    fn resistance_roll_cost() {
        assert_eq!(BladesSystem::resistance_roll_cost(0), 6);
        assert_eq!(BladesSystem::resistance_roll_cost(1), 5);
        assert_eq!(BladesSystem::resistance_roll_cost(3), 3);
        assert_eq!(BladesSystem::resistance_roll_cost(6), 0);
    }

    #[test]
    fn load_levels() {
        assert_eq!(LoadLevel::Light.max_items(), 3);
        assert_eq!(LoadLevel::Normal.max_items(), 5);
        assert_eq!(LoadLevel::Heavy.max_items(), 6);
        assert_eq!(LoadLevel::Heavy.prowess_penalty(), 1);
    }

    #[test]
    fn progress_clock() {
        let mut clock = ProgressClock::new("Pick Lock", 6);
        assert_eq!(clock.remaining(), 6);

        clock.tick(2);
        assert_eq!(clock.filled, 2);
        assert!(!clock.is_complete());

        clock.tick(10); // Should cap at segments
        assert_eq!(clock.filled, 6);
        assert!(clock.is_complete());
    }

    #[test]
    fn playbook_info() {
        assert_eq!(Playbook::Lurk.starting_action(), ("Prowl", 2));
        assert!(Playbook::Cutter
            .xp_trigger()
            .contains("violence or coercion"));
    }

    #[test]
    fn system_identification() {
        let system = BladesSystem::new();
        assert_eq!(system.system_id(), "blades");
        assert_eq!(system.display_name(), "Blades in the Dark");
    }

    #[test]
    fn character_sheet_schema_structure() {
        let system = BladesSystem::new();
        let schema = system.character_sheet_schema();

        assert_eq!(schema.system_id, "blades");
        assert_eq!(schema.system_name, "Blades in the Dark");
        assert_eq!(schema.sections.len(), 7);

        // Verify section IDs
        let section_ids: Vec<&str> = schema.sections.iter().map(|s| s.id.as_str()).collect();
        assert!(section_ids.contains(&"identity"));
        assert!(section_ids.contains(&"attributes_actions"));
        assert!(section_ids.contains(&"harm"));
        assert!(section_ids.contains(&"stress_trauma"));
        assert!(section_ids.contains(&"load_armor"));
        assert!(section_ids.contains(&"special_abilities"));

        // Verify creation steps
        assert_eq!(schema.creation_steps.len(), 4);
    }

    #[test]
    fn character_sheet_playbook_options() {
        let system = BladesSystem::new();
        let schema = system.character_sheet_schema();

        let identity = schema.sections.iter().find(|s| s.id == "identity").unwrap();
        let playbook_field = identity.fields.iter().find(|f| f.id == "PLAYBOOK").unwrap();

        if let SchemaFieldType::Select { options, .. } = &playbook_field.field_type {
            assert_eq!(options.len(), 7);
            let values: Vec<&str> = options.iter().map(|o| o.value.as_str()).collect();
            assert!(values.contains(&"cutter"));
            assert!(values.contains(&"hound"));
            assert!(values.contains(&"leech"));
            assert!(values.contains(&"lurk"));
            assert!(values.contains(&"slide"));
            assert!(values.contains(&"spider"));
            assert!(values.contains(&"whisper"));
        } else {
            panic!("PLAYBOOK should be a Select field");
        }
    }

    #[test]
    fn character_sheet_action_fields() {
        let system = BladesSystem::new();
        let schema = system.character_sheet_schema();

        let actions = schema
            .sections
            .iter()
            .find(|s| s.id == "attributes_actions")
            .unwrap();

        // Check that all 12 actions are present
        let action_ids = ["HUNT", "STUDY", "SURVEY", "TINKER", "FINESSE", "PROWL",
            "SKIRMISH", "WRECK", "ATTUNE", "COMMAND", "CONSORT", "SWAY"];

        for action in &action_ids {
            let field = actions.fields.iter().find(|f| f.id == *action);
            assert!(field.is_some(), "Action {} should exist", action);

            if let Some(f) = field {
                if let SchemaFieldType::DicePool { max_dice, die_type } = f.field_type {
                    assert_eq!(max_dice, 4);
                    assert_eq!(die_type, 6);
                } else {
                    panic!("{} should be a DicePool field", action);
                }
            }
        }
    }

    #[test]
    fn character_sheet_trauma_conditions() {
        let system = BladesSystem::new();
        let schema = system.character_sheet_schema();

        let stress_trauma = schema
            .sections
            .iter()
            .find(|s| s.id == "stress_trauma")
            .unwrap();

        let trauma_ids = ["TRAUMA_COLD", "TRAUMA_HAUNTED", "TRAUMA_OBSESSED",
            "TRAUMA_PARANOID", "TRAUMA_RECKLESS", "TRAUMA_SOFT",
            "TRAUMA_UNSTABLE", "TRAUMA_VICIOUS"];

        for trauma in &trauma_ids {
            let field = stress_trauma.fields.iter().find(|f| f.id == *trauma);
            assert!(field.is_some(), "Trauma {} should exist", trauma);

            if let Some(f) = field {
                assert!(matches!(f.field_type, SchemaFieldType::Boolean { .. }));
            }
        }
    }

    #[test]
    fn calculate_derived_attribute_ratings() {
        let system = BladesSystem::new();

        let mut values = HashMap::new();
        // Set some Insight actions
        values.insert("HUNT".to_string(), serde_json::json!(2));
        values.insert("STUDY".to_string(), serde_json::json!(1));
        values.insert("SURVEY".to_string(), serde_json::json!(0));
        values.insert("TINKER".to_string(), serde_json::json!(0));
        // Set some Prowess actions
        values.insert("FINESSE".to_string(), serde_json::json!(1));
        values.insert("PROWL".to_string(), serde_json::json!(2));
        values.insert("SKIRMISH".to_string(), serde_json::json!(1));
        values.insert("WRECK".to_string(), serde_json::json!(0));
        // Set some Resolve actions
        values.insert("ATTUNE".to_string(), serde_json::json!(0));
        values.insert("COMMAND".to_string(), serde_json::json!(1));
        values.insert("CONSORT".to_string(), serde_json::json!(0));
        values.insert("SWAY".to_string(), serde_json::json!(0));

        let derived = system.calculate_derived_values(&values);

        // Insight: 2 actions with dots (HUNT, STUDY)
        assert_eq!(derived.get("INSIGHT").unwrap(), &serde_json::json!(2));
        // Prowess: 3 actions with dots (FINESSE, PROWL, SKIRMISH)
        assert_eq!(derived.get("PROWESS").unwrap(), &serde_json::json!(3));
        // Resolve: 1 action with dots (COMMAND)
        assert_eq!(derived.get("RESOLVE").unwrap(), &serde_json::json!(1));
    }

    #[test]
    fn calculate_derived_trauma_count() {
        let system = BladesSystem::new();

        let mut values = HashMap::new();
        values.insert("TRAUMA_COLD".to_string(), serde_json::json!(true));
        values.insert("TRAUMA_HAUNTED".to_string(), serde_json::json!(false));
        values.insert("TRAUMA_OBSESSED".to_string(), serde_json::json!(true));
        values.insert("TRAUMA_PARANOID".to_string(), serde_json::json!(false));
        values.insert("TRAUMA_RECKLESS".to_string(), serde_json::json!(false));
        values.insert("TRAUMA_SOFT".to_string(), serde_json::json!(false));
        values.insert("TRAUMA_UNSTABLE".to_string(), serde_json::json!(true));
        values.insert("TRAUMA_VICIOUS".to_string(), serde_json::json!(false));

        let derived = system.calculate_derived_values(&values);

        assert_eq!(derived.get("TRAUMA_COUNT").unwrap(), &serde_json::json!(3));
    }

    #[test]
    fn calculate_derived_xp_trigger() {
        let system = BladesSystem::new();

        let mut values = HashMap::new();
        values.insert("PLAYBOOK".to_string(), serde_json::json!("lurk"));

        let derived = system.calculate_derived_values(&values);

        assert_eq!(
            derived.get("XP_TRIGGER").unwrap(),
            &serde_json::json!("Address challenges with stealth or evasion")
        );
    }

    #[test]
    fn calculate_derived_max_load() {
        let system = BladesSystem::new();

        let mut values = HashMap::new();
        values.insert("LOAD_LEVEL".to_string(), serde_json::json!("heavy"));

        let derived = system.calculate_derived_values(&values);

        assert_eq!(derived.get("MAX_LOAD").unwrap(), &serde_json::json!(6));
    }

    #[test]
    fn validate_action_rating() {
        let system = BladesSystem::new();
        let values = HashMap::new();

        // Valid rating
        assert!(system.validate_field("HUNT", &serde_json::json!(3), &values).is_none());

        // Invalid rating (too high)
        assert!(system.validate_field("HUNT", &serde_json::json!(5), &values).is_some());

        // Invalid type
        assert!(system.validate_field("HUNT", &serde_json::json!("two"), &values).is_some());
    }

    #[test]
    fn validate_stress() {
        let system = BladesSystem::new();
        let values = HashMap::new();

        // Valid stress
        assert!(system.validate_field("STRESS", &serde_json::json!(5), &values).is_none());

        // Invalid stress (too high)
        assert!(system.validate_field("STRESS", &serde_json::json!(10), &values).is_some());
    }

    #[test]
    fn validate_trauma_limit() {
        let system = BladesSystem::new();

        let mut values = HashMap::new();
        values.insert("TRAUMA_COLD".to_string(), serde_json::json!(true));
        values.insert("TRAUMA_HAUNTED".to_string(), serde_json::json!(true));
        values.insert("TRAUMA_OBSESSED".to_string(), serde_json::json!(true));
        values.insert("TRAUMA_PARANOID".to_string(), serde_json::json!(true));

        // Adding a 5th trauma should fail
        let result = system.validate_field("TRAUMA_RECKLESS", &serde_json::json!(true), &values);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Maximum 4 traumas"));
    }

    #[test]
    fn default_values() {
        let system = BladesSystem::new();
        let defaults = system.default_values();

        // Actions default to 0
        assert_eq!(defaults.get("HUNT").unwrap(), &serde_json::json!(0));
        assert_eq!(defaults.get("SWAY").unwrap(), &serde_json::json!(0));

        // Stress defaults to 0
        assert_eq!(defaults.get("STRESS").unwrap(), &serde_json::json!(0));

        // Traumas default to false
        assert_eq!(defaults.get("TRAUMA_COLD").unwrap(), &serde_json::json!(false));

        // Load defaults to normal
        assert_eq!(defaults.get("LOAD_LEVEL").unwrap(), &serde_json::json!("normal"));

        // Armor defaults to false
        assert_eq!(defaults.get("ARMOR_STANDARD").unwrap(), &serde_json::json!(false));
    }
}
