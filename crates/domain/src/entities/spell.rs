//! Spell entity for TTRPG spellcasting systems.
//!
//! Provides a universal spell representation that works across different
//! game systems (D&D 5e, Pathfinder, etc.) while supporting system-specific
//! details through flexible fields.

use serde::{Deserialize, Serialize};

use crate::value_objects::Tag;

/// A spell or magical ability.
///
/// This struct is designed to be system-agnostic while supporting the
/// common elements found in most TTRPG spell systems.
///
/// # Design Decision (ADR-008 Tier 4)
///
/// This struct uses **public fields** as a simple data struct because:
/// - No invariants to protect (any combination of spell properties is valid)
/// - No business logic methods that require guarded access
/// - Primarily a data carrier for spell information from external systems
/// - Direct field access is clearer than accessor boilerplate
///
/// See [ADR-008](docs/architecture/ADR-008-tiered-encapsulation.md) for rationale.
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
    pub tags: Vec<Tag>,
    /// Whether this spell can be cast as a ritual
    #[serde(default)]
    pub ritual: bool,
    /// Whether this spell requires concentration
    #[serde(default)]
    pub concentration: bool,
}

impl Spell {
    /// Create a new spell with required fields.
    pub fn new(
        id: impl Into<String>,
        system_id: impl Into<String>,
        name: impl Into<String>,
        level: SpellLevel,
        casting_time: CastingTime,
        range: SpellRange,
        components: SpellComponents,
        duration: SpellDuration,
        description: impl Into<String>,
        classes: Vec<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            system_id: system_id.into(),
            name: name.into(),
            level,
            school: None,
            casting_time,
            range,
            components,
            duration,
            description: description.into(),
            higher_levels: None,
            classes,
            source: source.into(),
            tags: Vec::new(),
            ritual: false,
            concentration: false,
        }
    }

    /// Reconstruct a spell from storage
    ///
    /// This is useful when reconstructing from storage or when you have
    /// all fields available and want to avoid builder pattern overhead.
    pub fn from_storage(
        id: impl Into<String>,
        system_id: impl Into<String>,
        name: impl Into<String>,
        level: SpellLevel,
        school: Option<String>,
        casting_time: CastingTime,
        range: SpellRange,
        components: SpellComponents,
        duration: SpellDuration,
        description: impl Into<String>,
        higher_levels: Option<String>,
        classes: Vec<String>,
        source: impl Into<String>,
        tags: Vec<Tag>,
        ritual: bool,
        concentration: bool,
    ) -> Self {
        Self {
            id: id.into(),
            system_id: system_id.into(),
            name: name.into(),
            level,
            school,
            casting_time,
            range,
            components,
            duration,
            description: description.into(),
            higher_levels,
            classes,
            source: source.into(),
            tags,
            ritual,
            concentration,
        }
    }
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

impl CastingTime {
    /// Create a new casting time.
    pub fn new(amount: u32, unit: CastingTimeUnit) -> Self {
        Self {
            amount,
            unit,
            condition: None,
        }
    }

    /// Set a condition for the casting time.
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

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
    /// Create empty spell components.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the verbal component requirement.
    pub fn with_verbal(mut self, verbal: bool) -> Self {
        self.verbal = verbal;
        self
    }

    /// Set the somatic component requirement.
    pub fn with_somatic(mut self, somatic: bool) -> Self {
        self.somatic = somatic;
        self
    }

    /// Set the material component.
    pub fn with_material(mut self, material: MaterialComponent) -> Self {
        self.material = Some(material);
        self
    }

    /// Create components with just verbal.
    pub fn verbal_only() -> Self {
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
            material: Some(MaterialComponent::new(material)),
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

impl MaterialComponent {
    /// Create a new material component.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            consumed: false,
            cost: None,
        }
    }

    /// Set whether the material is consumed.
    pub fn with_consumed(mut self, consumed: bool) -> Self {
        self.consumed = consumed;
        self
    }

    /// Set the cost in gold pieces.
    pub fn with_cost(mut self, cost: u32) -> Self {
        self.cost = Some(cost);
        self
    }
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
    fn spell_equality() {
        let mut spell = Spell::new(
            "dnd5e_fireball",
            "dnd5e",
            "Fireball",
            SpellLevel::Level(3),
            CastingTime::action(),
            SpellRange::feet(150),
            SpellComponents::all("a tiny ball of bat guano and sulfur"),
            SpellDuration::instantaneous(),
            "A bright streak flashes...",
            vec!["sorcerer".into(), "wizard".into()],
            "PHB p.241",
        );
        spell.school = Some("Evocation".to_string());
        spell.higher_levels = Some("When cast at 4th level or higher...".to_string());
        spell.tags = vec![Tag::new("damage").unwrap(), Tag::new("fire").unwrap()];

        let other = spell.clone();
        assert_eq!(spell, other);
    }

    #[test]
    fn spell_accessors() {
        let spell = Spell::from_parts(
            "test_spell",
            "test_system",
            "Test Spell",
            SpellLevel::Level(1),
            None,
            CastingTime::action(),
            SpellRange::touch(),
            SpellComponents::verbal_somatic(),
            SpellDuration::minutes(10),
            "Test description",
            None,
            vec!["wizard".into()],
            "Test Source",
            vec![],
            true,
            true,
        );

        assert_eq!(spell.id, "test_spell");
        assert_eq!(spell.system_id, "test_system");
        assert_eq!(spell.name, "Test Spell");
        assert_eq!(spell.level, SpellLevel::Level(1));
        assert!(spell.ritual);
        assert!(spell.concentration);
    }

    #[test]
    fn casting_time_constructors() {
        assert_eq!(CastingTime::action().unit, CastingTimeUnit::Action);
        assert_eq!(
            CastingTime::bonus_action().unit,
            CastingTimeUnit::BonusAction
        );
        assert_eq!(CastingTime::minutes(10).amount, 10);
    }

    #[test]
    fn spell_range_constructors() {
        assert!(matches!(
            SpellRange::self_only(),
            SpellRange::SelfOnly { area: None }
        ));
        assert!(matches!(SpellRange::touch(), SpellRange::Touch));
        assert!(matches!(
            SpellRange::feet(60),
            SpellRange::Feet { distance: 60 }
        ));
    }

    #[test]
    fn spell_components_accessors() {
        let components = SpellComponents::all("diamond dust worth 100 gp");
        assert!(components.verbal);
        assert!(components.somatic);
        assert!(components.material.is_some());
    }

    #[test]
    fn material_component_builder() {
        let material = MaterialComponent::new("diamond")
            .with_consumed(true)
            .with_cost(500);

        assert_eq!(material.description, "diamond");
        assert!(material.consumed);
        assert_eq!(material.cost, Some(500));
    }
}
