//! Game Flag entity - Persistent boolean flags for gameplay state
//!
//! Flags can be set/unset during gameplay to track story progress,
//! player choices, and other persistent state. Used by:
//! - Scene entry conditions (`SceneCondition::FlagSet`)
//! - Narrative trigger conditions (`NarrativeTriggerType::FlagSet`)
//! - Interaction requirements
//!
//! ## Graph Design
//!
//! Flags are stored as edges from World to a Flag pseudo-node:
//! - `(World)-[:HAS_FLAG {value: true}]->(Flag {name: "flag_name"})`
//!
//! For PC-scoped flags:
//! - `(PlayerCharacter)-[:HAS_FLAG {value: true}]->(Flag {name: "flag_name"})`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wrldbldr_domain::{PlayerCharacterId, WorldId};

/// A game flag with its current value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFlag {
    /// The flag name (unique within scope)
    name: String,
    /// The flag value (true = set, false = unset)
    value: bool,
    /// When the flag was last modified
    updated_at: DateTime<Utc>,
}

impl GameFlag {
    /// Create a new flag
    pub fn new(name: impl Into<String>, value: bool, now: DateTime<Utc>) -> Self {
        Self {
            name: name.into(),
            value,
            updated_at: now,
        }
    }

    /// Create a set flag
    pub fn set(name: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self::new(name, true, now)
    }

    /// Create an unset flag
    pub fn unset(name: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self::new(name, false, now)
    }

    // Read accessors
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> bool {
        self.value
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // Builder methods
    pub fn with_value(mut self, value: bool) -> Self {
        self.value = value;
        self
    }

    pub fn with_updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = updated_at;
        self
    }
}

/// Scope for a game flag
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlagScope {
    /// World-scoped flag (shared by all players)
    World(WorldId),
    /// PC-scoped flag (specific to one player character)
    PlayerCharacter(PlayerCharacterId),
}

impl FlagScope {
    /// Returns the WorldId if this is a World scope
    pub fn world_id(&self) -> Option<WorldId> {
        match self {
            FlagScope::World(id) => Some(*id),
            FlagScope::PlayerCharacter(_) => None,
        }
    }

    /// Returns the PlayerCharacterId if this is a PlayerCharacter scope
    pub fn player_character_id(&self) -> Option<PlayerCharacterId> {
        match self {
            FlagScope::World(_) => None,
            FlagScope::PlayerCharacter(id) => Some(*id),
        }
    }

    /// Returns true if this is a world-scoped flag
    pub fn is_world(&self) -> bool {
        matches!(self, FlagScope::World(_))
    }

    /// Returns true if this is a player-character-scoped flag
    pub fn is_player_character(&self) -> bool {
        matches!(self, FlagScope::PlayerCharacter(_))
    }
}
