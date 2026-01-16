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
    from: CampbellArchetype,
    to: CampbellArchetype,
    reason: String,
    timestamp: chrono::DateTime<chrono::Utc>,
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

    // ── Accessors ────────────────────────────────────────────────────────

    /// Get the archetype the character changed from
    pub fn from(&self) -> CampbellArchetype {
        self.from
    }

    /// Get the archetype the character changed to
    pub fn to(&self) -> CampbellArchetype {
        self.to
    }

    /// Get the reason for the archetype change
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Get when the archetype change occurred
    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }
}
