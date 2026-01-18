//! Powered by the Apocalypse game system implementation.
//!
//! PbtA uses 2d6 + stat with move-based resolution.
//! Key features:
//! - Three-tier outcomes: 6- (miss), 7-9 (partial), 10+ (full success)
//! - Moves as core mechanic
//! - Forward/Ongoing modifiers
//! - Hold mechanic
//! - Playbook-based characters

use std::collections::HashMap;

use super::traits::{
    AllocationSystem, CalculationEngine, CharacterSheetProvider, CharacterSheetSchema,
    CreationStep, DerivationType, DerivedField, FieldDefinition, FieldLayout, FieldValidation,
    GameSystem, ProficiencyLevel, ResourceColor, SchemaFieldType, SchemaSection,
    SchemaSelectOption, SectionType, SheetValue, StatArrayOption,
};
use wrldbldr_domain::value_objects::{StatBlock, StatModifier};

/// PbtA roll outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PbtaOutcome {
    /// 10+ - Full success, you do it
    FullSuccess,
    /// 7-9 - Partial success, you do it but with cost/complication
    PartialSuccess,
    /// 6- - Miss, GM makes a move
    Miss,
}

impl PbtaOutcome {
    /// Determine outcome from total (2d6 + stat).
    pub fn from_total(total: i32) -> Self {
        if total >= 10 {
            PbtaOutcome::FullSuccess
        } else if total >= 7 {
            PbtaOutcome::PartialSuccess
        } else {
            PbtaOutcome::Miss
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, PbtaOutcome::FullSuccess | PbtaOutcome::PartialSuccess)
    }
}

/// Common stats used across PbtA games.
/// Different games use different stat sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PbtaStatSet {
    /// Apocalypse World: Cool, Hard, Hot, Sharp, Weird
    ApocalypseWorld,
    /// Dungeon World: STR, DEX, CON, INT, WIS, CHA
    DungeonWorld,
    /// Monster of the Week: Charm, Cool, Sharp, Tough, Weird
    MonsterOfTheWeek,
    /// Custom stats defined per game
    Custom,
}

/// Harm system variant.
#[derive(Debug, Clone)]
pub enum HarmSystem {
    /// Apocalypse World style: 6-segment clock
    Clock { segments: u8, current: u8 },
    /// Dungeon World style: HP
    HitPoints { max: i32, current: i32 },
    /// Monster of the Week style: harm track
    HarmTrack {
        boxes: u8,
        filled: u8,
        unstable_at: u8,
    },
    /// Masks/Monsterhearts style: conditions
    Conditions { active: Vec<String> },
}

/// A PbtA move.
#[derive(Debug, Clone)]
pub struct PbtaMove {
    pub id: String,
    pub name: String,
    pub trigger: String,
    pub stat: Option<String>,
    pub full_success: String,
    pub partial_success: String,
    pub miss: Option<String>,
}

impl PbtaMove {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        trigger: impl Into<String>,
        stat: Option<impl Into<String>>,
        full_success: impl Into<String>,
        partial_success: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            trigger: trigger.into(),
            stat: stat.map(|s| s.into()),
            full_success: full_success.into(),
            partial_success: partial_success.into(),
            miss: None,
        }
    }

    pub fn with_miss(mut self, miss: impl Into<String>) -> Self {
        self.miss = Some(miss.into());
        self
    }

    pub fn requires_roll(&self) -> bool {
        self.stat.is_some()
    }
}

/// Powered by the Apocalypse game system.
pub struct PbtaSystem {
    variant: PbtaVariant,
    stat_names: Vec<&'static str>,
}

/// PbtA game variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PbtaVariant {
    ApocalypseWorld,
    DungeonWorld,
    MonsterOfTheWeek,
    Generic,
}

impl PbtaSystem {
    /// Create a generic PbtA system.
    pub fn new() -> Self {
        Self::generic()
    }

    /// Create Apocalypse World variant.
    pub fn apocalypse_world() -> Self {
        Self {
            variant: PbtaVariant::ApocalypseWorld,
            stat_names: vec!["Cool", "Hard", "Hot", "Sharp", "Weird"],
        }
    }

    /// Create Dungeon World variant.
    pub fn dungeon_world() -> Self {
        Self {
            variant: PbtaVariant::DungeonWorld,
            stat_names: vec!["STR", "DEX", "CON", "INT", "WIS", "CHA"],
        }
    }

    /// Create Monster of the Week variant.
    pub fn monster_of_the_week() -> Self {
        Self {
            variant: PbtaVariant::MonsterOfTheWeek,
            stat_names: vec!["Charm", "Cool", "Sharp", "Tough", "Weird"],
        }
    }

    /// Create generic PbtA system.
    pub fn generic() -> Self {
        Self {
            variant: PbtaVariant::Generic,
            stat_names: vec!["Stat1", "Stat2", "Stat3", "Stat4", "Stat5"],
        }
    }

    pub fn variant(&self) -> PbtaVariant {
        self.variant
    }

    /// Get basic moves for this variant.
    pub fn basic_moves(&self) -> Vec<PbtaMove> {
        match self.variant {
            PbtaVariant::ApocalypseWorld => apocalypse_world_basic_moves(),
            PbtaVariant::DungeonWorld => dungeon_world_basic_moves(),
            PbtaVariant::MonsterOfTheWeek => monster_of_the_week_basic_moves(),
            PbtaVariant::Generic => vec![],
        }
    }
}

impl Default for PbtaSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSystem for PbtaSystem {
    fn system_id(&self) -> &str {
        match self.variant {
            PbtaVariant::ApocalypseWorld => "pbta_aw",
            PbtaVariant::DungeonWorld => "pbta_dw",
            PbtaVariant::MonsterOfTheWeek => "pbta_motw",
            PbtaVariant::Generic => "pbta",
        }
    }

    fn display_name(&self) -> &str {
        match self.variant {
            PbtaVariant::ApocalypseWorld => "Apocalypse World",
            PbtaVariant::DungeonWorld => "Dungeon World",
            PbtaVariant::MonsterOfTheWeek => "Monster of the Week",
            PbtaVariant::Generic => "Powered by the Apocalypse",
        }
    }

    fn calculation_engine(&self) -> &dyn CalculationEngine {
        self
    }

    fn stat_names(&self) -> &[&str] {
        &self.stat_names
    }

    fn skill_names(&self) -> &[&str] {
        // PbtA doesn't have skills - moves replace them
        &[]
    }
}

impl CalculationEngine for PbtaSystem {
    fn ability_modifier(&self, score: i32) -> i32 {
        // PbtA stats are already modifiers (-2 to +3 typically)
        // For Dungeon World, convert from D&D-style scores
        if self.variant == PbtaVariant::DungeonWorld {
            // Dungeon World uses D&D-style ability scores
            let diff = score - 10;
            if diff >= 0 {
                diff / 2
            } else {
                (diff - 1) / 2
            }
        } else {
            // Other PbtA games use direct stat values
            score
        }
    }

    fn proficiency_bonus(&self, _level: u8) -> i32 {
        // PbtA has no proficiency system
        0
    }

    fn spell_save_dc(&self, _stats: &StatBlock, _casting_stat: &str) -> i32 {
        // PbtA magic uses moves, not DCs
        0
    }

    fn spell_attack_bonus(&self, _stats: &StatBlock, _casting_stat: &str) -> i32 {
        // PbtA magic uses moves
        0
    }

    fn attack_bonus(&self, stats: &StatBlock, attack_stat: &str, _proficient: bool) -> i32 {
        // Return stat value directly
        let value = stats.get_stat(attack_stat).unwrap_or(0);
        self.ability_modifier(value)
    }

    fn stack_modifiers(&self, modifiers: &[StatModifier]) -> i32 {
        // PbtA modifiers stack (forward, ongoing, etc.)
        modifiers
            .iter()
            .filter(|m| m.is_active())
            .map(|m| m.value())
            .sum()
    }

    fn calculate_ac(
        &self,
        stats: &StatBlock,
        armor_value: Option<i32>,
        _shield_bonus: Option<i32>,
        _allows_dex: bool,
        _max_dex_bonus: Option<i32>,
    ) -> i32 {
        // Only Dungeon World has armor
        if self.variant == PbtaVariant::DungeonWorld {
            armor_value.unwrap_or(0)
        } else {
            stats.get_stat("Armor").unwrap_or(0)
        }
    }

    fn skill_modifier(
        &self,
        stats: &StatBlock,
        stat: &str,
        _proficiency_level: ProficiencyLevel,
    ) -> i32 {
        let value = stats.get_stat(stat).unwrap_or(0);
        self.ability_modifier(value)
    }

    fn saving_throw_modifier(&self, stats: &StatBlock, ability: &str, _proficient: bool) -> i32 {
        let value = stats.get_stat(ability).unwrap_or(0);
        self.ability_modifier(value)
    }

