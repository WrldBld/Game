//! Call of Cthulhu 7th Edition game system implementation.
//!
//! CoC 7e uses a percentile (d100) roll-under system.
//! Key features:
//! - Eight characteristics (STR, CON, SIZ, DEX, APP, INT, POW, EDU)
//! - Skills as percentile values
//! - Three success tiers: Regular, Hard (half), Extreme (fifth)
//! - Sanity system
//! - Luck as a spendable resource

use super::traits::{
    AllocationSystem, CalculationEngine, CharacterSheetProvider, CharacterSheetSchema,
    CreationStep, DerivationType, DerivedField, FieldDefinition, FieldLayout, FieldValidation,
    GameSystem, PercentileCategory, ProficiencyLevel, ResourceColor, SchemaFieldType,
    SchemaSection, SchemaSelectOption, SectionType,
};
use crate::entities::{StatBlock, StatModifier};
use std::collections::HashMap;

/// Success levels for CoC 7e skill checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SuccessLevel {
    /// Roll of 01 - always succeeds, exceptional outcome
    Critical,
    /// Roll <= skill / 5
    Extreme,
    /// Roll <= skill / 2
    Hard,
    /// Roll <= skill
    Regular,
    /// Roll > skill but not a fumble
    Failure,
    /// 96-100 if skill < 50, or 100 if skill >= 50
    Fumble,
}

impl SuccessLevel {
    /// Check if this is any form of success.
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            SuccessLevel::Critical
                | SuccessLevel::Extreme
                | SuccessLevel::Hard
                | SuccessLevel::Regular
        )
    }
}

/// Determine success level for a CoC 7e roll.
pub fn check_success(roll: u8, skill: u8) -> SuccessLevel {
    // Critical: roll of 01
    if roll == 1 {
        return SuccessLevel::Critical;
    }

    // Fumble check
    if is_fumble(roll, skill) {
        return SuccessLevel::Fumble;
    }

    // Calculate thresholds
    let hard = skill / 2;
    let extreme = skill / 5;

    if roll <= extreme {
        SuccessLevel::Extreme
    } else if roll <= hard {
        SuccessLevel::Hard
    } else if roll <= skill {
        SuccessLevel::Regular
    } else {
        SuccessLevel::Failure
    }
}

/// Check if a roll is a fumble.
pub fn is_fumble(roll: u8, skill: u8) -> bool {
    if skill < 50 {
        roll >= 96
    } else {
        roll == 100
    }
}

/// Check if a roll is a critical (01).
pub fn is_critical(roll: u8) -> bool {
    roll == 1
}

/// Sanity check result.
#[derive(Debug, Clone)]
pub struct SanityCheckResult {
    pub passed: bool,
    pub loss: u8,
    pub bout_of_madness: bool,
}

/// Perform a sanity check.
///
/// # Arguments
/// * `roll` - The d100 roll
/// * `current_sanity` - Current sanity value
/// * `pass_loss` - Sanity lost on success (e.g., "0" or "1d3")
/// * `fail_loss` - Sanity lost on failure (e.g., "1d6")
/// * `actual_loss` - The actual loss after rolling (pre-computed)
pub fn sanity_check(roll: u8, current_sanity: u8, actual_loss: u8) -> SanityCheckResult {
    let passed = roll <= current_sanity;
    let bout_of_madness = actual_loss >= 5;

    SanityCheckResult {
        passed,
        loss: actual_loss,
        bout_of_madness,
    }
}

/// Call of Cthulhu 7th Edition game system.
pub struct Coc7eSystem {
    stat_names: Vec<&'static str>,
    skill_names: Vec<&'static str>,
}

impl Coc7eSystem {
    pub fn new() -> Self {
        Self {
            stat_names: vec!["STR", "CON", "SIZ", "DEX", "APP", "INT", "POW", "EDU"],
            skill_names: vec![
                // Combat
                "Dodge",
                "Fighting (Brawl)",
                "Firearms (Handgun)",
                "Firearms (Rifle/Shotgun)",
                "Throw",
                // Investigation
                "Appraise",
                "Library Use",
                "Listen",
                "Spot Hidden",
                "Track",
                // Social
                "Charm",
                "Fast Talk",
                "Intimidate",
                "Persuade",
                "Psychology",
                // Knowledge
                "Accounting",
                "Anthropology",
                "Archaeology",
                "Art/Craft",
                "Cthulhu Mythos",
                "History",
                "Language (Other)",
                "Language (Own)",
                "Law",
                "Medicine",
                "Natural World",
                "Navigate",
                "Occult",
                "Science",
                // Practical
                "Climb",
                "Drive Auto",
                "Electrical Repair",
                "First Aid",
                "Jump",
                "Locksmith",
                "Mechanical Repair",
                "Operate Heavy Machinery",
                "Pilot",
                "Ride",
                "Sleight of Hand",
                "Stealth",
                "Survival",
                "Swim",
            ],
        }
    }

    /// Calculate derived HP from CON and SIZ.
    pub fn calculate_hp(con: u8, siz: u8) -> u8 {
        (con as u16 + siz as u16) as u8 / 10
    }

    /// Calculate starting sanity from POW.
    pub fn calculate_starting_sanity(pow: u8) -> u8 {
        pow
    }

    /// Calculate magic points from POW.
    pub fn calculate_magic_points(pow: u8) -> u8 {
        pow / 5
    }

    /// Calculate move rate from STR, DEX, SIZ.
    pub fn calculate_move_rate(str_val: u8, dex: u8, siz: u8) -> u8 {
        if dex < siz && str_val < siz {
            7
        } else if str_val > siz && dex > siz {
            9
        } else {
            8
        }
    }

    /// Calculate damage bonus from STR + SIZ.
    pub fn calculate_damage_bonus(str_val: u8, siz: u8) -> &'static str {
        let total = str_val as u16 + siz as u16;
        match total {
            2..=64 => "-2",
            65..=84 => "-1",
            85..=124 => "None",
            125..=164 => "+1d4",
            165..=204 => "+1d6",
            205..=284 => "+2d6",
            285..=364 => "+3d6",
            _ => "+4d6",
        }
    }

    /// Calculate build from STR + SIZ.
    pub fn calculate_build(str_val: u8, siz: u8) -> i8 {
        let total = str_val as u16 + siz as u16;
        match total {
            2..=64 => -2,
            65..=84 => -1,
            85..=124 => 0,
            125..=164 => 1,
            165..=204 => 2,
            205..=284 => 3,
            285..=364 => 4,
            _ => 5,
        }
    }
}

impl Default for Coc7eSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSystem for Coc7eSystem {
    fn system_id(&self) -> &str {
        "coc7e"
    }

