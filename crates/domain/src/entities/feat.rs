//! Feat and feature entity for TTRPG character abilities.
//!
//! Provides a universal representation for feats, talents, and special
//! abilities that characters can acquire in various game systems.

use serde::{Deserialize, Serialize};

use crate::value_objects::Tag;

/// A feat, talent, or special ability that a character can acquire.
///
/// This struct supports various TTRPG systems' concepts of character
/// customization options (D&D feats, Pathfinder feats, etc.).
///
/// # Design Decision (ADR-008 Tier 4)
///
/// This struct uses public fields because it is a **simple data struct** with no invariants to protect:
/// - No business rules that could be violated (e.g., any combination of fields is valid)
/// - No complex state transitions
/// - Primarily used for data transfer and storage
///
/// Adding private fields with accessors would add boilerplate without providing any safety benefits.
/// See [ADR-008](docs/architecture/ADR-008-tiered-encapsulation.md) for rationale.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Feat {
    /// Unique identifier for this feat
    pub id: String,
    /// Which game system this feat belongs to (e.g., "dnd5e", "pf2e")
    pub system_id: String,
    /// Display name of the feat
    pub name: String,
    /// Full description of what the feat does
    pub description: String,
    /// Requirements to take this feat
    #[serde(default)]
    pub prerequisites: Vec<Prerequisite>,
    /// Mechanical benefits granted by the feat
    #[serde(default)]
    pub benefits: Vec<FeatBenefit>,
    /// Source book reference (e.g., "PHB p.165")
    pub source: String,
    /// Category of feat (system-specific, e.g., "general", "combat", "skill")
    pub category: Option<String>,
    /// Whether this feat can be taken multiple times
    #[serde(default)]
    pub repeatable: bool,
    /// Tags for filtering and categorization
    #[serde(default)]
    pub tags: Vec<Tag>,
}

impl Feat {
    /// Create a new feat with required fields.
    pub fn new(
        id: impl Into<String>,
        system_id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            system_id: system_id.into(),
            name: name.into(),
            description: description.into(),
            prerequisites: Vec::new(),
            benefits: Vec::new(),
            source: source.into(),
            category: None,
            repeatable: false,
            tags: Vec::new(),
        }
    }

    /// Reconstruct a Feat from storage parts.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: String,
        system_id: String,
        name: String,
        description: String,
        prerequisites: Vec<Prerequisite>,
        benefits: Vec<FeatBenefit>,
        source: String,
        category: Option<String>,
        repeatable: bool,
        tags: Vec<Tag>,
    ) -> Self {
        Self {
            id,
            system_id,
            name,
            description,
            prerequisites,
            benefits,
            source,
            category,
            repeatable,
            tags,
        }
    }
}

/// A prerequisite for acquiring a feat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Prerequisite {
    /// Minimum ability score requirement
    MinStat {
        /// The stat name (e.g., "STR", "Dexterity")
        stat: String,
        /// Minimum value required
        value: i32,
    },
    /// Minimum character level
    MinLevel {
        /// Minimum level required
        level: u8,
    },
    /// Must have another feat
    HasFeat {
        /// ID of the required feat
        feat_id: String,
        /// Display name (for UI)
        #[serde(default)]
        feat_name: Option<String>,
    },
    /// Must have levels in a specific class
    HasClass {
        /// ID of the required class
        class_id: String,
        /// Display name (for UI)
        #[serde(default)]
        class_name: Option<String>,
        /// Minimum levels in that class (if any)
        min_level: Option<u8>,
    },
    /// Must have a specific proficiency
    HasProficiency {
        /// Type of proficiency (e.g., "armor", "weapon", "skill")
        proficiency_type: String,
        /// The specific proficiency (e.g., "heavy armor", "Athletics")
        proficiency: String,
    },
    /// Must be a specific race or ancestry
    Race {
        /// Race ID or name
        race: String,
    },
    /// Spellcasting ability requirement
    Spellcaster {
        /// Minimum spell level they must be able to cast
        min_spell_level: Option<u8>,
    },
    /// Custom prerequisite with free-form text
    Custom {
        /// Description of the requirement
        description: String,
    },
    /// Any one of the listed prerequisites
    AnyOf {
        /// List of alternative prerequisites
        options: Vec<Prerequisite>,
    },
    /// All of the listed prerequisites
    AllOf {
        /// List of required prerequisites
        requirements: Vec<Prerequisite>,
    },
}

