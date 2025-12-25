//! Want entity - Character desires (Actantial Model)
//!
//! # Graph-First Design (Phase 0.C)
//!
//! A Want is a node that represents a character's desire. The target of the want
//! is stored as a TARGETS edge, NOT embedded in the Want node:
//!
//! ```cypher
//! (character:Character)-[:HAS_WANT {priority: 1}]->(want:Want)
//! (want:Want)-[:TARGETS]->(target)  // Character, Item, or Goal
//! ```
//!
//! Actantial roles (Helper, Opponent, Sender, Receiver) are edges from the
//! character to other characters, referencing the want_id:
//!
//! ```cypher
//! (subject:Character)-[:VIEWS_AS_HELPER {want_id: "...", reason: "..."}]->(helper:Character)
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use wrldbldr_domain::WantId;

/// Visibility level for a Want - how much the player knows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WantVisibility {
    /// Player knows this motivation openly
    Known,
    /// Player suspects something but doesn't know details
    Suspected,
    /// Player has no idea (default)
    #[default]
    Hidden,
}

impl WantVisibility {
    /// Convert from legacy known_to_player bool
    pub fn from_known_to_player(known: bool) -> Self {
        if known {
            WantVisibility::Known
        } else {
            WantVisibility::Hidden
        }
    }

    /// Check if this is visible to player at all
    pub fn is_known(&self) -> bool {
        matches!(self, WantVisibility::Known)
    }

    /// Check if player has some awareness
    pub fn is_at_least_suspected(&self) -> bool {
        matches!(self, WantVisibility::Known | WantVisibility::Suspected)
    }
}

/// A character's desire or goal (Actantial model)
///
/// The want's target is stored via a `TARGETS` edge to:
/// - Character (wants something from/about a person)
/// - Item (wants a specific item)
/// - Goal (wants an abstract outcome)
#[derive(Debug, Clone)]
pub struct Want {
    pub id: WantId,
    /// Description of what the character wants
    pub description: String,
    /// Intensity of the want (0.0 = mild interest, 1.0 = obsession)
    pub intensity: f32,
    /// How much the player knows about this want
    pub visibility: WantVisibility,
    /// When this want was created
    pub created_at: DateTime<Utc>,
    
    // === Behavioral Guidance for Secret Wants ===
    
    /// How the NPC should behave when probed about this want (for Hidden/Suspected)
    /// Example: "Deflect with a sad smile; change subject to present dangers"
    pub deflection_behavior: Option<String>,
    
    /// Subtle behavioral tells that hint at this want
    /// Example: ["Avoids eye contact when past is mentioned", "Tenses at the word 'village'"]
    pub tells: Vec<String>,
}

impl Want {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: WantId::new(),
            description: description.into(),
            intensity: 0.5,
            visibility: WantVisibility::Hidden,
            created_at: Utc::now(),
            deflection_behavior: None,
            tells: Vec::new(),
        }
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    pub fn with_visibility(mut self, visibility: WantVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn known(mut self) -> Self {
        self.visibility = WantVisibility::Known;
        self
    }

    pub fn with_deflection(mut self, behavior: impl Into<String>) -> Self {
        self.deflection_behavior = Some(behavior.into());
        self
    }

    pub fn with_tells(mut self, tells: Vec<String>) -> Self {
        self.tells = tells;
        self
    }

    pub fn add_tell(mut self, tell: impl Into<String>) -> Self {
        self.tells.push(tell.into());
        self
    }

    /// Generate default deflection behavior based on intensity
    pub fn default_deflection(&self) -> String {
        if self.intensity > 0.8 {
            "Become visibly uncomfortable; firmly redirect conversation".to_string()
        } else if self.intensity > 0.5 {
            "Give a vague, non-committal response".to_string()
        } else {
            "Smoothly change the subject".to_string()
        }
    }

    /// Get deflection behavior, falling back to default
    pub fn effective_deflection(&self) -> String {
        self.deflection_behavior
            .clone()
            .unwrap_or_else(|| self.default_deflection())
    }
}

/// Data for the HAS_WANT edge between Character and Want
#[derive(Debug, Clone)]
pub struct CharacterWant {
    /// The want node
    pub want: Want,
    /// Priority (1 = primary want, 2 = secondary, etc.)
    pub priority: u32,
    /// When this want was acquired
    pub acquired_at: DateTime<Utc>,
}

impl CharacterWant {
    pub fn new(want: Want, priority: u32) -> Self {
        Self {
            want,
            priority,
            acquired_at: Utc::now(),
        }
    }

    /// Check if this want is visible to player
    pub fn is_known(&self) -> bool {
        self.want.visibility.is_known()
    }

    /// Check if player has some awareness
    pub fn is_at_least_suspected(&self) -> bool {
        self.want.visibility.is_at_least_suspected()
    }
}

/// The type of target a want can have (for querying purposes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WantTargetType {
    Character,
    Item,
    Goal,
}

/// Actantial role type for character views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActantialRole {
    /// Character sees target as helping their want
    Helper,
    /// Character sees target as opposing their want
    Opponent,
    /// Character sees target as having initiated/motivated their want
    Sender,
    /// Character sees target as benefiting from their want's fulfillment
    Receiver,
}

/// Data for actantial view edges (VIEWS_AS_HELPER, etc.)
#[derive(Debug, Clone)]
pub struct ActantialView {
    /// Which want this relates to
    pub want_id: WantId,
    /// Why the character views the target this way
    pub reason: String,
    /// When this view was assigned
    pub assigned_at: DateTime<Utc>,
}

impl ActantialView {
    pub fn new(want_id: WantId, reason: impl Into<String>) -> Self {
        Self {
            want_id,
            reason: reason.into(),
            assigned_at: Utc::now(),
        }
    }
}
