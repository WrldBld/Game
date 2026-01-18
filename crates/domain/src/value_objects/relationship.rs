//! Character relationships for social network modeling

use serde::{Deserialize, Serialize};
use wrldbldr_domain::{CharacterId, DomainError, RelationshipId};

/// A relationship between two characters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    id: RelationshipId,
    from_character: CharacterId,
    to_character: CharacterId,
    relationship_type: RelationshipType,
    /// Sentiment from -1.0 (hatred) to 1.0 (love)
    sentiment: f32,
    history: Vec<RelationshipEvent>,
    /// Whether players know about this relationship
    known_to_player: bool,
}

impl Relationship {
    /// Create a new relationship between two characters.
    ///
    /// The relationship starts with neutral sentiment (0.0) and is visible
    /// to the player by default.
    ///
    /// # Arguments
    /// * `from` - The character this relationship originates from
    /// * `to` - The character this relationship points to
    /// * `relationship_type` - The type of relationship (ally, enemy, family, etc.)
    ///
    /// # Example
    /// ```ignore
    /// let friendship = Relationship::new(
    ///     alice_id,
    ///     bob_id,
    ///     RelationshipType::Friendship,
    /// );
    /// ```
    pub fn new(from: CharacterId, to: CharacterId, relationship_type: RelationshipType) -> Self {
        Self {
            id: RelationshipId::new(),
            from_character: from,
            to_character: to,
            relationship_type,
            sentiment: 0.0,
            history: Vec::new(),
            known_to_player: true,
        }
    }

    /// Create a relationship with explicit sentiment.
    ///
    /// The sentiment is clamped to the range -1.0..=1.0 where -1.0 represents
    /// hatred and 1.0 represents love/deep affection.
    ///
    /// # Arguments
    /// * `from` - The character this relationship originates from
    /// * `to` - The character this relationship points to
    /// * `relationship_type` - The type of relationship
    /// * `sentiment` - Initial sentiment value (-1.0 to 1.0)
    ///
    /// # Example
    /// ```ignore
    /// let rivalry = Relationship::new_with_sentiment(
    ///     hero_id,
    ///     villain_id,
    ///     RelationshipType::Rivalry,
    ///     -0.8,
    /// );
    /// ```
    pub fn new_with_sentiment(
        from: CharacterId,
        to: CharacterId,
        relationship_type: RelationshipType,
        sentiment: f32,
    ) -> Self {
        Self {
            id: RelationshipId::new(),
            from_character: from,
            to_character: to,
            relationship_type,
            sentiment: sentiment.clamp(-1.0, 1.0),
            history: Vec::new(),
            known_to_player: true,
        }
    }

    /// Reconstruct a relationship from persisted data (e.g., database).
    ///
    /// This bypasses the normal constructor to restore an existing relationship
    /// with all its historical state.
    pub fn from_persisted(
        id: RelationshipId,
        from_character: CharacterId,
        to_character: CharacterId,
        relationship_type: RelationshipType,
        sentiment: f32,
        history: Vec<RelationshipEvent>,
        known_to_player: bool,
    ) -> Self {
        Self {
            id,
            from_character,
            to_character,
            relationship_type,
            sentiment: sentiment.clamp(-1.0, 1.0),
            history,
            known_to_player,
        }
    }

    // --- Accessors ---

    /// Get the relationship ID
    pub fn id(&self) -> RelationshipId {
        self.id
    }

    /// Get the source character ID
    pub fn from_character(&self) -> CharacterId {
        self.from_character
    }

    /// Get the target character ID
    pub fn to_character(&self) -> CharacterId {
        self.to_character
    }

    /// Get the relationship type
    pub fn relationship_type(&self) -> &RelationshipType {
        &self.relationship_type
    }

    /// Get the sentiment value (-1.0 to 1.0)
    pub fn sentiment(&self) -> f32 {
        self.sentiment
    }

    /// Get the history of relationship events
    pub fn history(&self) -> &[RelationshipEvent] {
        &self.history
    }

    /// Check if this relationship is known to the player
    pub fn known_to_player(&self) -> bool {
        self.known_to_player
    }

    // --- Builder methods ---

    /// Set the sentiment of this relationship using builder pattern.
    ///
    /// The sentiment is clamped to -1.0..=1.0.
    pub fn with_sentiment(self, sentiment: f32) -> Self {
        Self {
            sentiment: sentiment.clamp(-1.0, 1.0),
            ..self
        }
    }