impl Prerequisite {
    /// Create a minimum stat prerequisite.
    pub fn min_stat(stat: impl Into<String>, value: i32) -> Self {
        Prerequisite::MinStat {
            stat: stat.into(),
            value,
        }
    }

    /// Create a minimum level prerequisite.
    pub fn min_level(level: u8) -> Self {
        Prerequisite::MinLevel { level }
    }

    /// Create a has-feat prerequisite.
    pub fn has_feat(feat_id: impl Into<String>) -> Self {
        Prerequisite::HasFeat {
            feat_id: feat_id.into(),
            feat_name: None,
        }
    }

    /// Create a has-class prerequisite.
    pub fn has_class(class_id: impl Into<String>) -> Self {
        Prerequisite::HasClass {
            class_id: class_id.into(),
            class_name: None,
            min_level: None,
        }
    }

    /// Create a custom prerequisite.
    pub fn custom(description: impl Into<String>) -> Self {
        Prerequisite::Custom {
            description: description.into(),
        }
    }
}

/// A mechanical benefit granted by a feat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FeatBenefit {
    /// Increase an ability score
    StatIncrease {
        /// The stat to increase
        stat: String,
        /// Amount to increase by
        value: i32,
    },
    /// Choose from multiple stats to increase
    StatChoice {
        /// Options to choose from
        options: Vec<String>,
        /// Amount to increase by
        value: i32,
        /// Number of choices to make
        #[serde(default = "default_one")]
        count: u8,
    },
    /// Grant proficiency in something
    GrantProficiency {
        /// Type of proficiency (e.g., "skill", "weapon", "armor", "tool")
        proficiency_type: String,
        /// The specific proficiency (e.g., "Athletics", "longsword")
        proficiency: String,
    },
    /// Choose proficiency from a list
    ChooseProficiency {
        /// Type of proficiency
        proficiency_type: String,
        /// Options to choose from
        options: Vec<String>,
        /// Number of choices to make
        #[serde(default = "default_one")]
        count: u8,
    },
    /// Grant a special ability
    GrantAbility {
        /// Name of the ability
        ability: String,
        /// Description of what the ability does
        description: String,
        /// Uses per rest (if limited)
        uses: Option<AbilityUses>,
    },
    /// Grant additional hit points
    BonusHitPoints {
        /// Fixed amount to add
        fixed: Option<i32>,
        /// Amount per level
        per_level: Option<i32>,
    },
    /// Increase speed
    SpeedIncrease {
        /// Movement type (e.g., "walk", "fly", "swim")
        movement_type: String,
        /// Amount to increase by (in feet)
        value: u32,
    },
    /// Grant resistance to a damage type
    DamageResistance {
        /// Damage type (e.g., "fire", "cold", "psychic")
        damage_type: String,
    },
    /// Grant advantage on certain rolls
    Advantage {
        /// What the advantage applies to
        on: String,
    },
    /// Custom benefit with free-form description
    Custom {
        /// Description of the benefit
        description: String,
    },
}

fn default_one() -> u8 {
    1
}

/// Limited uses for an ability granted by a feat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AbilityUses {
    /// Maximum uses
    pub max: UsesFormula,
    /// When uses are restored
    pub recharge: RechargeType,
}

impl AbilityUses {
    /// Create new ability uses.
    pub fn new(max: UsesFormula, recharge: RechargeType) -> Self {
        Self { max, recharge }
    }
}

/// Formula for calculating ability uses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UsesFormula {
    /// Fixed number of uses
    Fixed { value: u8 },
    /// Uses equal to proficiency bonus
    ProficiencyBonus,
    /// Uses equal to a stat modifier (minimum 1)
    StatModifier {
        /// Which stat modifier to use
        stat: String,
        /// Minimum value (usually 1)
        #[serde(default = "default_one_i32")]
        min: i32,
    },
    /// Custom formula
    Formula {
        /// Expression (e.g., "level / 2")
        expression: String,
    },
}

