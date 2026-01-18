//! System-agnostic rule configuration types
//!
//! Supports multiple TTRPG systems through presets and customization.

use serde::{Deserialize, Serialize};

/// The type of rule system (determines dice mechanics and success calculation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RuleSystemType {
    /// Roll d20 + modifier vs DC (D&D, Pathfinder)
    #[default]
    D20,
    /// Roll d100 under skill value (Call of Cthulhu, RuneQuest)
    D100,
    /// Fiction-first with descriptive outcomes (Kids on Bikes, FATE, PbtA)
    Narrative,
    /// User-defined dice mechanics
    Custom,
    /// Unknown system type (for forward compatibility)
    #[serde(other)]
    Unknown,
}

/// Known presets for rule systems
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RuleSystemVariant {
    // D20 variants
    Dnd5e,
    Pathfinder2e,
    #[default]
    GenericD20,
    // D100 variants
    CallOfCthulhu7e,
    RuneQuest,
    GenericD100,
    // Narrative variants
    KidsOnBikes,
    FateCore,
    PoweredByApocalypse,
    BladesInTheDark,
    // Custom
    Custom(String),
    // Unknown for forward compatibility
    // Note: #[serde(other)] doesn't work with tuple variants, so Unknown will
    // only match if explicitly sent as "unknown" in JSON
    Unknown,
}

impl RuleSystemVariant {
    /// Get the display name for this variant
    pub fn display_name(&self) -> &str {
        match self {
            Self::Dnd5e => "D&D 5th Edition",
            Self::Pathfinder2e => "Pathfinder 2nd Edition",
            Self::GenericD20 => "Generic D20 System",
            Self::CallOfCthulhu7e => "Call of Cthulhu 7th Edition",
            Self::RuneQuest => "RuneQuest",
            Self::GenericD100 => "Generic D100 System",
            Self::KidsOnBikes => "Kids on Bikes",
            Self::FateCore => "FATE Core",
            Self::PoweredByApocalypse => "Powered by the Apocalypse",
            Self::BladesInTheDark => "Blades in the Dark",
            Self::Custom(name) => name,
            Self::Unknown => "Unknown",
        }
    }

    /// Get the rule system type for this variant
    pub fn system_type(&self) -> RuleSystemType {
        match self {
            Self::Dnd5e | Self::Pathfinder2e | Self::GenericD20 => RuleSystemType::D20,
            Self::CallOfCthulhu7e | Self::RuneQuest | Self::GenericD100 => RuleSystemType::D100,
            Self::KidsOnBikes
            | Self::FateCore
            | Self::PoweredByApocalypse
            | Self::BladesInTheDark => RuleSystemType::Narrative,
            Self::Custom(_) | Self::Unknown => RuleSystemType::Unknown,
        }
    }

    /// Get all variants for a given system type
    pub fn variants_for_type(system_type: RuleSystemType) -> Vec<Self> {
        match system_type {
            RuleSystemType::D20 => vec![Self::Dnd5e, Self::Pathfinder2e, Self::GenericD20],
            RuleSystemType::D100 => vec![Self::CallOfCthulhu7e, Self::RuneQuest, Self::GenericD100],
            RuleSystemType::Narrative => {
                vec![
                    Self::KidsOnBikes,
                    Self::FateCore,
                    Self::PoweredByApocalypse,
                    Self::BladesInTheDark,
                ]
            }
            RuleSystemType::Custom | RuleSystemType::Unknown => vec![],
        }
    }
}

/// Configuration for a game's rule system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleSystemConfig {
    /// Display name for this configuration
    pub name: String,
    /// Description of how the system works
    pub description: String,
    /// The type of rule system
    pub system_type: RuleSystemType,
    /// The specific variant/preset used
    pub variant: RuleSystemVariant,
    /// Character stat definitions
    pub stat_definitions: Vec<StatDefinition>,
    /// The dice system used for resolution
    pub dice_system: DiceSystem,
    /// How success/failure is determined
    pub success_comparison: SuccessComparison,
    /// Formula for skill checks (display only)
    pub skill_check_formula: String,
    /// Configuration for narrative resolution systems (PbtA, Fate, Blades)
    /// Only used when system_type is Narrative
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrative_config: Option<NarrativeResolutionConfig>,
}

impl Default for RuleSystemConfig {
    fn default() -> Self {
        Self::from_variant(RuleSystemVariant::GenericD20)
    }
}

