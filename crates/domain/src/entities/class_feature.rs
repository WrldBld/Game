//! Class feature entity for TTRPG character abilities.
//!
//! Represents abilities and features that characters gain from their class
//! or subclass as they level up.

use serde::{Deserialize, Serialize};

use super::feat::{RechargeType, UsesFormula};

/// A class feature that a character gains from their class or subclass.
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
    pub tags: Vec<String>,
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
    pub tags: Vec<String>,
}

/// A background feature.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_feature_serialization() {
        let feature = ClassFeature {
            id: "dnd5e_fighter_second_wind".into(),
            system_id: "dnd5e".into(),
            class_id: "fighter".into(),
            subclass_id: None,
            name: "Second Wind".into(),
            level: 1,
            description: "You have a limited well of stamina...".into(),
            uses: Some(FeatureUses::short_rest(UsesFormula::Fixed { value: 1 })),
            source: "PHB p.72".into(),
            has_choices: false,
            tags: vec!["healing".into()],
        };

        let json = serde_json::to_string(&feature).unwrap();
        let deserialized: ClassFeature = serde_json::from_str(&json).unwrap();
        assert_eq!(feature, deserialized);
    }

    #[test]
    fn subclass_feature() {
        let feature = ClassFeature {
            id: "dnd5e_champion_improved_critical".into(),
            system_id: "dnd5e".into(),
            class_id: "fighter".into(),
            subclass_id: Some("champion".into()),
            name: "Improved Critical".into(),
            level: 3,
            description: "Your weapon attacks score a critical hit on a roll of 19 or 20.".into(),
            uses: None,
            source: "PHB p.72".into(),
            has_choices: false,
            tags: vec!["combat".into()],
        };

        assert!(feature.subclass_id.is_some());
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
    fn racial_trait_serialization() {
        let trait_ = RacialTrait {
            id: "dnd5e_dwarf_darkvision".into(),
            system_id: "dnd5e".into(),
            race_id: "dwarf".into(),
            subrace_id: None,
            name: "Darkvision".into(),
            description: "You can see in dim light within 60 feet...".into(),
            uses: None,
            source: "PHB p.20".into(),
            tags: vec!["vision".into()],
        };

        let json = serde_json::to_string(&trait_).unwrap();
        let deserialized: RacialTrait = serde_json::from_str(&json).unwrap();
        assert_eq!(trait_, deserialized);
    }
}
