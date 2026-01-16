//! NPC Disposition and Relationship tracking value objects
//!
//! P1.4: Character Disposition & Relationship Tracking
//!
//! ## Three-Tier Emotional Model
//!
//! This module implements Tier 1 (Disposition) of the emotional model:
//!
//! - **DispositionLevel**: How an NPC emotionally feels about a specific PC (per NPC-PC pair)
//! - **RelationshipLevel**: Social distance/familiarity (per NPC-PC pair)
//! - **NpcDispositionState**: Complete disposition/relationship state for an NPC toward a specific PC
//!
//! ## Disposition vs Relationship
//!
//! Both are stored on the same `DISPOSITION_TOWARD` Neo4j edge, allowing combinations like:
//! - "Suspicious Ally" - close relationship, but currently doubts the PC
//! - "Friendly Stranger" - warm first impression, just met
//! - "Hostile Acquaintance" - knows the PC, actively dislikes them
//!
//! ## Related Concepts (defined elsewhere)
//!
//! - **MoodState** (Tier 2): NPC's current emotional state, independent of any PC (defined in mood.rs)
//! - **Expression** (Tier 3): Visual expression shown during dialogue (transient, in dialogue markers)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{CharacterId, PlayerCharacterId};

// Re-export the core enums from types module
pub use crate::types::{DispositionLevel, MoodState, RelationshipLevel};

/// Complete disposition and relationship state for an NPC toward a specific PC
///
/// Stored as a Neo4j edge: `(npc:Character)-[:DISPOSITION_TOWARD]->(pc:PlayerCharacter)`
///
/// Combines two dimensions:
/// - disposition: Emotional stance (how they feel about the PC)
/// - relationship: Social distance (how well they know each other)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcDispositionState {
    /// The NPC this disposition belongs to
    npc_id: CharacterId,
    /// The PC this disposition is toward
    pc_id: PlayerCharacterId,
    /// Current emotional stance toward the PC
    disposition: DispositionLevel,
    /// Long-term relationship level (social distance)
    relationship: RelationshipLevel,
    /// Fine-grained sentiment score (-1.0 to 1.0)
    sentiment: f32,
    /// When this state was last updated
    updated_at: DateTime<Utc>,
    /// Reason for the last disposition change (for DM reference)
    disposition_reason: Option<String>,
    /// Accumulated relationship points (for gradual relationship changes)
    relationship_points: i32,
}

impl NpcDispositionState {
    /// Create a new disposition state with defaults
    pub fn new(npc_id: CharacterId, pc_id: PlayerCharacterId, now: DateTime<Utc>) -> Self {
        Self {
            npc_id,
            pc_id,
            disposition: DispositionLevel::Neutral,
            relationship: RelationshipLevel::Stranger,
            sentiment: 0.0,
            updated_at: now,
            disposition_reason: None,
            relationship_points: 0,
        }
    }

    /// Reconstruct from storage (database hydration)
    #[allow(clippy::too_many_arguments)]
    pub fn from_storage(
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        disposition: DispositionLevel,
        relationship: RelationshipLevel,
        sentiment: f32,
        updated_at: DateTime<Utc>,
        disposition_reason: Option<String>,
        relationship_points: i32,
    ) -> Self {
        Self {
            npc_id,
            pc_id,
            disposition,
            relationship,
            sentiment,
            updated_at,
            disposition_reason,
            relationship_points,
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Read accessors
    // ──────────────────────────────────────────────────────────────────────────

    /// Get the NPC this disposition belongs to
    pub fn npc_id(&self) -> CharacterId {
        self.npc_id
    }

    /// Get the PC this disposition is toward
    pub fn pc_id(&self) -> PlayerCharacterId {
        self.pc_id
    }

    /// Get the current emotional stance toward the PC
    pub fn disposition(&self) -> DispositionLevel {
        self.disposition
    }

    /// Get the long-term relationship level (social distance)
    pub fn relationship(&self) -> RelationshipLevel {
        self.relationship
    }

    /// Get the fine-grained sentiment score (-1.0 to 1.0)
    pub fn sentiment(&self) -> f32 {
        self.sentiment
    }

    /// Get when this state was last updated
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Get the reason for the last disposition change
    pub fn disposition_reason(&self) -> Option<&str> {
        self.disposition_reason.as_deref()
    }

    /// Get the accumulated relationship points
    pub fn relationship_points(&self) -> i32 {
        self.relationship_points
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Builder-style methods (consume self, return new instance)
    // ──────────────────────────────────────────────────────────────────────────

    /// Create with a specific starting disposition
    pub fn with_disposition(mut self, disposition: DispositionLevel) -> Self {
        self.disposition = disposition;
        self.sentiment = disposition.base_sentiment();
        self
    }

    /// Create with a specific relationship
    pub fn with_relationship(mut self, relationship: RelationshipLevel) -> Self {
        self.relationship = relationship;
        self
    }

    /// Return a new state with the disposition updated.
    ///
    /// This consumes self and returns a new instance with the updated values.
    pub fn updating_disposition(
        self,
        disposition: DispositionLevel,
        reason: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            disposition,
            sentiment: disposition.base_sentiment(),
            disposition_reason: reason,
            updated_at: now,
            ..self
        }
    }

    /// Return a new state with the relationship level updated.
    ///
    /// This consumes self and returns a new instance with the updated values.
    pub fn updating_relationship(
        self,
        relationship: RelationshipLevel,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            relationship,
            updated_at: now,
            ..self
        }
    }

    /// Return a new state with sentiment adjusted and disposition potentially updated.
    ///
    /// This consumes self and returns a new instance with the updated values.
    pub fn adjusting_sentiment(
        self,
        delta: f32,
        reason: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        let new_sentiment = (self.sentiment + delta).clamp(-1.0, 1.0);
        Self {
            sentiment: new_sentiment,
            disposition: DispositionLevel::from_sentiment(new_sentiment),
            disposition_reason: reason,
            updated_at: now,
            ..self
        }
    }

    /// Return a new state with relationship points added and relationship level potentially changed.
    ///
    /// This consumes self and returns a new instance with the updated values.
    ///
    /// Thresholds for relationship changes:
    /// - Positive: 10 = Acquaintance, 25 = Friend, 50 = Ally
    /// - Negative: -10 = Rival, -25 = Enemy, -50 = Nemesis
    pub fn adding_relationship_points(self, points: i32, now: DateTime<Utc>) -> Self {
        let new_points = self.relationship_points + points;

        let new_relationship = match new_points {
            p if p >= 50 => RelationshipLevel::Ally,
            p if p >= 25 => RelationshipLevel::Friend,
            p if p >= 10 => RelationshipLevel::Acquaintance,
            p if p > -10 => RelationshipLevel::Stranger,
            p if p > -25 => RelationshipLevel::Rival,
            p if p > -50 => RelationshipLevel::Enemy,
            _ => RelationshipLevel::Nemesis,
        };

        Self {
            relationship_points: new_points,
            relationship: new_relationship,
            updated_at: now,
            ..self
        }
    }

    /// Get a text description for LLM context
    pub fn describe_for_llm(&self) -> String {
        format!(
            "{} ({})",
            self.disposition.display_name(),
            self.relationship.display_name()
        )
    }
}

/// Interaction outcome for disposition updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionOutcome {
    /// Positive interaction
    Positive { magnitude: f32, reason: String },
    /// Negative interaction
    Negative { magnitude: f32, reason: String },
    /// Neutral interaction
    Neutral,
    /// Challenge outcome
    ChallengeResult {
        succeeded: bool,
        skill_name: String,
        /// How much this challenge mattered to the NPC
        significance: ChallengeSignificance,
    },
}

