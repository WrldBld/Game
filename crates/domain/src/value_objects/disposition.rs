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
use std::fmt;

use wrldbldr_domain::{CharacterId, PlayerCharacterId};

/// Disposition level - how an NPC emotionally feels about a specific PC
///
/// This represents the NPC's subjective emotional stance toward a particular PC,
/// which can change based on interactions, challenge outcomes, or DM direction.
///
/// This is SEPARATE from RelationshipLevel (social distance).
/// - DispositionLevel: How the NPC feels about the PC (Hostile → Grateful)
/// - RelationshipLevel: How well they know each other (Stranger → Ally)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DispositionLevel {
    /// Actively wants to harm/hinder the PC
    Hostile,
    /// Wary, distrustful of the PC
    Suspicious,
    /// Doesn't care about the PC, ignores them
    Dismissive,
    /// Default - no strong feelings either way
    #[default]
    Neutral,
    /// Regards the PC positively, professional respect
    Respectful,
    /// Positive and warm toward the PC
    Friendly,
    /// Owes the PC, deeply appreciative
    Grateful,
}

impl DispositionLevel {
    /// Get all disposition levels for UI dropdowns
    pub fn all() -> &'static [DispositionLevel] {
        &[
            DispositionLevel::Hostile,
            DispositionLevel::Suspicious,
            DispositionLevel::Dismissive,
            DispositionLevel::Neutral,
            DispositionLevel::Respectful,
            DispositionLevel::Friendly,
            DispositionLevel::Grateful,
        ]
    }

    /// Get a display name for the disposition
    pub fn display_name(&self) -> &'static str {
        match self {
            DispositionLevel::Hostile => "Hostile",
            DispositionLevel::Suspicious => "Suspicious",
            DispositionLevel::Dismissive => "Dismissive",
            DispositionLevel::Neutral => "Neutral",
            DispositionLevel::Respectful => "Respectful",
            DispositionLevel::Friendly => "Friendly",
            DispositionLevel::Grateful => "Grateful",
        }
    }

    /// Get an emoji representation for UI
    pub fn emoji(&self) -> &'static str {
        match self {
            DispositionLevel::Hostile => "😠",
            DispositionLevel::Suspicious => "🤨",
            DispositionLevel::Dismissive => "😒",
            DispositionLevel::Neutral => "😐",
            DispositionLevel::Respectful => "🤝",
            DispositionLevel::Friendly => "😊",
            DispositionLevel::Grateful => "🙏",
        }
    }

    /// Convert sentiment score (-1.0 to 1.0) to disposition level
    pub fn from_sentiment(sentiment: f32) -> Self {
        match sentiment {
            s if s >= 0.6 => DispositionLevel::Grateful,
            s if s >= 0.3 => DispositionLevel::Friendly,
            s if s >= 0.1 => DispositionLevel::Respectful,
            s if s >= -0.1 => DispositionLevel::Neutral,
            s if s >= -0.3 => DispositionLevel::Dismissive,
            s if s >= -0.5 => DispositionLevel::Suspicious,
            _ => DispositionLevel::Hostile,
        }
    }

    /// Get the base sentiment value for this disposition
    pub fn base_sentiment(&self) -> f32 {
        match self {
            DispositionLevel::Grateful => 0.7,
            DispositionLevel::Friendly => 0.5,
            DispositionLevel::Respectful => 0.2,
            DispositionLevel::Neutral => 0.0,
            DispositionLevel::Dismissive => -0.2,
            DispositionLevel::Suspicious => -0.4,
            DispositionLevel::Hostile => -0.8,
        }
    }
}

impl fmt::Display for DispositionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for DispositionLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hostile" => Ok(DispositionLevel::Hostile),
            "suspicious" => Ok(DispositionLevel::Suspicious),
            "dismissive" => Ok(DispositionLevel::Dismissive),
            "neutral" => Ok(DispositionLevel::Neutral),
            "respectful" => Ok(DispositionLevel::Respectful),
            "friendly" => Ok(DispositionLevel::Friendly),
            "grateful" => Ok(DispositionLevel::Grateful),
            _ => Err(format!("Unknown disposition level: {}", s)),
        }
    }
}

