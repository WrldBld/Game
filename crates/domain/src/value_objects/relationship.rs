//! Character relationships for social network modeling

use wrldbldr_domain::{CharacterId, RelationshipId};

/// A relationship between two characters
#[derive(Debug, Clone)]
pub struct Relationship {
    pub id: RelationshipId,
    pub from_character: CharacterId,
    pub to_character: CharacterId,
    pub relationship_type: RelationshipType,
    /// Sentiment from -1.0 (hatred) to 1.0 (love)
    pub sentiment: f32,
    pub history: Vec<RelationshipEvent>,
    /// Whether players know about this relationship
    pub known_to_player: bool,
}

impl Relationship {
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

    pub fn with_sentiment(mut self, sentiment: f32) -> Self {
        self.sentiment = sentiment.clamp(-1.0, 1.0);
        self
    }

    pub fn secret(mut self) -> Self {
        self.known_to_player = false;
        self
    }

    pub fn add_event(&mut self, event: RelationshipEvent) {
        self.history.push(event);
    }
}

/// Types of relationships between characters
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
pub struct RelationshipEvent {
    pub description: String,
    pub sentiment_change: f32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl std::str::FromStr for FamilyRelation {
    type Err = String;

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
            _ => Err(format!("Unknown family relation: {}", s)),
        }
    }
}

impl std::str::FromStr for RelationshipType {
    type Err = String;

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