impl RuleSystemConfig {
    /// Create a configuration from a preset variant
    pub fn from_variant(variant: RuleSystemVariant) -> Self {
        match variant {
            RuleSystemVariant::Dnd5e => Self::dnd_5e(),
            RuleSystemVariant::Pathfinder2e => Self::pathfinder_2e(),
            RuleSystemVariant::GenericD20 => Self::generic_d20(),
            RuleSystemVariant::CallOfCthulhu7e => Self::call_of_cthulhu_7e(),
            RuleSystemVariant::RuneQuest => Self::runequest(),
            RuleSystemVariant::GenericD100 => Self::generic_d100(),
            RuleSystemVariant::KidsOnBikes => Self::kids_on_bikes(),
            RuleSystemVariant::FateCore => Self::fate_core(),
            RuleSystemVariant::PoweredByApocalypse => Self::powered_by_apocalypse(),
            RuleSystemVariant::BladesInTheDark => Self::blades_in_the_dark(),
            RuleSystemVariant::Custom(name) => Self::custom(name),
            RuleSystemVariant::Unknown => Self::generic_d20(), // Default to generic D20 for unknown
        }
    }

    /// D&D 5th Edition preset
    pub fn dnd_5e() -> Self {
        Self {
            name: "D&D 5th Edition".to_string(),
            system_type: RuleSystemType::D20,
            variant: RuleSystemVariant::Dnd5e,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 20, 10),
                StatDefinition::new("Dexterity", "DEX", 1, 20, 10),
                StatDefinition::new("Constitution", "CON", 1, 20, 10),
                StatDefinition::new("Intelligence", "INT", 1, 20, 10),
                StatDefinition::new("Wisdom", "WIS", 1, 20, 10),
                StatDefinition::new("Charisma", "CHA", 1, 20, 10),
            ],
            dice_system: DiceSystem::D20,
            success_comparison: SuccessComparison::GreaterOrEqual,
            skill_check_formula: "1d20 + ability modifier + proficiency (if proficient)"
                .to_string(),
            description: "Roll d20, add modifiers. Meet or beat the DC to succeed.".to_string(),
            narrative_config: None,
        }
    }

    /// Pathfinder 2e preset
    pub fn pathfinder_2e() -> Self {
        Self {
            name: "Pathfinder 2nd Edition".to_string(),
            system_type: RuleSystemType::D20,
            variant: RuleSystemVariant::Pathfinder2e,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 20, 10),
                StatDefinition::new("Dexterity", "DEX", 1, 20, 10),
                StatDefinition::new("Constitution", "CON", 1, 20, 10),
                StatDefinition::new("Intelligence", "INT", 1, 20, 10),
                StatDefinition::new("Wisdom", "WIS", 1, 20, 10),
                StatDefinition::new("Charisma", "CHA", 1, 20, 10),
            ],
            dice_system: DiceSystem::D20,
            success_comparison: SuccessComparison::GreaterOrEqual,
            skill_check_formula: "1d20 + modifier vs DC (4 degrees of success)".to_string(),
            description: "Roll d20 + modifier. Crit success on DC+10, crit fail on DC-10."
                .to_string(),
            narrative_config: None,
        }
    }

    /// Generic D20 preset
    pub fn generic_d20() -> Self {
        Self {
            name: "Generic D20 System".to_string(),
            system_type: RuleSystemType::D20,
            variant: RuleSystemVariant::GenericD20,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 20, 10),
                StatDefinition::new("Dexterity", "DEX", 1, 20, 10),
                StatDefinition::new("Constitution", "CON", 1, 20, 10),
                StatDefinition::new("Intelligence", "INT", 1, 20, 10),
                StatDefinition::new("Wisdom", "WIS", 1, 20, 10),
                StatDefinition::new("Charisma", "CHA", 1, 20, 10),
            ],
            dice_system: DiceSystem::D20,
            success_comparison: SuccessComparison::GreaterOrEqual,
            skill_check_formula: "1d20 + modifier vs DC".to_string(),
            description: "Roll d20, add modifiers. Meet or beat the DC to succeed.".to_string(),
            narrative_config: None,
        }
    }

    /// Call of Cthulhu 7e preset
    pub fn call_of_cthulhu_7e() -> Self {
        Self {
            name: "Call of Cthulhu 7th Edition".to_string(),
            system_type: RuleSystemType::D100,
            variant: RuleSystemVariant::CallOfCthulhu7e,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 100, 50),
                StatDefinition::new("Constitution", "CON", 1, 100, 50),
                StatDefinition::new("Size", "SIZ", 1, 100, 50),
                StatDefinition::new("Dexterity", "DEX", 1, 100, 50),
                StatDefinition::new("Appearance", "APP", 1, 100, 50),
                StatDefinition::new("Intelligence", "INT", 1, 100, 50),
                StatDefinition::new("Power", "POW", 1, 100, 50),
                StatDefinition::new("Education", "EDU", 1, 100, 50),
                StatDefinition::new("Luck", "LCK", 1, 100, 50),
            ],
            dice_system: DiceSystem::D100,
            success_comparison: SuccessComparison::LessOrEqual,
            skill_check_formula: "Roll d100 ≤ skill value".to_string(),
            description: "Roll d100. Regular success ≤ skill, Hard ≤ half, Extreme ≤ fifth."
                .to_string(),
            narrative_config: None,
        }
    }

    /// RuneQuest preset
    pub fn runequest() -> Self {
        Self {
            name: "RuneQuest".to_string(),
            system_type: RuleSystemType::D100,
            variant: RuleSystemVariant::RuneQuest,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 21, 10),
                StatDefinition::new("Constitution", "CON", 1, 21, 10),
                StatDefinition::new("Size", "SIZ", 1, 21, 10),
                StatDefinition::new("Dexterity", "DEX", 1, 21, 10),
                StatDefinition::new("Intelligence", "INT", 1, 21, 10),
                StatDefinition::new("Power", "POW", 1, 21, 10),
                StatDefinition::new("Charisma", "CHA", 1, 21, 10),
            ],
            dice_system: DiceSystem::D100,
            success_comparison: SuccessComparison::LessOrEqual,
            skill_check_formula: "Roll d100 ≤ skill value".to_string(),
            description: "Roll d100 under skill. Critical on 1/20th, special on 1/5th.".to_string(),
            narrative_config: None,
        }
    }

    /// Generic D100 preset
    pub fn generic_d100() -> Self {
        Self {
            name: "Generic D100 System".to_string(),
            system_type: RuleSystemType::D100,
            variant: RuleSystemVariant::GenericD100,
            stat_definitions: vec![
                StatDefinition::new("Strength", "STR", 1, 100, 50),
                StatDefinition::new("Dexterity", "DEX", 1, 100, 50),
                StatDefinition::new("Constitution", "CON", 1, 100, 50),
                StatDefinition::new("Intelligence", "INT", 1, 100, 50),
                StatDefinition::new("Wisdom", "WIS", 1, 100, 50),
                StatDefinition::new("Charisma", "CHA", 1, 100, 50),
            ],
            dice_system: DiceSystem::D100,
            success_comparison: SuccessComparison::LessOrEqual,
            skill_check_formula: "Roll d100 ≤ skill value".to_string(),
            description: "Roll d100 and compare to skill value. Lower is better.".to_string(),
            narrative_config: None,
        }
    }

    /// Kids on Bikes preset
    pub fn kids_on_bikes() -> Self {
        Self {
            name: "Kids on Bikes".to_string(),
            system_type: RuleSystemType::Narrative,
            variant: RuleSystemVariant::KidsOnBikes,
            stat_definitions: vec![
                StatDefinition::new("Brains", "BRN", 4, 20, 8),
                StatDefinition::new("Brawn", "BRW", 4, 20, 8),
                StatDefinition::new("Fight", "FGT", 4, 20, 8),
                StatDefinition::new("Flight", "FLT", 4, 20, 8),
                StatDefinition::new("Charm", "CHM", 4, 20, 8),
                StatDefinition::new("Grit", "GRT", 4, 20, 8),
            ],
            dice_system: DiceSystem::Custom("Variable die (d4-d20)".to_string()),
            success_comparison: SuccessComparison::Narrative,
            skill_check_formula: "Roll stat die vs difficulty (1-6 scale)".to_string(),
            description: "Roll your stat die. Higher stat = bigger die. Narrative outcomes."
                .to_string(),
            // Kids on Bikes uses a custom system, default to PbtA-like
            narrative_config: Some(NarrativeResolutionConfig {
                style: NarrativeResolutionStyle::Custom,
                ..Default::default()
            }),
        }
    }

    /// FATE Core preset
    pub fn fate_core() -> Self {
        Self {
            name: "FATE Core".to_string(),
            system_type: RuleSystemType::Narrative,
            variant: RuleSystemVariant::FateCore,
            stat_definitions: vec![
                StatDefinition::new("Careful", "CAR", 0, 4, 1),
                StatDefinition::new("Clever", "CLV", 0, 4, 1),
                StatDefinition::new("Flashy", "FLS", 0, 4, 1),
                StatDefinition::new("Forceful", "FRC", 0, 4, 1),
                StatDefinition::new("Quick", "QCK", 0, 4, 1),
                StatDefinition::new("Sneaky", "SNK", 0, 4, 1),
            ],
            dice_system: DiceSystem::Fate,
            success_comparison: SuccessComparison::Narrative,
            skill_check_formula: "4dF + approach vs difficulty ladder".to_string(),
            description: "Roll 4 Fate dice (+/-/blank) + approach. Compare to ladder.".to_string(),
            narrative_config: Some(NarrativeResolutionConfig::fate_core()),
        }
    }

    /// Powered by the Apocalypse preset
    pub fn powered_by_apocalypse() -> Self {
        Self {
            name: "Powered by the Apocalypse".to_string(),
            system_type: RuleSystemType::Narrative,
            variant: RuleSystemVariant::PoweredByApocalypse,
            stat_definitions: vec![
                StatDefinition::new("Cool", "COL", -2, 3, 0),
                StatDefinition::new("Hard", "HRD", -2, 3, 0),
                StatDefinition::new("Hot", "HOT", -2, 3, 0),
                StatDefinition::new("Sharp", "SHP", -2, 3, 0),
                StatDefinition::new("Weird", "WRD", -2, 3, 0),
            ],
            dice_system: DiceSystem::Custom("2d6".to_string()),
            success_comparison: SuccessComparison::Narrative,
            skill_check_formula: "2d6 + stat: 10+ full success, 7-9 partial, 6- miss".to_string(),
            description: "Roll 2d6 + stat. 10+ success, 7-9 success with cost, 6- trouble."
                .to_string(),
            narrative_config: Some(NarrativeResolutionConfig::pbta()),
        }
    }

    /// Blades in the Dark preset
    pub fn blades_in_the_dark() -> Self {
        Self {
            name: "Blades in the Dark".to_string(),
            system_type: RuleSystemType::Narrative,
            variant: RuleSystemVariant::BladesInTheDark,
            stat_definitions: vec![
                // Actions are grouped by attributes
                // Insight
                StatDefinition::new("Hunt", "HNT", 0, 4, 0),
                StatDefinition::new("Study", "STD", 0, 4, 0),
                StatDefinition::new("Survey", "SRV", 0, 4, 0),
                StatDefinition::new("Tinker", "TNK", 0, 4, 0),
                // Prowess
                StatDefinition::new("Finesse", "FNS", 0, 4, 0),
                StatDefinition::new("Prowl", "PRW", 0, 4, 0),
                StatDefinition::new("Skirmish", "SKR", 0, 4, 0),
                StatDefinition::new("Wreck", "WRK", 0, 4, 0),
                // Resolve
                StatDefinition::new("Attune", "ATN", 0, 4, 0),
                StatDefinition::new("Command", "CMD", 0, 4, 0),
                StatDefinition::new("Consort", "CNS", 0, 4, 0),
                StatDefinition::new("Sway", "SWY", 0, 4, 0),
            ],
            dice_system: DiceSystem::DicePool {
                die_type: 6,
                success_threshold: 6,
            },
            success_comparison: SuccessComparison::Narrative,
            skill_check_formula: "d6 pool (action rating): 6=success, 4-5=partial, 1-3=failure"
                .to_string(),
            description:
                "Roll d6 pool equal to action rating. Position sets risk, Effect sets impact."
                    .to_string(),
            narrative_config: Some(NarrativeResolutionConfig::blades()),
        }
    }

    /// Custom system (blank slate)
    pub fn custom(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            system_type: RuleSystemType::Custom,
            variant: RuleSystemVariant::Custom("Custom".to_string()),
            stat_definitions: vec![],
            dice_system: DiceSystem::Custom("Custom".to_string()),
            success_comparison: SuccessComparison::Narrative,
            skill_check_formula: "Custom resolution".to_string(),
            description: "A custom rule system. Define your own stats and mechanics.".to_string(),
            narrative_config: Some(NarrativeResolutionConfig::default()),
        }
    }

    /// Get the narrative resolution config, or a default if not set
    pub fn narrative_config_or_default(&self) -> NarrativeResolutionConfig {
        self.narrative_config.clone().unwrap_or_default()
    }
}

