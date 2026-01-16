//! Region relationship value objects
//!
//! These types define how characters relate to regions (locations) in the game world.
//! They are domain concepts used across the application for NPC presence determination.

use serde::{Deserialize, Serialize};
use wrldbldr_domain::TimeOfDay;

use crate::error::DomainError;

/// Work shift for a region (when NPC works there)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegionShift {
    Day,
    Night,
    Always,
}

impl std::fmt::Display for RegionShift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegionShift::Day => write!(f, "day"),
            RegionShift::Night => write!(f, "night"),
            RegionShift::Always => write!(f, "always"),
        }
    }
}

impl std::str::FromStr for RegionShift {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "day" => Ok(RegionShift::Day),
            "night" => Ok(RegionShift::Night),
            "always" | "" => Ok(RegionShift::Always),
            _ => Err(DomainError::parse(format!("Invalid region shift: {}", s))),
        }
    }
}

/// How often an NPC visits a region
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegionFrequency {
    Often,
    Sometimes,
    Rarely,
}

impl std::fmt::Display for RegionFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegionFrequency::Often => write!(f, "often"),
            RegionFrequency::Sometimes => write!(f, "sometimes"),
            RegionFrequency::Rarely => write!(f, "rarely"),
        }
    }
}

impl std::str::FromStr for RegionFrequency {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "often" => Ok(RegionFrequency::Often),
            "sometimes" | "" => Ok(RegionFrequency::Sometimes),
            "rarely" => Ok(RegionFrequency::Rarely),
            _ => Err(DomainError::parse(format!(
                "Invalid region frequency: {}",
                s
            ))),
        }
    }
}

/// Type of relationship between character and region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegionRelationshipType {
    Home,
    WorksAt { shift: RegionShift },
    Frequents { frequency: RegionFrequency },
    Avoids { reason: String },
}

impl RegionRelationshipType {
    /// Determine if an NPC with this relationship would be present at the given time of day.
    ///
    /// This is the canonical implementation of NPC presence rules:
    /// - Home: present at night/evening (sleeping/resting time)
    /// - WorksAt: depends on shift (day workers in morning/afternoon, night workers evening/night)
    /// - Frequents: depends on frequency (often=always, sometimes=afternoon/evening, rarely=never)
    /// - Avoids: never present
    pub fn is_npc_present(&self, time_of_day: TimeOfDay) -> bool {
        match self {
            RegionRelationshipType::Home => {
                matches!(time_of_day, TimeOfDay::Night | TimeOfDay::Evening)
            }
            RegionRelationshipType::WorksAt { shift } => match shift {
                RegionShift::Always => true,
                RegionShift::Day => {
                    matches!(time_of_day, TimeOfDay::Morning | TimeOfDay::Afternoon)
                }
                RegionShift::Night => matches!(time_of_day, TimeOfDay::Evening | TimeOfDay::Night),
            },
            RegionRelationshipType::Frequents { frequency } => match frequency {
                RegionFrequency::Often => true,
                RegionFrequency::Sometimes => {
                    matches!(time_of_day, TimeOfDay::Afternoon | TimeOfDay::Evening)
                }
                RegionFrequency::Rarely => false,
            },
            RegionRelationshipType::Avoids { .. } => false,
        }
    }

    /// Get a human-readable reasoning for presence at the given time of day.
    pub fn presence_reasoning(&self, time_of_day: TimeOfDay) -> String {
        match self {
            RegionRelationshipType::Home => {
                format!(
                    "Lives here. {} is typically home time.",
                    time_of_day.display_name()
                )
            }
            RegionRelationshipType::WorksAt { shift } => {
                format!(
                    "Works here ({} shift). Current time: {}",
                    shift,
                    time_of_day.display_name()
                )
            }
            RegionRelationshipType::Frequents { frequency } => {
                format!(
                    "Frequents here ({}). Current time: {}",
                    frequency,
                    time_of_day.display_name()
                )
            }
            RegionRelationshipType::Avoids { reason } => {
                format!("Avoids this location: {}", reason)
            }
        }
    }
}

/// A character's relationship to a region
#[derive(Debug, Clone)]
pub struct RegionRelationship {
    region_id: wrldbldr_domain::RegionId,
    region_name: String,
    relationship_type: RegionRelationshipType,
}

impl RegionRelationship {
    /// Create a new region relationship
    pub fn new(
        region_id: wrldbldr_domain::RegionId,
        region_name: impl Into<String>,
        relationship_type: RegionRelationshipType,
    ) -> Self {
        Self {
            region_id,
            region_name: region_name.into(),
            relationship_type,
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────

    /// Get the region ID
    pub fn region_id(&self) -> wrldbldr_domain::RegionId {
        self.region_id
    }

    /// Get the region name
    pub fn region_name(&self) -> &str {
        &self.region_name
    }

    /// Get the relationship type
    pub fn relationship_type(&self) -> &RegionRelationshipType {
        &self.relationship_type
    }
}
