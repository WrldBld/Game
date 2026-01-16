//! Class feature entity for TTRPG character abilities.
//!
//! Represents abilities and features that characters gain from their class
//! or subclass as they level up.

use serde::{Deserialize, Serialize};

use super::feat::{RechargeType, UsesFormula};
use crate::value_objects::Tag;

/// A class feature that a character gains from their class or subclass.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClassFeature {
    /// Unique identifier for this feature
    id: String,
    /// Which game system this feature belongs to (e.g., "dnd5e", "pf2e")
    system_id: String,
    /// ID of the class that grants this feature
    class_id: String,
    /// ID of the subclass (if this is a subclass feature)
    subclass_id: Option<String>,
    /// Display name of the feature
    name: String,
    /// Level at which this feature is gained
    level: u8,
    /// Full description of what the feature does
    description: String,
    /// Uses tracking (if the feature has limited uses)
    uses: Option<FeatureUses>,
    /// Source book reference
    source: String,
    /// Whether this feature grants choices
    #[serde(default)]
    has_choices: bool,
    /// Tags for categorization
    #[serde(default)]
    tags: Vec<Tag>,
}

impl ClassFeature {
    /// Create a new class feature with required fields.
    pub fn new(
        id: impl Into<String>,
        system_id: impl Into<String>,
        class_id: impl Into<String>,
        name: impl Into<String>,
        level: u8,
        description: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            system_id: system_id.into(),
            class_id: class_id.into(),
            subclass_id: None,
            name: name.into(),
            level,
            description: description.into(),
            uses: None,
            source: source.into(),
            has_choices: false,
            tags: Vec::new(),
        }
    }

    // Read-only accessors

    /// Get the feature's unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the system ID this feature belongs to.
    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    /// Get the class ID that grants this feature.
    pub fn class_id(&self) -> &str {
        &self.class_id
    }

    /// Get the subclass ID (if this is a subclass feature).
    pub fn subclass_id(&self) -> Option<&str> {
        self.subclass_id.as_deref()
    }

    /// Get the feature's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the level at which this feature is gained.
    pub fn level(&self) -> u8 {
        self.level
    }

    /// Get the feature's description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the uses tracking.
    pub fn uses(&self) -> Option<&FeatureUses> {
        self.uses.as_ref()
    }

    /// Get the source book reference.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Check if this feature grants choices.
    pub fn has_choices(&self) -> bool {
        self.has_choices
    }

    /// Get the tags for categorization.
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    // Builder-style methods for optional fields

    /// Set the subclass ID.
    pub fn with_subclass_id(mut self, subclass_id: impl Into<String>) -> Self {
        self.subclass_id = Some(subclass_id.into());
        self
    }

    /// Set the uses tracking.
    pub fn with_uses(mut self, uses: FeatureUses) -> Self {
        self.uses = Some(uses);
        self
    }

    /// Set whether this feature grants choices.
    pub fn with_has_choices(mut self, has_choices: bool) -> Self {
        self.has_choices = has_choices;
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
}

/// Limited uses tracking for a class feature.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FeatureUses {
    /// How many uses the feature has
    pub max: UsesFormula,
    /// When uses are restored
    pub recharge: RechargeType,
}

impl FeatureUses {
    /// Create new feature uses.
    pub fn new(max: UsesFormula, recharge: RechargeType) -> Self {
        Self { max, recharge }
    }

    /// Create uses that recharge on a short rest.
    pub fn short_rest(max: UsesFormula) -> Self {
        Self {
            max,
            recharge: RechargeType::ShortRest,
        }
    }

    /// Create uses that recharge on a long rest.
    pub fn long_rest(max: UsesFormula) -> Self {
        Self {
            max,
            recharge: RechargeType::LongRest,
        }
    }

    /// Create fixed uses that recharge on a long rest.
    pub fn fixed_long_rest(value: u8) -> Self {
        Self {
            max: UsesFormula::Fixed { value },
            recharge: RechargeType::LongRest,
        }
    }

    /// Create uses equal to proficiency bonus, recharging on a long rest.
    pub fn proficiency_long_rest() -> Self {
        Self {
            max: UsesFormula::ProficiencyBonus,
            recharge: RechargeType::LongRest,
        }
    }
}

/// A racial trait or ancestry feature.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RacialTrait {
    /// Unique identifier for this trait
    id: String,
    /// Which game system this trait belongs to
    system_id: String,
    /// ID of the race/ancestry that grants this trait
    race_id: String,
    /// ID of the subrace (if this is a subrace trait)
    subrace_id: Option<String>,
    /// Display name of the trait
    name: String,
    /// Full description of what the trait does
    description: String,
    /// Uses tracking (if the trait has limited uses)
    uses: Option<FeatureUses>,
    /// Source book reference
    source: String,
    /// Tags for categorization
    #[serde(default)]
    tags: Vec<Tag>,
}

