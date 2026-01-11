//! Pathfinder 2nd Edition game system implementation.
//!
//! PF2e uses a d20 + modifier vs DC system with four degrees of success.
//! Key differences from D&D 5e:
//! - Proficiency is level-dependent (level + rank bonus)
//! - Four degrees of success (Critical Success, Success, Failure, Critical Failure)
//! - Three-action economy per turn
//! - Multiple Attack Penalty (MAP)
//! - Conditions have numeric values

use super::traits::{
    CalculationEngine, CasterType, CharacterSheetProvider, CharacterSheetSchema, CreationStep,
    DerivedField, DerivationType, FieldDefinition, FieldLayout, FieldValidation, GameSystem,
    ProficiencyLevel, ProficiencyOption, ResourceColor, SchemaFieldType, SchemaSection,
    SchemaSelectOption, SectionType, SpellcastingSystem,
};
use crate::entities::{StatBlock, StatModifier};
use std::collections::HashMap;

/// Pathfinder 2e proficiency ranks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pf2eProficiencyRank {
    /// Not trained in the skill
    Untrained,
    /// Basic training (+2 + level)
    Trained,
    /// Advanced training (+4 + level)
    Expert,
    /// Mastery (+6 + level)
    Master,
    /// Ultimate mastery (+8 + level)
    Legendary,
}

impl Pf2eProficiencyRank {
    /// Get the rank bonus (before adding level).
    pub fn rank_bonus(&self) -> i32 {
        match self {
            Pf2eProficiencyRank::Untrained => 0,
            Pf2eProficiencyRank::Trained => 2,
            Pf2eProficiencyRank::Expert => 4,
            Pf2eProficiencyRank::Master => 6,
            Pf2eProficiencyRank::Legendary => 8,
        }
    }

    /// Calculate full proficiency bonus including level.
    pub fn proficiency_bonus(&self, level: u8) -> i32 {
        match self {
            Pf2eProficiencyRank::Untrained => 0, // Untrained doesn't add level
            _ => self.rank_bonus() + level as i32,
        }
    }
}

impl From<ProficiencyLevel> for Pf2eProficiencyRank {
    fn from(level: ProficiencyLevel) -> Self {
        match level {
            ProficiencyLevel::None => Pf2eProficiencyRank::Untrained,
            ProficiencyLevel::Half => Pf2eProficiencyRank::Trained, // Approximate
            ProficiencyLevel::Proficient => Pf2eProficiencyRank::Trained,
            ProficiencyLevel::Expert => Pf2eProficiencyRank::Expert,
        }
    }
}

/// Four degrees of success in PF2e.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegreeOfSuccess {
    /// Beat DC by 10+ OR natural 20 that succeeds
    CriticalSuccess,
    /// Meet or beat DC
    Success,
    /// Below DC
    Failure,
    /// Miss DC by 10+ OR natural 1 that fails
    CriticalFailure,
}

impl DegreeOfSuccess {
    /// Upgrade the degree by one step (e.g., nat 20).
    pub fn upgrade(self) -> Self {
        match self {
            DegreeOfSuccess::CriticalFailure => DegreeOfSuccess::Failure,
            DegreeOfSuccess::Failure => DegreeOfSuccess::Success,
            DegreeOfSuccess::Success => DegreeOfSuccess::CriticalSuccess,
            DegreeOfSuccess::CriticalSuccess => DegreeOfSuccess::CriticalSuccess,
        }
    }

    /// Downgrade the degree by one step (e.g., nat 1).
    pub fn downgrade(self) -> Self {
        match self {
            DegreeOfSuccess::CriticalSuccess => DegreeOfSuccess::Success,
            DegreeOfSuccess::Success => DegreeOfSuccess::Failure,
            DegreeOfSuccess::Failure => DegreeOfSuccess::CriticalFailure,
            DegreeOfSuccess::CriticalFailure => DegreeOfSuccess::CriticalFailure,
        }
    }
}

/// Determine success level for a PF2e roll.
pub fn determine_success(roll: i32, modifier: i32, dc: i32, is_nat_20: bool, is_nat_1: bool) -> DegreeOfSuccess {
    let total = roll + modifier;
    let diff = total - dc;

    // Base success level from difference
    let base = if diff >= 0 {
        DegreeOfSuccess::Success
    } else {
        DegreeOfSuccess::Failure
    };

    // Apply +/- 10 rule
    let adjusted = if diff >= 10 {
        base.upgrade()
    } else if diff <= -10 {
        base.downgrade()
    } else {
        base
    };

    // Natural 20/1 adjustments
    if is_nat_20 {
        adjusted.upgrade()
    } else if is_nat_1 {
        adjusted.downgrade()
    } else {
        adjusted
    }
}

/// Pathfinder 2nd Edition game system.
pub struct Pf2eSystem {
    stat_names: Vec<&'static str>,
    skill_names: Vec<&'static str>,
}

impl Pf2eSystem {
    pub fn new() -> Self {
        Self {
            stat_names: vec!["STR", "DEX", "CON", "INT", "WIS", "CHA"],
            skill_names: vec![
                "Acrobatics",
                "Arcana",
                "Athletics",
                "Crafting",
                "Deception",
                "Diplomacy",
                "Intimidation",
                "Lore",
                "Medicine",
                "Nature",
                "Occultism",
                "Performance",
                "Religion",
                "Society",
                "Stealth",
                "Survival",
                "Thievery",
            ],
        }
    }
}

impl Default for Pf2eSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSystem for Pf2eSystem {
    fn system_id(&self) -> &str {
        "pf2e"
    }

    fn display_name(&self) -> &str {
        "Pathfinder 2nd Edition"
    }

    fn calculation_engine(&self) -> &dyn CalculationEngine {
        self
    }

    fn spellcasting_system(&self) -> Option<&dyn SpellcastingSystem> {
        Some(self)
    }

    fn stat_names(&self) -> &[&str] {
        &self.stat_names
    }

    fn skill_names(&self) -> &[&str] {
        &self.skill_names
    }
}

impl CalculationEngine for Pf2eSystem {
    fn ability_modifier(&self, score: i32) -> i32 {
        // Same formula as D&D 5e: (score - 10) / 2
        let diff = score - 10;
        if diff >= 0 {
            diff / 2
        } else {
            (diff - 1) / 2 // Floor division for negative
        }
    }

    fn proficiency_bonus(&self, level: u8) -> i32 {
        // In PF2e, proficiency bonus depends on rank AND level
        // This returns just level for use with Trained rank
        // Real calculation uses Pf2eProficiencyRank::proficiency_bonus()
        level as i32 + 2 // Trained baseline
    }

    fn spell_save_dc(&self, stats: &StatBlock, casting_stat: &str) -> i32 {
        // PF2e: 10 + proficiency + casting stat modifier
        let stat_value = stats.get_stat(casting_stat).unwrap_or(10);
        let modifier = self.ability_modifier(stat_value);
        let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;
        // Assume trained spellcasting proficiency
        let prof = Pf2eProficiencyRank::Trained.proficiency_bonus(level);
        10 + prof + modifier
    }

