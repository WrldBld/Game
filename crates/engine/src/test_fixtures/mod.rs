//! Test fixtures loader for JSON fixture files and common test helpers.
//!
//! This module provides utilities for loading test data from the `test_data/` directory
//! and helper functions for creating test scenarios.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::test_fixtures::{load_fixture, characters};
//!
//! #[test]
//! fn test_fighter_has_feats() {
//!     let pc = characters::fighter_5();
//!     // ... test logic
//! }
//! ```

pub mod image_mocks;
pub mod llm_integration;
pub mod world_seeder;

use std::collections::HashMap;
use std::path::PathBuf;

use wrldbldr_domain::{
    CharacterSheetData, NarrativeTriggerType, PlayerCharacter, TriggerContext, World,
};

// =============================================================================
// Fixture Loading
// =============================================================================

/// Load a JSON fixture from test_data/ directory.
///
/// # Panics
///
/// Panics if the fixture file cannot be read or parsed.
pub fn load_fixture<T: serde::de::DeserializeOwned>(path: &str) -> T {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join(path);
    let content = std::fs::read_to_string(&fixture_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read fixture '{}': {}",
            fixture_path.display(),
            e
        )
    });
    serde_json::from_str(&content).unwrap_or_else(|e| {
        panic!(
            "Failed to parse fixture '{}': {}",
            fixture_path.display(),
            e
        )
    })
}

/// Load a fixture and return Option instead of panicking.
pub fn try_load_fixture<T: serde::de::DeserializeOwned>(path: &str) -> Option<T> {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join(path);
    let content = std::fs::read_to_string(&fixture_path).ok()?;
    serde_json::from_str(&content).ok()
}

// =============================================================================
// Character Fixtures
// =============================================================================

/// Pre-built character fixtures for testing.
pub mod characters {
    use super::*;

    /// Load the Level 5 Fighter (Tharion Ironforge).
    ///
    /// - Race: Human
    /// - Class: Fighter 5 (Champion)
    /// - Feat: Great Weapon Master
    /// - High STR (18), good CON (16)
    pub fn fighter_5() -> PlayerCharacter {
        load_fixture("dnd5e/characters/fighter_5.json")
    }

    /// Load the Level 3 Wizard (Elara Moonwhisper).
    ///
    /// - Race: High Elf
    /// - Class: Wizard 3 (Evocation)
    /// - Cantrips: Fire Bolt, Prestidigitation, Light, Mage Hand
    /// - Spells: Magic Missile, Shield, Fireball, etc.
    /// - High INT (17)
    pub fn wizard_3() -> PlayerCharacter {
        load_fixture("dnd5e/characters/wizard_3.json")
    }

    /// Load the Multiclass Fighter/Wizard (Kael Stormborn).
    ///
    /// - Race: Human Variant
    /// - Class: Fighter 3 (Eldritch Knight) / Wizard 2
    /// - Feat: War Caster
    /// - Balanced STR (16) and INT (14)
    pub fn multiclass() -> PlayerCharacter {
        load_fixture("dnd5e/characters/multiclass.json")
    }

    /// Get all test characters.
    pub fn all() -> Vec<PlayerCharacter> {
        vec![fighter_5(), wizard_3(), multiclass()]
    }
}

// =============================================================================
// World Fixtures
// =============================================================================

/// Pre-built world fixtures for testing.
pub mod worlds {
    use super::*;

    /// Create a D&D 5e example world (The Realm of Shadows).
    ///
    /// Created programmatically to ensure all nested structures are correct.
    pub fn dnd5e() -> World {
        use chrono::Utc;
        use wrldbldr_domain::value_objects::RuleSystemConfig;

        let mut world = World::new(
            "The Realm of Shadows",
            "A classic fantasy world for testing D&D 5e mechanics.",
            Utc::now(),
        )
        .expect("valid world");
        world.rule_system = RuleSystemConfig::dnd_5e();
        world.id = wrldbldr_domain::WorldId::from(
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        );
        world
    }
}

// =============================================================================
// Trigger Context Helpers
// =============================================================================

