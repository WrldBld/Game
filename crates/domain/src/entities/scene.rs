//! Scene entity - Complete storytelling unit
//!
//! # Graph-First Design (Phase 0.D)
//!
//! The following relationships are stored as Neo4j edges, NOT embedded fields:
//! - Location: `(Scene)-[:AT_LOCATION]->(Location)`
//! - Featured characters: `(Scene)-[:FEATURES_CHARACTER {role, entrance_cue}]->(Character)`
//!
//! Entry conditions remain as JSON (acceptable per ADR - complex nested non-relational)

use serde::{Deserialize, Serialize};
use wrldbldr_domain::{CharacterId, SceneId, TimeOfDay};

/// Time context for a scene
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimeContext {
    /// No specific time
    Unspecified,
    /// Time of day
    TimeOfDay(TimeOfDay),
    /// Relative to an event
    During(String),
    /// Specific description
    Custom(String),
}

// TimeOfDay is imported from value_objects - the canonical 4-variant type

/// Condition for entering a scene
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SceneCondition {
    /// Must have completed another scene
    CompletedScene(SceneId),
    /// Must have a specific item
    HasItem(wrldbldr_domain::ItemId),
    /// Must have a relationship with a character
    KnowsCharacter(wrldbldr_domain::CharacterId),
    /// A flag must be set
    FlagSet(String),
    /// Custom condition expression
    Custom(String),
}

/// Data for the FEATURES_CHARACTER edge between Scene and Character
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneCharacter {
    /// The character's ID
    character_id: CharacterId,
    /// The character's role in this scene
    role: SceneCharacterRole,
    /// When/how the character enters the scene
    entrance_cue: Option<String>,
}

impl SceneCharacter {
    pub fn new(character_id: CharacterId, role: SceneCharacterRole) -> Self {
        Self {
            character_id,
            role,
            entrance_cue: None,
        }
    }

    /// Create a scene character from parts (for reconstitution from storage)
    pub fn from_parts(
        character_id: CharacterId,
        role: SceneCharacterRole,
        entrance_cue: Option<String>,
    ) -> Self {
        Self {
            character_id,
            role,
            entrance_cue,
        }
    }

    // Read-only accessors

    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    pub fn role(&self) -> SceneCharacterRole {
        self.role
    }

    pub fn entrance_cue(&self) -> Option<&str> {
        self.entrance_cue.as_deref()
    }

    // Builder methods

    pub fn with_entrance_cue(mut self, cue: impl Into<String>) -> Self {
        self.entrance_cue = Some(cue.into());
        self
    }
}

/// Role a character plays in a scene
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SceneCharacterRole {
    /// Primary character in the scene
    Primary,
    /// Secondary/supporting character
    Secondary,
    /// Background character (ambient presence)
    Background,
}

impl std::fmt::Display for SceneCharacterRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "Primary"),
            Self::Secondary => write!(f, "Secondary"),
            Self::Background => write!(f, "Background"),
        }
    }
}

impl std::str::FromStr for SceneCharacterRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Primary" => Ok(Self::Primary),
            "Secondary" => Ok(Self::Secondary),
            "Background" => Ok(Self::Background),
            _ => Err(()),
        }
    }
}