    fn passive_perception(&self, stats: &StatBlock, _proficiency_level: ProficiencyLevel) -> i32 {
        // Use Sharp/WIS equivalent
        let stat = match self.variant {
            PbtaVariant::ApocalypseWorld => "Sharp",
            PbtaVariant::DungeonWorld => "WIS",
            PbtaVariant::MonsterOfTheWeek => "Sharp",
            PbtaVariant::Generic => "Stat4",
        };
        let value = stats.get_stat(stat).unwrap_or(0);
        10 + self.ability_modifier(value)
    }

    fn hit_die(&self, class_name: &str) -> u8 {
        // Only Dungeon World uses hit dice
        if self.variant == PbtaVariant::DungeonWorld {
            match class_name.to_lowercase().as_str() {
                "fighter" | "paladin" => 10,
                "wizard" => 4,
                "cleric" | "druid" | "ranger" | "thief" | "bard" => 8,
                _ => 6,
            }
        } else {
            0
        }
    }

    fn calculate_max_hp(
        &self,
        _level: u8,
        class_name: &str,
        constitution: i32,
        _additional_hp: i32,
    ) -> i32 {
        // Only Dungeon World uses HP
        if self.variant == PbtaVariant::DungeonWorld {
            // DW: Class base + CON score
            let class_base = match class_name.to_lowercase().as_str() {
                "fighter" | "paladin" => 10,
                "cleric" | "ranger" => 8,
                "thief" | "bard" | "druid" => 6,
                "wizard" => 4,
                _ => 6,
            };
            class_base + constitution
        } else {
            // Other PbtA games use harm tracks
            0
        }
    }
}

impl CharacterSheetProvider for PbtaSystem {
    fn character_sheet_schema(&self) -> CharacterSheetSchema {
        CharacterSheetSchema {
            system_id: self.system_id().to_string(),
            system_name: self.display_name().to_string(),
            sections: self.build_sections(),
            creation_steps: self.build_creation_steps(),
        }
    }

    fn calculate_derived_values(
        &self,
        values: &HashMap<String, SheetValue>,
    ) -> HashMap<String, SheetValue> {
        let mut derived = HashMap::new();

        match self.variant {
            PbtaVariant::DungeonWorld => {
                // Dungeon World uses D&D-style ability scores -> modifiers
                for stat in &["STR", "DEX", "CON", "INT", "WIS", "CHA"] {
                    if let Some(score) = values.get(*stat).and_then(SheetValue::as_i64) {
                        let modifier = self.ability_modifier(score as i32);
                        derived.insert(format!("{}_MOD", stat), SheetValue::Integer(modifier));
                    }
                }

                // Calculate HP if class and CON are set
                if let (Some(class), Some(con)) = (
                    values.get("PLAYBOOK").and_then(SheetValue::as_str),
                    values.get("CON").and_then(SheetValue::as_i64),
                ) {
                    let max_hp = self.calculate_max_hp(1, class, con as i32, 0);
                    derived.insert("MAX_HP".to_string(), SheetValue::Integer(max_hp));
                }
            }
            _ => {
                // For non-DW PbtA games, stats ARE the modifiers (no conversion needed)
                // But we still track them for consistency
                for stat in self.stat_names() {
                    if let Some(value) = values.get(*stat).and_then(SheetValue::as_i64) {
                        derived.insert(format!("{}_MOD", stat), SheetValue::Integer(value as i32));
                    }
                }
            }
        }

        // Calculate total XP (if tracking advancement)
        if let Some(xp) = values.get("XP").and_then(SheetValue::as_i64) {
            let xp_max = match self.variant {
                PbtaVariant::DungeonWorld => {
                    // DW: Level + 7
                    let level = values
                        .get("LEVEL")
                        .and_then(SheetValue::as_i64)
                        .unwrap_or(1);
                    level + 7
                }
                _ => 5, // Most PbtA games use 5 XP to advance
            };
            derived.insert("XP_MAX".to_string(), SheetValue::Integer(xp_max as i32));
            let xp_remaining = (xp_max - xp).max(0);
            derived.insert(
                "XP_REMAINING".to_string(),
                SheetValue::Integer(xp_remaining as i32),
            );
        }

        derived
    }

