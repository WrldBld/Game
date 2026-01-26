//! Class feature entity for TTRPG character abilities.
//!
//! Represents abilities and features that characters gain from their class
//! or subclass as they level up.

use serde::{Deserialize, Serialize};

use super::feat::{RechargeType, UsesFormula};
use crate::value_objects::Tag;

/// A class feature that a character gains from their class or subclass.
///
/// # ADR-008 Tier 4: Simple Data Struct
///
/// This is a data-carrying struct with no invariants to protect. All fields are public
/// because there's no invalid state that can be constructed - any combination of values
/// is valid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClassFeature {
    /// Unique identifier for this feature
    pub id: String,
    /// Which game system this feature belongs to (e.g., "dnd5e", "pf2e")
    pub system_id: String,
    /// ID of the class that grants this feature
    pub class_id: String,
    /// ID of the subclass (if this is a subclass feature)
    pub subclass_id: Option<String>,
    /// Display name of the feature
    pub name: String,
    /// Level at which this feature is gained
    pub level: u8,
    /// Full description of what the feature does
    pub description: String,
    /// Uses tracking (if the feature has limited uses)
    pub uses: Option<FeatureUses>,
    /// Source book reference
    pub source: String,
    /// Whether this feature grants choices
    #[serde(default)]
    pub has_choices: bool,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<Tag>,
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
///
/// # ADR-008 Tier 4: Simple Data Struct
///
/// This is a data-carrying struct with no invariants to protect. All fields are public
/// because there's no invalid state that can be constructed - any combination of values
/// is valid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RacialTrait {
    /// Unique identifier for this trait
    pub id: String,
    /// Which game system this trait belongs to
    pub system_id: String,
    /// ID of the race/ancestry that grants this trait
    pub race_id: String,
    /// ID of the subrace (if this is a subrace trait)
    pub subrace_id: Option<String>,
    /// Display name of the trait
    pub name: String,
    /// Full description of what the trait does
    pub description: String,
    /// Uses tracking (if the trait has limited uses)
    pub uses: Option<FeatureUses>,
    /// Source book reference
    pub source: String,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<Tag>,
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
}

/// A background feature.
///
/// # ADR-008 Tier 4: Simple Data Struct
///
/// This is a data-carrying struct with no invariants to protect. All fields are public
/// because there's no invalid state that can be constructed - any combination of values
/// is valid.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundFeature {
    /// Unique identifier for this feature
    pub id: String,
    /// Which game system this feature belongs to
    pub system_id: String,
    /// ID of the background that grants this feature
    pub background_id: String,
    /// Display name of the feature
    pub name: String,
    /// Full description of what the feature does
    pub description: String,
    /// Source book reference
    pub source: String,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_feature_equality() {
        let mut feature = ClassFeature::new(
            "dnd5e_fighter_second_wind",
            "dnd5e",
            "fighter",
            "Second Wind",
            1,
            "You have a limited well of stamina...",
            "PHB p.72",
        );
        feature.uses = Some(FeatureUses::short_rest(UsesFormula::Fixed { value: 1 }));
        feature.tags.push(Tag::new("healing").unwrap());

        let other = feature.clone();
        assert_eq!(feature, other);
    }

    #[test]
    fn class_feature_accessors() {
        let mut feature = ClassFeature::new(
            "test_feature",
            "test_system",
            "test_class",
            "Test Feature",
            5,
            "Test description",
            "Test Source",
        );
        feature.has_choices = true;

        assert_eq!(feature.id, "test_feature");
        assert_eq!(feature.system_id, "test_system");
        assert_eq!(feature.class_id, "test_class");
        assert_eq!(feature.name, "Test Feature");
        assert_eq!(feature.level, 5);
        assert!(feature.has_choices);
    }

    #[test]
    fn subclass_feature() {
        let mut feature = ClassFeature::new(
            "dnd5e_champion_improved_critical",
            "dnd5e",
            "fighter",
            "Improved Critical",
            3,
            "Your weapon attacks score a critical hit on a roll of 19 or 20.",
            "PHB p.72",
        );
        feature.subclass_id = Some("champion".to_string());
        feature.tags.push(Tag::new("combat").unwrap());

        assert_eq!(feature.subclass_id.as_deref(), Some("champion"));
        assert!(feature.uses.is_none());
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
        let mut trait_ = RacialTrait::new(
            "dnd5e_dwarf_darkvision",
            "dnd5e",
            "dwarf",
            "Darkvision",
            "You can see in dim light within 60 feet...",
            "PHB p.20",
        );
        trait_.tags.push(Tag::new("vision").unwrap());

        let other = trait_.clone();
        assert_eq!(trait_, other);
    }

    #[test]
    fn racial_trait_accessors() {
        let mut trait_ = RacialTrait::new(
            "test_trait",
            "test_system",
            "test_race",
            "Test Trait",
            "Test description",
            "Test Source",
        );
        trait_.subrace_id = Some("test_subrace".to_string());

        assert_eq!(trait_.id, "test_trait");
        assert_eq!(trait_.race_id, "test_race");
        assert_eq!(trait_.subrace_id.as_deref(), Some("test_subrace"));
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

        assert_eq!(feature.id, "test_bg_feature");
        assert_eq!(feature.background_id, "test_background");
        assert_eq!(feature.name, "Test Background Feature");
    }
}