    fn spell_attack_bonus(&self, stats: &StatBlock, casting_stat: &str) -> i32 {
        let stat_value = stats.get_stat(casting_stat).unwrap_or(10);
        let modifier = self.ability_modifier(stat_value);
        let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;
        let prof = Pf2eProficiencyRank::Trained.proficiency_bonus(level);
        prof + modifier
    }

    fn attack_bonus(&self, stats: &StatBlock, attack_stat: &str, proficient: bool) -> i32 {
        let stat_value = stats.get_stat(attack_stat).unwrap_or(10);
        let modifier = self.ability_modifier(stat_value);
        let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;

        if proficient {
            let prof = Pf2eProficiencyRank::Trained.proficiency_bonus(level);
            prof + modifier
        } else {
            modifier // Untrained doesn't add level
        }
    }

    fn stack_modifiers(&self, modifiers: &[StatModifier]) -> i32 {
        // PF2e stacking rules: bonuses stack by TYPE (circumstance, item, status)
        // Since our StatModifier doesn't have a type field, we group by source prefix
        // as a heuristic: bonuses from same source category don't stack (take highest)
        // Penalties all stack

        use std::collections::HashMap;
        let mut source_bonuses: HashMap<String, i32> = HashMap::new();
        let mut total_penalties = 0;

        for modifier in modifiers.iter().filter(|m| m.active) {
            if modifier.value >= 0 {
                // Extract source category (first word) as a proxy for type
                let source_type = modifier
                    .source
                    .split_whitespace()
                    .next()
                    .unwrap_or("untyped")
                    .to_lowercase();
                let current = source_bonuses.entry(source_type).or_insert(0);
                *current = (*current).max(modifier.value);
            } else {
                // Penalty - all stack
                total_penalties += modifier.value;
            }
        }

        source_bonuses.values().sum::<i32>() + total_penalties
    }

    fn calculate_ac(
        &self,
        stats: &StatBlock,
        armor_ac: Option<i32>,
        shield_bonus: Option<i32>,
        allows_dex: bool,
        max_dex_bonus: Option<i32>,
    ) -> i32 {
        // PF2e AC = 10 + DEX mod (with cap) + proficiency + armor bonus + shield bonus
        let dex_value = stats.get_stat("DEX").unwrap_or(10);
        let dex_mod = self.ability_modifier(dex_value);
        let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;

        let dex_contribution = if allows_dex {
            match max_dex_bonus {
                Some(cap) => dex_mod.min(cap),
                None => dex_mod,
            }
        } else {
            0
        };

        // Assume trained armor proficiency
        let armor_prof = Pf2eProficiencyRank::Trained.proficiency_bonus(level);
        let armor_item_bonus = armor_ac.unwrap_or(0);
        let shield = shield_bonus.unwrap_or(0);

        10 + dex_contribution + armor_prof + armor_item_bonus + shield
    }

    fn skill_modifier(
        &self,
        stats: &StatBlock,
        ability: &str,
        proficiency_level: ProficiencyLevel,
    ) -> i32 {
        let stat_value = stats.get_stat(ability).unwrap_or(10);
        let ability_mod = self.ability_modifier(stat_value);
        let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;
        let rank = Pf2eProficiencyRank::from(proficiency_level);
        let prof = rank.proficiency_bonus(level);

        ability_mod + prof
    }

    fn saving_throw_modifier(
        &self,
        stats: &StatBlock,
        ability: &str,
        proficient: bool,
    ) -> i32 {
        let stat_value = stats.get_stat(ability).unwrap_or(10);
        let ability_mod = self.ability_modifier(stat_value);
        let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;

        if proficient {
            let prof = Pf2eProficiencyRank::Trained.proficiency_bonus(level);
            ability_mod + prof
        } else {
            ability_mod
        }
    }

    fn passive_perception(&self, stats: &StatBlock, proficiency_level: ProficiencyLevel) -> i32 {
        // PF2e: 10 + Perception modifier
        let wis_value = stats.get_stat("WIS").unwrap_or(10);
        let ability_mod = self.ability_modifier(wis_value);
        let level = stats.get_stat("LEVEL").unwrap_or(1) as u8;
        let rank = Pf2eProficiencyRank::from(proficiency_level);
        let prof = rank.proficiency_bonus(level);

        10 + ability_mod + prof
    }

    fn hit_die(&self, class_name: &str) -> u8 {
        // PF2e hit points per level (not dice, but fixed values)
        match class_name.to_lowercase().as_str() {
            "wizard" | "sorcerer" => 6,
            "alchemist" | "bard" | "cleric" | "druid" | "investigator"
            | "oracle" | "rogue" | "swashbuckler" | "witch" => 8,
            "barbarian" | "champion" | "fighter" | "magus" | "monk" | "ranger" => 10,
            _ => 8, // Default
        }
    }

    fn calculate_max_hp(
        &self,
        level: u8,
        class_name: &str,
        constitution_modifier: i32,
        additional_hp: i32,
    ) -> i32 {
        // PF2e: Ancestry HP + (Class HP + CON mod) per level
        // Simplified: Just class HP + CON per level
        let hp_per_level = self.hit_die(class_name) as i32;
        let level_hp = (hp_per_level + constitution_modifier) * level as i32;

        // Ancestry HP varies (typically 6-10), assume 8
        let ancestry_hp = 8;

        ancestry_hp + level_hp + additional_hp
    }
}

impl SpellcastingSystem for Pf2eSystem {
    fn caster_type(&self, class: &str) -> Option<CasterType> {
        match class.to_lowercase().as_str() {
            "wizard" | "cleric" | "druid" | "sorcerer" | "bard" | "witch" | "oracle" => {
                Some(CasterType::Full)
            }
            "magus" | "summoner" => Some(CasterType::Half),
            "champion" => Some(CasterType::Half), // Focus spells
            _ => None,
        }
    }

    fn spellcasting_stat(&self, class: &str) -> Option<&str> {
        match class.to_lowercase().as_str() {
            "wizard" | "witch" | "alchemist" | "investigator" => Some("INT"),
            "cleric" | "druid" | "ranger" | "monk" => Some("WIS"),
            "sorcerer" | "bard" | "oracle" | "summoner" | "swashbuckler" => Some("CHA"),
            "magus" | "champion" | "fighter" => Some("CHA"), // Varies
            _ => None,
        }
    }

    fn uses_spell_preparation(&self, class: &str) -> bool {
        matches!(
            class.to_lowercase().as_str(),
            "wizard" | "cleric" | "druid" | "witch" | "magus"
        )
    }

    fn max_prepared_spells(&self, class: &str, _level: u8, _stat_mod: i32) -> u8 {
        // PF2e prepared casters prepare a fixed number based on slots
        // This is simplified - actual is slots per level
        match class.to_lowercase().as_str() {
            "wizard" => 3, // Per spell level
            "cleric" | "druid" => 3,
            "witch" => 2,
            _ => 0,
        }
    }