/// Create a TriggerContext from a PlayerCharacter.
///
/// This extracts compendium data (race, class, spells, feats) from the
/// character's sheet_data and populates the corresponding TriggerContext fields.
pub fn trigger_context_from_pc(pc: &PlayerCharacter) -> TriggerContext {
    let mut ctx = TriggerContext::default();

    let Some(sheet_data) = pc.sheet_data() else {
        return ctx;
    };

    // Extract origin (race)
    if let Some(identity_json) = sheet_data.get("character_identity") {
        if let Ok(identity) =
            serde_json::from_value::<wrldbldr_domain::CharacterIdentity>(identity_json.clone())
        {
            ctx.origin_id = identity.race.clone().map(|r| r.to_lowercase());
            for class_entry in &identity.classes {
                ctx.class_levels
                    .insert(class_entry.class_id.to_lowercase(), class_entry.level);
            }
        }
    }

    // Fallback: extract from individual fields
    if ctx.origin_id.is_none() {
        if let Some(race) = sheet_data.get_string("RACE") {
            ctx.origin_id = Some(race.to_lowercase());
        }
    }

    if ctx.class_levels.is_empty() {
        if let Some(class_name) = sheet_data.get_string("CLASS") {
            let level = sheet_data.get_number("LEVEL").unwrap_or(1) as u8;
            ctx.class_levels.insert(class_name.to_lowercase(), level);
        }
    }

    // Extract spells
    if let Some(spells_json) = sheet_data.get("character_spells") {
        if let Ok(spells) =
            serde_json::from_value::<wrldbldr_domain::CharacterSpells>(spells_json.clone())
        {
            ctx.known_spells = spells
                .known
                .iter()
                .map(|s| s.spell_id.to_lowercase())
                .collect();
        }
    }

    // Extract feats
    if let Some(feats_json) = sheet_data.get("character_feats") {
        if let Ok(feats) =
            serde_json::from_value::<wrldbldr_domain::CharacterFeats>(feats_json.clone())
        {
            ctx.character_feats = feats
                .feats
                .iter()
                .map(|f| f.feat_id.to_lowercase())
                .collect();
        }
    }

    ctx
}

// =============================================================================
// Trigger Fixtures
// =============================================================================

/// Pre-built trigger fixtures for testing.
pub mod triggers {
    use super::*;

    /// HasClass trigger for Fighter with minimum level.
    pub fn has_class_fighter(min_level: Option<u8>) -> NarrativeTriggerType {
        NarrativeTriggerType::HasClass {
            class_id: "fighter".to_string(),
            class_name: "Fighter".to_string(),
            min_level,
        }
    }

    /// HasClass trigger for Wizard with minimum level.
    pub fn has_class_wizard(min_level: Option<u8>) -> NarrativeTriggerType {
        NarrativeTriggerType::HasClass {
            class_id: "wizard".to_string(),
            class_name: "Wizard".to_string(),
            min_level,
        }
    }

    /// HasOrigin trigger for a specific race.
    pub fn has_origin(origin_id: &str, origin_name: &str) -> NarrativeTriggerType {
        NarrativeTriggerType::HasOrigin {
            origin_id: origin_id.to_string(),
            origin_name: origin_name.to_string(),
        }
    }

    /// KnowsSpell trigger for a specific spell.
    pub fn knows_spell(spell_id: &str, spell_name: &str) -> NarrativeTriggerType {
        NarrativeTriggerType::KnowsSpell {
            spell_id: spell_id.to_string(),
            spell_name: spell_name.to_string(),
        }
    }

    /// HasFeat trigger for a specific feat.
    pub fn has_feat(feat_id: &str, feat_name: &str) -> NarrativeTriggerType {
        NarrativeTriggerType::HasFeat {
            feat_id: feat_id.to_string(),
            feat_name: feat_name.to_string(),
        }
    }
}

// =============================================================================
// Sheet Data Helpers
// =============================================================================

/// Helper to build CharacterSheetData for tests.
pub struct SheetDataBuilder {
    values: HashMap<String, serde_json::Value>,
}

impl SheetDataBuilder {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn with_ability_scores(
        mut self,
        str: i64,
        dex: i64,
        con: i64,
        int: i64,
        wis: i64,
        cha: i64,
    ) -> Self {
        self.values
            .insert("STR".to_string(), serde_json::json!(str));
        self.values
            .insert("DEX".to_string(), serde_json::json!(dex));
        self.values
            .insert("CON".to_string(), serde_json::json!(con));
        self.values
            .insert("INT".to_string(), serde_json::json!(int));
        self.values
            .insert("WIS".to_string(), serde_json::json!(wis));
        self.values
            .insert("CHA".to_string(), serde_json::json!(cha));
        self
    }

