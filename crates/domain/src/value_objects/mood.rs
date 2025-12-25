//! NPC Mood and Relationship tracking value objects
//!
//! P1.4: Character Mood & Relationship Tracking
//!
//! - MoodLevel: Temporary emotional state (can change within a scene)
//! - RelationshipLevel: Long-term disposition (changes over time through interactions)
//! - NpcMoodState: Complete mood/relationship state for an NPC toward a specific PC

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use wrldbldr_domain::{CharacterId, PlayerCharacterId};
use uuid::Uuid;

/// Temporary emotional state of an NPC
/// 
/// This represents the NPC's current emotional state, which can change
/// rapidly based on immediate interactions, challenge outcomes, or DM direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MoodLevel {
    /// Positive and warm toward the player
    Friendly,
    /// Default neutral state
    #[default]
    Neutral,
    /// Wary, distrustful
    Suspicious,
    /// Actively antagonistic
    Hostile,
    /// Fearful, wants to avoid
    Afraid,
    /// Thankful, owes a debt
    Grateful,
    /// Irritated but not hostile
    Annoyed,
    /// Excited, interested
    Curious,
    /// Sad, melancholic
    Melancholic,
}

impl MoodLevel {
    /// Get all mood levels for UI dropdowns
    pub fn all() -> &'static [MoodLevel] {
        &[
            MoodLevel::Friendly,
            MoodLevel::Neutral,
            MoodLevel::Suspicious,
            MoodLevel::Hostile,
            MoodLevel::Afraid,
            MoodLevel::Grateful,
            MoodLevel::Annoyed,
            MoodLevel::Curious,
            MoodLevel::Melancholic,
        ]
    }

    /// Get a display name for the mood
    pub fn display_name(&self) -> &'static str {
        match self {
            MoodLevel::Friendly => "Friendly",
            MoodLevel::Neutral => "Neutral",
            MoodLevel::Suspicious => "Suspicious",
            MoodLevel::Hostile => "Hostile",
            MoodLevel::Afraid => "Afraid",
            MoodLevel::Grateful => "Grateful",
            MoodLevel::Annoyed => "Annoyed",
            MoodLevel::Curious => "Curious",
            MoodLevel::Melancholic => "Melancholic",
        }
    }

    /// Get an emoji representation for UI
    pub fn emoji(&self) -> &'static str {
        match self {
            MoodLevel::Friendly => "😊",
            MoodLevel::Neutral => "😐",
            MoodLevel::Suspicious => "🤨",
            MoodLevel::Hostile => "😠",
            MoodLevel::Afraid => "😨",
            MoodLevel::Grateful => "🙏",
            MoodLevel::Annoyed => "😤",
            MoodLevel::Curious => "🧐",
            MoodLevel::Melancholic => "😢",
        }
    }

    /// Convert sentiment score (-1.0 to 1.0) to mood level
    pub fn from_sentiment(sentiment: f32) -> Self {
        match sentiment {
            s if s >= 0.6 => MoodLevel::Friendly,
            s if s >= 0.3 => MoodLevel::Grateful,
            s if s >= 0.1 => MoodLevel::Curious,
            s if s >= -0.1 => MoodLevel::Neutral,
            s if s >= -0.3 => MoodLevel::Annoyed,
            s if s >= -0.5 => MoodLevel::Suspicious,
            s if s >= -0.7 => MoodLevel::Afraid,
            _ => MoodLevel::Hostile,
        }
    }

    /// Get the base sentiment value for this mood
    pub fn base_sentiment(&self) -> f32 {
        match self {
            MoodLevel::Friendly => 0.7,
            MoodLevel::Grateful => 0.5,
            MoodLevel::Curious => 0.2,
            MoodLevel::Neutral => 0.0,
            MoodLevel::Annoyed => -0.2,
            MoodLevel::Suspicious => -0.4,
            MoodLevel::Melancholic => -0.3,
            MoodLevel::Afraid => -0.6,
            MoodLevel::Hostile => -0.8,
        }
    }
}

impl fmt::Display for MoodLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for MoodLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "friendly" => Ok(MoodLevel::Friendly),
            "neutral" => Ok(MoodLevel::Neutral),
            "suspicious" => Ok(MoodLevel::Suspicious),
            "hostile" => Ok(MoodLevel::Hostile),
            "afraid" => Ok(MoodLevel::Afraid),
            "grateful" => Ok(MoodLevel::Grateful),
            "annoyed" => Ok(MoodLevel::Annoyed),
            "curious" => Ok(MoodLevel::Curious),
            "melancholic" => Ok(MoodLevel::Melancholic),
            _ => Err(format!("Unknown mood level: {}", s)),
        }
    }
}

/// Long-term relationship disposition between NPC and PC
///
/// This represents the NPC's overall relationship with a PC,
/// which changes gradually over many interactions.
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

/// Complete mood and relationship state for an NPC toward a specific PC
///
/// Stored as a Neo4j edge: `(npc:Character)-[:DISPOSITION_TOWARD]->(pc:PlayerCharacter)`
///
/// Note: We don't derive Serialize/Deserialize because the ID types don't support it.
/// This struct is used internally; for wire format, convert to/from protocol types.
#[derive(Debug, Clone)]
pub struct NpcMoodState {
    /// The NPC this mood belongs to
    pub npc_id: CharacterId,
    /// The PC this mood is toward
    pub pc_id: PlayerCharacterId,
    /// Current emotional state (temporary)
    pub mood: MoodLevel,
    /// Long-term relationship disposition
    pub relationship: RelationshipLevel,
    /// Fine-grained sentiment score (-1.0 to 1.0)
    pub sentiment: f32,
    /// When this state was last updated
    pub updated_at: DateTime<Utc>,
    /// Reason for the last mood change (for DM reference)
    pub mood_reason: Option<String>,
    /// Accumulated relationship points (for gradual relationship changes)
    pub relationship_points: i32,
}