/// How success is determined
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuccessComparison {
    /// Roll must be >= target (D20 systems)
    GreaterOrEqual,
    /// Roll must be <= target (D100 systems)
    LessOrEqual,
    /// Success is determined narratively
    Narrative,
    /// Unknown comparison type (for forward compatibility)
    #[serde(other)]
    Unknown,
}

/// Definition of a character stat
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatDefinition {
    pub name: String,
    pub abbreviation: String,
    pub min_value: i32,
    pub max_value: i32,
    pub default_value: i32,
}

impl StatDefinition {
    pub fn new(
        name: impl Into<String>,
        abbreviation: impl Into<String>,
        min_value: i32,
        max_value: i32,
        default_value: i32,
    ) -> Self {
        Self {
            name: name.into(),
            abbreviation: abbreviation.into(),
            min_value,
            max_value,
            default_value,
        }
    }
}

/// The dice system used for resolution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiceSystem {
    /// Classic d20 system (D&D, Pathfinder)
    D20,
    /// Percentile system (Call of Cthulhu)
    D100,
    /// Dice pool system (World of Darkness)
    DicePool { die_type: u8, success_threshold: u8 },
    /// FATE/Fudge dice
    Fate,
    /// Custom dice expression
    Custom(String),
}

// =============================================================================
// Narrative Resolution System
// =============================================================================