    fn validate_field(
        &self,
        field_id: &str,
        value: &SheetValue,
        _all_values: &HashMap<String, SheetValue>,
    ) -> Option<String> {
        match self.variant {
            PbtaVariant::DungeonWorld => {
                // DW uses D&D-style ability scores
                match field_id {
                    "STR" | "DEX" | "CON" | "INT" | "WIS" | "CHA" => {
                        if let Some(score) = value.as_i64() {
                            if !(3..=18).contains(&(score as i32)) {
                                return Some("Ability scores must be between 3 and 18".to_string());
                            }
                        } else {
                            return Some("Ability score must be a number".to_string());
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                // Other PbtA games use stats from -1 to +3 (sometimes -2 to +3)
                let stat_names: Vec<&str> = self.stat_names().to_vec();
                if stat_names.contains(&field_id) {
                    if let Some(stat) = value.as_i64() {
                        if !(-2..=3).contains(&(stat as i32)) {
                            return Some("Stats must be between -2 and +3".to_string());
                        }
                    } else {
                        return Some("Stat must be a number".to_string());
                    }
                }
            }
        }

        // Common validations
        match field_id {
            "NAME" => {
                if let Some(name) = value.as_str() {
                    if name.is_empty() {
                        return Some("Name is required".to_string());
                    }
                } else {
                    return Some("Name must be a string".to_string());
                }
            }
            "XP" => {
                if let Some(xp) = value.as_i64() {
                    if xp < 0 {
                        return Some("XP cannot be negative".to_string());
                    }
                }
            }
            "HOLD" => {
                if let Some(hold) = value.as_i64() {
                    if hold < 0 {
                        return Some("Hold cannot be negative".to_string());
                    }
                }
            }
            _ => {}
        }

        None
    }

    fn default_values(&self) -> HashMap<String, SheetValue> {
        let mut defaults = HashMap::new();

        match self.variant {
            PbtaVariant::DungeonWorld => {
                // DW uses D&D-style scores, default to 10
                defaults.insert("STR".to_string(), SheetValue::Integer(10));
                defaults.insert("DEX".to_string(), SheetValue::Integer(10));
                defaults.insert("CON".to_string(), SheetValue::Integer(10));
                defaults.insert("INT".to_string(), SheetValue::Integer(10));
                defaults.insert("WIS".to_string(), SheetValue::Integer(10));
                defaults.insert("CHA".to_string(), SheetValue::Integer(10));
                defaults.insert("LEVEL".to_string(), SheetValue::Integer(1));
                defaults.insert("ARMOR".to_string(), SheetValue::Integer(0));
            }
            PbtaVariant::ApocalypseWorld => {
                defaults.insert("Cool".to_string(), SheetValue::Integer(0));
                defaults.insert("Hard".to_string(), SheetValue::Integer(0));
                defaults.insert("Hot".to_string(), SheetValue::Integer(0));
                defaults.insert("Sharp".to_string(), SheetValue::Integer(0));
                defaults.insert("Weird".to_string(), SheetValue::Integer(0));
            }
            PbtaVariant::MonsterOfTheWeek => {
                defaults.insert("Charm".to_string(), SheetValue::Integer(0));
                defaults.insert("Cool".to_string(), SheetValue::Integer(0));
                defaults.insert("Sharp".to_string(), SheetValue::Integer(0));
                defaults.insert("Tough".to_string(), SheetValue::Integer(0));
                defaults.insert("Weird".to_string(), SheetValue::Integer(0));
            }
            PbtaVariant::Generic => {
                for stat in self.stat_names() {
                    defaults.insert(stat.to_string(), SheetValue::Integer(0));
                }
            }
        }

        // Common defaults
        defaults.insert("XP".to_string(), SheetValue::Integer(0));
        defaults.insert("HOLD".to_string(), SheetValue::Integer(0));
        defaults.insert("HARM".to_string(), SheetValue::Integer(0));
        defaults.insert("CURRENT_HP".to_string(), SheetValue::Integer(0));

        defaults
    }
}

// Helper methods for building the character sheet schema
impl PbtaSystem {
    /// Create the stat array allocation for this PbtA variant.
    ///
    /// PbtA games typically use stat arrays where players choose from
    /// pre-defined distributions of stat values.
    /// Values are positional - first value goes to first target_field, etc.
    #[allow(dead_code)] // Unfinished allocation systems feature - PbtA needs public allocation_systems() method
    fn stat_array_allocation(&self) -> AllocationSystem {
        match self.variant {
            PbtaVariant::DungeonWorld => {
                // Dungeon World uses D&D-style standard array
                // Order matches target_fields: STR, DEX, CON, INT, WIS, CHA
                AllocationSystem::StatArray {
                    arrays: vec![StatArrayOption {
                        id: "standard".to_string(),
                        description: Some(
                            "Standard Array: Assign 16, 15, 13, 12, 9, 8 to abilities".to_string(),
                        ),
                        values: vec![16, 15, 13, 12, 9, 8],
                    }],
                    target_fields: vec![
                        "STR".to_string(),
                        "DEX".to_string(),
                        "CON".to_string(),
                        "INT".to_string(),
                        "WIS".to_string(),
                        "CHA".to_string(),
                    ],
                }
            }
            PbtaVariant::ApocalypseWorld => {
                // Apocalypse World: Each playbook has specific stat arrays
                // Order matches target_fields: Cool, Hard, Hot, Sharp, Weird
                AllocationSystem::StatArray {
                    arrays: vec![
                        StatArrayOption {
                            id: "balanced".to_string(),
                            description: Some("Balanced: +1/0/+1/+1/-1".to_string()),
                            values: vec![1, 0, 1, 1, -1],
                        },
                        StatArrayOption {
                            id: "focused".to_string(),
                            description: Some("Focused: +2/+1/-1/+1/0".to_string()),
                            values: vec![2, 1, -1, 1, 0],
                        },
                        StatArrayOption {
                            id: "specialist".to_string(),
                            description: Some("Specialist: 0/+2/0/-1/+2".to_string()),
                            values: vec![0, 2, 0, -1, 2],
                        },
                    ],
                    target_fields: vec![
                        "Cool".to_string(),
                        "Hard".to_string(),
                        "Hot".to_string(),
                        "Sharp".to_string(),
                        "Weird".to_string(),
                    ],
                }
            }
            PbtaVariant::MonsterOfTheWeek => {
                // Monster of the Week: Similar to AW, playbook-based arrays
                // Order matches target_fields: Charm, Cool, Sharp, Tough, Weird
                AllocationSystem::StatArray {
                    arrays: vec![
                        StatArrayOption {
                            id: "action".to_string(),
                            description: Some("Action-Oriented: 0/+1/+1/+2/-1".to_string()),
                            values: vec![0, 1, 1, 2, -1],
                        },
                        StatArrayOption {
                            id: "investigator".to_string(),
                            description: Some("Investigator: +1/0/+2/-1/+1".to_string()),
                            values: vec![1, 0, 2, -1, 1],
                        },
                        StatArrayOption {
                            id: "social".to_string(),
                            description: Some("Social: +2/+1/0/-1/+1".to_string()),
                            values: vec![2, 1, 0, -1, 1],
                        },
                        StatArrayOption {
                            id: "weird".to_string(),
                            description: Some("Weird: -1/0/+1/+1/+2".to_string()),
                            values: vec![-1, 0, 1, 1, 2],
                        },
                    ],
                    target_fields: vec![
                        "Charm".to_string(),
                        "Cool".to_string(),
                        "Sharp".to_string(),
                        "Tough".to_string(),
                        "Weird".to_string(),
                    ],
                }
            }
            PbtaVariant::Generic => {
                // Generic: Flexible stat assignment
                // Order matches target_fields: Stat1-5
                AllocationSystem::StatArray {
                    arrays: vec![
                        StatArrayOption {
                            id: "standard".to_string(),
                            description: Some("Standard: +2/+1/+1/0/-1".to_string()),
                            values: vec![2, 1, 1, 0, -1],
                        },
                        StatArrayOption {
                            id: "balanced".to_string(),
                            description: Some("Balanced: +1/+1/+1/0/0".to_string()),
                            values: vec![1, 1, 1, 0, 0],
                        },
                    ],
                    target_fields: vec![
                        "Stat1".to_string(),
                        "Stat2".to_string(),
                        "Stat3".to_string(),
                        "Stat4".to_string(),
                        "Stat5".to_string(),
                    ],
                }
            }
        }
    }

    fn build_sections(&self) -> Vec<SchemaSection> {
        let mut sections = vec![self.identity_section(), self.stats_section()];

        // Add harm/HP section based on variant
        sections.push(self.harm_section());

        // Add moves section
        sections.push(self.moves_section());

        // Add resources section
        sections.push(self.resources_section());

        // Add bonds section
        sections.push(self.bonds_section());

        // Add modifiers/conditions section
        sections.push(self.modifiers_section());

        sections
    }

    fn build_creation_steps(&self) -> Vec<CreationStep> {
        vec![
            CreationStep {
                id: "identity".to_string(),
                label: "Who Are You?".to_string(),
                description: Some("Choose your name, playbook, and look.".to_string()),
                sections: vec!["identity".to_string()],
                optional: false,
            },
            CreationStep {
                id: "stats".to_string(),
                label: "Stats".to_string(),
                description: Some("Assign your stats according to your playbook.".to_string()),
                sections: vec!["stats".to_string()],
                optional: false,
            },
            CreationStep {
                id: "moves".to_string(),
                label: "Moves".to_string(),
                description: Some("Choose your starting moves.".to_string()),
                sections: vec!["moves".to_string()],
                optional: false,
            },
            CreationStep {
                id: "bonds".to_string(),
                label: "Bonds".to_string(),
                description: Some(
                    "Establish your relationships with other characters.".to_string(),
                ),
                sections: vec!["bonds".to_string()],
                optional: true,
            },
        ]
    }

    fn identity_section(&self) -> SchemaSection {
        let playbook_options = self.get_playbook_options();

        SchemaSection {
            id: "identity".to_string(),
            label: "Character Identity".to_string(),
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
                        column_span: 6,
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Enter character name".to_string()),
                },
                FieldDefinition {
                    id: "PLAYBOOK".to_string(),
                    label: self.playbook_label().to_string(),
                    field_type: SchemaFieldType::Select {
                        options: playbook_options,
                        allow_custom: true,
                    },
                    editable: true,
                    required: true,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        column_span: 6,
                        ..Default::default()
                    },
                    description: Some(self.playbook_description().to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "LOOK".to_string(),
                    label: "Look".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: true,
                        max_length: Some(500),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        column_span: 12,
                        ..Default::default()
                    },
                    description: Some("Describe your character's appearance".to_string()),
                    placeholder: Some("Describe your look...".to_string()),
                },
            ],
            collapsible: false,
            collapsed_default: false,
            description: None,
        }
    }

    fn stats_section(&self) -> SchemaSection {
        let fields = match self.variant {
            PbtaVariant::DungeonWorld => self.dungeon_world_stats_fields(),
            _ => self.standard_pbta_stats_fields(),
        };

        SchemaSection {
            id: "stats".to_string(),
            label: "Stats".to_string(),
            section_type: SectionType::AbilityScores,
            fields,
            collapsible: false,
            collapsed_default: false,
            description: Some(self.stats_description().to_string()),
        }
    }

    fn standard_pbta_stats_fields(&self) -> Vec<FieldDefinition> {
        self.stat_names()
            .iter()
            .map(|stat| FieldDefinition {
                id: stat.to_string(),
                label: stat.to_string(),
                field_type: SchemaFieldType::Integer {
                    min: Some(-2),
                    max: Some(3),
                    show_modifier: true,
                },
                editable: true,
                required: true,
                derived_from: None,
                validation: Some(FieldValidation {
                    min: Some(-2),
                    max: Some(3),
                    pattern: None,
                    message: Some("Stats must be between -2 and +3".to_string()),
                }),
                layout: FieldLayout {
                    column_span: 2,
                    ..Default::default()
                },
                description: self.stat_description(stat).map(|s| s.to_string()),
                placeholder: None,
            })
            .collect()
    }