impl RacialTrait {
    /// Create a new racial trait with required fields.
    pub fn new(
        id: impl Into<String>,
        system_id: impl Into<String>,
        race_id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            system_id: system_id.into(),
            race_id: race_id.into(),
            subrace_id: None,
            name: name.into(),
            description: description.into(),
            uses: None,
            source: source.into(),
            tags: Vec::new(),
        }
    }

    // Read-only accessors

    /// Get the trait's unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the system ID this trait belongs to.
    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    /// Get the race ID that grants this trait.
    pub fn race_id(&self) -> &str {
        &self.race_id
    }

    /// Get the subrace ID (if this is a subrace trait).
    pub fn subrace_id(&self) -> Option<&str> {
        self.subrace_id.as_deref()
    }

    /// Get the trait's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the trait's description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the uses tracking.
    pub fn uses(&self) -> Option<&FeatureUses> {
        self.uses.as_ref()
    }

    /// Get the source book reference.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the tags for categorization.
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    // Builder-style methods for optional fields

    /// Set the subrace ID.
    pub fn with_subrace_id(mut self, subrace_id: impl Into<String>) -> Self {
        self.subrace_id = Some(subrace_id.into());
        self
    }

    /// Set the uses tracking.
    pub fn with_uses(mut self, uses: FeatureUses) -> Self {
        self.uses = Some(uses);
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
}

/// A background feature.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundFeature {
    /// Unique identifier for this feature
    id: String,
    /// Which game system this feature belongs to
    system_id: String,
    /// ID of the background that grants this feature
    background_id: String,
    /// Display name of the feature
    name: String,
    /// Full description of what the feature does
    description: String,
    /// Source book reference
    source: String,
}

impl BackgroundFeature {
    /// Create a new background feature with required fields.
    pub fn new(
        id: impl Into<String>,
        system_id: impl Into<String>,
        background_id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            system_id: system_id.into(),
            background_id: background_id.into(),
            name: name.into(),
            description: description.into(),
            source: source.into(),
        }
    }

    // Read-only accessors

    /// Get the feature's unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the system ID this feature belongs to.
    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    /// Get the background ID that grants this feature.
    pub fn background_id(&self) -> &str {
        &self.background_id
    }

    /// Get the feature's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the feature's description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the source book reference.
    pub fn source(&self) -> &str {
        &self.source
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_feature_equality() {
        let feature = ClassFeature::new(
            "dnd5e_fighter_second_wind",
            "dnd5e",
            "fighter",
            "Second Wind",
            1,
            "You have a limited well of stamina...",
            "PHB p.72",
        )
        .with_uses(FeatureUses::short_rest(UsesFormula::Fixed { value: 1 }))
        .with_tag(Tag::new("healing").unwrap());

        let other = feature.clone();
        assert_eq!(feature, other);
    }

    #[test]
    fn class_feature_accessors() {
        let feature = ClassFeature::new(
            "test_feature",
            "test_system",
            "test_class",
            "Test Feature",
            5,
            "Test description",
            "Test Source",
        )
        .with_has_choices(true);

        assert_eq!(feature.id(), "test_feature");
        assert_eq!(feature.system_id(), "test_system");
        assert_eq!(feature.class_id(), "test_class");
        assert_eq!(feature.name(), "Test Feature");
        assert_eq!(feature.level(), 5);
        assert!(feature.has_choices());
    }

    #[test]
    fn subclass_feature() {
        let feature = ClassFeature::new(
            "dnd5e_champion_improved_critical",
            "dnd5e",
            "fighter",
            "Improved Critical",
            3,
            "Your weapon attacks score a critical hit on a roll of 19 or 20.",
            "PHB p.72",
        )
        .with_subclass_id("champion")
        .with_tag(Tag::new("combat").unwrap());

        assert_eq!(feature.subclass_id(), Some("champion"));
        assert!(feature.uses().is_none());
    }

    #[test]
    fn feature_uses_constructors() {
        let uses = FeatureUses::fixed_long_rest(2);
        assert!(matches!(uses.max, UsesFormula::Fixed { value: 2 }));
        assert_eq!(uses.recharge, RechargeType::LongRest);

        let uses = FeatureUses::proficiency_long_rest();
        assert!(matches!(uses.max, UsesFormula::ProficiencyBonus));
    }

    #[test]
    fn racial_trait_equality() {
        let trait_ = RacialTrait::new(
            "dnd5e_dwarf_darkvision",
            "dnd5e",
            "dwarf",
            "Darkvision",
            "You can see in dim light within 60 feet...",
            "PHB p.20",
        )
        .with_tag(Tag::new("vision").unwrap());

        let other = trait_.clone();
        assert_eq!(trait_, other);
    }

    #[test]
    fn racial_trait_accessors() {
        let trait_ = RacialTrait::new(
            "test_trait",
            "test_system",
            "test_race",
            "Test Trait",
            "Test description",
            "Test Source",
        )
        .with_subrace_id("test_subrace");

        assert_eq!(trait_.id(), "test_trait");
        assert_eq!(trait_.race_id(), "test_race");
        assert_eq!(trait_.subrace_id(), Some("test_subrace"));
    }

    #[test]
    fn background_feature_accessors() {
        let feature = BackgroundFeature::new(
            "test_bg_feature",
            "test_system",
            "test_background",
            "Test Background Feature",
            "Test description",
            "Test Source",
        );

        assert_eq!(feature.id(), "test_bg_feature");
        assert_eq!(feature.background_id(), "test_background");
        assert_eq!(feature.name(), "Test Background Feature");
    }
}