/// Wire-format mood state for protocol serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMoodStateDto {
    /// The NPC's UUID
    pub npc_id: Uuid,
    /// The PC's UUID
    pub pc_id: Uuid,
    /// Current emotional state
    pub mood: MoodLevel,
    /// Long-term relationship disposition
    pub relationship: RelationshipLevel,
    /// Fine-grained sentiment score (-1.0 to 1.0)
    pub sentiment: f32,
    /// When this state was last updated (RFC 3339)
    pub updated_at: String,
    /// Reason for the last mood change
    pub mood_reason: Option<String>,
    /// Accumulated relationship points
    pub relationship_points: i32,
}

impl From<&NpcMoodState> for NpcMoodStateDto {
    fn from(state: &NpcMoodState) -> Self {
        Self {
            npc_id: state.npc_id.to_uuid(),
            pc_id: state.pc_id.to_uuid(),
            mood: state.mood,
            relationship: state.relationship,
            sentiment: state.sentiment,
            updated_at: state.updated_at.to_rfc3339(),
            mood_reason: state.mood_reason.clone(),
            relationship_points: state.relationship_points,
        }
    }
}

impl NpcMoodStateDto {
    /// Convert back to domain type
    pub fn to_domain(&self) -> NpcMoodState {
        NpcMoodState {
            npc_id: CharacterId::from_uuid(self.npc_id),
            pc_id: PlayerCharacterId::from_uuid(self.pc_id),
            mood: self.mood,
            relationship: self.relationship,
            sentiment: self.sentiment,
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            mood_reason: self.mood_reason.clone(),
            relationship_points: self.relationship_points,
        }
    }
}

impl NpcMoodState {
    /// Create a new mood state with defaults
    pub fn new(npc_id: CharacterId, pc_id: PlayerCharacterId) -> Self {
        Self {
            npc_id,
            pc_id,
            mood: MoodLevel::Neutral,
            relationship: RelationshipLevel::Stranger,
            sentiment: 0.0,
            updated_at: Utc::now(),
            mood_reason: None,
            relationship_points: 0,
        }
    }

    /// Create with a specific starting mood
    pub fn with_mood(mut self, mood: MoodLevel) -> Self {
        self.mood = mood;
        self.sentiment = mood.base_sentiment();
        self
    }

    /// Create with a specific relationship
    pub fn with_relationship(mut self, relationship: RelationshipLevel) -> Self {
        self.relationship = relationship;
        self
    }

    /// Update the mood with a reason
    pub fn set_mood(&mut self, mood: MoodLevel, reason: Option<String>) {
        self.mood = mood;
        self.sentiment = mood.base_sentiment();
        self.mood_reason = reason;
        self.updated_at = Utc::now();
    }

    /// Adjust sentiment and potentially update mood
    pub fn adjust_sentiment(&mut self, delta: f32, reason: Option<String>) {
        self.sentiment = (self.sentiment + delta).clamp(-1.0, 1.0);
        self.mood = MoodLevel::from_sentiment(self.sentiment);
        self.mood_reason = reason;
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
            self.mood.display_name(),
            self.relationship.display_name()
        )
    }
}

/// Interaction outcome for mood updates
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
    /// Get the mood delta for success
    pub fn success_delta(&self) -> f32 {
        match self {
            ChallengeSignificance::Minor => 0.05,
            ChallengeSignificance::Normal => 0.1,
            ChallengeSignificance::Significant => 0.2,
            ChallengeSignificance::Major => 0.4,
        }
    }

    /// Get the mood delta for failure
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
    fn test_mood_from_sentiment() {
        assert_eq!(MoodLevel::from_sentiment(0.8), MoodLevel::Friendly);
        assert_eq!(MoodLevel::from_sentiment(0.0), MoodLevel::Neutral);
        assert_eq!(MoodLevel::from_sentiment(-0.5), MoodLevel::Suspicious);
        assert_eq!(MoodLevel::from_sentiment(-0.9), MoodLevel::Hostile);
    }

    #[test]
    fn test_relationship_points() {
        let mut state = NpcMoodState::new(
            CharacterId::new(),
            PlayerCharacterId::new(),
        );

        assert_eq!(state.relationship, RelationshipLevel::Stranger);

        state.add_relationship_points(15);
        assert_eq!(state.relationship, RelationshipLevel::Acquaintance);

        state.add_relationship_points(20);
        assert_eq!(state.relationship, RelationshipLevel::Friend);

        state.add_relationship_points(-60);
        assert_eq!(state.relationship, RelationshipLevel::Rival);
    }

    #[test]
    fn test_mood_parse() {
        assert_eq!("friendly".parse::<MoodLevel>().unwrap(), MoodLevel::Friendly);
        assert_eq!("HOSTILE".parse::<MoodLevel>().unwrap(), MoodLevel::Hostile);
        assert!("unknown".parse::<MoodLevel>().is_err());
    }
}