    fn dungeon_world_stats_fields(&self) -> Vec<FieldDefinition> {
        let stats = [
            ("STR", "Strength", "Physical power, melee attacks"),
            ("DEX", "Dexterity", "Agility, ranged attacks, defense"),
            ("CON", "Constitution", "Endurance, health"),
            ("INT", "Intelligence", "Knowledge, spout lore"),
            ("WIS", "Wisdom", "Perception, discern realities"),
            ("CHA", "Charisma", "Influence, parley"),
        ];

        let mut fields: Vec<FieldDefinition> = stats
            .iter()
            .map(|(id, label, desc)| FieldDefinition {
                id: id.to_string(),
                label: label.to_string(),
                field_type: SchemaFieldType::AbilityScore {
                    min: Some(3),
                    max: Some(18),
                },
                editable: true,
                required: true,
                derived_from: None,
                validation: Some(FieldValidation {
                    min: Some(3),
                    max: Some(18),
                    pattern: None,
                    message: Some("Ability scores must be 3-18".to_string()),
                }),
                layout: FieldLayout {
                    column_span: 2,
                    ..Default::default()
                },
                description: Some(desc.to_string()),
                placeholder: None,
            })
            .collect();

        // Add Level field for Dungeon World
        fields.push(FieldDefinition {
            id: "LEVEL".to_string(),
            label: "Level".to_string(),
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
                message: Some("Level must be 1-10".to_string()),
            }),
            layout: FieldLayout {
                column_span: 2,
                ..Default::default()
            },
            description: Some("Character level".to_string()),
            placeholder: None,
        });

        // Add Armor field for Dungeon World
        fields.push(FieldDefinition {
            id: "ARMOR".to_string(),
            label: "Armor".to_string(),
            field_type: SchemaFieldType::Integer {
                min: Some(0),
                max: Some(5),
                show_modifier: false,
            },
            editable: true,
            required: false,
            derived_from: None,
            validation: None,
            layout: FieldLayout {
                column_span: 2,
                ..Default::default()
            },
            description: Some("Damage reduction from armor".to_string()),
            placeholder: None,
        });

        fields
    }

    fn harm_section(&self) -> SchemaSection {
        let fields = match self.variant {
            PbtaVariant::ApocalypseWorld => self.apocalypse_world_harm_fields(),
            PbtaVariant::DungeonWorld => self.dungeon_world_hp_fields(),
            PbtaVariant::MonsterOfTheWeek => self.monster_of_the_week_harm_fields(),
            PbtaVariant::Generic => self.generic_harm_fields(),
        };

        SchemaSection {
            id: "harm".to_string(),
            label: self.harm_label().to_string(),
            section_type: SectionType::Combat,
            fields,
            collapsible: false,
            collapsed_default: false,
            description: Some(self.harm_description().to_string()),
        }
    }

    fn apocalypse_world_harm_fields(&self) -> Vec<FieldDefinition> {
        vec![
            FieldDefinition {
                id: "HARM".to_string(),
                label: "Harm".to_string(),
                field_type: SchemaFieldType::Clock { segments: 6 },
                editable: true,
                required: false,
                derived_from: None,
                validation: Some(FieldValidation {
                    min: Some(0),
                    max: Some(6),
                    pattern: None,
                    message: Some("Harm must be 0-6".to_string()),
                }),
                layout: FieldLayout {
                    column_span: 6,
                    ..Default::default()
                },
                description: Some(
                    "When you take harm, mark segments. At 6, you're at death's door.".to_string(),
                ),
                placeholder: None,
            },
            FieldDefinition {
                id: "STABILIZED".to_string(),
                label: "Stabilized".to_string(),
                field_type: SchemaFieldType::Boolean {
                    checked_label: Some("Stabilized".to_string()),
                    unchecked_label: Some("Unstable".to_string()),
                },
                editable: true,
                required: false,
                derived_from: None,
                validation: None,
                layout: FieldLayout {
                    column_span: 3,
                    ..Default::default()
                },
                description: Some("Whether you've been stabilized after taking harm".to_string()),
                placeholder: None,
            },
        ]
    }

    fn dungeon_world_hp_fields(&self) -> Vec<FieldDefinition> {
        vec![
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
                    column_span: 6,
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
                    dependencies: vec!["PLAYBOOK".to_string(), "CON".to_string()],
                    display_format: None,
                }),
                validation: None,
                layout: FieldLayout {
                    column_span: 3,
                    ..Default::default()
                },
                description: Some("Class base + Constitution score".to_string()),
                placeholder: None,
            },
            FieldDefinition {
                id: "DAMAGE_DIE".to_string(),
                label: "Damage".to_string(),
                field_type: SchemaFieldType::Text {
                    multiline: false,
                    max_length: Some(10),
                },
                editable: true,
                required: false,
                derived_from: None,
                validation: None,
                layout: FieldLayout {
                    column_span: 3,
                    ..Default::default()
                },
                description: Some("Your class damage die".to_string()),
                placeholder: Some("d8".to_string()),
            },
        ]
    }

    fn monster_of_the_week_harm_fields(&self) -> Vec<FieldDefinition> {
        vec![
            FieldDefinition {
                id: "HARM".to_string(),
                label: "Harm".to_string(),
                field_type: SchemaFieldType::Clock { segments: 7 },
                editable: true,
                required: false,
                derived_from: None,
                validation: Some(FieldValidation {
                    min: Some(0),
                    max: Some(7),
                    pattern: None,
                    message: Some("Harm must be 0-7".to_string()),
                }),
                layout: FieldLayout {
                    column_span: 6,
                    ..Default::default()
                },
                description: Some(
                    "Mark harm as you take it. At 4+, you're unstable. At 7, you're dying."
                        .to_string(),
                ),
                placeholder: None,
            },
            FieldDefinition {
                id: "UNSTABLE".to_string(),
                label: "Unstable".to_string(),
                field_type: SchemaFieldType::Boolean {
                    checked_label: Some("Unstable".to_string()),
                    unchecked_label: Some("Stable".to_string()),
                },
                editable: true,
                required: false,
                derived_from: None,
                validation: None,
                layout: FieldLayout {
                    column_span: 3,
                    ..Default::default()
                },
                description: Some("At 4+ harm, you become unstable".to_string()),
                placeholder: None,
            },
            FieldDefinition {
                id: "LUCK".to_string(),
                label: "Luck".to_string(),
                field_type: SchemaFieldType::Clock { segments: 7 },
                editable: true,
                required: false,
                derived_from: None,
                validation: Some(FieldValidation {
                    min: Some(0),
                    max: Some(7),
                    pattern: None,
                    message: Some("Luck must be 0-7".to_string()),
                }),
                layout: FieldLayout {
                    column_span: 6,
                    ..Default::default()
                },
                description: Some("Spend luck to change a roll or avoid harm".to_string()),
                placeholder: None,
            },
        ]
    }

    fn generic_harm_fields(&self) -> Vec<FieldDefinition> {
        vec![FieldDefinition {
            id: "HARM".to_string(),
            label: "Harm".to_string(),
            field_type: SchemaFieldType::Clock { segments: 6 },
            editable: true,
            required: false,
            derived_from: None,
            validation: Some(FieldValidation {
                min: Some(0),
                max: Some(6),
                pattern: None,
                message: Some("Harm must be 0-6".to_string()),
            }),
            layout: FieldLayout {
                column_span: 6,
                ..Default::default()
            },
            description: Some("Track harm as you take damage".to_string()),
            placeholder: None,
        }]
    }

    fn moves_section(&self) -> SchemaSection {
        SchemaSection {
            id: "moves".to_string(),
            label: "Moves".to_string(),
            section_type: SectionType::Moves,
            fields: vec![
                FieldDefinition {
                    id: "BASIC_MOVES".to_string(),
                    label: "Basic Moves".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: true,
                        max_length: None,
                    },
                    editable: false,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        column_span: 12,
                        ..Default::default()
                    },
                    description: Some("Moves available to all characters".to_string()),
                    placeholder: None,
                },
                FieldDefinition {
                    id: "PLAYBOOK_MOVES".to_string(),
                    label: "Playbook Moves".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: true,
                        max_length: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        column_span: 12,
                        ..Default::default()
                    },
                    description: Some("Moves from your playbook".to_string()),
                    placeholder: Some("Enter your playbook moves...".to_string()),
                },
                FieldDefinition {
                    id: "ADVANCED_MOVES".to_string(),
                    label: "Advanced Moves".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: true,
                        max_length: None,
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        column_span: 12,
                        ..Default::default()
                    },
                    description: Some("Moves gained through advancement".to_string()),
                    placeholder: Some("Enter advanced moves...".to_string()),
                },
            ],
            collapsible: true,
            collapsed_default: false,
            description: Some("Your character's moves".to_string()),
        }
    }

    fn resources_section(&self) -> SchemaSection {
        let mut fields = vec![
            FieldDefinition {
                id: "XP".to_string(),
                label: "Experience".to_string(),
                field_type: SchemaFieldType::Clock {
                    segments: self.xp_track_size(),
                },
                editable: true,
                required: false,
                derived_from: None,
                validation: Some(FieldValidation {
                    min: Some(0),
                    max: Some(self.xp_track_size() as i32),
                    pattern: None,
                    message: None,
                }),
                layout: FieldLayout {
                    column_span: 6,
                    ..Default::default()
                },
                description: Some(self.xp_description().to_string()),
                placeholder: None,
            },
            FieldDefinition {
                id: "HOLD".to_string(),
                label: "Hold".to_string(),
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
                    column_span: 3,
                    ..Default::default()
                },
                description: Some("Spend hold from moves".to_string()),
                placeholder: None,
            },
        ];

        // Add Forward/Ongoing for all PbtA games
        fields.push(FieldDefinition {
            id: "FORWARD".to_string(),
            label: "Forward".to_string(),
            field_type: SchemaFieldType::Integer {
                min: Some(-3),
                max: Some(3),
                show_modifier: true,
            },
            editable: true,
            required: false,
            derived_from: None,
            validation: None,
            layout: FieldLayout {
                column_span: 3,
                ..Default::default()
            },
            description: Some("One-time bonus to your next roll".to_string()),
            placeholder: None,
        });

        fields.push(FieldDefinition {
            id: "ONGOING".to_string(),
            label: "Ongoing".to_string(),
            field_type: SchemaFieldType::Integer {
                min: Some(-3),
                max: Some(3),
                show_modifier: true,
            },
            editable: true,
            required: false,
            derived_from: None,
            validation: None,
            layout: FieldLayout {
                column_span: 3,
                ..Default::default()
            },
            description: Some("Persistent bonus until condition ends".to_string()),
            placeholder: None,
        });

        // Add variant-specific resources
        match self.variant {
            PbtaVariant::ApocalypseWorld => {
                fields.push(FieldDefinition {
                    id: "BARTER".to_string(),
                    label: "Barter".to_string(),
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
                        column_span: 3,
                        ..Default::default()
                    },
                    description: Some("Trade goods and currency".to_string()),
                    placeholder: None,
                });
            }
            PbtaVariant::DungeonWorld => {
                fields.push(FieldDefinition {
                    id: "COIN".to_string(),
                    label: "Coin".to_string(),
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
                        column_span: 3,
                        ..Default::default()
                    },
                    description: Some("Gold coins".to_string()),
                    placeholder: None,
                });
                fields.push(FieldDefinition {
                    id: "LOAD".to_string(),
                    label: "Load".to_string(),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(20),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        column_span: 3,
                        ..Default::default()
                    },
                    description: Some("Current / Max load".to_string()),
                    placeholder: Some("0 / 12".to_string()),
                });
            }
            _ => {}
        }

        SchemaSection {
            id: "resources".to_string(),
            label: "Resources".to_string(),
            section_type: SectionType::Resources,
            fields,
            collapsible: true,
            collapsed_default: false,
            description: None,
        }
    }

    fn bonds_section(&self) -> SchemaSection {
        let bond_label = match self.variant {
            PbtaVariant::ApocalypseWorld => "Hx",
            PbtaVariant::DungeonWorld => "Bonds",
            PbtaVariant::MonsterOfTheWeek => "History",
            PbtaVariant::Generic => "Bonds",
        };

        SchemaSection {
            id: "bonds".to_string(),
            label: bond_label.to_string(),
            section_type: SectionType::Custom,
            fields: vec![
                FieldDefinition {
                    id: "BOND_1".to_string(),
                    label: format!("{} 1", bond_label),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        column_span: 12,
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Relationship with another character...".to_string()),
                },
                FieldDefinition {
                    id: "BOND_2".to_string(),
                    label: format!("{} 2", bond_label),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        column_span: 12,
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Relationship with another character...".to_string()),
                },
                FieldDefinition {
                    id: "BOND_3".to_string(),
                    label: format!("{} 3", bond_label),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        column_span: 12,
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Relationship with another character...".to_string()),
                },
                FieldDefinition {
                    id: "BOND_4".to_string(),
                    label: format!("{} 4", bond_label),
                    field_type: SchemaFieldType::Text {
                        multiline: false,
                        max_length: Some(200),
                    },
                    editable: true,
                    required: false,
                    derived_from: None,
                    validation: None,
                    layout: FieldLayout {
                        column_span: 12,
                        ..Default::default()
                    },
                    description: None,
                    placeholder: Some("Relationship with another character...".to_string()),
                },
            ],
            collapsible: true,
            collapsed_default: false,
            description: Some(self.bonds_description().to_string()),
        }
    }

    // Helper methods for variant-specific content

    fn playbook_label(&self) -> &str {
        match self.variant {
            PbtaVariant::DungeonWorld => "Class",
            _ => "Playbook",
        }
    }

    fn playbook_description(&self) -> &str {
        match self.variant {
            PbtaVariant::ApocalypseWorld => "Your character archetype in the apocalypse",
            PbtaVariant::DungeonWorld => "Your adventuring class",
            PbtaVariant::MonsterOfTheWeek => "Your hunter type",
            PbtaVariant::Generic => "Your character archetype",
        }
    }

    fn get_playbook_options(&self) -> Vec<SchemaSelectOption> {
        match self.variant {
            PbtaVariant::ApocalypseWorld => vec![
                SchemaSelectOption {
                    value: "angel".to_string(),
                    label: "The Angel".to_string(),
                    description: Some("A healer and medic".to_string()),
                },
                SchemaSelectOption {
                    value: "battlebabe".to_string(),
                    label: "The Battlebabe".to_string(),
                    description: Some("A dangerous and sexy warrior".to_string()),
                },
                SchemaSelectOption {
                    value: "brainer".to_string(),
                    label: "The Brainer".to_string(),
                    description: Some("A psychic weirdo".to_string()),
                },
                SchemaSelectOption {
                    value: "chopper".to_string(),
                    label: "The Chopper".to_string(),
                    description: Some("A biker gang leader".to_string()),
                },
                SchemaSelectOption {
                    value: "driver".to_string(),
                    label: "The Driver".to_string(),
                    description: Some("A road warrior".to_string()),
                },
                SchemaSelectOption {
                    value: "gunlugger".to_string(),
                    label: "The Gunlugger".to_string(),
                    description: Some("A walking armory".to_string()),
                },
                SchemaSelectOption {
                    value: "hardholder".to_string(),
                    label: "The Hardholder".to_string(),
                    description: Some("A warlord with a holding".to_string()),
                },
                SchemaSelectOption {
                    value: "hocus".to_string(),
                    label: "The Hocus".to_string(),
                    description: Some("A cult leader".to_string()),
                },
                SchemaSelectOption {
                    value: "maestrod".to_string(),
                    label: "The Maestro D'".to_string(),
                    description: Some("A proprietor of entertainment".to_string()),
                },
                SchemaSelectOption {
                    value: "savvyhead".to_string(),
                    label: "The Savvyhead".to_string(),
                    description: Some("A techie and mechanic".to_string()),
                },
                SchemaSelectOption {
                    value: "skinner".to_string(),
                    label: "The Skinner".to_string(),
                    description: Some("An artist and performer".to_string()),
                },
            ],
            PbtaVariant::DungeonWorld => vec![
                SchemaSelectOption {
                    value: "bard".to_string(),
                    label: "Bard".to_string(),
                    description: Some("A storyteller and performer".to_string()),
                },
                SchemaSelectOption {
                    value: "cleric".to_string(),
                    label: "Cleric".to_string(),
                    description: Some("A servant of the gods".to_string()),
                },
                SchemaSelectOption {
                    value: "druid".to_string(),
                    label: "Druid".to_string(),
                    description: Some("A shapeshifter and nature's champion".to_string()),
                },
                SchemaSelectOption {
                    value: "fighter".to_string(),
                    label: "Fighter".to_string(),
                    description: Some("A master of combat".to_string()),
                },
                SchemaSelectOption {
                    value: "paladin".to_string(),
                    label: "Paladin".to_string(),
                    description: Some("A holy warrior".to_string()),
                },
                SchemaSelectOption {
                    value: "ranger".to_string(),
                    label: "Ranger".to_string(),
                    description: Some("A tracker and hunter".to_string()),
                },
                SchemaSelectOption {
                    value: "thief".to_string(),
                    label: "Thief".to_string(),
                    description: Some("A cunning rogue".to_string()),
                },
                SchemaSelectOption {
                    value: "wizard".to_string(),
                    label: "Wizard".to_string(),
                    description: Some("A wielder of arcane magic".to_string()),
                },
            ],
            PbtaVariant::MonsterOfTheWeek => vec![
                SchemaSelectOption {
                    value: "chosen".to_string(),
                    label: "The Chosen".to_string(),
                    description: Some("Destined to fight evil".to_string()),
                },
                SchemaSelectOption {
                    value: "crooked".to_string(),
                    label: "The Crooked".to_string(),
                    description: Some("A criminal with a past".to_string()),
                },
                SchemaSelectOption {
                    value: "divine".to_string(),
                    label: "The Divine".to_string(),
                    description: Some("An agent of a higher power".to_string()),
                },
                SchemaSelectOption {
                    value: "expert".to_string(),
                    label: "The Expert".to_string(),
                    description: Some("The one who knows things".to_string()),
                },
                SchemaSelectOption {
                    value: "flake".to_string(),
                    label: "The Flake".to_string(),
                    description: Some("A conspiracy theorist who's right".to_string()),
                },
                SchemaSelectOption {
                    value: "initiate".to_string(),
                    label: "The Initiate".to_string(),
                    description: Some("Member of a secret society".to_string()),
                },
                SchemaSelectOption {
                    value: "monstrous".to_string(),
                    label: "The Monstrous".to_string(),
                    description: Some("A monster fighting for good".to_string()),
                },
                SchemaSelectOption {
                    value: "mundane".to_string(),
                    label: "The Mundane".to_string(),
                    description: Some("An ordinary person in an extraordinary world".to_string()),
                },
                SchemaSelectOption {
                    value: "professional".to_string(),
                    label: "The Professional".to_string(),
                    description: Some("A monster hunting organization member".to_string()),
                },
                SchemaSelectOption {
                    value: "spellslinger".to_string(),
                    label: "The Spell-slinger".to_string(),
                    description: Some("A practitioner of magic".to_string()),
                },
                SchemaSelectOption {
                    value: "spooky".to_string(),
                    label: "The Spooky".to_string(),
                    description: Some("Touched by dark powers".to_string()),
                },
                SchemaSelectOption {
                    value: "wronged".to_string(),
                    label: "The Wronged".to_string(),
                    description: Some("Seeking vengeance for a loss".to_string()),
                },
            ],
            PbtaVariant::Generic => vec![SchemaSelectOption {
                value: "custom".to_string(),
                label: "Custom Playbook".to_string(),
                description: Some("Define your own archetype".to_string()),
            }],
        }
    }

    fn stats_description(&self) -> &str {
        match self.variant {
            PbtaVariant::DungeonWorld => {
                "Assign scores using the standard array: 16, 15, 13, 12, 9, 8"
            }
            _ => "Assign stats according to your playbook. Stats range from -1 to +3.",
        }
    }

    fn stat_description(&self, stat: &str) -> Option<&str> {
        match self.variant {
            PbtaVariant::ApocalypseWorld => match stat {
                "Cool" => Some("Keep your cool under pressure"),
                "Hard" => Some("Use violence and intimidation"),
                "Hot" => Some("Seduce and manipulate"),
                "Sharp" => Some("Perceive and understand"),
                "Weird" => Some("Connect to the psychic maelstrom"),
                _ => None,
            },
            PbtaVariant::MonsterOfTheWeek => match stat {
                "Charm" => Some("Manipulate and influence"),
                "Cool" => Some("Stay calm under pressure"),
                "Sharp" => Some("Investigate and perceive"),
                "Tough" => Some("Fight and endure"),
                "Weird" => Some("Use magic and the supernatural"),
                _ => None,
            },
            _ => None,
        }
    }

    fn harm_label(&self) -> &str {
        match self.variant {
            PbtaVariant::DungeonWorld => "Hit Points",
            _ => "Harm",
        }
    }

    fn harm_description(&self) -> &str {
        match self.variant {
            PbtaVariant::ApocalypseWorld => {
                "Track harm on the clock. At 6, you're at death's door."
            }
            PbtaVariant::DungeonWorld => {
                "Your hit points. When you reach 0, take your Last Breath."
            }
            PbtaVariant::MonsterOfTheWeek => {
                "Track harm. At 4+, you're unstable. At 7, you're dying."
            }
            PbtaVariant::Generic => "Track harm as you take damage.",
        }
    }

    fn xp_track_size(&self) -> u8 {
        match self.variant {
            PbtaVariant::DungeonWorld => 8, // Level + 7 at level 1
            _ => 5,                         // Most PbtA games use 5
        }
    }

    fn xp_description(&self) -> &str {
        match self.variant {
            PbtaVariant::DungeonWorld => "Mark XP when you fail a roll or at end of session",
            PbtaVariant::MonsterOfTheWeek => "Mark XP on a miss or end of mystery",
            _ => "Mark XP when you fail a roll",
        }
    }

    fn bonds_description(&self) -> &str {
        match self.variant {
            PbtaVariant::ApocalypseWorld => {
                "Hx represents how well you know other characters. Higher is better for helping."
            }
            PbtaVariant::DungeonWorld => "Bonds describe your relationships with other characters.",
            PbtaVariant::MonsterOfTheWeek => "History with other hunters in your team.",
            PbtaVariant::Generic => "Your relationships with other characters.",
        }
    }

    fn modifiers_section(&self) -> SchemaSection {
        let description = match self.variant {
            PbtaVariant::ApocalypseWorld => "Track debilities (Shattered, Crippled, Disfigured, Broken) and ongoing effects affecting your moves.",
            PbtaVariant::DungeonWorld => "Track debilities (Weak, Shaky, Sick, Stunned, Confused, Scarred) affecting your stat modifiers.",
            PbtaVariant::MonsterOfTheWeek => "Track conditions and ongoing effects affecting your hunter.",
            PbtaVariant::Generic => "Track conditions and ongoing effects affecting your character.",
        };

        SchemaSection {
            id: "modifiers".to_string(),
            label: "Conditions & Effects".to_string(),
            section_type: SectionType::Modifiers,
            fields: vec![FieldDefinition {
                id: "ACTIVE_MODIFIERS".to_string(),
                label: "Active Conditions".to_string(),
                field_type: SchemaFieldType::ModifierList { filter_stat: None },
                editable: false,
                required: false,
                derived_from: None,
                validation: None,
                layout: FieldLayout {
                    column_span: 12,
                    ..Default::default()
                },
                description: Some(
                    "Active conditions, debilities, and ongoing effects modifying your rolls."
                        .to_string(),
                ),
                placeholder: None,
            }],
            collapsible: true,
            collapsed_default: false,
            description: Some(description.to_string()),
        }
    }
}