/// Long-term relationship level between NPC and PC
///
/// This represents how well the NPC knows the PC (social distance),
/// which changes gradually over many interactions.
///
/// This is SEPARATE from DispositionLevel (emotional stance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipLevel {
    /// Close bond, trusted ally
    Ally,
    /// Positive relationship
    Friend,
    /// Familiar, somewhat positive
    Acquaintance,
    /// No established relationship
    #[default]
    Stranger,
    /// Negative history
    Rival,
    /// Actively opposed
    Enemy,
    /// Deeply opposed, vendetta
    Nemesis,
}

impl RelationshipLevel {
    /// Get all relationship levels for UI dropdowns
    pub fn all() -> &'static [RelationshipLevel] {
        &[
            RelationshipLevel::Ally,
            RelationshipLevel::Friend,
            RelationshipLevel::Acquaintance,
            RelationshipLevel::Stranger,
            RelationshipLevel::Rival,
            RelationshipLevel::Enemy,
            RelationshipLevel::Nemesis,
        ]
    }

    /// Get a display name
    pub fn display_name(&self) -> &'static str {
        match self {
            RelationshipLevel::Ally => "Ally",
            RelationshipLevel::Friend => "Friend",
            RelationshipLevel::Acquaintance => "Acquaintance",
            RelationshipLevel::Stranger => "Stranger",
            RelationshipLevel::Rival => "Rival",
            RelationshipLevel::Enemy => "Enemy",
            RelationshipLevel::Nemesis => "Nemesis",
        }
    }

    /// Get an emoji representation
    pub fn emoji(&self) -> &'static str {
        match self {
            RelationshipLevel::Ally => "🤝",
            RelationshipLevel::Friend => "😄",
            RelationshipLevel::Acquaintance => "👋",
            RelationshipLevel::Stranger => "❓",
            RelationshipLevel::Rival => "😒",
            RelationshipLevel::Enemy => "⚔️",
            RelationshipLevel::Nemesis => "💀",
        }
    }

    /// Get base modifier for interactions (-1.0 to 1.0)
    pub fn interaction_modifier(&self) -> f32 {
        match self {
            RelationshipLevel::Ally => 0.5,
            RelationshipLevel::Friend => 0.3,
            RelationshipLevel::Acquaintance => 0.1,
            RelationshipLevel::Stranger => 0.0,
            RelationshipLevel::Rival => -0.2,
            RelationshipLevel::Enemy => -0.4,
            RelationshipLevel::Nemesis => -0.6,
        }
    }
}

impl fmt::Display for RelationshipLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for RelationshipLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ally" => Ok(RelationshipLevel::Ally),
            "friend" => Ok(RelationshipLevel::Friend),
            "acquaintance" => Ok(RelationshipLevel::Acquaintance),
            "stranger" => Ok(RelationshipLevel::Stranger),
            "rival" => Ok(RelationshipLevel::Rival),
            "enemy" => Ok(RelationshipLevel::Enemy),
            "nemesis" => Ok(RelationshipLevel::Nemesis),
            _ => Err(format!("Unknown relationship level: {}", s)),
        }
    }
}

/// Complete disposition and relationship state for an NPC toward a specific PC
///
/// Stored as a Neo4j edge: `(npc:Character)-[:DISPOSITION_TOWARD]->(pc:PlayerCharacter)`
///
/// Combines two dimensions:
/// - disposition: Emotional stance (how they feel about the PC)
/// - relationship: Social distance (how well they know each other)
///
/// Note: We don't derive Serialize/Deserialize because the ID types don't support it.
/// This struct is used internally; for wire format, convert to/from protocol types.
#[derive(Debug, Clone)]
pub struct NpcDispositionState {
    /// The NPC this disposition belongs to
    pub npc_id: CharacterId,
    /// The PC this disposition is toward
    pub pc_id: PlayerCharacterId,
    /// Current emotional stance toward the PC
    pub disposition: DispositionLevel,
    /// Long-term relationship level (social distance)
    pub relationship: RelationshipLevel,
    /// Fine-grained sentiment score (-1.0 to 1.0)
    pub sentiment: f32,
    /// When this state was last updated
    pub updated_at: DateTime<Utc>,
    /// Reason for the last disposition change (for DM reference)
    pub disposition_reason: Option<String>,
    /// Accumulated relationship points (for gradual relationship changes)
    pub relationship_points: i32,
}

