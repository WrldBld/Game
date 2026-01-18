//! Player action DTOs (application layer)
//!
//! These types represent client-side actions that are sent to the Engine.
//! They are application-owned so higher layers don't depend on the player binary crate.

use serde::{Deserialize, Serialize};

/// Types of actions a player can perform
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// String representation used by the websocket protocol.
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
    /// Optional dialogue text (Talk / Custom)
    pub dialogue: Option<String>,
    /// Choice id when selecting from dialogue choices
    pub choice_id: Option<String>,
}

impl PlayerAction {
    pub fn talk(target: &str, dialogue: Option<&str>) -> Self {
        Self {
            action_type: PlayerActionType::Talk,
            target: Some(target.to_string()),
            dialogue: dialogue.map(|s| s.to_string()),
            choice_id: None,
        }
    }

    pub fn examine(target: &str) -> Self {
        Self {
            action_type: PlayerActionType::Examine,
            target: Some(target.to_string()),
            dialogue: None,
            choice_id: None,
        }
    }

    pub fn use_item(item_id: &str, target: Option<&str>) -> Self {
        Self {
            action_type: PlayerActionType::UseItem,
            target: target.map(|s| s.to_string()),
            dialogue: Some(item_id.to_string()),
            choice_id: None,
        }
    }

    pub fn travel(location_id: &str) -> Self {
        Self {
            action_type: PlayerActionType::Travel,
            target: Some(location_id.to_string()),
            dialogue: None,
            choice_id: None,
        }
    }

    pub fn dialogue_choice(choice_id: &str) -> Self {
        Self {
            action_type: PlayerActionType::DialogueChoice,
            target: None,
            dialogue: None,
            choice_id: Some(choice_id.to_string()),
        }
    }

    pub fn custom(text: &str) -> Self {
        Self {
            action_type: PlayerActionType::Custom,
            target: None,
            dialogue: Some(text.to_string()),
            choice_id: None,
        }
    }

    pub fn custom_targeted(target: &str, text: &str) -> Self {
        Self {
            action_type: PlayerActionType::Custom,
            target: Some(target.to_string()),
            dialogue: Some(text.to_string()),
            choice_id: None,
        }
    }
}