/// How significant a challenge was to an NPC
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChallengeSignificance {
    /// Minor interaction
    Minor,
    /// Standard interaction
    Normal,
    /// Important moment
    Significant,
    /// Life-changing event
    Major,
}

impl ChallengeSignificance {
    /// Get the disposition delta for success
    pub fn success_delta(&self) -> f32 {
        match self {
            ChallengeSignificance::Minor => 0.05,
            ChallengeSignificance::Normal => 0.1,
            ChallengeSignificance::Significant => 0.2,
            ChallengeSignificance::Major => 0.4,
        }
    }

    /// Get the disposition delta for failure
    pub fn failure_delta(&self) -> f32 {
        match self {
            ChallengeSignificance::Minor => -0.03,
            ChallengeSignificance::Normal => -0.08,
            ChallengeSignificance::Significant => -0.15,
            ChallengeSignificance::Major => -0.3,
        }
    }

    /// Get relationship points for success
    pub fn success_points(&self) -> i32 {
        match self {
            ChallengeSignificance::Minor => 1,
            ChallengeSignificance::Normal => 2,
            ChallengeSignificance::Significant => 5,
            ChallengeSignificance::Major => 10,
        }
    }

    /// Get relationship points for failure
    pub fn failure_points(&self) -> i32 {
        match self {
            ChallengeSignificance::Minor => 0,
            ChallengeSignificance::Normal => -1,
            ChallengeSignificance::Significant => -3,
            ChallengeSignificance::Major => -5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_time() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn test_disposition_from_sentiment() {
        assert_eq!(
            DispositionLevel::from_sentiment(0.8),
            DispositionLevel::Grateful
        );
        assert_eq!(
            DispositionLevel::from_sentiment(0.4),
            DispositionLevel::Friendly
        );
        assert_eq!(
            DispositionLevel::from_sentiment(0.0),
            DispositionLevel::Neutral
        );
        assert_eq!(
            DispositionLevel::from_sentiment(-0.4),
            DispositionLevel::Suspicious
        );
        assert_eq!(
            DispositionLevel::from_sentiment(-0.9),
            DispositionLevel::Hostile
        );
    }

    #[test]
    fn test_relationship_points() {
        let now = fixed_time();
        let state = NpcDispositionState::new(CharacterId::new(), PlayerCharacterId::new(), now);

        // Starts as Stranger (0 points)
        assert_eq!(state.relationship(), RelationshipLevel::Stranger);

        // +15 = 15 points -> Acquaintance (>= 10)
        let state = state.adding_relationship_points(15, now);
        assert_eq!(state.relationship(), RelationshipLevel::Acquaintance);

        // +20 = 35 points -> Friend (>= 25)
        let state = state.adding_relationship_points(20, now);
        assert_eq!(state.relationship(), RelationshipLevel::Friend);

        // -60 = -25 points -> Enemy (> -50 but <= -25)
        let state = state.adding_relationship_points(-60, now);
        assert_eq!(state.relationship(), RelationshipLevel::Enemy);
    }

    #[test]
    fn test_disposition_parse() {
        assert_eq!(
            "friendly".parse::<DispositionLevel>().unwrap(),
            DispositionLevel::Friendly
        );
        assert_eq!(
            "HOSTILE".parse::<DispositionLevel>().unwrap(),
            DispositionLevel::Hostile
        );
        assert_eq!(
            "grateful".parse::<DispositionLevel>().unwrap(),
            DispositionLevel::Grateful
        );
        assert_eq!(
            "unknown".parse::<DispositionLevel>().unwrap(),
            DispositionLevel::Unknown
        );
        // With forward-compatibility, unrecognized strings map to Unknown
        assert_eq!(
            "invalid_value".parse::<DispositionLevel>().unwrap(),
            DispositionLevel::Unknown
        );
    }
}
