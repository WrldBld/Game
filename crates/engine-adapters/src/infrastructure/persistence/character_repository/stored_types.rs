//! Persistence serde models for Character data

use serde::{Deserialize, Serialize};
use wrldbldr_domain::entities::StatBlock;
use wrldbldr_domain::value_objects::{ArchetypeChange, CampbellArchetype};

/// Stored representation of StatBlock for Neo4j persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StatBlockStored {
    pub stats: std::collections::HashMap<String, i32>,
    pub current_hp: Option<i32>,
    pub max_hp: Option<i32>,
}

impl From<StatBlock> for StatBlockStored {
    fn from(value: StatBlock) -> Self {
        Self {
            stats: value.stats,
            current_hp: value.current_hp,
            max_hp: value.max_hp,
        }
    }
}

impl From<StatBlockStored> for StatBlock {
    fn from(value: StatBlockStored) -> Self {
        Self {
            stats: value.stats,
            current_hp: value.current_hp,
            max_hp: value.max_hp,
        }
    }
}

/// Stored representation of ArchetypeChange for Neo4j persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ArchetypeChangeStored {
    pub from: String,
    pub to: String,
    pub reason: String,
    pub timestamp: String,
}

impl From<ArchetypeChange> for ArchetypeChangeStored {
    fn from(value: ArchetypeChange) -> Self {
        Self {
            from: format!("{:?}", value.from),
            to: format!("{:?}", value.to),
            reason: value.reason,
            timestamp: value.timestamp.to_rfc3339(),
        }
    }
}

impl From<ArchetypeChangeStored> for ArchetypeChange {
    fn from(value: ArchetypeChangeStored) -> Self {
        // Note: From trait impls can't access ClockPort, so we use epoch as fallback.
        // This is only hit when stored data is corrupted.
        let timestamp = wrldbldr_common::datetime::parse_datetime_or(
            &value.timestamp,
            chrono::DateTime::UNIX_EPOCH,
        );
        Self {
            from: value.from.parse().unwrap_or(CampbellArchetype::Ally),
            to: value.to.parse().unwrap_or(CampbellArchetype::Ally),
            reason: value.reason,
            timestamp,
        }
    }
}