    fn spell_slots(&self, class: &str, level: u8) -> HashMap<u8, u8> {
        // PF2e spell slot progression (full casters)
        // Format: spell_level -> number of slots
        let mut slots = HashMap::new();

        if self.caster_type(class).is_none() {
            return slots;
        }

        // Full caster progression (simplified)
        // Gets slots for spell levels up to (level + 1) / 2
        let max_spell_level = ((level + 1) / 2).min(10);

        for spell_level in 1..=max_spell_level {
            let slot_count = if spell_level == max_spell_level {
                2 // Highest available level
            } else {
                3 // Lower levels
            };
            slots.insert(spell_level, slot_count);
        }

        slots
    }

    fn cantrips_known(&self, class: &str, _level: u8) -> u8 {
        match class.to_lowercase().as_str() {
            "wizard" => 5,
            "sorcerer" | "bard" | "cleric" | "druid" => 5,
            "witch" => 3,
            "magus" => 3,
            _ => 0,
        }
    }

    fn spells_known(&self, class: &str, level: u8) -> Option<u8> {
        // PF2e spontaneous casters (sorcerer, bard)
        match class.to_lowercase().as_str() {
            "sorcerer" | "bard" | "oracle" => Some(level * 2 + 2),
            _ => None, // Prepared casters don't have "spells known"
        }
    }
}