    pub fn with_race(mut self, race: &str) -> Self {
        self.values
            .insert("RACE".to_string(), serde_json::json!(race));
        self
    }

    pub fn with_class(mut self, class: &str, level: u8) -> Self {
        self.values
            .insert("CLASS".to_string(), serde_json::json!(class));
        self.values
            .insert("LEVEL".to_string(), serde_json::json!(level));
        self
    }

    pub fn with_identity(mut self, identity: wrldbldr_domain::CharacterIdentity) -> Self {
        self.values.insert(
            "character_identity".to_string(),
            serde_json::to_value(identity).unwrap(),
        );
        self
    }

    pub fn with_spells(mut self, spells: wrldbldr_domain::CharacterSpells) -> Self {
        self.values.insert(
            "character_spells".to_string(),
            serde_json::to_value(spells).unwrap(),
        );
        self
    }

    pub fn with_feats(mut self, feats: wrldbldr_domain::CharacterFeats) -> Self {
        self.values.insert(
            "character_feats".to_string(),
            serde_json::to_value(feats).unwrap(),
        );
        self
    }

    pub fn build(self) -> CharacterSheetData {
        CharacterSheetData { values: self.values }
    }
}

impl Default for SheetDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_fighter_fixture() {
        let pc = characters::fighter_5();
        assert_eq!(pc.name().as_str(), "Tharion Ironforge");
        assert!(pc.sheet_data().is_some());
    }

    #[test]
    fn test_load_wizard_fixture() {
        let pc = characters::wizard_3();
        assert_eq!(pc.name().as_str(), "Elara Moonwhisper");
        assert!(pc.sheet_data().is_some());
    }

    #[test]
    fn test_load_multiclass_fixture() {
        let pc = characters::multiclass();
        assert_eq!(pc.name().as_str(), "Kael Stormborn");
        assert!(pc.sheet_data().is_some());
    }

    #[test]
    fn test_trigger_context_from_fighter() {
        let pc = characters::fighter_5();
        let ctx = trigger_context_from_pc(&pc);

        assert_eq!(ctx.origin_id, Some("human".to_string()));
        assert_eq!(ctx.class_levels.get("fighter"), Some(&5));
        assert!(ctx.known_spells.is_empty());
        assert!(ctx.character_feats.contains(&"great_weapon_master".to_string()));
    }

    #[test]
    fn test_trigger_context_from_wizard() {
        let pc = characters::wizard_3();
        let ctx = trigger_context_from_pc(&pc);

        assert_eq!(ctx.origin_id, Some("elf".to_string()));
        assert_eq!(ctx.class_levels.get("wizard"), Some(&3));
        assert!(ctx.known_spells.contains(&"fireball".to_string()));
        assert!(ctx.known_spells.contains(&"magic_missile".to_string()));
        assert!(ctx.character_feats.is_empty());
    }

    #[test]
    fn test_trigger_context_from_multiclass() {
        let pc = characters::multiclass();
        let ctx = trigger_context_from_pc(&pc);

        assert_eq!(ctx.origin_id, Some("human".to_string()));
        assert_eq!(ctx.class_levels.get("fighter"), Some(&3));
        assert_eq!(ctx.class_levels.get("wizard"), Some(&2));
        assert!(ctx.known_spells.contains(&"shield".to_string()));
        assert!(ctx.character_feats.contains(&"war_caster".to_string()));
    }

    #[test]
    fn test_sheet_data_builder() {
        let sheet = SheetDataBuilder::new()
            .with_ability_scores(16, 14, 12, 10, 13, 8)
            .with_race("human")
            .with_class("fighter", 5)
            .build();

        assert_eq!(sheet.get_number("STR"), Some(16));
        assert_eq!(sheet.get_string("RACE"), Some("human"));
        assert_eq!(sheet.get_number("LEVEL"), Some(5));
    }

    #[test]
    fn test_load_world_fixture() {
        let world = worlds::dnd5e();
        assert_eq!(world.name, "The Realm of Shadows");
    }
}
