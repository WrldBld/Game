//! Spell entity for TTRPG spellcasting systems.
//!
//! Provides a universal spell representation that works across different
//! game systems (D&D 5e, Pathfinder, etc.) while supporting system-specific
//! details through flexible fields.

use serde::{Deserialize, Serialize};

/// A spell or magical ability.
///
/// This struct is designed to be system-agnostic while supporting the
/// common elements found in most TTRPG spell systems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Spell {
    /// Unique identifier for this spell
    pub id: String,
    /// Which game system this spell belongs to (e.g., "dnd5e", "pf2e")
    pub system_id: String,
    /// Display name of the spell
    pub name: String,
    /// Spell level (cantrip = 0 for D&D-like systems)
    pub level: SpellLevel,
    /// School of magic (e.g., "Evocation", "Necromancy")
    pub school: Option<String>,
    /// How long it takes to cast
    pub casting_time: CastingTime,
    /// Range of the spell
    pub range: SpellRange,
    /// Required components (verbal, somatic, material)
    pub components: SpellComponents,
    /// How long the spell lasts
    pub duration: SpellDuration,
    /// Full description of the spell's effects
    pub description: String,
    /// Description of effects when cast at higher levels
    pub higher_levels: Option<String>,
    /// Classes that can learn this spell
    pub classes: Vec<String>,
    /// Source book reference (e.g., "PHB p.211")
    pub source: String,
    /// Tags for filtering and categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether this spell can be cast as a ritual
    #[serde(default)]
    pub ritual: bool,
    /// Whether this spell requires concentration
    #[serde(default)]
    pub concentration: bool,
}

/// Spell level representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpellLevel {
    /// Cantrip (level 0 spell, can be cast at will)
    Cantrip,
    /// Leveled spell (1-9 for D&D-like systems)
    Level(u8),
}

impl SpellLevel {
    /// Convert to numeric level (cantrip = 0).
    pub fn as_number(&self) -> u8 {
        match self {
            SpellLevel::Cantrip => 0,
            SpellLevel::Level(n) => *n,
        }
    }

    /// Check if this is a cantrip.
    pub fn is_cantrip(&self) -> bool {
        matches!(self, SpellLevel::Cantrip)
    }
}

impl From<u8> for SpellLevel {
    fn from(level: u8) -> Self {
        if level == 0 {
            SpellLevel::Cantrip
        } else {
            SpellLevel::Level(level)
        }
    }
}

/// How long it takes to cast a spell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastingTime {
    /// The amount of time
    pub amount: u32,
    /// The unit of time
    pub unit: CastingTimeUnit,
    /// Additional condition (e.g., "which you take when..." for reactions)
    pub condition: Option<String>,
}

/// Unit of time for casting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CastingTimeUnit {
    Action,
    BonusAction,
    Reaction,
    Minute,
    Hour,
    /// Special timing (e.g., "special", "see text")
    Special,
}

impl CastingTime {
    /// Create a standard action casting time.
    pub fn action() -> Self {
        Self {
            amount: 1,
            unit: CastingTimeUnit::Action,
            condition: None,
        }
    }

    /// Create a bonus action casting time.
    pub fn bonus_action() -> Self {
        Self {
            amount: 1,
            unit: CastingTimeUnit::BonusAction,
            condition: None,
        }
    }

    /// Create a reaction casting time with optional condition.
    pub fn reaction(condition: Option<String>) -> Self {
        Self {
            amount: 1,
            unit: CastingTimeUnit::Reaction,
            condition,
        }
    }

    /// Create a casting time in minutes.
    pub fn minutes(amount: u32) -> Self {
        Self {
            amount,
            unit: CastingTimeUnit::Minute,
            condition: None,
        }
    }

    /// Create a casting time in hours.
    pub fn hours(amount: u32) -> Self {
        Self {
            amount,
            unit: CastingTimeUnit::Hour,
            condition: None,
        }
    }
}

/// Range of a spell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SpellRange {
    /// Spell affects only the caster
    #[serde(rename = "self")]
    SelfOnly {
        /// Optional area of effect (e.g., "15-foot cone")
        area: Option<String>,
    },
    /// Touch range
    Touch,
    /// Specific distance in feet
    Feet { distance: u32 },
    /// Specific distance in miles
    Miles { distance: u32 },
    /// Unlimited range (same plane)
    Unlimited,
    /// Sight range
    Sight,
    /// Special range (see spell description)
    Special { description: String },
}

impl SpellRange {
    /// Create a self-only range.
    pub fn self_only() -> Self {
        SpellRange::SelfOnly { area: None }
    }

    /// Create a self range with an area of effect.
    pub fn self_with_area(area: impl Into<String>) -> Self {
        SpellRange::SelfOnly {
            area: Some(area.into()),
        }
    }

    /// Create a touch range.
    pub fn touch() -> Self {
        SpellRange::Touch
    }

    /// Create a range in feet.
    pub fn feet(distance: u32) -> Self {
        SpellRange::Feet { distance }
    }
}

