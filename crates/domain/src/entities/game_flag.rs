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
    pub name: String,
    /// The flag value (true = set, false = unset)
    pub value: bool,
    /// When the flag was last modified
    pub updated_at: DateTime<Utc>,
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
