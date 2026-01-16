//! NPC Disposition and Relationship level enumerations
//!
//! These represent the emotional stance and social distance between NPCs and PCs.

use crate::error::DomainError;
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
    /// Unknown disposition (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl DispositionLevel {
    /// Get all disposition levels for UI dropdowns (excludes Unknown)
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
            DispositionLevel::Unknown => "Unknown",
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
            DispositionLevel::Unknown => "❓",
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
            DispositionLevel::Unknown => 0.0,
        }
    }
}

impl fmt::Display for DispositionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for DispositionLevel {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "hostile" => DispositionLevel::Hostile,
            "suspicious" => DispositionLevel::Suspicious,
            "dismissive" => DispositionLevel::Dismissive,
            "neutral" => DispositionLevel::Neutral,
            "respectful" => DispositionLevel::Respectful,
            "friendly" => DispositionLevel::Friendly,
            "grateful" => DispositionLevel::Grateful,
            _ => DispositionLevel::Unknown,
        })
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
    /// Unknown relationship (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl RelationshipLevel {
    /// Get all relationship levels for UI dropdowns (excludes Unknown)
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
            RelationshipLevel::Unknown => "Unknown",
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
            RelationshipLevel::Unknown => "❔",
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
            RelationshipLevel::Unknown => 0.0,
        }
    }
}

impl fmt::Display for RelationshipLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for RelationshipLevel {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "ally" => RelationshipLevel::Ally,
            "friend" => RelationshipLevel::Friend,
            "acquaintance" => RelationshipLevel::Acquaintance,
            "stranger" => RelationshipLevel::Stranger,
            "rival" => RelationshipLevel::Rival,
            "enemy" => RelationshipLevel::Enemy,
            "nemesis" => RelationshipLevel::Nemesis,
            _ => RelationshipLevel::Unknown,
        })
    }
}
