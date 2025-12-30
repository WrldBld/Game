//! NPC Disposition and Relationship level enumerations
//!
//! These represent the emotional stance and social distance between NPCs and PCs.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Disposition level - how an NPC emotionally feels about a specific PC
///
/// This represents the NPC's subjective emotional stance toward a particular PC,
/// which can change based on interactions, challenge outcomes, or DM direction.
///
/// This is SEPARATE from RelationshipLevel (social distance).
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
