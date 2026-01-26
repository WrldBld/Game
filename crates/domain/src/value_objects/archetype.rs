//! Campbell's character archetypes from "The Hero with a Thousand Faces"
//!
//! Re-exports shared types from the types module.
//!
//! # Tier Classification
//!
//! - **Tier 2: Validated Enum** - `CampbellArchetype` (re-exported from types module)
//!   represents mutually exclusive character archetypes.
//! - **Tier 3a: Composite VO (Simple Data)** - `ArchetypeChange` is a simple data
//!   struct with public fields (no invariants).
//!
//! See [docs/architecture/tier-levels.md](../../../../docs/architecture/tier-levels.md)
//! for complete tier classification system.

use serde::{Deserialize, Serialize};

// Re-export the core archetype enum from types module
pub use crate::types::CampbellArchetype;

/// Record of an archetype change for a character
///
/// This is a domain-specific struct that tracks when and why a character's archetype changed.
/// It uses the shared CampbellArchetype enum but adds domain-specific metadata.
///
/// # Tier Classification
///
/// **Tier 4: Simple Data Struct (Public Fields)** - This is a record with no invariants,
/// just grouping related data. Public fields are appropriate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeChange {
    /// The archetype the character changed from
    pub from: CampbellArchetype,
    /// The archetype the character changed to
    pub to: CampbellArchetype,
    /// The reason for the archetype change
    pub reason: String,
    /// When the archetype change occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ArchetypeChange {
    /// Create a new archetype change record.
    ///
    /// # Hexagonal Architecture Note
    /// Timestamp is injected rather than using direct time sources to keep domain pure.
    /// Call sites should use `clock_port.now()` to get the current time.
    pub fn new(
        from: CampbellArchetype,
        to: CampbellArchetype,
        reason: impl Into<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            from,
            to,
            reason: reason.into(),
            timestamp,
        }
    }
}