impl NpcDispositionState {
    /// Create a new disposition state with defaults
    pub fn new(npc_id: CharacterId, pc_id: PlayerCharacterId) -> Self {
        Self {
            npc_id,
            pc_id,
            disposition: DispositionLevel::Neutral,
            relationship: RelationshipLevel::Stranger,
            sentiment: 0.0,
            updated_at: Utc::now(),
            disposition_reason: None,
            relationship_points: 0,
        }
    }

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

    /// Update the disposition with a reason
    pub fn set_disposition(&mut self, disposition: DispositionLevel, reason: Option<String>) {
        self.disposition = disposition;
        self.sentiment = disposition.base_sentiment();
        self.disposition_reason = reason;
        self.updated_at = Utc::now();
    }

    /// Adjust sentiment and potentially update disposition
    pub fn adjust_sentiment(&mut self, delta: f32, reason: Option<String>) {
        self.sentiment = (self.sentiment + delta).clamp(-1.0, 1.0);
        self.disposition = DispositionLevel::from_sentiment(self.sentiment);
        self.disposition_reason = reason;
        self.updated_at = Utc::now();
    }

    /// Add relationship points and potentially upgrade/downgrade relationship
    pub fn add_relationship_points(&mut self, points: i32) {
        self.relationship_points += points;
        self.updated_at = Utc::now();

        // Thresholds for relationship changes
        // Positive: 10 = Acquaintance, 25 = Friend, 50 = Ally
        // Negative: -10 = Rival, -25 = Enemy, -50 = Nemesis
        self.relationship = match self.relationship_points {
            p if p >= 50 => RelationshipLevel::Ally,
            p if p >= 25 => RelationshipLevel::Friend,
            p if p >= 10 => RelationshipLevel::Acquaintance,
            p if p > -10 => RelationshipLevel::Stranger,
            p if p > -25 => RelationshipLevel::Rival,
            p if p > -50 => RelationshipLevel::Enemy,
            _ => RelationshipLevel::Nemesis,
        };
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
    Positive {
        magnitude: f32,
        reason: String,
    },
    /// Negative interaction
    Negative {
        magnitude: f32,
        reason: String,
    },
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

    #[test]
    fn test_disposition_from_sentiment() {
        assert_eq!(DispositionLevel::from_sentiment(0.8), DispositionLevel::Grateful);
        assert_eq!(DispositionLevel::from_sentiment(0.4), DispositionLevel::Friendly);
        assert_eq!(DispositionLevel::from_sentiment(0.0), DispositionLevel::Neutral);
        assert_eq!(DispositionLevel::from_sentiment(-0.4), DispositionLevel::Suspicious);
        assert_eq!(DispositionLevel::from_sentiment(-0.9), DispositionLevel::Hostile);
    }

    #[test]
    fn test_relationship_points() {
        let mut state = NpcDispositionState::new(
            CharacterId::new(),
            PlayerCharacterId::new(),
        );

        // Starts as Stranger (0 points)
        assert_eq!(state.relationship, RelationshipLevel::Stranger);

        // +15 = 15 points -> Acquaintance (>= 10)
        state.add_relationship_points(15);
        assert_eq!(state.relationship, RelationshipLevel::Acquaintance);

        // +20 = 35 points -> Friend (>= 25)
        state.add_relationship_points(20);
        assert_eq!(state.relationship, RelationshipLevel::Friend);

        // -60 = -25 points -> Enemy (> -50 but <= -25)
        state.add_relationship_points(-60);
        assert_eq!(state.relationship, RelationshipLevel::Enemy);
    }

    #[test]
    fn test_disposition_parse() {
        assert_eq!("friendly".parse::<DispositionLevel>().unwrap(), DispositionLevel::Friendly);
        assert_eq!("HOSTILE".parse::<DispositionLevel>().unwrap(), DispositionLevel::Hostile);
        assert_eq!("grateful".parse::<DispositionLevel>().unwrap(), DispositionLevel::Grateful);
        assert!("unknown".parse::<DispositionLevel>().is_err());
    }

}