/// Configuration for narrative/fiction-first resolution systems.
///
/// Supports three major styles:
/// - PbtA: Fixed thresholds (10+/7-9/6-), 2d6+stat
/// - Ladder: Descriptor maps to target number, NdF+skill vs target (Fate)
/// - Blades: Position determines consequences, Effect determines progress
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrativeResolutionConfig {
    /// The narrative resolution style
    pub style: NarrativeResolutionStyle,

    /// Thresholds for outcome tiers (used by PbtA and Custom styles)
    #[serde(default)]
    pub thresholds: NarrativeThresholds,

    /// Difficulty ladder mapping descriptors to target values (used by Ladder style)
    #[serde(default)]
    pub ladder: DifficultyLadder,

    /// Dice configuration for narrative systems
    #[serde(default)]
    pub dice_config: NarrativeDiceConfig,

    /// Position/Effect configuration (used by Blades style)
    #[serde(default)]
    pub position_effect: PositionEffectConfig,
}

impl Default for NarrativeResolutionConfig {
    fn default() -> Self {
        Self::pbta()
    }
}

impl NarrativeResolutionConfig {
    /// Standard PbtA configuration (2d6, 10+/7-9/6-)
    pub fn pbta() -> Self {
        Self {
            style: NarrativeResolutionStyle::PbtA,
            thresholds: NarrativeThresholds::default(),
            ladder: DifficultyLadder::default(),
            dice_config: NarrativeDiceConfig::pbta(),
            position_effect: PositionEffectConfig::default(),
        }
    }