/// Get Apocalypse World basic moves.
fn apocalypse_world_basic_moves() -> Vec<PbtaMove> {
    vec![
        PbtaMove::new(
            "act_under_fire",
            "Act Under Fire",
            "When you do something under fire, or dig in to endure fire",
            Some("Cool"),
            "You do it",
            "You stumble, hesitate, or flinch: the MC offers you a worse outcome, a hard bargain, or an ugly choice",
        ),
        PbtaMove::new(
            "go_aggro",
            "Go Aggro",
            "When you go aggro on someone",
            Some("Hard"),
            "They have to choose: force your hand and suffer, or comply",
            "They can instead choose: get the hell out, barricade themselves in, give you something they think you want, or back off calmly",
        ),
        PbtaMove::new(
            "seize_by_force",
            "Seize by Force",
            "When you try to seize something by force, or to secure your hold on something",
            Some("Hard"),
            "Choose 3 from the list",
            "Choose 2 from the list",
        ),
        PbtaMove::new(
            "seduce_manipulate",
            "Seduce or Manipulate",
            "When you try to seduce or manipulate someone",
            Some("Hot"),
            "For NPCs: They do it. For PCs: They mark XP if they do it",
            "For NPCs: They'll do it, but need something first. For PCs: They mark XP if they do it",
        ),
        PbtaMove::new(
            "read_person",
            "Read a Person",
            "When you read a person in a charged interaction",
            Some("Sharp"),
            "Hold 3, ask questions",
            "Hold 1, ask questions",
        ),
        PbtaMove::new(
            "read_sitch",
            "Read a Sitch",
            "When you read a charged situation",
            Some("Sharp"),
            "Ask the MC 3 questions",
            "Ask the MC 1 question",
        ),
        PbtaMove::new(
            "open_brain",
            "Open Your Brain",
            "When you open your brain to the world's psychic maelstrom",
            Some("Weird"),
            "The MC tells you something new and interesting, and might ask you a question",
            "The MC tells you something new and interesting, and it's probably distressing or alarming",
        ),
    ]
}

