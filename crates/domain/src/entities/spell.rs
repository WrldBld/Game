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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Spell {
    /// Unique identifier for this spell
    id: String,
    /// Which game system this spell belongs to (e.g., "dnd5e", "pf2e")
    system_id: String,
    /// Display name of the spell
    name: String,
    /// Spell level (cantrip = 0 for D&D-like systems)
    level: SpellLevel,
    /// School of magic (e.g., "Evocation", "Necromancy")
    school: Option<String>,
    /// How long it takes to cast
    casting_time: CastingTime,
    /// Range of the spell
    range: SpellRange,
    /// Required components (verbal, somatic, material)
    components: SpellComponents,
    /// How long the spell lasts
    duration: SpellDuration,
    /// Full description of the spell's effects
    description: String,
    /// Description of effects when cast at higher levels
    higher_levels: Option<String>,
    /// Classes that can learn this spell
    classes: Vec<String>,
    /// Source book reference (e.g., "PHB p.211")
    source: String,
    /// Tags for filtering and categorization
    #[serde(default)]
    tags: Vec<Tag>,
    /// Whether this spell can be cast as a ritual
    #[serde(default)]
    ritual: bool,
    /// Whether this spell requires concentration
    #[serde(default)]
    concentration: bool,
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

    // Read-only accessors

    /// Get the spell's unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the system ID this spell belongs to.
    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    /// Get the spell's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the spell's level.
    pub fn level(&self) -> SpellLevel {
        self.level
    }

    /// Get the school of magic.
    pub fn school(&self) -> Option<&str> {
        self.school.as_deref()
    }

    /// Get the casting time.
    pub fn casting_time(&self) -> &CastingTime {
        &self.casting_time
    }

    /// Get the spell's range.
    pub fn range(&self) -> &SpellRange {
        &self.range
    }

    /// Get the required components.
    pub fn components(&self) -> &SpellComponents {
        &self.components
    }

    /// Get the spell's duration.
    pub fn duration(&self) -> &SpellDuration {
        &self.duration
    }

    /// Get the spell's description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the higher levels description.
    pub fn higher_levels(&self) -> Option<&str> {
        self.higher_levels.as_deref()
    }

    /// Get the classes that can learn this spell.
    pub fn classes(&self) -> &[String] {
        &self.classes
    }

    /// Get the source book reference.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the tags for filtering.
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    /// Check if this spell can be cast as a ritual.
    pub fn ritual(&self) -> bool {
        self.ritual
    }

    /// Check if this spell requires concentration.
    pub fn concentration(&self) -> bool {
        self.concentration
    }

    // Builder-style methods for optional fields

    /// Set the school of magic.
    pub fn with_school(mut self, school: impl Into<String>) -> Self {
        self.school = Some(school.into());
        self
    }

    /// Set the higher levels description.
    pub fn with_higher_levels(mut self, higher_levels: impl Into<String>) -> Self {
        self.higher_levels = Some(higher_levels.into());
        self
    }

    /// Set the tags.
    pub fn with_tags(mut self, tags: Vec<Tag>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a single tag.
    pub fn with_tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Set whether the spell is a ritual.
    pub fn with_ritual(mut self, ritual: bool) -> Self {
        self.ritual = ritual;
        self
    }

    /// Set whether the spell requires concentration.
    pub fn with_concentration(mut self, concentration: bool) -> Self {
        self.concentration = concentration;
        self
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
        let spell = Spell::new(
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
        )
        .with_school("Evocation")
        .with_higher_levels("When cast at 4th level or higher...")
        .with_tag(Tag::new("damage").unwrap())
        .with_tag(Tag::new("fire").unwrap());

        let other = spell.clone();
        assert_eq!(spell, other);
    }

    #[test]
    fn spell_accessors() {
        let spell = Spell::new(
            "test_spell",
            "test_system",
            "Test Spell",
            SpellLevel::Level(1),
            CastingTime::action(),
            SpellRange::touch(),
            SpellComponents::verbal_somatic(),
            SpellDuration::minutes(10),
            "Test description",
            vec!["wizard".into()],
            "Test Source",
        )
        .with_ritual(true)
        .with_concentration(true);

        assert_eq!(spell.id(), "test_spell");
        assert_eq!(spell.system_id(), "test_system");
        assert_eq!(spell.name(), "Test Spell");
        assert_eq!(spell.level(), SpellLevel::Level(1));
        assert!(spell.ritual());
        assert!(spell.concentration());
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