    /// Standard Fate Core configuration (4dF, ladder)
    pub fn fate_core() -> Self {
        Self {
            style: NarrativeResolutionStyle::Ladder,
            thresholds: NarrativeThresholds::default(),
            ladder: DifficultyLadder::fate_core(),
            dice_config: NarrativeDiceConfig::fate(4),
            position_effect: PositionEffectConfig::default(),
        }
    }

    /// Standard Blades in the Dark configuration (d6 pool, position/effect)
    pub fn blades() -> Self {
        Self {
            style: NarrativeResolutionStyle::Blades,
            thresholds: NarrativeThresholds::default(),
            ladder: DifficultyLadder::default(),
            dice_config: NarrativeDiceConfig::blades(),
            position_effect: PositionEffectConfig::default(),
        }
    }

    /// Get the display formula for the current dice configuration
    pub fn dice_formula(&self) -> &str {
        &self.dice_config.display_formula
    }
}

/// The narrative resolution style determines how rolls are evaluated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NarrativeResolutionStyle {
    /// PbtA: Fixed thresholds (10+/7-9/6-), descriptor affects narrative only
    #[default]
    PbtA,
    /// Ladder: Descriptor maps to target number, compare roll vs ladder (Fate)
    Ladder,
    /// Blades: Position determines consequence severity, Effect determines progress
    Blades,
    /// Custom: Use configurable thresholds with any dice system
    Custom,
    /// Unknown style (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl NarrativeResolutionStyle {
    /// Get display name for this style
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::PbtA => "PbtA (2d6+stat)",
            Self::Ladder => "Fate/Ladder (NdF vs target)",
            Self::Blades => "Blades (d6 pool, Position/Effect)",
            Self::Custom => "Custom",
            Self::Unknown => "Unknown",
        }
    }

    /// Get description for this style
    pub fn description(&self) -> &'static str {
        match self {
            Self::PbtA => "Roll 2d6 + stat. 10+ = full success, 7-9 = partial, 6- = miss.",
            Self::Ladder => "Roll Fudge dice + skill vs difficulty ladder. Shifts determine outcome.",
            Self::Blades => "Roll d6 pool, take highest. Position sets consequence severity, Effect sets progress.",
            Self::Custom => "Custom thresholds and dice configuration.",
            Self::Unknown => "Unknown resolution style.",
        }
    }
}