/// Get Dungeon World basic moves.
fn dungeon_world_basic_moves() -> Vec<PbtaMove> {
    vec![
        PbtaMove::new(
            "hack_slash",
            "Hack and Slash",
            "When you attack an enemy in melee",
            Some("STR"),
            "You deal your damage to the enemy and avoid their attack",
            "You deal your damage to the enemy and the enemy makes an attack against you",
        ),
        PbtaMove::new(
            "volley",
            "Volley",
            "When you take aim and shoot at an enemy at range",
            Some("DEX"),
            "You deal your damage",
            "Choose one: move to avoid fire, take what you can get (-1d6 damage), or spend ammo",
        ),
        PbtaMove::new(
            "defy_danger",
            "Defy Danger",
            "When you act despite an imminent threat or suffer a calamity",
            None::<String>, // Varies by approach
            "You do what you set out to, the threat doesn't come to bear",
            "You stumble, hesitate, or flinch: the GM offers a worse outcome, hard bargain, or ugly choice",
        ),
        PbtaMove::new(
            "defend",
            "Defend",
            "When you stand in defense of a person, item, or location under attack",
            Some("CON"),
            "Hold 3",
            "Hold 1",
        ),
        PbtaMove::new(
            "spout_lore",
            "Spout Lore",
            "When you consult your accumulated knowledge about something",
            Some("INT"),
            "The GM tells you something interesting and useful",
            "The GM tells you something interesting—it's on you to make it useful",
        ),
        PbtaMove::new(
            "discern_realities",
            "Discern Realities",
            "When you closely study a situation or person",
            Some("WIS"),
            "Ask the GM 3 questions from the list",
            "Ask the GM 1 question from the list",
        ),
        PbtaMove::new(
            "parley",
            "Parley",
            "When you have leverage on an NPC and manipulate them",
            Some("CHA"),
            "They do what you ask if you first promise what they ask of you",
            "They'll do what you ask, but need concrete assurance first",
        ),
        PbtaMove::new(
            "aid_interfere",
            "Aid or Interfere",
            "When you help or hinder someone",
            Some("Bond"),
            "They take +1 or -2 to their roll (your choice)",
            "They take +1 or -2, but you expose yourself to danger, retribution, or cost",
        ),
    ]
}