/// Spell components (what's required to cast).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpellComponents {
    /// Requires verbal component
    #[serde(default)]
    pub verbal: bool,
    /// Requires somatic component
    #[serde(default)]
    pub somatic: bool,
    /// Material component details
    pub material: Option<MaterialComponent>,
}

impl SpellComponents {
    /// Create components with just verbal.
    pub fn verbal() -> Self {
        Self {
            verbal: true,
            somatic: false,
            material: None,
        }
    }

    /// Create components with verbal and somatic.
    pub fn verbal_somatic() -> Self {
        Self {
            verbal: true,
            somatic: true,
            material: None,
        }
    }

    /// Create components with all three.
    pub fn all(material: impl Into<String>) -> Self {
        Self {
            verbal: true,
            somatic: true,
            material: Some(MaterialComponent {
                description: material.into(),
                consumed: false,
                cost: None,
            }),
        }
    }
}

/// Material component for a spell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialComponent {
    /// Description of the material
    pub description: String,
    /// Whether the material is consumed by the spell
    #[serde(default)]
    pub consumed: bool,
    /// Cost in gold pieces (if any)
    pub cost: Option<u32>,
}

/// How long a spell's effects last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SpellDuration {
    /// Effect happens instantly
    Instantaneous,
    /// Lasts for a specific amount of time
    Timed {
        amount: u32,
        unit: DurationUnit,
        /// Whether concentration is required
        #[serde(default)]
        concentration: bool,
    },
    /// Until dispelled or specific condition
    UntilDispelled {
        /// Optional triggering condition
        trigger: Option<String>,
    },
    /// Special duration (see spell description)
    Special { description: String },
}

/// Unit of time for duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DurationUnit {
    Round,
    Minute,
    Hour,
    Day,
}

impl SpellDuration {
    /// Create an instantaneous duration.
    pub fn instantaneous() -> Self {
        SpellDuration::Instantaneous
    }

    /// Create a duration in rounds.
    pub fn rounds(amount: u32) -> Self {
        SpellDuration::Timed {
            amount,
            unit: DurationUnit::Round,
            concentration: false,
        }
    }

    /// Create a duration in minutes.
    pub fn minutes(amount: u32) -> Self {
        SpellDuration::Timed {
            amount,
            unit: DurationUnit::Minute,
            concentration: false,
        }
    }

    /// Create a duration in hours.
    pub fn hours(amount: u32) -> Self {
        SpellDuration::Timed {
            amount,
            unit: DurationUnit::Hour,
            concentration: false,
        }
    }

    /// Create a concentration duration in minutes.
    pub fn concentration_minutes(amount: u32) -> Self {
        SpellDuration::Timed {
            amount,
            unit: DurationUnit::Minute,
            concentration: true,
        }
    }

    /// Create a concentration duration in hours.
    pub fn concentration_hours(amount: u32) -> Self {
        SpellDuration::Timed {
            amount,
            unit: DurationUnit::Hour,
            concentration: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_level_conversion() {
        assert_eq!(SpellLevel::from(0), SpellLevel::Cantrip);
        assert_eq!(SpellLevel::from(1), SpellLevel::Level(1));
        assert_eq!(SpellLevel::from(9), SpellLevel::Level(9));
    }

    #[test]
    fn spell_level_as_number() {
        assert_eq!(SpellLevel::Cantrip.as_number(), 0);
        assert_eq!(SpellLevel::Level(3).as_number(), 3);
    }

    #[test]
    fn spell_serialization() {
        let spell = Spell {
            id: "dnd5e_fireball".into(),
            system_id: "dnd5e".into(),
            name: "Fireball".into(),
            level: SpellLevel::Level(3),
            school: Some("Evocation".into()),
            casting_time: CastingTime::action(),
            range: SpellRange::feet(150),
            components: SpellComponents::all("a tiny ball of bat guano and sulfur"),
            duration: SpellDuration::instantaneous(),
            description: "A bright streak flashes...".into(),
            higher_levels: Some("When cast at 4th level or higher...".into()),
            classes: vec!["sorcerer".into(), "wizard".into()],
            source: "PHB p.241".into(),
            tags: vec!["damage".into(), "fire".into()],
            ritual: false,
            concentration: false,
        };

        let json = serde_json::to_string(&spell).unwrap();
        let deserialized: Spell = serde_json::from_str(&json).unwrap();
        assert_eq!(spell, deserialized);
    }

    #[test]
    fn casting_time_constructors() {
        assert_eq!(CastingTime::action().unit, CastingTimeUnit::Action);
        assert_eq!(CastingTime::bonus_action().unit, CastingTimeUnit::BonusAction);
        assert_eq!(CastingTime::minutes(10).amount, 10);
    }

    #[test]
    fn spell_range_constructors() {
        assert!(matches!(SpellRange::self_only(), SpellRange::SelfOnly { area: None }));
        assert!(matches!(SpellRange::touch(), SpellRange::Touch));
        assert!(matches!(SpellRange::feet(60), SpellRange::Feet { distance: 60 }));
    }
}