/// Configurable thresholds for PbtA-style resolution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrativeThresholds {
    /// Total needed for critical success (optional, default: None)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critical_success: Option<i32>,

    /// Total needed for full success (default: 10)
    pub full_success: i32,

    /// Total needed for partial success (default: 7)
    pub partial_success: i32,

    /// Total at or below which is critical failure (optional, default: None)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critical_failure: Option<i32>,
}

impl Default for NarrativeThresholds {
    fn default() -> Self {
        Self {
            critical_success: None, // PbtA doesn't have crit by default
            full_success: 10,
            partial_success: 7,
            critical_failure: None,
        }
    }
}

impl NarrativeThresholds {
    /// Create thresholds with optional critical values
    pub fn with_criticals(
        full_success: i32,
        partial_success: i32,
        critical_success: Option<i32>,
        critical_failure: Option<i32>,
    ) -> Self {
        Self {
            critical_success,
            full_success,
            partial_success,
            critical_failure,
        }
    }
}

/// Dice configuration for narrative systems
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrativeDiceConfig {
    /// Type of dice system
    pub dice_type: NarrativeDiceType,

    /// Number of dice to roll (e.g., 2 for 2d6, 4 for 4dF)
    pub dice_count: u8,

    /// Display formula for UI (e.g., "2d6", "4dF", "d6 pool")
    pub display_formula: String,
}

impl Default for NarrativeDiceConfig {
    fn default() -> Self {
        Self::pbta()
    }
}

impl NarrativeDiceConfig {
    /// Standard PbtA dice (2d6)
    pub fn pbta() -> Self {
        Self {
            dice_type: NarrativeDiceType::Standard { sides: 6 },
            dice_count: 2,
            display_formula: "2d6".to_string(),
        }
    }

    /// Fate dice with configurable count
    pub fn fate(count: u8) -> Self {
        Self {
            dice_type: NarrativeDiceType::Fudge,
            dice_count: count,
            display_formula: format!("{}dF", count),
        }
    }

    /// Blades-style d6 pool
    pub fn blades() -> Self {
        Self {
            dice_type: NarrativeDiceType::Pool { sides: 6 },
            dice_count: 0, // Pool size determined by action rating
            display_formula: "d6 pool".to_string(),
        }
    }

    /// Custom dice configuration
    pub fn custom(dice_type: NarrativeDiceType, count: u8, formula: impl Into<String>) -> Self {
        Self {
            dice_type,
            dice_count: count,
            display_formula: formula.into(),
        }
    }
}

/// Types of dice for narrative systems
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NarrativeDiceType {
    /// Standard numbered dice (d6, d10, etc.) - sum all dice
    Standard { sides: u8 },

    /// Fudge/Fate dice (+1, -1, 0) - sum all dice
    Fudge,

    /// Dice pool - roll multiple, take highest (Blades style)
    Pool { sides: u8 },
}

impl NarrativeDiceType {
    /// Get description of this dice type
    pub fn description(&self) -> &'static str {
        match self {
            Self::Standard { .. } => "Sum all dice rolled",
            Self::Fudge => "Fudge dice: +1, -1, or 0 per die",
            Self::Pool { .. } => "Roll pool, take highest die",
        }
    }
}

/// Difficulty ladder for Fate-style systems
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DifficultyLadder {
    /// Ladder entries mapping descriptors to values
    pub entries: Vec<LadderEntry>,

    /// Shifts needed for success with style (default: 3)
    pub style_threshold: i32,

    /// Shift value that counts as a tie (default: 0)
    pub tie_threshold: i32,
}

impl Default for DifficultyLadder {
    fn default() -> Self {
        Self::fate_core()
    }
}