/// Get Monster of the Week basic moves.
fn monster_of_the_week_basic_moves() -> Vec<PbtaMove> {
    vec![
        PbtaMove::new(
            "act_under_pressure",
            "Act Under Pressure",
            "When you act under pressure, do something difficult or dangerous",
            Some("Cool"),
            "You do it",
            "The Keeper tells you something bad happens or offers a hard choice",
        ),
        PbtaMove::new(
            "help_out",
            "Help Out",
            "When you help another hunter",
            Some("Cool"),
            "They get +1 to their roll",
            "They get +1, but you also expose yourself to trouble",
        ),
        PbtaMove::new(
            "investigate_mystery",
            "Investigate a Mystery",
            "When you investigate a mystery",
            Some("Sharp"),
            "Hold 2. Ask the Keeper questions.",
            "Hold 1. Ask the Keeper one question.",
        ),
        PbtaMove::new(
            "kick_ass",
            "Kick Some Ass",
            "When you get into a fight",
            Some("Tough"),
            "You and the enemy inflict harm on each other",
            "You and the enemy inflict harm, plus pick one bad thing from the list",
        ),
        PbtaMove::new(
            "manipulate",
            "Manipulate Someone",
            "When you try to manipulate someone",
            Some("Charm"),
            "They do what you want",
            "They'll do it but need something in return",
        ),
        PbtaMove::new(
            "protect",
            "Protect Someone",
            "When you protect someone from harm",
            Some("Tough"),
            "You protect them. Choose an extra benefit.",
            "You protect them, but you suffer harm or are put in danger",
        ),
        PbtaMove::new(
            "read_bad_situation",
            "Read a Bad Situation",
            "When you look around to work out what's going on",
            Some("Sharp"),
            "Hold 3. Ask questions.",
            "Hold 1. Ask one question.",
        ),
        PbtaMove::new(
            "use_magic",
            "Use Magic",
            "When you use magic",
            Some("Weird"),
            "The magic works without issues",
            "It works imperfectly—the Keeper picks one drawback",
        ),
    ]
}

/// Forward/Ongoing modifier.
#[derive(Debug, Clone)]
pub struct PbtaModifier {
    pub value: i8,
    pub modifier_type: ModifierType,
    pub source: String,
    pub applies_to: Option<String>, // Specific move or stat, None = any
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierType {
    /// One-time bonus, consumed after use
    Forward,
    /// Persistent bonus until condition ends
    Ongoing,
}

/// Hold from a move.
#[derive(Debug, Clone)]
pub struct MoveHold {
    pub move_id: String,
    pub amount: u8,
    pub options: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_determination() {
        assert_eq!(PbtaOutcome::from_total(12), PbtaOutcome::FullSuccess);
        assert_eq!(PbtaOutcome::from_total(10), PbtaOutcome::FullSuccess);
        assert_eq!(PbtaOutcome::from_total(9), PbtaOutcome::PartialSuccess);
        assert_eq!(PbtaOutcome::from_total(7), PbtaOutcome::PartialSuccess);
        assert_eq!(PbtaOutcome::from_total(6), PbtaOutcome::Miss);
        assert_eq!(PbtaOutcome::from_total(2), PbtaOutcome::Miss);
    }

    #[test]
    fn outcome_is_success() {
        assert!(PbtaOutcome::FullSuccess.is_success());
        assert!(PbtaOutcome::PartialSuccess.is_success());
        assert!(!PbtaOutcome::Miss.is_success());
    }

    #[test]
    fn variant_system_ids() {
        assert_eq!(PbtaSystem::apocalypse_world().system_id(), "pbta_aw");
        assert_eq!(PbtaSystem::dungeon_world().system_id(), "pbta_dw");
        assert_eq!(PbtaSystem::monster_of_the_week().system_id(), "pbta_motw");
        assert_eq!(PbtaSystem::generic().system_id(), "pbta");
    }

    #[test]
    fn variant_stat_names() {
        let aw = PbtaSystem::apocalypse_world();
        assert!(aw.stat_names().contains(&"Cool"));
        assert!(aw.stat_names().contains(&"Weird"));

        let dw = PbtaSystem::dungeon_world();
        assert!(dw.stat_names().contains(&"STR"));
        assert!(dw.stat_names().contains(&"CHA"));

        let motw = PbtaSystem::monster_of_the_week();
        assert!(motw.stat_names().contains(&"Charm"));
        assert!(motw.stat_names().contains(&"Tough"));
    }

    #[test]
    fn dungeon_world_hp() {
        let system = PbtaSystem::dungeon_world();
        // Fighter: base 10 + CON
        assert_eq!(system.calculate_max_hp(1, "Fighter", 16, 0), 26);
        // Wizard: base 4 + CON
        assert_eq!(system.calculate_max_hp(1, "Wizard", 8, 0), 12);
    }

    #[test]
    fn move_creation() {
        let mv = PbtaMove::new(
            "test",
            "Test Move",
            "When you test",
            Some("Cool"),
            "You succeed",
            "You succeed at a cost",
        )
        .with_miss("The GM makes a hard move");

        assert!(mv.requires_roll());
        assert_eq!(mv.stat, Some("Cool".to_string()));
        assert!(mv.miss.is_some());
    }

    #[test]
    fn basic_moves_exist() {
        let aw = PbtaSystem::apocalypse_world();
        let moves = aw.basic_moves();
        assert!(!moves.is_empty());
        assert!(moves.iter().any(|m| m.id == "act_under_fire"));

        let dw = PbtaSystem::dungeon_world();
        let moves = dw.basic_moves();
        assert!(moves.iter().any(|m| m.id == "hack_slash"));
    }

    // CharacterSheetProvider tests

    #[test]
    fn character_sheet_schema_has_correct_system_info() {
        let aw = PbtaSystem::apocalypse_world();
        let schema = aw.character_sheet_schema();
        assert_eq!(schema.system_id, "pbta_aw");
        assert_eq!(schema.system_name, "Apocalypse World");

        let dw = PbtaSystem::dungeon_world();
        let schema = dw.character_sheet_schema();
        assert_eq!(schema.system_id, "pbta_dw");
        assert_eq!(schema.system_name, "Dungeon World");

        let motw = PbtaSystem::monster_of_the_week();
        let schema = motw.character_sheet_schema();
        assert_eq!(schema.system_id, "pbta_motw");
        assert_eq!(schema.system_name, "Monster of the Week");
    }

    #[test]
    fn character_sheet_has_required_sections() {
        let system = PbtaSystem::apocalypse_world();
        let schema = system.character_sheet_schema();

        let section_ids: Vec<&str> = schema.sections.iter().map(|s| s.id.as_str()).collect();
        assert!(section_ids.contains(&"identity"));
        assert!(section_ids.contains(&"stats"));
        assert!(section_ids.contains(&"harm"));
        assert!(section_ids.contains(&"moves"));
        assert!(section_ids.contains(&"resources"));
        assert!(section_ids.contains(&"bonds"));
    }

    #[test]
    fn apocalypse_world_has_correct_stats() {
        let system = PbtaSystem::apocalypse_world();
        let schema = system.character_sheet_schema();

        let stats_section = schema.sections.iter().find(|s| s.id == "stats").unwrap();
        let stat_ids: Vec<&str> = stats_section.fields.iter().map(|f| f.id.as_str()).collect();

        assert!(stat_ids.contains(&"Cool"));
        assert!(stat_ids.contains(&"Hard"));
        assert!(stat_ids.contains(&"Hot"));
        assert!(stat_ids.contains(&"Sharp"));
        assert!(stat_ids.contains(&"Weird"));
    }

    #[test]
    fn dungeon_world_has_dnd_style_stats() {
        let system = PbtaSystem::dungeon_world();
        let schema = system.character_sheet_schema();

        let stats_section = schema.sections.iter().find(|s| s.id == "stats").unwrap();
        let stat_ids: Vec<&str> = stats_section.fields.iter().map(|f| f.id.as_str()).collect();

        assert!(stat_ids.contains(&"STR"));
        assert!(stat_ids.contains(&"DEX"));
        assert!(stat_ids.contains(&"CON"));
        assert!(stat_ids.contains(&"INT"));
        assert!(stat_ids.contains(&"WIS"));
        assert!(stat_ids.contains(&"CHA"));
        assert!(stat_ids.contains(&"LEVEL"));
        assert!(stat_ids.contains(&"ARMOR"));
    }

