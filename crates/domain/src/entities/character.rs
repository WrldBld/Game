//! Character entity - NPCs with Campbell archetypes
//!
//! # Graph-First Design (Phase 0.C)
//!
//! The following relationships are stored as Neo4j edges, NOT embedded fields:
//! - Wants: `(Character)-[:HAS_WANT]->(Want)`
//! - Inventory: `(Character)-[:POSSESSES]->(Item)`
//! - Location relationships: `HOME_LOCATION`, `WORKS_AT`, `FREQUENTS`, `AVOIDS`
//! - Actantial views: `VIEWS_AS_HELPER`, `VIEWS_AS_OPPONENT`, etc.
//!
//! Archetype history remains as JSON (acceptable per ADR - complex nested non-relational)

use crate::error::DomainError;
use crate::value_objects::{
    ArchetypeChange, CampbellArchetype, DispositionLevel, ExpressionConfig, MoodState,
};
use serde::{Deserialize, Serialize};
use wrldbldr_domain::{CharacterId, WorldId};

// Re-export from value_objects for backwards compatibility
pub use crate::value_objects::{StatBlock, StatModifier, StatValue};

/// A character (NPC) in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub id: CharacterId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    /// Path to sprite image asset
    pub sprite_asset: Option<String>,
    /// Path to portrait image asset
    pub portrait_asset: Option<String>,

    // Campbell Archetype System (Layered)
    /// The character's base archetype
    pub base_archetype: CampbellArchetype,
    /// Current archetype (may differ from base)
    pub current_archetype: CampbellArchetype,
    /// History of archetype changes (stored as JSON - acceptable per ADR)
    pub archetype_history: Vec<ArchetypeChange>,

    // Game Stats (stored as JSON - acceptable per ADR)
    pub stats: StatBlock,

    // Character state
    pub is_alive: bool,
    pub is_active: bool,

    /// Default disposition for this NPC (used when no PC-specific disposition is set)
    pub default_disposition: DispositionLevel,

    // Mood & Expression System (Three-Tier Model)
    /// Default mood for this NPC (Tier 2)
    /// Used when staging doesn't specify a mood; affects default expression and dialogue tone
    pub default_mood: MoodState,
    /// Expression configuration (Tier 3)
    /// Defines available expressions and actions for this character's sprite sheet
    pub expression_config: ExpressionConfig,
}

impl Character {
    pub fn new(
        world_id: WorldId,
        name: impl Into<String>,
        archetype: CampbellArchetype,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        let name = name.trim().to_string();

        if name.is_empty() {
            return Err(DomainError::validation("Character name cannot be empty"));
        }
        if name.len() > 200 {
            return Err(DomainError::validation(
                "Character name cannot exceed 200 characters",
            ));
        }

        Ok(Self {
            id: CharacterId::new(),
            world_id,
            name,
            description: String::new(),
            sprite_asset: None,
            portrait_asset: None,
            base_archetype: archetype,
            current_archetype: archetype,
            archetype_history: Vec::new(),
            stats: StatBlock::default(),
            is_alive: true,
            is_active: true,
            default_disposition: DispositionLevel::Neutral,
            default_mood: MoodState::default(),
            expression_config: ExpressionConfig::default(),
        })
    }

    pub fn with_default_disposition(mut self, disposition: DispositionLevel) -> Self {
        self.default_disposition = disposition;
        self
    }

    pub fn with_default_mood(mut self, mood: MoodState) -> Self {
        self.default_mood = mood;
        self
    }

    pub fn with_expression_config(mut self, config: ExpressionConfig) -> Self {
        self.expression_config = config;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_sprite(mut self, asset_path: impl Into<String>) -> Self {
        self.sprite_asset = Some(asset_path.into());
        self
    }

    pub fn with_portrait(mut self, asset_path: impl Into<String>) -> Self {
        self.portrait_asset = Some(asset_path.into());
        self
    }

    /// Change the character's current archetype
    pub fn change_archetype(
        &mut self,
        new_archetype: CampbellArchetype,
        reason: impl Into<String>,
        now: chrono::DateTime<chrono::Utc>,
    ) {
        let change = ArchetypeChange {
            from: self.current_archetype,
            to: new_archetype,
            reason: reason.into(),
            timestamp: now,
        };
        self.archetype_history.push(change);
        self.current_archetype = new_archetype;
    }

    /// Temporarily assume a different archetype (for a scene)
    pub fn assume_archetype(&mut self, archetype: CampbellArchetype) {
        // Only changes current, doesn't record in history (temporary)
        self.current_archetype = archetype;
    }

    /// Revert to base archetype
    pub fn revert_to_base(&mut self) {
        self.current_archetype = self.base_archetype;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Character::new() validation tests
    // =========================================================================

    #[test]
    fn character_new_empty_name_returns_error() {
        use crate::value_objects::CampbellArchetype;
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let result = Character::new(world_id, "", CampbellArchetype::Hero);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn character_new_whitespace_only_name_returns_error() {
        use crate::value_objects::CampbellArchetype;
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let result = Character::new(world_id, "   \t\n  ", CampbellArchetype::Hero);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn character_new_name_exceeds_200_chars_returns_error() {
        use crate::value_objects::CampbellArchetype;
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let long_name = "a".repeat(201);
        let result = Character::new(world_id, long_name, CampbellArchetype::Hero);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("200"));
    }

    #[test]
    fn character_new_name_exactly_200_chars_succeeds() {
        use crate::value_objects::CampbellArchetype;
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let name_200 = "a".repeat(200);
        let result = Character::new(world_id, name_200.clone(), CampbellArchetype::Hero);

        assert!(result.is_ok());
        let character = result.unwrap();
        assert_eq!(character.name, name_200);
    }

    #[test]
    fn character_new_valid_name_succeeds() {
        use crate::value_objects::CampbellArchetype;
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let result = Character::new(world_id, "Gandalf", CampbellArchetype::Mentor);

        assert!(result.is_ok());
        let character = result.unwrap();
        assert_eq!(character.name, "Gandalf");
        assert_eq!(character.world_id, world_id);
        assert_eq!(character.base_archetype, CampbellArchetype::Mentor);
        assert_eq!(character.current_archetype, CampbellArchetype::Mentor);
    }

    #[test]
    fn character_new_trims_whitespace_from_name() {
        use crate::value_objects::CampbellArchetype;
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let result = Character::new(world_id, "  Frodo Baggins  ", CampbellArchetype::Hero);

        assert!(result.is_ok());
        let character = result.unwrap();
        assert_eq!(character.name, "Frodo Baggins");
    }
}
