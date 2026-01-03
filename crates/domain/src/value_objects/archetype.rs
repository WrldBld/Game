//! Campbell's character archetypes from "The Hero with a Thousand Faces"
//!
//! Re-exports shared types from the types module.

use serde::{Deserialize, Serialize};

// Re-export the core archetype enum from types module
pub use crate::types::CampbellArchetype;

/// Record of an archetype change for a character
///
/// This is a domain-specific struct that tracks when and why a character's archetype changed.
/// It uses the shared CampbellArchetype enum but adds domain-specific metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeChange {
    pub from: CampbellArchetype,
    pub to: CampbellArchetype,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