    #[test]
    fn monster_of_the_week_has_luck_field() {
        let system = PbtaSystem::monster_of_the_week();
        let schema = system.character_sheet_schema();

        let harm_section = schema.sections.iter().find(|s| s.id == "harm").unwrap();
        let field_ids: Vec<&str> = harm_section.fields.iter().map(|f| f.id.as_str()).collect();

        assert!(field_ids.contains(&"HARM"));
        assert!(field_ids.contains(&"LUCK"));
        assert!(field_ids.contains(&"UNSTABLE"));
    }

    #[test]
    fn dungeon_world_has_hp_fields() {
        let system = PbtaSystem::dungeon_world();
        let schema = system.character_sheet_schema();

        let harm_section = schema.sections.iter().find(|s| s.id == "harm").unwrap();
        let field_ids: Vec<&str> = harm_section.fields.iter().map(|f| f.id.as_str()).collect();

        assert!(field_ids.contains(&"CURRENT_HP"));
        assert!(field_ids.contains(&"MAX_HP"));
        assert!(field_ids.contains(&"DAMAGE_DIE"));
    }

    #[test]
    fn default_values_are_set_correctly() {
        let aw = PbtaSystem::apocalypse_world();
        let defaults = aw.default_values();

        assert_eq!(defaults.get("Cool"), Some(&SheetValue::Integer(0)));
        assert_eq!(defaults.get("Hard"), Some(&SheetValue::Integer(0)));
        assert_eq!(defaults.get("XP"), Some(&SheetValue::Integer(0)));
        assert_eq!(defaults.get("HOLD"), Some(&SheetValue::Integer(0)));

        let dw = PbtaSystem::dungeon_world();
        let defaults = dw.default_values();

        assert_eq!(defaults.get("STR"), Some(&SheetValue::Integer(10)));
        assert_eq!(defaults.get("LEVEL"), Some(&SheetValue::Integer(1)));
        assert_eq!(defaults.get("ARMOR"), Some(&SheetValue::Integer(0)));
    }

    #[test]
    fn validate_pbta_stats() {
        let system = PbtaSystem::apocalypse_world();
        let all_values = HashMap::new();

        // Valid stat
        assert!(system
            .validate_field("Cool", &SheetValue::Integer(2), &all_values)
            .is_none());
        assert!(system
            .validate_field("Cool", &SheetValue::Integer(-1), &all_values)
            .is_none());

        // Invalid stat (out of range)
        assert!(system
            .validate_field("Cool", &SheetValue::Integer(5), &all_values)
            .is_some());
        assert!(system
            .validate_field("Cool", &SheetValue::Integer(-3), &all_values)
            .is_some());
    }

    #[test]
    fn validate_dungeon_world_ability_scores() {
        let system = PbtaSystem::dungeon_world();
        let all_values = HashMap::new();

        // Valid score
        assert!(system
            .validate_field("STR", &SheetValue::Integer(16), &all_values)
            .is_none());

        // Invalid score (out of range)
        assert!(system
            .validate_field("STR", &SheetValue::Integer(20), &all_values)
            .is_some());
        assert!(system
            .validate_field("STR", &SheetValue::Integer(2), &all_values)
            .is_some());
    }

    #[test]
    fn calculate_derived_values_aw() {
        let system = PbtaSystem::apocalypse_world();
        let mut values = HashMap::new();
        values.insert("Cool".to_string(), SheetValue::Integer(2));
        values.insert("XP".to_string(), SheetValue::Integer(3));

        let derived = system.calculate_derived_values(&values);

        assert_eq!(derived.get("Cool_MOD"), Some(&SheetValue::Integer(2)));
        assert_eq!(derived.get("XP_MAX"), Some(&SheetValue::Integer(5)));
        assert_eq!(derived.get("XP_REMAINING"), Some(&SheetValue::Integer(2)));
    }

    #[test]
    fn calculate_derived_values_dw() {
        let system = PbtaSystem::dungeon_world();
        let mut values = HashMap::new();
        values.insert("STR".to_string(), SheetValue::Integer(16));
        values.insert("CON".to_string(), SheetValue::Integer(14));
        values.insert(
            "PLAYBOOK".to_string(),
            SheetValue::String("fighter".to_string()),
        );
        values.insert("LEVEL".to_string(), SheetValue::Integer(1));
        values.insert("XP".to_string(), SheetValue::Integer(2));

        let derived = system.calculate_derived_values(&values);

        // STR 16 -> modifier +3
        assert_eq!(derived.get("STR_MOD"), Some(&SheetValue::Integer(3)));
        // CON 14 -> modifier +2
        assert_eq!(derived.get("CON_MOD"), Some(&SheetValue::Integer(2)));
        // Fighter base 10 + CON 14 = 24
        assert_eq!(derived.get("MAX_HP"), Some(&SheetValue::Integer(24)));
        // Level 1 + 7 = 8 XP to level
        assert_eq!(derived.get("XP_MAX"), Some(&SheetValue::Integer(8)));
    }

    #[test]
    fn creation_steps_are_ordered() {
        let system = PbtaSystem::generic();
        let schema = system.character_sheet_schema();

        assert!(!schema.creation_steps.is_empty());

        // Verify all steps have valid IDs (order is implicit by vector position)
        for (i, step) in schema.creation_steps.iter().enumerate() {
            assert!(!step.id.is_empty(), "Step {} should have a non-empty id", i);
        }
    }

    #[test]
    fn playbook_options_vary_by_variant() {
        let aw = PbtaSystem::apocalypse_world();
        let aw_options = aw.get_playbook_options();
        assert!(aw_options.iter().any(|o| o.value == "angel"));
        assert!(aw_options.iter().any(|o| o.value == "battlebabe"));

        let dw = PbtaSystem::dungeon_world();
        let dw_options = dw.get_playbook_options();
        assert!(dw_options.iter().any(|o| o.value == "fighter"));
        assert!(dw_options.iter().any(|o| o.value == "wizard"));

        let motw = PbtaSystem::monster_of_the_week();
        let motw_options = motw.get_playbook_options();
        assert!(motw_options.iter().any(|o| o.value == "chosen"));
        assert!(motw_options.iter().any(|o| o.value == "spooky"));
    }

    #[test]
    fn resources_section_has_forward_and_ongoing() {
        let system = PbtaSystem::apocalypse_world();
        let schema = system.character_sheet_schema();

        let resources = schema
            .sections
            .iter()
            .find(|s| s.id == "resources")
            .unwrap();
        let field_ids: Vec<&str> = resources.fields.iter().map(|f| f.id.as_str()).collect();

        assert!(field_ids.contains(&"XP"));
        assert!(field_ids.contains(&"HOLD"));
        assert!(field_ids.contains(&"FORWARD"));
        assert!(field_ids.contains(&"ONGOING"));
    }

    #[test]
    fn apocalypse_world_has_barter() {
        let system = PbtaSystem::apocalypse_world();
        let schema = system.character_sheet_schema();

        let resources = schema
            .sections
            .iter()
            .find(|s| s.id == "resources")
            .unwrap();
        let field_ids: Vec<&str> = resources.fields.iter().map(|f| f.id.as_str()).collect();

        assert!(field_ids.contains(&"BARTER"));
    }

    #[test]
    fn dungeon_world_has_coin_and_load() {
        let system = PbtaSystem::dungeon_world();
        let schema = system.character_sheet_schema();

        let resources = schema
            .sections
            .iter()
            .find(|s| s.id == "resources")
            .unwrap();
        let field_ids: Vec<&str> = resources.fields.iter().map(|f| f.id.as_str()).collect();

        assert!(field_ids.contains(&"COIN"));
        assert!(field_ids.contains(&"LOAD"));
    }

    #[test]
    fn bonds_section_varies_by_variant() {
        let aw = PbtaSystem::apocalypse_world();
        let aw_schema = aw.character_sheet_schema();
        let aw_bonds = aw_schema.sections.iter().find(|s| s.id == "bonds").unwrap();
        assert_eq!(aw_bonds.label, "Hx");

        let dw = PbtaSystem::dungeon_world();
        let dw_schema = dw.character_sheet_schema();
        let dw_bonds = dw_schema.sections.iter().find(|s| s.id == "bonds").unwrap();
        assert_eq!(dw_bonds.label, "Bonds");

        let motw = PbtaSystem::monster_of_the_week();
        let motw_schema = motw.character_sheet_schema();
        let motw_bonds = motw_schema
            .sections
            .iter()
            .find(|s| s.id == "bonds")
            .unwrap();
        assert_eq!(motw_bonds.label, "History");
    }
}