    fn display_name(&self) -> &str {
        "Call of Cthulhu 7th Edition"
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

impl CalculationEngine for Coc7eSystem {
    fn ability_modifier(&self, score: i32) -> i32 {
        // CoC doesn't use modifiers like D&D - skills are percentile
        // Return the score itself as the "modifier" (the percentage chance)
        score
    }

    fn proficiency_bonus(&self, _level: u8) -> i32 {
        // CoC has no proficiency bonus system
        0
    }

    fn spell_save_dc(&self, stats: &StatBlock, _casting_stat: &str) -> i32 {
        // CoC uses POW for magic resistance
        // Spells are resisted on resistance table
        stats.get_stat("POW").unwrap_or(50)
    }

    fn spell_attack_bonus(&self, stats: &StatBlock, _casting_stat: &str) -> i32 {
        // CoC magic uses POW vs target's POW on resistance table
        stats.get_stat("POW").unwrap_or(50)
    }

    fn attack_bonus(&self, stats: &StatBlock, _attack_stat: &str, _proficient: bool) -> i32 {
        // CoC uses skill percentages directly
        // Return the Fighting skill or Firearms skill
        stats.get_stat("Fighting (Brawl)").unwrap_or(25)
    }

    fn stack_modifiers(&self, modifiers: &[StatModifier]) -> i32 {
        // CoC modifiers typically stack (they're flat bonuses/penalties)
        modifiers.iter().filter(|m| m.active).map(|m| m.value).sum()
    }

    fn calculate_ac(
        &self,
        _stats: &StatBlock,
        _armor_ac: Option<i32>,
        _shield_bonus: Option<i32>,
        _allows_dex: bool,
        _max_dex_bonus: Option<i32>,
    ) -> i32 {
        // CoC doesn't have AC - combat uses opposed rolls
        // Return 0 as a placeholder
        0
    }

    fn skill_modifier(
        &self,
        stats: &StatBlock,
        skill: &str,
        _proficiency_level: ProficiencyLevel,
    ) -> i32 {
        // In CoC, skill values ARE the percentile chance
        // Return the skill value directly
        stats.get_stat(skill).unwrap_or_else(|| {
            // Return default base values for skills
            get_skill_base(skill)
        })
    }

    fn saving_throw_modifier(&self, stats: &StatBlock, ability: &str, _proficient: bool) -> i32 {
        // CoC uses characteristic rolls directly
        stats.get_stat(ability).unwrap_or(50)
    }

    fn passive_perception(&self, stats: &StatBlock, _proficiency_level: ProficiencyLevel) -> i32 {
        // CoC uses Spot Hidden skill
        stats.get_stat("Spot Hidden").unwrap_or(25)
    }

    fn hit_die(&self, _class_name: &str) -> u8 {
        // CoC doesn't use hit dice - HP is derived from CON + SIZ
        0
    }

    fn calculate_max_hp(
        &self,
        _level: u8,
        _class_name: &str,
        _constitution_modifier: i32,
        _additional_hp: i32,
    ) -> i32 {
        // HP should be calculated using calculate_hp(con, siz)
        // This method signature doesn't fit CoC well
        // Return a placeholder
        12
    }
}

impl CharacterSheetProvider for Coc7eSystem {
    fn character_sheet_schema(&self) -> CharacterSheetSchema {
        CharacterSheetSchema {
            system_id: "coc7e".to_string(),
            system_name: "Call of Cthulhu 7th Edition".to_string(),
            sections: vec![
                self.identity_section(),
                self.characteristics_section(),
                self.derived_attributes_section(),
                self.skills_section(),
                self.combat_section(),
                self.resources_section(),
                self.modifiers_section(),
            ],
            creation_steps: vec![
                CreationStep {
                    id: "identity".to_string(),
                    label: "Investigator Info".to_string(),
                    description: "Define your investigator's basic information.".to_string(),
                    section_ids: vec!["identity".to_string()],
                    order: 1,
                    required: true,
                    allocation: None,
                },
                CreationStep {
                    id: "characteristics".to_string(),
                    label: "Characteristics".to_string(),
                    description: "Roll or assign your eight characteristics (3d6*5 or 2d6+6*5)."
                        .to_string(),
                    section_ids: vec!["characteristics".to_string()],
                    order: 2,
                    required: true,
                    allocation: Some(Self::rolling_allocation()),
                },
                CreationStep {
                    id: "derived".to_string(),
                    label: "Derived Attributes".to_string(),
                    description: "Calculate derived values from characteristics.".to_string(),
                    section_ids: vec!["derived_attributes".to_string()],
                    order: 3,
                    required: true,
                    allocation: None,
                },
                CreationStep {
                    id: "skills".to_string(),
                    label: "Skills".to_string(),
                    description: "Allocate occupation and personal interest skill points."
                        .to_string(),
                    section_ids: vec!["skills".to_string(), "combat".to_string()],
                    order: 4,
                    required: true,
                    allocation: Some(Self::skill_point_allocation()),
                },
            ],
        }
    }

    fn calculate_derived_values(
        &self,
        values: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut derived = HashMap::new();

        // Get characteristics
        let str_val = values.get("STR").and_then(|v| v.as_i64()).unwrap_or(50) as u8;
        let con = values.get("CON").and_then(|v| v.as_i64()).unwrap_or(50) as u8;
        let siz = values.get("SIZ").and_then(|v| v.as_i64()).unwrap_or(50) as u8;
        let dex = values.get("DEX").and_then(|v| v.as_i64()).unwrap_or(50) as u8;
        let pow = values.get("POW").and_then(|v| v.as_i64()).unwrap_or(50) as u8;
        let edu = values.get("EDU").and_then(|v| v.as_i64()).unwrap_or(50) as u8;

        // Calculate HP = (CON + SIZ) / 10
        let max_hp = Self::calculate_hp(con, siz);
        derived.insert("MAX_HP".to_string(), serde_json::json!(max_hp));

        // Calculate Sanity = POW (starting value)
        let max_sanity = Self::calculate_starting_sanity(pow);
        derived.insert("MAX_SANITY".to_string(), serde_json::json!(max_sanity));

        // Calculate Magic Points = POW / 5
        let max_mp = Self::calculate_magic_points(pow);
        derived.insert("MAX_MP".to_string(), serde_json::json!(max_mp));

        // Calculate Move Rate
        let move_rate = Self::calculate_move_rate(str_val, dex, siz);
        derived.insert("MOVE".to_string(), serde_json::json!(move_rate));

        // Calculate Build
        let build = Self::calculate_build(str_val, siz);
        derived.insert("BUILD".to_string(), serde_json::json!(build));

        // Calculate Damage Bonus
        let damage_bonus = Self::calculate_damage_bonus(str_val, siz);
        derived.insert("DAMAGE_BONUS".to_string(), serde_json::json!(damage_bonus));

        // Calculate half and fifth values for characteristics
        for (id, val) in [
            ("STR", str_val),
            ("CON", con),
            ("SIZ", siz),
            ("DEX", dex),
            ("POW", pow),
            ("EDU", edu),
        ] {
            derived.insert(format!("{}_HALF", id), serde_json::json!(val / 2));
            derived.insert(format!("{}_FIFTH", id), serde_json::json!(val / 5));
        }

        // Also calculate APP and INT halves/fifths
        let app = values.get("APP").and_then(|v| v.as_i64()).unwrap_or(50) as u8;
        let int = values.get("INT").and_then(|v| v.as_i64()).unwrap_or(50) as u8;

        derived.insert("APP_HALF".to_string(), serde_json::json!(app / 2));
        derived.insert("APP_FIFTH".to_string(), serde_json::json!(app / 5));
        derived.insert("INT_HALF".to_string(), serde_json::json!(int / 2));
        derived.insert("INT_FIFTH".to_string(), serde_json::json!(int / 5));

        // Calculate Dodge base value (DEX / 2)
        let dodge_base = dex / 2;
        derived.insert("DODGE_BASE".to_string(), serde_json::json!(dodge_base));

        // Calculate Language (Own) base value (EDU)
        derived.insert("LANGUAGE_OWN_BASE".to_string(), serde_json::json!(edu));

        // Calculate half/fifth values for skills that have values set
        let skill_ids = self.get_all_skill_ids();
        for skill_id in skill_ids {
            if let Some(val) = values.get(&skill_id).and_then(|v| v.as_i64()) {
                let val = val as u8;
                derived.insert(format!("{}_HALF", skill_id), serde_json::json!(val / 2));
                derived.insert(format!("{}_FIFTH", skill_id), serde_json::json!(val / 5));
            }
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
            // Characteristics: 0-99 percentile
            "STR" | "CON" | "SIZ" | "DEX" | "APP" | "INT" | "POW" | "EDU" => {
                if let Some(score) = value.as_i64() {
                    if !(0..=99).contains(&score) {
                        return Some("Characteristics must be between 0 and 99".to_string());
                    }
                } else {
                    return Some("Characteristic must be a number".to_string());
                }
            }
            // Skills: 0-99 percentile (except Cthulhu Mythos which is special)
            _ if field_id.ends_with("_SKILL") || self.is_skill_field(field_id) => {
                if let Some(score) = value.as_i64() {
                    if !(0..=99).contains(&score) {
                        return Some("Skills must be between 0 and 99".to_string());
                    }
                } else {
                    return Some("Skill value must be a number".to_string());
                }
            }
            // Luck: 0-99
            "LUCK" | "CURRENT_LUCK" => {
                if let Some(luck) = value.as_i64() {
                    if !(0..=99).contains(&luck) {
                        return Some("Luck must be between 0 and 99".to_string());
                    }
                } else {
                    return Some("Luck must be a number".to_string());
                }
            }
            // Sanity: 0-99
            "CURRENT_SANITY" => {
                if let Some(san) = value.as_i64() {
                    if !(0..=99).contains(&san) {
                        return Some("Sanity must be between 0 and 99".to_string());
                    }
                } else {
                    return Some("Sanity must be a number".to_string());
                }
            }
            // Age: reasonable range
            "AGE" => {
                if let Some(age) = value.as_i64() {
                    if !(15..=90).contains(&age) {
                        return Some("Age must be between 15 and 90".to_string());
                    }
                } else {
                    return Some("Age must be a number".to_string());
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

        // Default characteristics (average human)
        defaults.insert("STR".to_string(), serde_json::json!(50));
        defaults.insert("CON".to_string(), serde_json::json!(50));
        defaults.insert("SIZ".to_string(), serde_json::json!(50));
        defaults.insert("DEX".to_string(), serde_json::json!(50));
        defaults.insert("APP".to_string(), serde_json::json!(50));
        defaults.insert("INT".to_string(), serde_json::json!(50));
        defaults.insert("POW".to_string(), serde_json::json!(50));
        defaults.insert("EDU".to_string(), serde_json::json!(50));

        // Luck starts at 3d6*5 average (52-53)
        defaults.insert("LUCK".to_string(), serde_json::json!(50));
        defaults.insert("CURRENT_LUCK".to_string(), serde_json::json!(50));

        // Current resources start at max
        defaults.insert("CURRENT_HP".to_string(), serde_json::json!(10));
        defaults.insert("CURRENT_SANITY".to_string(), serde_json::json!(50));
        defaults.insert("CURRENT_MP".to_string(), serde_json::json!(10));

        // Default skill values (base values)
        defaults.insert("DODGE".to_string(), serde_json::json!(25)); // DEX/2 base
        defaults.insert("FIGHTING_BRAWL".to_string(), serde_json::json!(25));
        defaults.insert("FIREARMS_HANDGUN".to_string(), serde_json::json!(20));
        defaults.insert("FIREARMS_RIFLE".to_string(), serde_json::json!(25));
        defaults.insert("FIRST_AID".to_string(), serde_json::json!(30));
        defaults.insert("LIBRARY_USE".to_string(), serde_json::json!(20));
        defaults.insert("LISTEN".to_string(), serde_json::json!(20));
        defaults.insert("SPOT_HIDDEN".to_string(), serde_json::json!(25));
        defaults.insert("STEALTH".to_string(), serde_json::json!(20));
        defaults.insert("PSYCHOLOGY".to_string(), serde_json::json!(10));
        defaults.insert("PERSUADE".to_string(), serde_json::json!(10));
        defaults.insert("FAST_TALK".to_string(), serde_json::json!(5));
        defaults.insert("CHARM".to_string(), serde_json::json!(15));
        defaults.insert("INTIMIDATE".to_string(), serde_json::json!(15));
        defaults.insert("CREDIT_RATING".to_string(), serde_json::json!(0));
        defaults.insert("CTHULHU_MYTHOS".to_string(), serde_json::json!(0));
        defaults.insert("OCCULT".to_string(), serde_json::json!(5));

        defaults
    }
}

// Helper methods for building the schema
impl Coc7eSystem {
    /// Create the Call of Cthulhu 7e rolling allocation system for characteristics.
    ///
    /// CoC 7e uses different formulas for different characteristics:
    /// - STR, CON, DEX, APP, POW: Roll 3d6*5 (range 15-90)
    /// - SIZ, INT, EDU: Roll (2d6+6)*5 (range 40-90)
    ///
    /// Players can also use point-buy as an alternative.
    pub fn rolling_allocation() -> AllocationSystem {
        AllocationSystem::DiceRoll {
            formula: "mixed".to_string(), // Different formulas per stat
            description: "Roll 3d6×5 for STR, CON, DEX, APP, POW; Roll (2d6+6)×5 for SIZ, INT, EDU"
                .to_string(),
            roll_count: 8,
            target_fields: vec![
                "STR".to_string(),
                "CON".to_string(),
                "SIZ".to_string(),
                "DEX".to_string(),
                "APP".to_string(),
                "INT".to_string(),
                "POW".to_string(),
                "EDU".to_string(),
            ],
            allow_reroll: true,
            minimum_total: None, // CoC doesn't have a minimum total requirement
        }
    }

    /// Create the Call of Cthulhu 7e skill point allocation system.
    ///
    /// Investigators receive skill points from two sources:
    /// - Occupation skills: Points equal to EDU×4 (or other combinations depending on occupation)
    /// - Personal interest skills: Points equal to INT×2
    ///
    /// Points can be distributed among skills with a maximum of 99 per skill
    /// (except Cthulhu Mythos which cannot receive points).
    pub fn skill_point_allocation() -> AllocationSystem {
        AllocationSystem::PercentilePool {
            total_points: 0, // Calculated from EDU×4 + INT×2 (varies by character)
            min_per_field: 0,
            max_per_field: 99,
            categories: vec![
                PercentileCategory {
                    id: "occupation".to_string(),
                    label: "Occupation Skills (EDU×4)".to_string(),
                    points: 0, // Calculated from EDU×4
                    fields: vec![], // Determined by occupation choice
                    formula: Some("EDU*4".to_string()),
                },
                PercentileCategory {
                    id: "personal_interest".to_string(),
                    label: "Personal Interest (INT×2)".to_string(),
                    points: 0, // Calculated from INT×2
                    fields: vec![], // Any skills except Cthulhu Mythos
                    formula: Some("INT*2".to_string()),
                },
            ],
        }
    }

    /// Get the available allocation systems for CoC 7e.
    /// Returns rolling method and optional point-buy alternative.
    pub fn allocation_systems() -> Vec<(&'static str, &'static str, AllocationSystem)> {
        vec![
            (
                "rolling",
                "Standard Rolling (Recommended)",
                Self::rolling_allocation(),
            ),
            (
                "quick_fire",
                "Quick-Fire Method (Point Assignment)",
                AllocationSystem::FreeAllocation {
                    total_points: 460, // Average total from rolling
                    min_per_field: 15,
                    max_per_field: 90,
                    target_fields: vec![
                        "STR".to_string(),
                        "CON".to_string(),
                        "SIZ".to_string(),
                        "DEX".to_string(),
                        "APP".to_string(),
                        "INT".to_string(),
                        "POW".to_string(),
                        "EDU".to_string(),
                    ],
                },
            ),
        ]
    }

    fn identity_section(&self) -> SchemaSection {
        SchemaSection {
            id: "identity".to_string(),
            label: "Investigator Identity".to_string(),
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
                        width: Some(6),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Enter investigator name".to_string()),
                },
                FieldDefinition {
                    id: "OCCUPATION".to_string(),
                    label: "Occupation".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "antiquarian".to_string(),
                                label: "Antiquarian".to_string(),
                                description: Some("Collector and dealer of antiques".to_string()),
                            },
                            SchemaSelectOption {
                                value: "author".to_string(),
                                label: "Author".to_string(),
                                description: Some("Writer of fiction or non-fiction".to_string()),
                            },
                            SchemaSelectOption {
                                value: "detective".to_string(),
                                label: "Private Detective".to_string(),
                                description: Some("Private investigator for hire".to_string()),
                            },
                            SchemaSelectOption {
                                value: "dilettante".to_string(),
                                label: "Dilettante".to_string(),
                                description: Some(
                                    "Wealthy amateur with varied interests".to_string(),
                                ),
                            },
                            SchemaSelectOption {
                                value: "doctor".to_string(),
                                label: "Doctor of Medicine".to_string(),
                                description: Some("Licensed physician".to_string()),
                            },
                            SchemaSelectOption {
                                value: "journalist".to_string(),
                                label: "Journalist".to_string(),
                                description: Some("Reporter or editor for news media".to_string()),
                            },
                            SchemaSelectOption {
                                value: "lawyer".to_string(),
                                label: "Lawyer".to_string(),
                                description: Some("Attorney or legal counsel".to_string()),
                            },
                            SchemaSelectOption {
                                value: "librarian".to_string(),
                                label: "Librarian".to_string(),
                                description: Some("Keeper of books and knowledge".to_string()),
                            },
                            SchemaSelectOption {
                                value: "parapsychologist".to_string(),
                                label: "Parapsychologist".to_string(),
                                description: Some("Researcher of paranormal phenomena".to_string()),
                            },
                            SchemaSelectOption {
                                value: "police_detective".to_string(),
                                label: "Police Detective".to_string(),
                                description: Some(
                                    "Official law enforcement investigator".to_string(),
                                ),
                            },
                            SchemaSelectOption {
                                value: "professor".to_string(),
                                label: "Professor".to_string(),
                                description: Some("Academic expert and teacher".to_string()),
                            },
                            SchemaSelectOption {
                                value: "soldier".to_string(),
                                label: "Soldier".to_string(),
                                description: Some("Military personnel".to_string()),
                            },
                        ],
                        allow_custom: true,
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(6),
                        ..Default::default()
                    },
                    description: Some(
                        "Determines skill points and Credit Rating range".to_string(),
                    ),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "AGE".to_string(),
                    label: "Age".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(15),
                        max: Some(90),
                        show_modifier: false,
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(15),
                        max: Some(90),
                        pattern: None,
                        error_message: Some("Age must be between 15 and 90".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(2),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Age affects EDU and physical characteristics".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "RESIDENCE".to_string(),
                    label: "Residence".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(100),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(5),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Where the investigator lives".to_string()),
                },
                FieldDefinition {
                    id: "BIRTHPLACE".to_string(),
                    label: "Birthplace".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(100),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(5),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Where the investigator was born".to_string()),
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: None,
        }
    }

    fn characteristics_section(&self) -> SchemaSection {
        let characteristics = [
            ("STR", "Strength", "Physical power (3d6*5)"),
            ("CON", "Constitution", "Health and resilience (3d6*5)"),
            ("SIZ", "Size", "Physical mass (2d6+6*5)"),
            ("DEX", "Dexterity", "Agility and coordination (3d6*5)"),
            ("APP", "Appearance", "Physical attractiveness (3d6*5)"),
            ("INT", "Intelligence", "Learning and reasoning (2d6+6*5)"),
            ("POW", "Power", "Willpower and magical potential (3d6*5)"),
            ("EDU", "Education", "Formal and life knowledge (2d6+6*5)"),
        ];

        let fields: Vec<FieldDefinition> = characteristics
            .iter()
            .map(|(id, label, description)| FieldDefinition {
                id: id.to_string(),
                label: label.to_string(),
                field_type: SchemaFieldType::PercentileSkill { show_derived: true },
                editable: true,
                required: true,
                derived_from: None,
                validation: Some(FieldValidation {
                    min: Some(0),
                    max: Some(99),
                    pattern: None,
                    error_message: Some("Characteristics must be 0-99".to_string()),
                }),
                layout: FieldLayout {
                    width: Some(3),
                    ..Default::default()
                },
                description: Some(description.to_string()),
                placeholder: None,
            })
            .collect();

        SchemaSection {
            id: "characteristics".to_string(),
            label: "Characteristics".to_string(),
            section_type: SectionType::AbilityScores,
            fields,
            collapsible: false,
            collapsed_default: false,
            description: Some(
                "Eight core characteristics as percentile values. Shows Regular/Half/Fifth."
                    .to_string(),
            ),
        }
    }

    fn derived_attributes_section(&self) -> SchemaSection {
        SchemaSection {
            id: "derived_attributes".to_string(),
            label: "Derived Attributes".to_string(),
            section_type: SectionType::Combat,
            fields: vec![
                FieldDefinition {
                    id: "MAX_HP".to_string(),
                    label: "Hit Points".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(1),
                        max: None,
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["CON".to_string(), "SIZ".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("(CON + SIZ) / 10".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "MAX_SANITY".to_string(),
                    label: "Sanity".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: Some(99),
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["POW".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Starting value equals POW".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "MAX_MP".to_string(),
                    label: "Magic Points".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: None,
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Fifth,
                        dependencies: vec!["POW".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("POW / 5".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "LUCK".to_string(),
                    label: "Luck".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: Some(99),
                        show_modifier: false,
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(0),
                        max: Some(99),
                        pattern: None,
                        error_message: Some("Luck must be 0-99".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Roll 3d6*5. Expendable resource.".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "MOVE".to_string(),
                    label: "Move Rate".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: None,
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["STR".to_string(), "DEX".to_string(), "SIZ".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Based on STR, DEX, SIZ comparison".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "BUILD".to_string(),
                    label: "Build".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: None,
                        max: None,
                        show_modifier: true,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["STR".to_string(), "SIZ".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Based on STR + SIZ".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "DAMAGE_BONUS".to_string(),
                    label: "Damage Bonus".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(10),
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["STR".to_string(), "SIZ".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Based on STR + SIZ".to_string()),
                    placeholder: None,
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: Some("Values calculated from characteristics".to_string()),
        }
    }

    fn skills_section(&self) -> SchemaSection {
        // Define skills with their base values
        let skills = [
            ("ACCOUNTING", "Accounting", 5),
            ("ANTHROPOLOGY", "Anthropology", 1),
            ("ARCHAEOLOGY", "Archaeology", 1),
            ("ART_CRAFT", "Art/Craft", 5),
            ("CHARM", "Charm", 15),
            ("CLIMB", "Climb", 20),
            ("CREDIT_RATING", "Credit Rating", 0),
            ("CTHULHU_MYTHOS", "Cthulhu Mythos", 0),
            ("DISGUISE", "Disguise", 5),
            ("DRIVE_AUTO", "Drive Auto", 20),
            ("ELECTRICAL_REPAIR", "Electrical Repair", 10),
            ("FAST_TALK", "Fast Talk", 5),
            ("FIRST_AID", "First Aid", 30),
            ("HISTORY", "History", 5),
            ("INTIMIDATE", "Intimidate", 15),
            ("JUMP", "Jump", 20),
            ("LANGUAGE_OWN", "Language (Own)", 0), // EDU as base, handled specially
            ("LANGUAGE_OTHER", "Language (Other)", 1),
            ("LAW", "Law", 5),
            ("LIBRARY_USE", "Library Use", 20),
            ("LISTEN", "Listen", 20),
            ("LOCKSMITH", "Locksmith", 1),
            ("MECHANICAL_REPAIR", "Mechanical Repair", 10),
            ("MEDICINE", "Medicine", 1),
            ("NATURAL_WORLD", "Natural World", 10),
            ("NAVIGATE", "Navigate", 10),
            ("OCCULT", "Occult", 5),
            ("OPERATE_HEAVY_MACHINERY", "Operate Heavy Machinery", 1),
            ("PERSUADE", "Persuade", 10),
            ("PILOT", "Pilot", 1),
            ("PSYCHOLOGY", "Psychology", 10),
            ("PSYCHOANALYSIS", "Psychoanalysis", 1),
            ("RIDE", "Ride", 5),
            ("SCIENCE", "Science", 1),
            ("SLEIGHT_OF_HAND", "Sleight of Hand", 10),
            ("SPOT_HIDDEN", "Spot Hidden", 25),
            ("STEALTH", "Stealth", 20),
            ("SURVIVAL", "Survival", 10),
            ("SWIM", "Swim", 20),
            ("THROW", "Throw", 20),
            ("TRACK", "Track", 10),
        ];

        let fields: Vec<FieldDefinition> = skills
            .iter()
            .map(|(id, label, base)| {
                let description = if *id == "CTHULHU_MYTHOS" {
                    Some(
                        "Cannot be raised normally. Gains from reading forbidden tomes."
                            .to_string(),
                    )
                } else if *id == "LANGUAGE_OWN" {
                    Some("Base equals EDU".to_string())
                } else {
                    Some(format!("Base: {}%", base))
                };

                FieldDefinition {
                    id: id.to_string(),
                    label: label.to_string(),
                    field_type: SchemaFieldType::PercentileSkill { show_derived: true },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(0),
                        max: Some(99),
                        pattern: None,
                        error_message: Some("Skills must be 0-99".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description,
                    placeholder: Some(format!("{}", base)),
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
            description: Some(
                "Percentile skills. Shows Regular/Half/Fifth values for difficulty levels."
                    .to_string(),
            ),
        }
    }

    fn combat_section(&self) -> SchemaSection {
        SchemaSection {
            id: "combat".to_string(),
            label: "Combat Skills".to_string(),
            section_type: SectionType::Combat,
            fields: vec![
                FieldDefinition {
                    id: "DODGE".to_string(),
                    label: "Dodge".to_string(),
                    field_type: SchemaFieldType::PercentileSkill { show_derived: true },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(0),
                        max: Some(99),
                        pattern: None,
                        error_message: Some("Skills must be 0-99".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: Some("Base: DEX/2".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "FIGHTING_BRAWL".to_string(),
                    label: "Fighting (Brawl)".to_string(),
                    field_type: SchemaFieldType::PercentileSkill { show_derived: true },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(0),
                        max: Some(99),
                        pattern: None,
                        error_message: Some("Skills must be 0-99".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: Some("Base: 25%".to_string()),
                    placeholder: Some("25".to_string()),
                },
                FieldDefinition {
                    id: "FIREARMS_HANDGUN".to_string(),
                    label: "Firearms (Handgun)".to_string(),
                    field_type: SchemaFieldType::PercentileSkill { show_derived: true },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(0),
                        max: Some(99),
                        pattern: None,
                        error_message: Some("Skills must be 0-99".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: Some("Base: 20%".to_string()),
                    placeholder: Some("20".to_string()),
                },
                FieldDefinition {
                    id: "FIREARMS_RIFLE".to_string(),
                    label: "Firearms (Rifle/Shotgun)".to_string(),
                    field_type: SchemaFieldType::PercentileSkill { show_derived: true },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(0),
                        max: Some(99),
                        pattern: None,
                        error_message: Some("Skills must be 0-99".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: Some("Base: 25%".to_string()),
                    placeholder: Some("25".to_string()),
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: Some("Combat-related skills".to_string()),
        }
    }

    fn resources_section(&self) -> SchemaSection {
        SchemaSection {
            id: "resources".to_string(),
            label: "Current Resources".to_string(),
            section_type: SectionType::Resources,
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
                        width: Some(6),
                        ..Default::default()
                    },
                    description: None,
                    placeholder: None,
                },
                FieldDefinition {
                    id: "CURRENT_SANITY".to_string(),
                    label: "Current Sanity".to_string(),
                    field_type: SchemaFieldType::ResourceBar {
                        max_field: "MAX_SANITY".to_string(),
                        color: ResourceColor::Blue,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(6),
                        ..Default::default()
                    },
                    description: Some(
                        "Sanity loss leads to temporary and indefinite insanity".to_string(),
                    ),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "CURRENT_MP".to_string(),
                    label: "Current Magic Points".to_string(),
                    field_type: SchemaFieldType::ResourceBar {
                        max_field: "MAX_MP".to_string(),
                        color: ResourceColor::Purple,
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
                    description: Some("Regenerates at rate of 1 per day".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "CURRENT_LUCK".to_string(),
                    label: "Current Luck".to_string(),
                    field_type: SchemaFieldType::ResourceBar {
                        max_field: "LUCK".to_string(),
                        color: ResourceColor::Green,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(6),
                        ..Default::default()
                    },
                    description: Some(
                        "Spend to adjust roll results. Does not regenerate.".to_string(),
                    ),
                    placeholder: None,
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: Some("Track current values of expendable resources".to_string()),
        }
    }

    /// Get all skill field IDs for derived value calculations.
    fn get_all_skill_ids(&self) -> Vec<String> {
        vec![
            "ACCOUNTING".to_string(),
            "ANTHROPOLOGY".to_string(),
            "ARCHAEOLOGY".to_string(),
            "ART_CRAFT".to_string(),
            "CHARM".to_string(),
            "CLIMB".to_string(),
            "CREDIT_RATING".to_string(),
            "CTHULHU_MYTHOS".to_string(),
            "DISGUISE".to_string(),
            "DODGE".to_string(),
            "DRIVE_AUTO".to_string(),
            "ELECTRICAL_REPAIR".to_string(),
            "FAST_TALK".to_string(),
            "FIGHTING_BRAWL".to_string(),
            "FIREARMS_HANDGUN".to_string(),
            "FIREARMS_RIFLE".to_string(),
            "FIRST_AID".to_string(),
            "HISTORY".to_string(),
            "INTIMIDATE".to_string(),
            "JUMP".to_string(),
            "LANGUAGE_OWN".to_string(),
            "LANGUAGE_OTHER".to_string(),
            "LAW".to_string(),
            "LIBRARY_USE".to_string(),
            "LISTEN".to_string(),
            "LOCKSMITH".to_string(),
            "MECHANICAL_REPAIR".to_string(),
            "MEDICINE".to_string(),
            "NATURAL_WORLD".to_string(),
            "NAVIGATE".to_string(),
            "OCCULT".to_string(),
            "OPERATE_HEAVY_MACHINERY".to_string(),
            "PERSUADE".to_string(),
            "PILOT".to_string(),
            "PSYCHOLOGY".to_string(),
            "PSYCHOANALYSIS".to_string(),
            "RIDE".to_string(),
            "SCIENCE".to_string(),
            "SLEIGHT_OF_HAND".to_string(),
            "SPOT_HIDDEN".to_string(),
            "STEALTH".to_string(),
            "SURVIVAL".to_string(),
            "SWIM".to_string(),
            "THROW".to_string(),
            "TRACK".to_string(),
        ]
    }

    /// Check if a field ID corresponds to a skill.
    fn is_skill_field(&self, field_id: &str) -> bool {
        self.get_all_skill_ids().contains(&field_id.to_string())
    }

    fn modifiers_section(&self) -> SchemaSection {
        SchemaSection {
            id: "modifiers".to_string(),
            label: "Status & Conditions".to_string(),
            section_type: SectionType::Modifiers,
            fields: vec![
                FieldDefinition {
                    id: "ACTIVE_MODIFIERS".to_string(),
                    label: "Active Conditions".to_string(),
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
                        "Active conditions affecting your investigator (injuries, temporary insanity, phobias, etc.)".to_string(),
                    ),
                    placeholder: None,
                },
            ],
            collapsible: true,
            collapsed_default: false,
            description: Some("Track injuries, bouts of madness, phobias, manias, and other conditions affecting skill checks.".to_string()),
        }
    }
}

/// Get the base value for a skill in CoC 7e.
pub fn get_skill_base(skill: &str) -> i32 {
    match skill.to_lowercase().as_str() {
        // Combat
        "dodge" => 0, // DEX/2, calculated separately
        "fighting (brawl)" => 25,
        "firearms (handgun)" => 20,
        "firearms (rifle/shotgun)" => 25,
        "throw" => 20,
        // Investigation
        "appraise" => 5,
        "library use" => 20,
        "listen" => 20,
        "spot hidden" => 25,
        "track" => 10,
        // Social
        "charm" => 15,
        "fast talk" => 5,
        "intimidate" => 15,
        "persuade" => 10,
        "psychology" => 10,
        // Knowledge
        "accounting" => 5,
        "anthropology" => 1,
        "archaeology" => 1,
        "cthulhu mythos" => 0, // Special: cannot be raised normally
        "history" => 5,
        "law" => 5,
        "medicine" => 1,
        "natural world" => 10,
        "navigate" => 10,
        "occult" => 5,
        // Practical
        "climb" => 20,
        "drive auto" => 20,
        "electrical repair" => 10,
        "first aid" => 30,
        "jump" => 20,
        "locksmith" => 1,
        "mechanical repair" => 10,
        "operate heavy machinery" => 1,
        "ride" => 5,
        "sleight of hand" => 10,
        "stealth" => 20,
        "swim" => 20,
        // Art/Craft and Language have variable bases
        _ if skill.to_lowercase().starts_with("art/craft") => 5,
        _ if skill.to_lowercase().starts_with("language (own)") => 0, // EDU
        _ if skill.to_lowercase().starts_with("language") => 1,
        _ if skill.to_lowercase().starts_with("pilot") => 1,
        _ if skill.to_lowercase().starts_with("science") => 1,
        _ if skill.to_lowercase().starts_with("survival") => 10,
        _ => 1, // Default for specialized skills
    }
}

/// Credit Rating to lifestyle mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifestyle {
    Penniless,
    Poor,
    Average,
    Affluent,
    Wealthy,
    SuperRich,
}

impl Lifestyle {
    pub fn from_credit_rating(rating: u8) -> Self {
        match rating {
            0 => Lifestyle::Penniless,
            1..=9 => Lifestyle::Poor,
            10..=49 => Lifestyle::Average,
            50..=89 => Lifestyle::Affluent,
            90..=98 => Lifestyle::Wealthy,
            _ => Lifestyle::SuperRich,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_level_determination() {
        // Critical on 01
        assert_eq!(check_success(1, 50), SuccessLevel::Critical);

        // Extreme success (roll <= skill/5)
        assert_eq!(check_success(10, 50), SuccessLevel::Extreme); // 10 <= 10

        // Hard success (roll <= skill/2)
        assert_eq!(check_success(20, 50), SuccessLevel::Hard); // 20 <= 25

        // Regular success (roll <= skill)
        assert_eq!(check_success(45, 50), SuccessLevel::Regular);

        // Failure
        assert_eq!(check_success(60, 50), SuccessLevel::Failure);

        // Fumble (skill < 50, roll >= 96)
        assert_eq!(check_success(96, 40), SuccessLevel::Fumble);

        // Fumble (skill >= 50, roll == 100)
        assert_eq!(check_success(100, 60), SuccessLevel::Fumble);

        // Not fumble (skill >= 50, roll 96-99)
        assert_eq!(check_success(96, 60), SuccessLevel::Failure);
    }

    #[test]
    fn hp_calculation() {
        assert_eq!(Coc7eSystem::calculate_hp(60, 65), 12); // (60+65)/10 = 12
        assert_eq!(Coc7eSystem::calculate_hp(50, 50), 10);
    }

    #[test]
    fn magic_points_calculation() {
        assert_eq!(Coc7eSystem::calculate_magic_points(50), 10);
        assert_eq!(Coc7eSystem::calculate_magic_points(65), 13);
    }

    #[test]
    fn move_rate_calculation() {
        // Both DEX and STR < SIZ
        assert_eq!(Coc7eSystem::calculate_move_rate(40, 40, 60), 7);

        // Both DEX and STR > SIZ
        assert_eq!(Coc7eSystem::calculate_move_rate(60, 60, 40), 9);

        // Mixed
        assert_eq!(Coc7eSystem::calculate_move_rate(50, 50, 50), 8);
    }

    #[test]
    fn damage_bonus_calculation() {
        assert_eq!(Coc7eSystem::calculate_damage_bonus(40, 40), "-1"); // 80
        assert_eq!(Coc7eSystem::calculate_damage_bonus(50, 50), "None"); // 100
        assert_eq!(Coc7eSystem::calculate_damage_bonus(70, 70), "+1d4"); // 140
    }

    #[test]
    fn build_calculation() {
        assert_eq!(Coc7eSystem::calculate_build(40, 40), -1); // 80
        assert_eq!(Coc7eSystem::calculate_build(50, 50), 0); // 100
        assert_eq!(Coc7eSystem::calculate_build(70, 70), 1); // 140
    }

    #[test]
    fn skill_base_values() {
        assert_eq!(get_skill_base("Spot Hidden"), 25);
        assert_eq!(get_skill_base("First Aid"), 30);
        assert_eq!(get_skill_base("Cthulhu Mythos"), 0);
        assert_eq!(get_skill_base("Medicine"), 1);
    }

    #[test]
    fn lifestyle_from_credit_rating() {
        assert_eq!(Lifestyle::from_credit_rating(0), Lifestyle::Penniless);
        assert_eq!(Lifestyle::from_credit_rating(5), Lifestyle::Poor);
        assert_eq!(Lifestyle::from_credit_rating(30), Lifestyle::Average);
        assert_eq!(Lifestyle::from_credit_rating(60), Lifestyle::Affluent);
        assert_eq!(Lifestyle::from_credit_rating(95), Lifestyle::Wealthy);
        assert_eq!(Lifestyle::from_credit_rating(99), Lifestyle::SuperRich);
    }

    #[test]
    fn system_identification() {
        let system = Coc7eSystem::new();
        assert_eq!(system.system_id(), "coc7e");
        assert_eq!(system.display_name(), "Call of Cthulhu 7th Edition");
    }

    // CharacterSheetProvider tests

    #[test]
    fn character_sheet_schema_structure() {
        let system = Coc7eSystem::new();
        let schema = system.character_sheet_schema();

        assert_eq!(schema.system_id, "coc7e");
        assert_eq!(schema.system_name, "Call of Cthulhu 7th Edition");
        assert_eq!(schema.sections.len(), 7);

        // Verify section IDs
        let section_ids: Vec<&str> = schema.sections.iter().map(|s| s.id.as_str()).collect();
        assert!(section_ids.contains(&"identity"));
        assert!(section_ids.contains(&"characteristics"));
        assert!(section_ids.contains(&"derived_attributes"));
        assert!(section_ids.contains(&"skills"));
        assert!(section_ids.contains(&"combat"));
        assert!(section_ids.contains(&"resources"));
    }

    #[test]
    fn character_sheet_creation_steps() {
        let system = Coc7eSystem::new();
        let schema = system.character_sheet_schema();

        assert_eq!(schema.creation_steps.len(), 4);

        let step_ids: Vec<&str> = schema
            .creation_steps
            .iter()
            .map(|s| s.id.as_str())
            .collect();
        assert!(step_ids.contains(&"identity"));
        assert!(step_ids.contains(&"characteristics"));
        assert!(step_ids.contains(&"derived"));
        assert!(step_ids.contains(&"skills"));
    }

    #[test]
    fn characteristics_section_has_all_eight() {
        let system = Coc7eSystem::new();
        let schema = system.character_sheet_schema();

        let char_section = schema
            .sections
            .iter()
            .find(|s| s.id == "characteristics")
            .unwrap();

        assert_eq!(char_section.fields.len(), 8);

        let char_ids: Vec<&str> = char_section.fields.iter().map(|f| f.id.as_str()).collect();
        assert!(char_ids.contains(&"STR"));
        assert!(char_ids.contains(&"CON"));
        assert!(char_ids.contains(&"SIZ"));
        assert!(char_ids.contains(&"DEX"));
        assert!(char_ids.contains(&"APP"));
        assert!(char_ids.contains(&"INT"));
        assert!(char_ids.contains(&"POW"));
        assert!(char_ids.contains(&"EDU"));
    }

    #[test]
    fn derived_values_calculation() {
        let system = Coc7eSystem::new();
        let mut values = HashMap::new();

        // Set characteristics
        values.insert("STR".to_string(), serde_json::json!(60));
        values.insert("CON".to_string(), serde_json::json!(65));
        values.insert("SIZ".to_string(), serde_json::json!(55));
        values.insert("DEX".to_string(), serde_json::json!(50));
        values.insert("APP".to_string(), serde_json::json!(45));
        values.insert("INT".to_string(), serde_json::json!(70));
        values.insert("POW".to_string(), serde_json::json!(55));
        values.insert("EDU".to_string(), serde_json::json!(60));

        let derived = system.calculate_derived_values(&values);

        // HP = (CON + SIZ) / 10 = (65 + 55) / 10 = 12
        assert_eq!(derived.get("MAX_HP").unwrap().as_i64().unwrap(), 12);

        // Sanity = POW = 55
        assert_eq!(derived.get("MAX_SANITY").unwrap().as_i64().unwrap(), 55);

        // MP = POW / 5 = 55 / 5 = 11
        assert_eq!(derived.get("MAX_MP").unwrap().as_i64().unwrap(), 11);

        // Move = 8 (STR > SIZ but DEX < SIZ, so mixed)
        assert_eq!(derived.get("MOVE").unwrap().as_i64().unwrap(), 8);

        // Build = 0 (STR + SIZ = 115, in 85-124 range)
        assert_eq!(derived.get("BUILD").unwrap().as_i64().unwrap(), 0);

        // Damage Bonus = "None" (STR + SIZ = 115)
        assert_eq!(
            derived.get("DAMAGE_BONUS").unwrap().as_str().unwrap(),
            "None"
        );

        // Half/Fifth values for STR
        assert_eq!(derived.get("STR_HALF").unwrap().as_i64().unwrap(), 30);
        assert_eq!(derived.get("STR_FIFTH").unwrap().as_i64().unwrap(), 12);
    }

    #[test]
    fn skill_half_fifth_values() {
        let system = Coc7eSystem::new();
        let mut values = HashMap::new();

        values.insert("SPOT_HIDDEN".to_string(), serde_json::json!(60));
        values.insert("LIBRARY_USE".to_string(), serde_json::json!(45));

        let derived = system.calculate_derived_values(&values);

        // Spot Hidden: 60 -> half=30, fifth=12
        assert_eq!(
            derived.get("SPOT_HIDDEN_HALF").unwrap().as_i64().unwrap(),
            30
        );
        assert_eq!(
            derived.get("SPOT_HIDDEN_FIFTH").unwrap().as_i64().unwrap(),
            12
        );

        // Library Use: 45 -> half=22, fifth=9
        assert_eq!(
            derived.get("LIBRARY_USE_HALF").unwrap().as_i64().unwrap(),
            22
        );
        assert_eq!(
            derived.get("LIBRARY_USE_FIFTH").unwrap().as_i64().unwrap(),
            9
        );
    }

    #[test]
    fn field_validation_characteristics() {
        let system = Coc7eSystem::new();
        let all_values = HashMap::new();

        // Valid characteristic
        assert!(system
            .validate_field("STR", &serde_json::json!(50), &all_values)
            .is_none());

        // Invalid: too high
        assert!(system
            .validate_field("STR", &serde_json::json!(100), &all_values)
            .is_some());

        // Invalid: negative
        assert!(system
            .validate_field("CON", &serde_json::json!(-5), &all_values)
            .is_some());

        // Invalid: not a number
        assert!(system
            .validate_field("DEX", &serde_json::json!("fifty"), &all_values)
            .is_some());
    }

    #[test]
    fn field_validation_skills() {
        let system = Coc7eSystem::new();
        let all_values = HashMap::new();

        // Valid skill
        assert!(system
            .validate_field("SPOT_HIDDEN", &serde_json::json!(50), &all_values)
            .is_none());

        // Invalid: too high
        assert!(system
            .validate_field("LIBRARY_USE", &serde_json::json!(100), &all_values)
            .is_some());
    }

    #[test]
    fn field_validation_sanity_and_luck() {
        let system = Coc7eSystem::new();
        let all_values = HashMap::new();

        // Valid sanity
        assert!(system
            .validate_field("CURRENT_SANITY", &serde_json::json!(45), &all_values)
            .is_none());

        // Invalid sanity
        assert!(system
            .validate_field("CURRENT_SANITY", &serde_json::json!(100), &all_values)
            .is_some());

        // Valid luck
        assert!(system
            .validate_field("LUCK", &serde_json::json!(55), &all_values)
            .is_none());

        // Invalid luck
        assert!(system
            .validate_field("CURRENT_LUCK", &serde_json::json!(-1), &all_values)
            .is_some());
    }

    #[test]
    fn default_values_set() {
        let system = Coc7eSystem::new();
        let defaults = system.default_values();

        // Characteristics default to 50
        assert_eq!(defaults.get("STR").unwrap().as_i64().unwrap(), 50);
        assert_eq!(defaults.get("CON").unwrap().as_i64().unwrap(), 50);
        assert_eq!(defaults.get("POW").unwrap().as_i64().unwrap(), 50);

        // Luck defaults
        assert_eq!(defaults.get("LUCK").unwrap().as_i64().unwrap(), 50);

        // Some skill defaults
        assert_eq!(defaults.get("FIRST_AID").unwrap().as_i64().unwrap(), 30);
        assert_eq!(defaults.get("SPOT_HIDDEN").unwrap().as_i64().unwrap(), 25);
        assert_eq!(defaults.get("CTHULHU_MYTHOS").unwrap().as_i64().unwrap(), 0);
    }

    #[test]
    fn skills_use_percentile_skill_type() {
        let system = Coc7eSystem::new();
        let schema = system.character_sheet_schema();

        let skills_section = schema.sections.iter().find(|s| s.id == "skills").unwrap();

        for field in &skills_section.fields {
            match &field.field_type {
                SchemaFieldType::PercentileSkill { show_derived } => {
                    assert!(
                        show_derived,
                        "Skill {} should show derived values",
                        field.id
                    );
                }
                _ => panic!("Skill {} should use PercentileSkill type", field.id),
            }
        }
    }

    #[test]
    fn resources_section_has_resource_bars() {
        let system = Coc7eSystem::new();
        let schema = system.character_sheet_schema();

        let resources_section = schema
            .sections
            .iter()
            .find(|s| s.id == "resources")
            .unwrap();

        assert_eq!(resources_section.fields.len(), 4);

        // Check that all fields use ResourceBar type
        for field in &resources_section.fields {
            match &field.field_type {
                SchemaFieldType::ResourceBar { max_field, .. } => {
                    assert!(!max_field.is_empty(), "ResourceBar should have max_field");
                }
                _ => panic!("Resource {} should use ResourceBar type", field.id),
            }
        }
    }

    #[test]
    fn identity_section_fields() {
        let system = Coc7eSystem::new();
        let schema = system.character_sheet_schema();

        let identity_section = schema.sections.iter().find(|s| s.id == "identity").unwrap();

        let field_ids: Vec<&str> = identity_section
            .fields
            .iter()
            .map(|f| f.id.as_str())
            .collect();
        assert!(field_ids.contains(&"NAME"));
        assert!(field_ids.contains(&"OCCUPATION"));
        assert!(field_ids.contains(&"AGE"));
        assert!(field_ids.contains(&"RESIDENCE"));
        assert!(field_ids.contains(&"BIRTHPLACE"));
    }
}