impl DifficultyLadder {
    /// Standard Fate Core ladder
    pub fn fate_core() -> Self {
        Self {
            entries: vec![
                LadderEntry::new(DifficultyDescriptor::Trivial, -2, "Terrible"),
                LadderEntry::new(DifficultyDescriptor::Easy, 0, "Mediocre"),
                LadderEntry::new(DifficultyDescriptor::Routine, 1, "Average"),
                LadderEntry::new(DifficultyDescriptor::Moderate, 2, "Fair"),
                LadderEntry::new(DifficultyDescriptor::Challenging, 3, "Good"),
                LadderEntry::new(DifficultyDescriptor::Hard, 4, "Great"),
                LadderEntry::new(DifficultyDescriptor::VeryHard, 5, "Superb"),
                LadderEntry::new(DifficultyDescriptor::Extreme, 6, "Fantastic"),
                LadderEntry::new(DifficultyDescriptor::Impossible, 8, "Legendary"),
            ],
            style_threshold: 3,
            tie_threshold: 0,
        }
    }

    /// Look up ladder value for a descriptor
    pub fn value_for(&self, descriptor: &DifficultyDescriptor) -> Option<i32> {
        self.entries
            .iter()
            .find(|e| &e.descriptor == descriptor)
            .map(|e| e.value)
    }

    /// Get display name for a descriptor from the ladder
    pub fn display_name_for(&self, descriptor: &DifficultyDescriptor) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| &e.descriptor == descriptor)
            .map(|e| e.display_name.as_str())
    }
}

/// Single entry in a difficulty ladder
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LadderEntry {
    /// The difficulty descriptor this maps
    pub descriptor: DifficultyDescriptor,

    /// The numeric value for this descriptor
    pub value: i32,

    /// Display name (e.g., "Fair", "Great", "Legendary")
    pub display_name: String,
}

impl LadderEntry {
    pub fn new(
        descriptor: DifficultyDescriptor,
        value: i32,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            descriptor,
            value,
            display_name: display_name.into(),
        }
    }
}

/// Descriptive difficulty for narrative systems (also used as ladder keys)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifficultyDescriptor {
    Trivial,
    Easy,
    Routine,
    Moderate,
    Challenging,
    Hard,
    VeryHard,
    Extreme,
    Impossible,
    // PbtA-style position indicators (alternative to Position enum for simpler use)
    Risky,
    Desperate,
    /// Unknown difficulty (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl DifficultyDescriptor {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Trivial => "Trivial",
            Self::Easy => "Easy",
            Self::Routine => "Routine",
            Self::Moderate => "Moderate",
            Self::Challenging => "Challenging",
            Self::Hard => "Hard",
            Self::VeryHard => "Very Hard",
            Self::Extreme => "Extreme",
            Self::Impossible => "Impossible",
            Self::Risky => "Risky",
            Self::Desperate => "Desperate",
            Self::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for DifficultyDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl DifficultyDescriptor {
    /// Get all standard descriptors (excluding position indicators)
    pub fn standard_descriptors() -> Vec<Self> {
        vec![
            Self::Trivial,
            Self::Easy,
            Self::Routine,
            Self::Moderate,
            Self::Challenging,
            Self::Hard,
            Self::VeryHard,
            Self::Extreme,
            Self::Impossible,
        ]
    }
}

// =============================================================================
// Position/Effect System (Blades in the Dark style)
// =============================================================================

/// Position/Effect configuration for Blades-style resolution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionEffectConfig {
    /// Thresholds for Blades-style d6 pool (highest die)
    pub pool_thresholds: BladesPoolThresholds,

    /// Effect levels and their clock tick values
    pub effect_ticks: EffectTickConfig,

    /// Whether to enable the critical rule (multiple 6s)
    pub enable_critical: bool,

    /// Minimum dice for critical (default: 2 sixes needed)
    pub critical_dice_count: u8,
}

impl Default for PositionEffectConfig {
    fn default() -> Self {
        Self {
            pool_thresholds: BladesPoolThresholds::default(),
            effect_ticks: EffectTickConfig::default(),
            enable_critical: true,
            critical_dice_count: 2,
        }
    }
}

/// Thresholds for Blades d6 pool resolution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BladesPoolThresholds {
    /// Highest die value for full success (default: 6)
    pub full_success: i32,

    /// Minimum die value for partial success (default: 4)
    pub partial_success_min: i32,

    /// Maximum die value for partial success (default: 5)
    pub partial_success_max: i32,
    // Below partial_success_min is failure
}

impl Default for BladesPoolThresholds {
    fn default() -> Self {
        Self {
            full_success: 6,
            partial_success_min: 4,
            partial_success_max: 5,
        }
    }
}