impl CharacterSheetProvider for Pf2eSystem {
    fn character_sheet_schema(&self) -> CharacterSheetSchema {
        CharacterSheetSchema {
            system_id: "pf2e".to_string(),
            system_name: "Pathfinder 2nd Edition".to_string(),
            sections: vec![
                self.identity_section(),
                self.ability_scores_section(),
                self.skills_section(),
                self.saves_section(),
                self.combat_section(),
                self.resources_section(),
                self.modifiers_section(),
            ],
            creation_steps: vec![
                CreationStep {
                    id: "basic_info".to_string(),
                    label: "Basic Info".to_string(),
                    description: "Choose your character's ancestry, class, and background."
                        .to_string(),
                    section_ids: vec!["identity".to_string()],
                    order: 1,
                    required: true,
                },
                CreationStep {
                    id: "ability_boosts".to_string(),
                    label: "Ability Boosts".to_string(),
                    description: "Apply ability boosts from ancestry, background, and class."
                        .to_string(),
                    section_ids: vec!["ability_scores".to_string()],
                    order: 2,
                    required: true,
                },
                CreationStep {
                    id: "skills".to_string(),
                    label: "Skills".to_string(),
                    description: "Choose your skill training from class and background."
                        .to_string(),
                    section_ids: vec!["skills".to_string()],
                    order: 3,
                    required: true,
                },
                CreationStep {
                    id: "equipment".to_string(),
                    label: "Equipment".to_string(),
                    description: "Select starting equipment and gear.".to_string(),
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

        // Calculate ability modifiers
        for ability in &["STR", "DEX", "CON", "INT", "WIS", "CHA"] {
            if let Some(score) = values.get(*ability).and_then(|v| v.as_i64()) {
                let modifier = self.ability_modifier(score as i32);
                derived.insert(format!("{}_MOD", ability), serde_json::json!(modifier));
            }
        }

        // Calculate skill modifiers using PF2e proficiency formula: level + rank_bonus
        for skill in self.skill_names() {
            let ability = skill_ability(skill);
            let ability_mod = derived
                .get(&format!("{}_MOD", ability))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let skill_id = skill.to_uppercase().replace(' ', "_");
            let proficiency_rank = values
                .get(&format!("{}_RANK", skill_id))
                .and_then(|v| v.as_str())
                .unwrap_or("untrained");

            let rank = match proficiency_rank {
                "legendary" => Pf2eProficiencyRank::Legendary,
                "master" => Pf2eProficiencyRank::Master,
                "expert" => Pf2eProficiencyRank::Expert,
                "trained" => Pf2eProficiencyRank::Trained,
                _ => Pf2eProficiencyRank::Untrained,
            };

            let prof_bonus = rank.proficiency_bonus(level);
            let skill_mod = ability_mod + prof_bonus;
            derived.insert(format!("{}_MOD", skill_id), serde_json::json!(skill_mod));
        }

        // Calculate saving throw modifiers
        let save_abilities = [("FORTITUDE", "CON"), ("REFLEX", "DEX"), ("WILL", "WIS")];
        for (save, ability) in &save_abilities {
            let ability_mod = derived
                .get(&format!("{}_MOD", ability))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let proficiency_rank = values
                .get(&format!("{}_RANK", save))
                .and_then(|v| v.as_str())
                .unwrap_or("trained");

            let rank = match proficiency_rank {
                "legendary" => Pf2eProficiencyRank::Legendary,
                "master" => Pf2eProficiencyRank::Master,
                "expert" => Pf2eProficiencyRank::Expert,
                "trained" => Pf2eProficiencyRank::Trained,
                _ => Pf2eProficiencyRank::Untrained,
            };

            let prof_bonus = rank.proficiency_bonus(level);
            let save_mod = ability_mod + prof_bonus;
            derived.insert(format!("{}_MOD", save), serde_json::json!(save_mod));
        }

        // Calculate Perception
        let wis_mod = derived
            .get("WIS_MOD")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let perception_rank = values
            .get("PERCEPTION_RANK")
            .and_then(|v| v.as_str())
            .unwrap_or("trained");
        let perception_prof_rank = match perception_rank {
            "legendary" => Pf2eProficiencyRank::Legendary,
            "master" => Pf2eProficiencyRank::Master,
            "expert" => Pf2eProficiencyRank::Expert,
            "trained" => Pf2eProficiencyRank::Trained,
            _ => Pf2eProficiencyRank::Untrained,
        };
        let perception_bonus = perception_prof_rank.proficiency_bonus(level);
        let perception_mod = wis_mod + perception_bonus;
        derived.insert("PERCEPTION_MOD".to_string(), serde_json::json!(perception_mod));

        // Calculate AC: 10 + DEX + proficiency + armor bonus
        let dex_mod = derived
            .get("DEX_MOD")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let armor_rank = values
            .get("ARMOR_RANK")
            .and_then(|v| v.as_str())
            .unwrap_or("trained");
        let armor_prof_rank = match armor_rank {
            "legendary" => Pf2eProficiencyRank::Legendary,
            "master" => Pf2eProficiencyRank::Master,
            "expert" => Pf2eProficiencyRank::Expert,
            "trained" => Pf2eProficiencyRank::Trained,
            _ => Pf2eProficiencyRank::Untrained,
        };
        let armor_prof_bonus = armor_prof_rank.proficiency_bonus(level);
        let armor_item_bonus = values
            .get("ARMOR_BONUS")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let dex_cap = values
            .get("DEX_CAP")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);
        let effective_dex = match dex_cap {
            Some(cap) => dex_mod.min(cap),
            None => dex_mod,
        };
        let ac = 10 + effective_dex + armor_prof_bonus + armor_item_bonus;
        derived.insert("AC".to_string(), serde_json::json!(ac));

        // Calculate Max HP: Ancestry HP + (Class HP + CON) per level
        let con_mod = derived
            .get("CON_MOD")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let ancestry_hp = values
            .get("ANCESTRY_HP")
            .and_then(|v| v.as_i64())
            .unwrap_or(8) as i32;
        let class_hp = values
            .get("CLASS_HP")
            .and_then(|v| v.as_i64())
            .unwrap_or(8) as i32;
        let max_hp = ancestry_hp + (class_hp + con_mod) * level as i32;
        derived.insert("MAX_HP".to_string(), serde_json::json!(max_hp.max(1)));

        // Calculate XP to next level (PF2e uses 1000 XP per level)
        let xp_current = values
            .get("XP_CURRENT")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        // XP_NEXT_LEVEL = 1000 - (XP_CURRENT % 1000), but always show 1000 if at 0
        let xp_remaining = if xp_current == 0 {
            1000
        } else {
            1000 - (xp_current % 1000)
        };
        derived.insert("XP_NEXT_LEVEL".to_string(), serde_json::json!(xp_remaining));

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
            "HERO_POINTS" => {
                if let Some(points) = value.as_i64() {
                    if points < 0 || points > 3 {
                        return Some("Hero Points must be between 0 and 3".to_string());
                    }
                } else {
                    return Some("Hero Points must be a number".to_string());
                }
            }
            _ => {}
        }
        None
    }

    fn default_values(&self) -> HashMap<String, serde_json::Value> {
        let mut defaults = HashMap::new();
        defaults.insert("LEVEL".to_string(), serde_json::json!(1));
        defaults.insert("STR".to_string(), serde_json::json!(10));
        defaults.insert("DEX".to_string(), serde_json::json!(10));
        defaults.insert("CON".to_string(), serde_json::json!(10));
        defaults.insert("INT".to_string(), serde_json::json!(10));
        defaults.insert("WIS".to_string(), serde_json::json!(10));
        defaults.insert("CHA".to_string(), serde_json::json!(10));
        defaults.insert("CURRENT_HP".to_string(), serde_json::json!(0));
        defaults.insert("HERO_POINTS".to_string(), serde_json::json!(1));
        defaults.insert("SPEED".to_string(), serde_json::json!(25));
        defaults.insert("ANCESTRY_HP".to_string(), serde_json::json!(8));
        defaults.insert("CLASS_HP".to_string(), serde_json::json!(8));
        defaults.insert("XP_CURRENT".to_string(), serde_json::json!(0));
        defaults
    }
}

// Helper methods for building the schema
impl Pf2eSystem {
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
                    id: "ANCESTRY".to_string(),
                    label: "Ancestry".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "human".to_string(),
                                label: "Human".to_string(),
                                description: Some("Versatile and ambitious".to_string()),
                            },
                            SchemaSelectOption {
                                value: "elf".to_string(),
                                label: "Elf".to_string(),
                                description: Some("Long-lived and graceful".to_string()),
                            },
                            SchemaSelectOption {
                                value: "dwarf".to_string(),
                                label: "Dwarf".to_string(),
                                description: Some("Stout and tradition-bound".to_string()),
                            },
                            SchemaSelectOption {
                                value: "gnome".to_string(),
                                label: "Gnome".to_string(),
                                description: Some("Curious and whimsical".to_string()),
                            },
                            SchemaSelectOption {
                                value: "goblin".to_string(),
                                label: "Goblin".to_string(),
                                description: Some("Scrappy and resourceful".to_string()),
                            },
                            SchemaSelectOption {
                                value: "halfling".to_string(),
                                label: "Halfling".to_string(),
                                description: Some("Lucky and optimistic".to_string()),
                            },
                            SchemaSelectOption {
                                value: "leshy".to_string(),
                                label: "Leshy".to_string(),
                                description: Some("Plant spirits with humanoid forms".to_string()),
                            },
                            SchemaSelectOption {
                                value: "orc".to_string(),
                                label: "Orc".to_string(),
                                description: Some("Strong and proud".to_string()),
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
                        new_row: true,
                        ..Default::default()
                    },
                    description: None,
                    placeholder: None,
                },
                FieldDefinition {
                    id: "HERITAGE".to_string(),
                    label: "Heritage".to_string(),
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
                    description: Some("Your ancestry's specific lineage".to_string()),
                    placeholder: Some("e.g., Skilled Human, Cavern Elf".to_string()),
                },
                FieldDefinition {
                    id: "CLASS".to_string(),
                    label: "Class".to_string(),
                    field_type: SchemaFieldType::Select {
                        options: vec![
                            SchemaSelectOption {
                                value: "alchemist".to_string(),
                                label: "Alchemist".to_string(),
                                description: Some("Master of alchemical creations".to_string()),
                            },
                            SchemaSelectOption {
                                value: "barbarian".to_string(),
                                label: "Barbarian".to_string(),
                                description: Some("Raging warrior".to_string()),
                            },
                            SchemaSelectOption {
                                value: "bard".to_string(),
                                label: "Bard".to_string(),
                                description: Some("Occult spellcaster and performer".to_string()),
                            },
                            SchemaSelectOption {
                                value: "champion".to_string(),
                                label: "Champion".to_string(),
                                description: Some("Divine warrior of a cause".to_string()),
                            },
                            SchemaSelectOption {
                                value: "cleric".to_string(),
                                label: "Cleric".to_string(),
                                description: Some("Divine spellcaster".to_string()),
                            },
                            SchemaSelectOption {
                                value: "druid".to_string(),
                                label: "Druid".to_string(),
                                description: Some("Primal spellcaster".to_string()),
                            },
                            SchemaSelectOption {
                                value: "fighter".to_string(),
                                label: "Fighter".to_string(),
                                description: Some("Master of martial combat".to_string()),
                            },
                            SchemaSelectOption {
                                value: "investigator".to_string(),
                                label: "Investigator".to_string(),
                                description: Some("Analytical detective".to_string()),
                            },
                            SchemaSelectOption {
                                value: "magus".to_string(),
                                label: "Magus".to_string(),
                                description: Some("Combines martial and arcane".to_string()),
                            },
                            SchemaSelectOption {
                                value: "monk".to_string(),
                                label: "Monk".to_string(),
                                description: Some("Martial artist".to_string()),
                            },
                            SchemaSelectOption {
                                value: "oracle".to_string(),
                                label: "Oracle".to_string(),
                                description: Some("Cursed divine spellcaster".to_string()),
                            },
                            SchemaSelectOption {
                                value: "ranger".to_string(),
                                label: "Ranger".to_string(),
                                description: Some("Wilderness warrior".to_string()),
                            },
                            SchemaSelectOption {
                                value: "rogue".to_string(),
                                label: "Rogue".to_string(),
                                description: Some("Skilled and stealthy".to_string()),
                            },
                            SchemaSelectOption {
                                value: "sorcerer".to_string(),
                                label: "Sorcerer".to_string(),
                                description: Some("Bloodline spellcaster".to_string()),
                            },
                            SchemaSelectOption {
                                value: "swashbuckler".to_string(),
                                label: "Swashbuckler".to_string(),
                                description: Some("Daring combatant".to_string()),
                            },
                            SchemaSelectOption {
                                value: "witch".to_string(),
                                label: "Witch".to_string(),
                                description: Some("Patron-bound spellcaster".to_string()),
                            },
                            SchemaSelectOption {
                                value: "wizard".to_string(),
                                label: "Wizard".to_string(),
                                description: Some("Arcane scholar".to_string()),
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
                                value: "acrobat".to_string(),
                                label: "Acrobat".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "artisan".to_string(),
                                label: "Artisan".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "barkeep".to_string(),
                                label: "Barkeep".to_string(),
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
                                value: "detective".to_string(),
                                label: "Detective".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "entertainer".to_string(),
                                label: "Entertainer".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "farmhand".to_string(),
                                label: "Farmhand".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "gladiator".to_string(),
                                label: "Gladiator".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "guard".to_string(),
                                label: "Guard".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "herbalist".to_string(),
                                label: "Herbalist".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "hunter".to_string(),
                                label: "Hunter".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "laborer".to_string(),
                                label: "Laborer".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "merchant".to_string(),
                                label: "Merchant".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "noble".to_string(),
                                label: "Noble".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "nomad".to_string(),
                                label: "Nomad".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "scholar".to_string(),
                                label: "Scholar".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "scout".to_string(),
                                label: "Scout".to_string(),
                                description: None,
                            },
                            SchemaSelectOption {
                                value: "warrior".to_string(),
                                label: "Warrior".to_string(),
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
            ("STR", "Strength", "Physical power and Athletics"),
            ("DEX", "Dexterity", "Agility, reflexes, and finesse"),
            ("CON", "Constitution", "Health and stamina"),
            ("INT", "Intelligence", "Reasoning and knowledge"),
            ("WIS", "Wisdom", "Perception and willpower"),
            ("CHA", "Charisma", "Force of personality"),
        ];

        let mut fields: Vec<FieldDefinition> = Vec::new();

        for (id, label, description) in &abilities {
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
                "Your six core abilities. In PF2e, 10 is the baseline for an average person."
                    .to_string(),
            ),
        }
    }

    fn skills_section(&self) -> SchemaSection {
        let skill_abilities: Vec<(&str, &str)> = vec![
            ("Acrobatics", "DEX"),
            ("Arcana", "INT"),
            ("Athletics", "STR"),
            ("Crafting", "INT"),
            ("Deception", "CHA"),
            ("Diplomacy", "CHA"),
            ("Intimidation", "CHA"),
            ("Lore", "INT"),
            ("Medicine", "WIS"),
            ("Nature", "WIS"),
            ("Occultism", "INT"),
            ("Performance", "CHA"),
            ("Religion", "WIS"),
            ("Society", "INT"),
            ("Stealth", "DEX"),
            ("Survival", "WIS"),
            ("Thievery", "DEX"),
        ];

        let proficiency_options = vec![
            ProficiencyOption {
                value: "untrained".to_string(),
                label: "Untrained (+0)".to_string(),
                multiplier: 0.0,
            },
            ProficiencyOption {
                value: "trained".to_string(),
                label: "Trained (+level+2)".to_string(),
                multiplier: 1.0,
            },
            ProficiencyOption {
                value: "expert".to_string(),
                label: "Expert (+level+4)".to_string(),
                multiplier: 1.0,
            },
            ProficiencyOption {
                value: "master".to_string(),
                label: "Master (+level+6)".to_string(),
                multiplier: 1.0,
            },
            ProficiencyOption {
                value: "legendary".to_string(),
                label: "Legendary (+level+8)".to_string(),
                multiplier: 1.0,
            },
        ];

        let fields: Vec<FieldDefinition> = skill_abilities
            .iter()
            .map(|(skill, ability)| {
                let skill_id = skill.to_uppercase().replace(' ', "_");
                FieldDefinition {
                    id: format!("{}_RANK", skill_id),
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
            description: Some(
                "Skills use proficiency ranks: Untrained, Trained, Expert, Master, Legendary"
                    .to_string(),
            ),
        }
    }

    fn saves_section(&self) -> SchemaSection {
        let saves = [
            ("FORTITUDE", "Fortitude", "CON", "Physical resilience"),
            ("REFLEX", "Reflex", "DEX", "Dodging and agility"),
            ("WILL", "Will", "WIS", "Mental fortitude"),
        ];

        let proficiency_options = vec![
            ProficiencyOption {
                value: "untrained".to_string(),
                label: "Untrained (+0)".to_string(),
                multiplier: 0.0,
            },
            ProficiencyOption {
                value: "trained".to_string(),
                label: "Trained (+level+2)".to_string(),
                multiplier: 1.0,
            },
            ProficiencyOption {
                value: "expert".to_string(),
                label: "Expert (+level+4)".to_string(),
                multiplier: 1.0,
            },
            ProficiencyOption {
                value: "master".to_string(),
                label: "Master (+level+6)".to_string(),
                multiplier: 1.0,
            },
            ProficiencyOption {
                value: "legendary".to_string(),
                label: "Legendary (+level+8)".to_string(),
                multiplier: 1.0,
            },
        ];

        let fields: Vec<FieldDefinition> = saves
            .iter()
            .map(|(id, label, ability, description)| FieldDefinition {
                id: format!("{}_RANK", id),
                label: label.to_string(),
                field_type: SchemaFieldType::Skill {
                    ability: ability.to_string(),
                    proficiency_levels: proficiency_options.clone(),
                },
                editable: true,
                required: false,
                derived_from: None,
                validation: None,
                layout: FieldLayout {
                    width: Some(4),
                    ..Default::default()
                },
                description: Some(description.to_string()),
                placeholder: None,
            })
            .collect();

        SchemaSection {
            id: "saves".to_string(),
            label: "Saving Throws".to_string(),
            section_type: SectionType::Combat,
            fields,
            collapsible: true,
            collapsed_default: false,
            description: Some("Your three saving throws with proficiency ranks".to_string()),
        }
    }

    fn combat_section(&self) -> SchemaSection {
        let armor_proficiency_options = vec![
            ProficiencyOption {
                value: "untrained".to_string(),
                label: "Untrained (+0)".to_string(),
                multiplier: 0.0,
            },
            ProficiencyOption {
                value: "trained".to_string(),
                label: "Trained (+level+2)".to_string(),
                multiplier: 1.0,
            },
            ProficiencyOption {
                value: "expert".to_string(),
                label: "Expert (+level+4)".to_string(),
                multiplier: 1.0,
            },
            ProficiencyOption {
                value: "master".to_string(),
                label: "Master (+level+6)".to_string(),
                multiplier: 1.0,
            },
            ProficiencyOption {
                value: "legendary".to_string(),
                label: "Legendary (+level+8)".to_string(),
                multiplier: 1.0,
            },
        ];

        let perception_proficiency_options = armor_proficiency_options.clone();

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
                        dependencies: vec![
                            "LEVEL".to_string(),
                            "CON".to_string(),
                            "ANCESTRY_HP".to_string(),
                            "CLASS_HP".to_string(),
                        ],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("Ancestry HP + (Class HP + CON) per level".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "ANCESTRY_HP".to_string(),
                    label: "Ancestry HP".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(6),
                        max: Some(12),
                        show_modifier: false,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(6),
                        max: Some(12),
                        pattern: None,
                        error_message: Some("Ancestry HP is typically 6-12".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("HP from your ancestry".to_string()),
                    placeholder: Some("8".to_string()),
                },
                FieldDefinition {
                    id: "CLASS_HP".to_string(),
                    label: "Class HP".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(6),
                        max: Some(12),
                        show_modifier: false,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(6),
                        max: Some(12),
                        pattern: None,
                        error_message: Some("Class HP is typically 6-12".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("HP per level from your class".to_string()),
                    placeholder: Some("8".to_string()),
                },
                FieldDefinition {
                    id: "AC".to_string(),
                    label: "Armor Class".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(1),
                        max: None,
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec![
                            "DEX".to_string(),
                            "LEVEL".to_string(),
                            "ARMOR_RANK".to_string(),
                            "ARMOR_BONUS".to_string(),
                        ],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("10 + DEX + proficiency + armor".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "ARMOR_RANK".to_string(),
                    label: "Armor Proficiency".to_string(),
                    field_type: SchemaFieldType::Skill {
                        ability: "DEX".to_string(),
                        proficiency_levels: armor_proficiency_options,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: Some("Your armor proficiency rank".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "ARMOR_BONUS".to_string(),
                    label: "Armor Item Bonus".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: Some(6),
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
                    description: Some("Item bonus from armor".to_string()),
                    placeholder: Some("0".to_string()),
                },
                FieldDefinition {
                    id: "DEX_CAP".to_string(),
                    label: "Dex Cap".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: Some(10),
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
                    description: Some("Max DEX bonus from armor".to_string()),
                    placeholder: Some("5".to_string()),
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
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some("Movement speed in feet".to_string()),
                    placeholder: Some("25".to_string()),
                },
                FieldDefinition {
                    id: "PERCEPTION_RANK".to_string(),
                    label: "Perception".to_string(),
                    field_type: SchemaFieldType::Skill {
                        ability: "WIS".to_string(),
                        proficiency_levels: perception_proficiency_options,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: Some("Based on WIS with proficiency".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "PERCEPTION_MOD".to_string(),
                    label: "Perception Modifier".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: None,
                        max: None,
                        show_modifier: true,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec![
                            "WIS".to_string(),
                            "LEVEL".to_string(),
                            "PERCEPTION_RANK".to_string(),
                        ],
                        display_format: Some("+{}".to_string()),
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some("WIS mod + proficiency".to_string()),
                    placeholder: None,
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: None,
        }
    }

    fn resources_section(&self) -> SchemaSection {
        SchemaSection {
            id: "resources".to_string(),
            label: "Resources".to_string(),
            section_type: SectionType::Resources,
            fields: vec![
                FieldDefinition {
                    id: "HERO_POINTS".to_string(),
                    label: "Hero Points".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: Some(3),
                        show_modifier: false,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: Some(FieldValidation {
                        min: Some(0),
                        max: Some(3),
                        pattern: None,
                        error_message: Some("Hero Points must be 0-3".to_string()),
                    }),
                    layout: FieldLayout {
                        width: Some(2),
                        ..Default::default()
                    },
                    description: Some(
                        "Spend to reroll or avoid death. Start each session with 1.".to_string(),
                    ),
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
                        width: Some(4),
                        new_row: true,
                        ..Default::default()
                    },
                    description: Some(
                        "Current XP toward next level. In PF2e, you need 1000 XP to level up.".to_string(),
                    ),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "XP_NEXT_LEVEL".to_string(),
                    label: "XP to Next Level".to_string(),
                    field_type: SchemaFieldType::Integer {
                        min: Some(0),
                        max: None,
                        show_modifier: false,
                    },
                    editable: false,
                    required: false,
                    derived_from: Some(DerivedField {
                        derivation_type: DerivationType::Custom,
                        dependencies: vec!["XP_CURRENT".to_string()],
                        display_format: None,
                    }),
                    validation: None,
                    layout: FieldLayout {
                        width: Some(4),
                        ..Default::default()
                    },
                    description: Some(
                        "XP remaining until you can level up (1000 XP per level).".to_string(),
                    ),
                    placeholder: None,
                },
            ],
            collapsible: true,
            collapsed_default: false,
            description: Some("Hero Points, experience points, and other trackable resources".to_string()),
        }
    }

    fn modifiers_section(&self) -> SchemaSection {
        SchemaSection {
            id: "modifiers".to_string(),
            label: "Conditions & Effects".to_string(),
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
                        "Active conditions affecting your character (Frightened, Sickened, Clumsy, etc.)".to_string(),
                    ),
                    placeholder: None,
                },
            ],
            collapsible: true,
            collapsed_default: false,
            description: Some("PF2e conditions apply penalties: Frightened (-1 to -4), Sickened (-1 to -4), Clumsy (DEX checks), Enfeebled (STR checks), Stupefied (mental), Drained (CON), Doomed (dying threshold).".to_string()),
        }
    }
}

/// Get the ability score associated with a skill in PF2e.
pub fn skill_ability(skill: &str) -> &'static str {
    match skill.to_lowercase().as_str() {
        "acrobatics" | "stealth" | "thievery" => "DEX",
        "arcana" | "crafting" | "lore" | "occultism" | "society" => "INT",
        "athletics" => "STR",
        "deception" | "diplomacy" | "intimidation" | "performance" => "CHA",
        "medicine" | "nature" | "religion" | "survival" => "WIS",
        _ => "INT", // Default for unknown Lore skills
    }
}

/// Calculate Multiple Attack Penalty for PF2e.
pub fn multiple_attack_penalty(attack_number: u8, is_agile: bool) -> i32 {
    match attack_number {
        0 | 1 => 0,
        2 => if is_agile { -4 } else { -5 },
        _ => if is_agile { -8 } else { -10 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proficiency_rank_bonuses() {
        assert_eq!(Pf2eProficiencyRank::Untrained.rank_bonus(), 0);
        assert_eq!(Pf2eProficiencyRank::Trained.rank_bonus(), 2);
        assert_eq!(Pf2eProficiencyRank::Expert.rank_bonus(), 4);
        assert_eq!(Pf2eProficiencyRank::Master.rank_bonus(), 6);
        assert_eq!(Pf2eProficiencyRank::Legendary.rank_bonus(), 8);
    }

    #[test]
    fn proficiency_with_level() {
        // Level 5 character
        assert_eq!(Pf2eProficiencyRank::Untrained.proficiency_bonus(5), 0);
        assert_eq!(Pf2eProficiencyRank::Trained.proficiency_bonus(5), 7); // 2 + 5
        assert_eq!(Pf2eProficiencyRank::Expert.proficiency_bonus(5), 9); // 4 + 5
        assert_eq!(Pf2eProficiencyRank::Master.proficiency_bonus(5), 11); // 6 + 5
        assert_eq!(Pf2eProficiencyRank::Legendary.proficiency_bonus(5), 13); // 8 + 5
    }

    #[test]
    fn degree_of_success_determination() {
        // Simple success (beat DC)
        assert_eq!(
            determine_success(15, 5, 18, false, false),
            DegreeOfSuccess::Success
        );

        // Simple failure (below DC)
        assert_eq!(
            determine_success(10, 5, 18, false, false),
            DegreeOfSuccess::Failure
        );

        // Critical success (beat by 10+)
        assert_eq!(
            determine_success(18, 10, 15, false, false),
            DegreeOfSuccess::CriticalSuccess
        );

        // Critical failure (miss by 10+)
        assert_eq!(
            determine_success(5, 0, 20, false, false),
            DegreeOfSuccess::CriticalFailure
        );

        // Nat 20 upgrades success to critical
        assert_eq!(
            determine_success(20, 0, 18, true, false),
            DegreeOfSuccess::CriticalSuccess
        );

        // Nat 1 downgrades
        assert_eq!(
            determine_success(1, 15, 10, false, true),
            DegreeOfSuccess::Failure
        );
    }

    #[test]
    fn ability_modifier_calculation() {
        let system = Pf2eSystem::new();
        assert_eq!(system.ability_modifier(10), 0);
        assert_eq!(system.ability_modifier(18), 4);
        assert_eq!(system.ability_modifier(8), -1);
        assert_eq!(system.ability_modifier(1), -5);
    }

    #[test]
    fn multiple_attack_penalty_values() {
        // Non-agile weapon
        assert_eq!(multiple_attack_penalty(1, false), 0);
        assert_eq!(multiple_attack_penalty(2, false), -5);
        assert_eq!(multiple_attack_penalty(3, false), -10);

        // Agile weapon
        assert_eq!(multiple_attack_penalty(1, true), 0);
        assert_eq!(multiple_attack_penalty(2, true), -4);
        assert_eq!(multiple_attack_penalty(3, true), -8);
    }

    #[test]
    fn skill_abilities_correct() {
        assert_eq!(skill_ability("Acrobatics"), "DEX");
        assert_eq!(skill_ability("Athletics"), "STR");
        assert_eq!(skill_ability("Arcana"), "INT");
        assert_eq!(skill_ability("Diplomacy"), "CHA");
        assert_eq!(skill_ability("Medicine"), "WIS");
    }

    #[test]
    fn system_identification() {
        let system = Pf2eSystem::new();
        assert_eq!(system.system_id(), "pf2e");
        assert_eq!(system.display_name(), "Pathfinder 2nd Edition");
    }

    #[test]
    fn character_sheet_schema_sections() {
        let system = Pf2eSystem::new();
        let schema = system.character_sheet_schema();

        assert_eq!(schema.system_id, "pf2e");
        assert_eq!(schema.system_name, "Pathfinder 2nd Edition");
        assert_eq!(schema.sections.len(), 6);

        // Verify section IDs
        let section_ids: Vec<&str> = schema.sections.iter().map(|s| s.id.as_str()).collect();
        assert!(section_ids.contains(&"identity"));
        assert!(section_ids.contains(&"ability_scores"));
        assert!(section_ids.contains(&"skills"));
        assert!(section_ids.contains(&"saves"));
        assert!(section_ids.contains(&"combat"));
        assert!(section_ids.contains(&"resources"));
    }

    #[test]
    fn character_sheet_creation_steps() {
        let system = Pf2eSystem::new();
        let schema = system.character_sheet_schema();

        assert_eq!(schema.creation_steps.len(), 4);
        assert_eq!(schema.creation_steps[0].id, "basic_info");
        assert_eq!(schema.creation_steps[1].id, "ability_boosts");
        assert_eq!(schema.creation_steps[2].id, "skills");
        assert_eq!(schema.creation_steps[3].id, "equipment");
    }

    #[test]
    fn calculate_derived_values_ability_modifiers() {
        let system = Pf2eSystem::new();
        let mut values = HashMap::new();
        values.insert("LEVEL".to_string(), serde_json::json!(1));
        values.insert("STR".to_string(), serde_json::json!(18)); // +4 mod
        values.insert("DEX".to_string(), serde_json::json!(14)); // +2 mod
        values.insert("CON".to_string(), serde_json::json!(12)); // +1 mod
        values.insert("INT".to_string(), serde_json::json!(10)); // +0 mod
        values.insert("WIS".to_string(), serde_json::json!(16)); // +3 mod
        values.insert("CHA".to_string(), serde_json::json!(8));  // -1 mod

        let derived = system.calculate_derived_values(&values);

        assert_eq!(derived.get("STR_MOD").unwrap(), &serde_json::json!(4));
        assert_eq!(derived.get("DEX_MOD").unwrap(), &serde_json::json!(2));
        assert_eq!(derived.get("CON_MOD").unwrap(), &serde_json::json!(1));
        assert_eq!(derived.get("INT_MOD").unwrap(), &serde_json::json!(0));
        assert_eq!(derived.get("WIS_MOD").unwrap(), &serde_json::json!(3));
        assert_eq!(derived.get("CHA_MOD").unwrap(), &serde_json::json!(-1));
    }

    #[test]
    fn calculate_derived_values_skill_modifiers_with_proficiency() {
        let system = Pf2eSystem::new();
        let mut values = HashMap::new();
        values.insert("LEVEL".to_string(), serde_json::json!(5));
        values.insert("STR".to_string(), serde_json::json!(16)); // +3 mod
        values.insert("DEX".to_string(), serde_json::json!(14)); // +2 mod
        values.insert("CON".to_string(), serde_json::json!(10));
        values.insert("INT".to_string(), serde_json::json!(10));
        values.insert("WIS".to_string(), serde_json::json!(10));
        values.insert("CHA".to_string(), serde_json::json!(10));
        values.insert("ATHLETICS_RANK".to_string(), serde_json::json!("trained"));
        values.insert("ACROBATICS_RANK".to_string(), serde_json::json!("expert"));
        values.insert("STEALTH_RANK".to_string(), serde_json::json!("untrained"));

        let derived = system.calculate_derived_values(&values);

        // Athletics: STR (+3) + Trained at level 5 (2 + 5 = 7) = 10
        assert_eq!(derived.get("ATHLETICS_MOD").unwrap(), &serde_json::json!(10));

        // Acrobatics: DEX (+2) + Expert at level 5 (4 + 5 = 9) = 11
        assert_eq!(derived.get("ACROBATICS_MOD").unwrap(), &serde_json::json!(11));

        // Stealth: DEX (+2) + Untrained (0) = 2
        assert_eq!(derived.get("STEALTH_MOD").unwrap(), &serde_json::json!(2));
    }

    #[test]
    fn calculate_derived_values_saves() {
        let system = Pf2eSystem::new();
        let mut values = HashMap::new();
        values.insert("LEVEL".to_string(), serde_json::json!(5));
        values.insert("STR".to_string(), serde_json::json!(10));
        values.insert("DEX".to_string(), serde_json::json!(14)); // +2 mod
        values.insert("CON".to_string(), serde_json::json!(16)); // +3 mod
        values.insert("INT".to_string(), serde_json::json!(10));
        values.insert("WIS".to_string(), serde_json::json!(12)); // +1 mod
        values.insert("CHA".to_string(), serde_json::json!(10));
        values.insert("FORTITUDE_RANK".to_string(), serde_json::json!("expert"));
        values.insert("REFLEX_RANK".to_string(), serde_json::json!("trained"));
        values.insert("WILL_RANK".to_string(), serde_json::json!("master"));

        let derived = system.calculate_derived_values(&values);

        // Fortitude: CON (+3) + Expert at level 5 (4 + 5 = 9) = 12
        assert_eq!(derived.get("FORTITUDE_MOD").unwrap(), &serde_json::json!(12));

        // Reflex: DEX (+2) + Trained at level 5 (2 + 5 = 7) = 9
        assert_eq!(derived.get("REFLEX_MOD").unwrap(), &serde_json::json!(9));

        // Will: WIS (+1) + Master at level 5 (6 + 5 = 11) = 12
        assert_eq!(derived.get("WILL_MOD").unwrap(), &serde_json::json!(12));
    }

    #[test]
    fn calculate_derived_values_ac() {
        let system = Pf2eSystem::new();
        let mut values = HashMap::new();
        values.insert("LEVEL".to_string(), serde_json::json!(5));
        values.insert("DEX".to_string(), serde_json::json!(14)); // +2 mod
        values.insert("STR".to_string(), serde_json::json!(10));
        values.insert("CON".to_string(), serde_json::json!(10));
        values.insert("INT".to_string(), serde_json::json!(10));
        values.insert("WIS".to_string(), serde_json::json!(10));
        values.insert("CHA".to_string(), serde_json::json!(10));
        values.insert("ARMOR_RANK".to_string(), serde_json::json!("trained"));
        values.insert("ARMOR_BONUS".to_string(), serde_json::json!(2)); // Leather

        let derived = system.calculate_derived_values(&values);

        // AC: 10 + DEX (+2) + Trained at level 5 (2 + 5 = 7) + armor bonus (2) = 21
        assert_eq!(derived.get("AC").unwrap(), &serde_json::json!(21));
    }

    #[test]
    fn calculate_derived_values_ac_with_dex_cap() {
        let system = Pf2eSystem::new();
        let mut values = HashMap::new();
        values.insert("LEVEL".to_string(), serde_json::json!(5));
        values.insert("DEX".to_string(), serde_json::json!(18)); // +4 mod, but capped
        values.insert("STR".to_string(), serde_json::json!(10));
        values.insert("CON".to_string(), serde_json::json!(10));
        values.insert("INT".to_string(), serde_json::json!(10));
        values.insert("WIS".to_string(), serde_json::json!(10));
        values.insert("CHA".to_string(), serde_json::json!(10));
        values.insert("ARMOR_RANK".to_string(), serde_json::json!("trained"));
        values.insert("ARMOR_BONUS".to_string(), serde_json::json!(4)); // Chain shirt
        values.insert("DEX_CAP".to_string(), serde_json::json!(2));

        let derived = system.calculate_derived_values(&values);

        // AC: 10 + DEX (capped to +2) + Trained at level 5 (7) + armor bonus (4) = 23
        assert_eq!(derived.get("AC").unwrap(), &serde_json::json!(23));
    }

    #[test]
    fn calculate_derived_values_max_hp() {
        let system = Pf2eSystem::new();
        let mut values = HashMap::new();
        values.insert("LEVEL".to_string(), serde_json::json!(5));
        values.insert("CON".to_string(), serde_json::json!(14)); // +2 mod
        values.insert("STR".to_string(), serde_json::json!(10));
        values.insert("DEX".to_string(), serde_json::json!(10));
        values.insert("INT".to_string(), serde_json::json!(10));
        values.insert("WIS".to_string(), serde_json::json!(10));
        values.insert("CHA".to_string(), serde_json::json!(10));
        values.insert("ANCESTRY_HP".to_string(), serde_json::json!(8)); // Human
        values.insert("CLASS_HP".to_string(), serde_json::json!(10)); // Fighter

        let derived = system.calculate_derived_values(&values);

        // HP: Ancestry (8) + (Class (10) + CON (+2)) * Level (5) = 8 + (12 * 5) = 8 + 60 = 68
        assert_eq!(derived.get("MAX_HP").unwrap(), &serde_json::json!(68));
    }

    #[test]
    fn validate_field_ability_scores() {
        let system = Pf2eSystem::new();
        let all_values = HashMap::new();

        // Valid ability score
        assert!(system.validate_field("STR", &serde_json::json!(10), &all_values).is_none());
        assert!(system.validate_field("DEX", &serde_json::json!(18), &all_values).is_none());

        // Invalid ability scores
        assert!(system.validate_field("STR", &serde_json::json!(0), &all_values).is_some());
        assert!(system.validate_field("STR", &serde_json::json!(31), &all_values).is_some());
    }

    #[test]
    fn validate_field_hero_points() {
        let system = Pf2eSystem::new();
        let all_values = HashMap::new();

        // Valid hero points
        assert!(system.validate_field("HERO_POINTS", &serde_json::json!(0), &all_values).is_none());
        assert!(system.validate_field("HERO_POINTS", &serde_json::json!(3), &all_values).is_none());

        // Invalid hero points
        assert!(system.validate_field("HERO_POINTS", &serde_json::json!(-1), &all_values).is_some());
        assert!(system.validate_field("HERO_POINTS", &serde_json::json!(4), &all_values).is_some());
    }

    #[test]
    fn default_values_correct() {
        let system = Pf2eSystem::new();
        let defaults = system.default_values();

        assert_eq!(defaults.get("LEVEL").unwrap(), &serde_json::json!(1));
        assert_eq!(defaults.get("STR").unwrap(), &serde_json::json!(10));
        assert_eq!(defaults.get("DEX").unwrap(), &serde_json::json!(10));
        assert_eq!(defaults.get("CON").unwrap(), &serde_json::json!(10));
        assert_eq!(defaults.get("INT").unwrap(), &serde_json::json!(10));
        assert_eq!(defaults.get("WIS").unwrap(), &serde_json::json!(10));
        assert_eq!(defaults.get("CHA").unwrap(), &serde_json::json!(10));
        assert_eq!(defaults.get("HERO_POINTS").unwrap(), &serde_json::json!(1));
        assert_eq!(defaults.get("SPEED").unwrap(), &serde_json::json!(25));
    }
}
