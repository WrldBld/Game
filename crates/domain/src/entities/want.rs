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
#[serde(rename_all = "camelCase")]
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
    pub fn from_storage(known: bool) -> Self {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Want {
    id: WantId,
    /// Description of what the character wants
    description: String,
    /// Intensity of the want (0.0 = mild interest, 1.0 = obsession)
    intensity: f32,
    /// How much the player knows about this want
    visibility: WantVisibility,
    /// When this want was created
    created_at: DateTime<Utc>,

    // === Behavioral Guidance for Secret Wants ===
    /// How the NPC should behave when probed about this want (for Hidden/Suspected)
    /// Example: "Deflect with a sad smile; change subject to present dangers"
    deflection_behavior: Option<String>,

    /// Subtle behavioral tells that hint at this want
    /// Example: ["Avoids eye contact when past is mentioned", "Tenses at the word 'village'"]
    tells: Vec<String>,
}

impl Want {
    /// Create a Want with explicit intensity.
    ///
    /// The intensity is clamped to the range 0.0..=1.0 where 0.0 represents
    /// mild interest and 1.0 represents obsession.
    ///
    /// # Arguments
    /// * `description` - A description of what the character wants
    /// * `intensity` - How strongly the character wants this (0.0 to 1.0)
    /// * `now` - The current timestamp for created_at
    ///
    /// # Example
    /// ```ignore
    /// use chrono::TimeZone;
    /// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    /// let want = Want::new_with_intensity("Avenge my father", 0.9, now);
    /// ```
    pub fn new_with_intensity(
        description: impl Into<String>,
        intensity: f32,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: WantId::new(),
            description: description.into(),
            intensity: intensity.clamp(0.0, 1.0),
            visibility: WantVisibility::Hidden,
            created_at: now,
            deflection_behavior: None,
            tells: Vec::new(),
        }
    }

    /// Create a new Want with default intensity (0.5).
    ///
    /// # Arguments
    /// * `description` - A description of what the character wants
    /// * `now` - The current timestamp for created_at
    ///
    /// # Example
    /// ```ignore
    /// use chrono::TimeZone;
    /// let now = chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    /// let want = Want::new("Find the ancient artifact", now);
    /// ```
    pub fn new(description: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self::new_with_intensity(description, 0.5, now)
    }

    // === Accessors ===

    pub fn id(&self) -> WantId {
        self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    pub fn visibility(&self) -> WantVisibility {
        self.visibility
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn deflection_behavior(&self) -> Option<&str> {
        self.deflection_behavior.as_deref()
    }

    pub fn tells(&self) -> &[String] {
        &self.tells
    }

    // === Builder Methods ===

    /// Set the intensity of this want using builder pattern.
    ///
    /// The intensity is clamped to 0.0..=1.0.
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set the visibility of this want using builder pattern.
    pub fn with_visibility(mut self, visibility: WantVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Mark this want as known to the player using builder pattern.
    pub fn known(mut self) -> Self {
        self.visibility = WantVisibility::Known;
        self
    }

    /// Set the deflection behavior for when the NPC is probed about this want.
    ///
    /// This is used for Hidden/Suspected wants to guide NPC behavior.
    pub fn with_deflection(mut self, behavior: impl Into<String>) -> Self {
        self.deflection_behavior = Some(behavior.into());
        self
    }

    /// Set the behavioral tells that hint at this want.
    pub fn with_tells(mut self, tells: Vec<String>) -> Self {
        self.tells = tells;
        self
    }

    /// Add a single behavioral tell to this want.
    pub fn add_tell(mut self, tell: impl Into<String>) -> Self {
        self.tells.push(tell.into());
        self
    }

    /// Set the ID of this want (for reconstitution from storage).
    pub fn with_id(mut self, id: WantId) -> Self {
        self.id = id;
        self
    }

    /// Set the description of this want.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the created_at timestamp (for reconstitution from storage).
    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = created_at;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterWant {
    /// The want node
    pub want: Want,
    /// Priority (1 = primary want, 2 = secondary, etc.)
    pub priority: u32,
    /// When this want was acquired
    pub acquired_at: DateTime<Utc>,
}

impl CharacterWant {
    pub fn new(want: Want, priority: u32, now: DateTime<Utc>) -> Self {
        Self {
            want,
            priority,
            acquired_at: now,
        }
    }

    // === Builder Methods ===

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Check if this want is visible to player
    pub fn is_known(&self) -> bool {
        self.want.visibility().is_known()
    }

    /// Check if player has some awareness
    pub fn is_at_least_suspected(&self) -> bool {
        self.want.visibility().is_at_least_suspected()
    }
}

/// The type of target a want can have (for querying purposes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WantTargetType {
    Character,
    Item,
    Goal,
    /// Unknown target type (for forward compatibility)
    #[serde(other)]
    Unknown,
}

/// Actantial role type for character views
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActantialRole {
    /// Character sees target as helping their want
    Helper,
    /// Character sees target as opposing their want
    Opponent,
    /// Character sees target as having initiated/motivated their want
    Sender,
    /// Character sees target as benefiting from their want's fulfillment
    Receiver,
    /// Unknown role type (for forward compatibility)
    #[serde(other)]
    Unknown,
}

/// Data for actantial view edges (VIEWS_AS_HELPER, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActantialView {
    /// Which want this relates to
    pub want_id: WantId,
    /// Why the character views the target this way
    pub reason: String,
    /// When this view was assigned
    pub assigned_at: DateTime<Utc>,
}

impl ActantialView {
    pub fn new(want_id: WantId, reason: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            want_id,
            reason: reason.into(),
            assigned_at: now,
        }
    }

    // === Builder Methods ===

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }
}