fn default_one_i32() -> i32 {
    1
}

impl UsesFormula {
    /// Create a fixed uses formula.
    pub fn fixed(value: u8) -> Self {
        UsesFormula::Fixed { value }
    }

    /// Create a proficiency bonus formula.
    pub fn proficiency_bonus() -> Self {
        UsesFormula::ProficiencyBonus
    }

    /// Create a stat modifier formula.
    pub fn stat_modifier(stat: impl Into<String>) -> Self {
        UsesFormula::StatModifier {
            stat: stat.into(),
            min: 1,
        }
    }
}

/// When ability uses are restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RechargeType {
    /// Restored on a short rest
    ShortRest,
    /// Restored on a long rest
    LongRest,
    /// Restored at dawn
    Dawn,
    /// Never restored automatically (manual tracking)
    Manual,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feat_equality() {
        let mut feat = Feat::new(
            "dnd5e_great_weapon_master",
            "dnd5e",
            "Great Weapon Master",
            "You've learned to put the weight of a weapon...",
            "PHB p.167",
        );
        feat.benefits = vec![
            FeatBenefit::Custom {
                description: "On a critical hit or kill, bonus action attack".into(),
            },
            FeatBenefit::Custom {
                description: "-5 to hit for +10 damage".into(),
            },
        ];
        feat.category = Some("combat".to_string());
        feat.tags.push(Tag::new("combat").unwrap());
        feat.tags.push(Tag::new("melee").unwrap());

        let other = feat.clone();
        assert_eq!(feat, other);
    }

    #[test]
    fn feat_accessors() {
        let mut feat = Feat::new(
            "test_feat",
            "test_system",
            "Test Feat",
            "Test description",
            "Test Source",
        );
        feat.repeatable = true;
        feat.category = Some("general".to_string());

        assert_eq!(feat.id, "test_feat");
        assert_eq!(feat.system_id, "test_system");
        assert_eq!(feat.name, "Test Feat");
        assert_eq!(feat.description, "Test description");
        assert_eq!(feat.source, "Test Source");
        assert!(feat.repeatable);
        assert_eq!(feat.category, Some("general".to_string()));
    }

    #[test]
    fn prerequisite_constructors() {
        let prereq = Prerequisite::min_stat("STR", 13);
        assert!(
            matches!(prereq, Prerequisite::MinStat { stat, value } if stat == "STR" && value == 13)
        );

        let prereq = Prerequisite::min_level(4);
        assert!(matches!(prereq, Prerequisite::MinLevel { level: 4 }));
    }

    #[test]
    fn feat_with_prerequisites() {
        let feat = Feat::new(
            "dnd5e_sentinel",
            "dnd5e",
            "Sentinel",
            "You have mastered techniques...",
            "PHB p.169",
        );

        assert!(feat.prerequisites.is_empty());
    }

    #[test]
    fn complex_prerequisites() {
        let prereq = Prerequisite::AnyOf {
            options: vec![
                Prerequisite::min_stat("STR", 13),
                Prerequisite::min_stat("DEX", 13),
            ],
        };

        if let Prerequisite::AnyOf { options } = prereq {
            assert_eq!(options.len(), 2);
        } else {
            panic!("Expected AnyOf prerequisite");
        }
    }

    #[test]
    fn uses_formula_constructors() {
        let uses = UsesFormula::fixed(3);
        assert!(matches!(uses, UsesFormula::Fixed { value: 3 }));

        let uses = UsesFormula::proficiency_bonus();
        assert!(matches!(uses, UsesFormula::ProficiencyBonus));

        let uses = UsesFormula::stat_modifier("WIS");
        assert!(matches!(uses, UsesFormula::StatModifier { stat, min: 1 } if stat == "WIS"));
    }

    #[test]
    fn ability_uses_accessors() {
        let uses = AbilityUses::new(UsesFormula::fixed(2), RechargeType::LongRest);
        assert!(matches!(uses.max, UsesFormula::Fixed { value: 2 }));
        assert_eq!(uses.recharge, RechargeType::LongRest);
    }
}
