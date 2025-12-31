//! State change DTOs
//!
//! These types represent discrete changes to game state that can be
//! produced by application services (e.g., tool execution, trigger execution)
//! and then applied/broadcast by orchestrating layers.

use serde::{Deserialize, Serialize};

/// Individual state changes caused by tool/trigger execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StateChange {
    /// An item was added to a character's inventory.
    ItemAdded { character: String, item: String },
    /// Information was revealed to the player.
    InfoRevealed { info: String },
    /// A relationship sentiment was changed.
    RelationshipChanged {
        from: String,
        to: String,
        delta: i32,
    },
    /// An event was triggered.
    EventTriggered { name: String },
    /// An NPC's motivation was modified.
    NpcMotivationChanged {
        npc_id: String,
        motivation_type: String,
        new_value: String,
        reason: String,
    },
    /// A character's description was updated.
    CharacterDescriptionUpdated {
        character_id: String,
        change_type: String,
        description: String,
    },
    /// An NPC's opinion of a PC changed.
    NpcOpinionChanged {
        npc_id: String,
        target_pc_id: String,
        opinion_change: String,
        reason: String,
    },
    /// An item was transferred between characters.
    ItemTransferred {
        from_id: String,
        to_id: String,
        item_name: String,
    },
    /// A condition was added to a character.
    ConditionAdded {
        character_id: String,
        condition_name: String,
        description: String,
        duration: Option<String>,
    },
    /// A condition was removed from a character.
    ConditionRemoved {
        character_id: String,
        condition_name: String,
    },
    /// A character's stat was updated.
    CharacterStatUpdated {
        character_id: String,
        stat_name: String,
        delta: i32,
    },
}