/// Effect tick configuration for progress clocks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectTickConfig {
    pub extreme_ticks: u8,
    pub great_ticks: u8,
    pub standard_ticks: u8,
    pub limited_ticks: u8,
    pub zero_ticks: u8,
}

impl Default for EffectTickConfig {
    fn default() -> Self {
        Self {
            extreme_ticks: 4,
            great_ticks: 3,
            standard_ticks: 2,
            limited_ticks: 1,
            zero_ticks: 0,
        }
    }
}

/// Position level for Blades-style resolution.
/// Determines consequence severity on partial success or failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Position {
    /// You act on your terms, exploit dominant advantage. Minor consequences.
    Controlled,

    /// Standard risk - you go head to head. Moderate consequences.
    #[default]
    Risky,

    /// You overreach, in serious trouble. Severe consequences.
    Desperate,

    /// Unknown position (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl Position {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Controlled => "Controlled",
            Self::Risky => "Risky",
            Self::Desperate => "Desperate",
            Self::Unknown => "Unknown",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Controlled => "You act on your terms. Minor consequences on failure.",
            Self::Risky => "You go head to head. Moderate consequences on failure.",
            Self::Desperate => "You're in serious trouble. Severe consequences on failure.",
            Self::Unknown => "Unknown position.",
        }
    }

    /// Get consequence severity description for each outcome type at this position
    pub fn consequence_severity(&self) -> &'static str {
        match self {
            Self::Controlled => "minor complication, reduced effect, or worse position",
            Self::Risky => "harm, complication, reduced effect, or desperate position",
            Self::Desperate => "severe harm, serious complication, or lost opportunity",
            Self::Unknown => "unknown consequences",
        }
    }

    /// Get all position variants (excludes Unknown)
    pub fn all() -> Vec<Self> {
        vec![Self::Controlled, Self::Risky, Self::Desperate]
    }
}

/// Effect level for Blades-style resolution.
/// Determines how much progress is made on success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EffectLevel {
    /// No effect possible
    Zero,

    /// Partial or weak effect
    Limited,

    /// Normal expected effect
    #[default]
    Standard,

    /// More than usual effect
    Great,

    /// Extraordinary effect (beyond great, from critical)
    Extreme,

    /// Unknown effect level (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl EffectLevel {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Zero => "Zero",
            Self::Limited => "Limited",
            Self::Standard => "Standard",
            Self::Great => "Great",
            Self::Extreme => "Extreme",
            Self::Unknown => "Unknown",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Zero => "No effect possible in this situation.",
            Self::Limited => "Partial or weak effect. Less progress than normal.",
            Self::Standard => "Normal expected effect for this action.",
            Self::Great => "You achieve more than usual. Extra benefit.",
            Self::Extreme => "Extraordinary effect. Maximum possible impact.",
            Self::Unknown => "Unknown effect level.",
        }
    }

    /// Get clock ticks for this effect level
    pub fn ticks(&self, config: &EffectTickConfig) -> u8 {
        match self {
            Self::Zero => config.zero_ticks,
            Self::Limited => config.limited_ticks,
            Self::Standard => config.standard_ticks,
            Self::Great => config.great_ticks,
            Self::Extreme => config.extreme_ticks,
            Self::Unknown => config.zero_ticks,
        }
    }

    /// Increase effect level (for critical success)
    pub fn increase(&self) -> Self {
        match self {
            Self::Zero => Self::Limited,
            Self::Limited => Self::Standard,
            Self::Standard => Self::Great,
            Self::Great | Self::Extreme => Self::Extreme,
            Self::Unknown => Self::Unknown,
        }
    }

    /// Decrease effect level (for reduced effect consequence)
    pub fn decrease(&self) -> Self {
        match self {
            Self::Extreme => Self::Great,
            Self::Great => Self::Standard,
            Self::Standard => Self::Limited,
            Self::Limited | Self::Zero => Self::Zero,
            Self::Unknown => Self::Unknown,
        }
    }

    /// Get all effect level variants (excluding Zero and Unknown for normal selection)
    pub fn selectable() -> Vec<Self> {
        vec![Self::Limited, Self::Standard, Self::Great]
    }

    /// Get all effect level variants (excludes Unknown)
    pub fn all() -> Vec<Self> {
        vec![
            Self::Zero,
            Self::Limited,
            Self::Standard,
            Self::Great,
            Self::Extreme,
        ]
    }
}