    /// Mark this relationship as secret (hidden from the player).
    pub fn secret(self) -> Self {
        Self {
            known_to_player: false,
            ..self
        }
    }

    /// Mark this relationship as known to the player.
    pub fn revealed(self) -> Self {
        Self {
            known_to_player: true,
            ..self
        }
    }

    /// Add a historical event to this relationship (builder pattern).
    ///
    /// Events track how the relationship has evolved over time.
    pub fn with_event(mut self, event: RelationshipEvent) -> Self {
        self.history.push(event);
        self
    }

    /// Add multiple historical events to this relationship (builder pattern).
    pub fn with_events(mut self, events: impl IntoIterator<Item = RelationshipEvent>) -> Self {
        self.history.extend(events);
        self
    }
}

/// Types of relationships between characters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipType {
    Family(FamilyRelation),
    Romantic,
    Professional,
    Rivalry,
    Friendship,
    Mentorship,
    Enmity,
    Custom(String),
}

/// Family relationship subtypes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FamilyRelation {
    Parent,
    Child,
    Sibling,
    Spouse,
    Grandparent,
    Grandchild,
    AuntUncle,
    NieceNephew,
    Cousin,
}

/// An event that affected a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEvent {
    pub description: String,
    pub sentiment_change: f32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl RelationshipEvent {
    /// Create a new relationship event
    pub fn new(
        description: impl Into<String>,
        sentiment_change: f32,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            description: description.into(),
            sentiment_change,
            timestamp,
        }
    }
}

impl std::str::FromStr for FamilyRelation {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_lowercase().replace(['_', ' '], "");
        match normalized.as_str() {
            "parent" => Ok(Self::Parent),
            "child" => Ok(Self::Child),
            "sibling" => Ok(Self::Sibling),
            "spouse" => Ok(Self::Spouse),
            "grandparent" => Ok(Self::Grandparent),
            "grandchild" => Ok(Self::Grandchild),
            "aunt" | "uncle" | "auntuncle" => Ok(Self::AuntUncle),
            "niece" | "nephew" | "niecenephew" => Ok(Self::NieceNephew),
            "cousin" => Ok(Self::Cousin),
            _ => Err(DomainError::parse(format!(
                "Unknown family relation: {}",
                s
            ))),
        }
    }
}

impl std::str::FromStr for RelationshipType {
    type Err = DomainError;

    /// Parse a relationship type from a string (case-insensitive)
    ///
    /// Supports:
    /// - Basic types: "romantic", "professional", "rivalry", "friendship", "mentorship", "enmity"
    /// - Aliases: "friend" -> Friendship, "mentor" -> Mentorship, "enemy" -> Enmity
    /// - Family types: "parent", "child", "sibling", "spouse", "grandparent", "grandchild",
    ///   "aunt", "uncle", "niece", "nephew", "cousin"
    /// - Unknown values become Custom(original_string)
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_lowercase().replace(['_', ' '], "");

        Ok(match normalized.as_str() {
            // Basic relationship types
            "romantic" => Self::Romantic,
            "professional" => Self::Professional,
            "rivalry" => Self::Rivalry,
            "friendship" | "friend" => Self::Friendship,
            "mentorship" | "mentor" => Self::Mentorship,
            "enmity" | "enemy" => Self::Enmity,

            // Family relations
            "parent" => Self::Family(FamilyRelation::Parent),
            "child" => Self::Family(FamilyRelation::Child),
            "sibling" => Self::Family(FamilyRelation::Sibling),
            "spouse" => Self::Family(FamilyRelation::Spouse),
            "grandparent" => Self::Family(FamilyRelation::Grandparent),
            "grandchild" => Self::Family(FamilyRelation::Grandchild),
            "aunt" | "uncle" | "auntuncle" => Self::Family(FamilyRelation::AuntUncle),
            "niece" | "nephew" | "niecenephew" => Self::Family(FamilyRelation::NieceNephew),
            "cousin" => Self::Family(FamilyRelation::Cousin),

            // Family with explicit prefix (e.g., "family:parent")
            _ if normalized.starts_with("family") => {
                let rest = normalized
                    .trim_start_matches("family")
                    .trim_start_matches(':');
                if let Ok(family) = rest.parse::<FamilyRelation>() {
                    Self::Family(family)
                } else {
                    Self::Custom(s.to_string())
                }
            }

            // Unknown -> Custom
            _ => Self::Custom(s.to_string()),
        })
    }
}
