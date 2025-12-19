//! Player action types
//!
//! Defines the actions a player can perform in the game.
//!
//! # Architectural Note (ADR-001: Domain Serialization)
//!
//! `PlayerActionType` intentionally includes serde derives because:
//! 1. It is serialized directly in WebSocket messages to the Engine
//! 2. The wire format IS the domain contract - no translation layer adds value
//! 3. This is a simple enum with no domain behavior beyond categorization
//!
//! This is an accepted exception to the "no serde in domain" rule.

use serde::{Deserialize, Serialize};

/// Types of actions a player can perform
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerActionType {
    /// Speak to an NPC
    Talk,
    /// Examine an object or area
    Examine,
    /// Use an item from inventory
    UseItem,
    /// Travel to a connected location
    Travel,
    /// Custom action entered via text
    Custom,
    /// Select a dialogue choice
    DialogueChoice,
}

impl PlayerActionType {
    /// Get the string representation for the WebSocket message
    pub fn as_str(&self) -> &'static str {
        match self {
            PlayerActionType::Talk => "talk",
            PlayerActionType::Examine => "examine",
            PlayerActionType::UseItem => "use_item",
            PlayerActionType::Travel => "travel",
            PlayerActionType::Custom => "custom",
            PlayerActionType::DialogueChoice => "dialogue_choice",
        }
    }
}

/// A player action to send to the Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerAction {
    /// Type of action
    pub action_type: PlayerActionType,
    /// Target of the action (NPC ID, item ID, location ID, etc.)
    pub target: Option<String>,
    /// Optional dialogue text (for Talk or Custom actions)
    pub dialogue: Option<String>,
    /// Choice ID if selecting from dialogue choices
    pub choice_id: Option<String>,
}

impl PlayerAction {
    /// Create a talk action targeting an NPC
    pub fn talk(target: &str, dialogue: Option<&str>) -> Self {
        Self {
            action_type: PlayerActionType::Talk,
            target: Some(target.to_string()),
            dialogue: dialogue.map(|s| s.to_string()),
            choice_id: None,
        }
    }

    /// Create an examine action
    pub fn examine(target: &str) -> Self {
        Self {
            action_type: PlayerActionType::Examine,
            target: Some(target.to_string()),
            dialogue: None,
            choice_id: None,
        }
    }

    /// Create a use item action
    pub fn use_item(item_id: &str, target: Option<&str>) -> Self {
        Self {
            action_type: PlayerActionType::UseItem,
            target: target.map(|s| s.to_string()),
            dialogue: Some(item_id.to_string()), // Using dialogue field for item_id
            choice_id: None,
        }
    }

    /// Create a travel action to a location
    pub fn travel(location_id: &str) -> Self {
        Self {
            action_type: PlayerActionType::Travel,
            target: Some(location_id.to_string()),
            dialogue: None,
            choice_id: None,
        }
    }

    /// Create a dialogue choice selection
    pub fn dialogue_choice(choice_id: &str) -> Self {
        Self {
            action_type: PlayerActionType::DialogueChoice,
            target: None,
            dialogue: None,
            choice_id: Some(choice_id.to_string()),
        }
    }

    /// Create a custom action with free-form text
    pub fn custom(text: &str) -> Self {
        Self {
            action_type: PlayerActionType::Custom,
            target: None,
            dialogue: Some(text.to_string()),
            choice_id: None,
        }
    }

    /// Create a custom action targeting something specific
    pub fn custom_targeted(target: &str, text: &str) -> Self {
        Self {
            action_type: PlayerActionType::Custom,
            target: Some(target.to_string()),
            dialogue: Some(text.to_string()),
            choice_id: None,
        }
    }
}
