// Game tools are defined for future LLM tool-calling workflows
#![allow(dead_code)]

//! Game tools used in LLM tool-call workflows.

use serde_json::Value;

/// Available tools/actions that an NPC can suggest via the LLM
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameTool {
    /// Give an item to the player
    GiveItem {
        item_name: String,
        description: String,
    },

    /// Reveal plot-relevant information
    RevealInfo {
        info_type: String,
        content: String,
        importance: InfoImportance,
    },

    /// Modify the relationship between an NPC and player
    ChangeRelationship {
        change: RelationshipChange,
        amount: ChangeAmount,
        reason: String,
    },

    /// Trigger a game event or narrative beat
    TriggerEvent {
        event_type: String,
        description: String,
    },

    /// Modify an NPC's motivation
    ModifyNpcMotivation {
        npc_id: String,
        motivation_type: String,
        new_value: String,
        reason: String,
    },

    /// Modify a character's description
    ModifyCharacterDescription {
        character_id: String,
        change_type: String,
        description: String,
    },

    /// Modify an NPC's opinion of a player character
    ModifyNpcOpinion {
        npc_id: String,
        target_pc_id: String,
        opinion_change: String,
        reason: String,
    },

    /// Transfer an item between characters
    TransferItem {
        from_id: String,
        to_id: String,
        item_name: String,
    },

    /// Add a condition to a character
    AddCondition {
        character_id: String,
        condition_name: String,
        description: String,
        duration: Option<String>,
    },

    /// Remove a condition from a character
    RemoveCondition {
        character_id: String,
        condition_name: String,
    },

    /// Update a character's stat value
    UpdateCharacterStat {
        character_id: String,
        stat_name: String,
        delta: i32,
    },
}

impl GameTool {
    /// Build a human-readable description from raw tool call data (name + JSON arguments).
    pub fn describe_from_json(name: &str, arguments: &Value) -> String {
        match name {
            "give_item" => {
                let item_name = arguments
                    .get("item_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("an item");
                format!("Give '{}' to the player", item_name)
            }
            "reveal_info" => {
                let info_type = arguments
                    .get("info_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("information");
                let importance = arguments
                    .get("importance")
                    .and_then(|v| v.as_str())
                    .unwrap_or("some");
                format!("Reveal {} {} to the player", importance, info_type)
            }
            "change_relationship" => {
                let change = arguments
                    .get("change")
                    .and_then(|v| v.as_str())
                    .unwrap_or("change");
                let amount = arguments
                    .get("amount")
                    .and_then(|v| v.as_str())
                    .unwrap_or("somewhat");
                let reason = arguments
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("interaction");
                format!(
                    "{} relationship {} with player ({})",
                    capitalize_first(change),
                    amount,
                    reason
                )
            }
            "trigger_event" => {
                let event_type = arguments
                    .get("event_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("narrative");
                let description = arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("An event occurs");
                format!("Trigger {} event: {}", event_type, description)
            }
            "modify_npc_motivation" => {
                let motivation_type = arguments
                    .get("motivation_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("motivation");
                let new_value = arguments
                    .get("new_value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("changed");
                let reason = arguments
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("events");
                format!(
                    "Change NPC's {}: '{}' ({})",
                    motivation_type, new_value, reason
                )
            }
            "modify_character_description" => {
                let change_type = arguments
                    .get("change_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("description");
                let description = arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("updated");
                format!(
                    "Update character's {}: {}",
                    change_type,
                    truncate_str(description, 50)
                )
            }
            "modify_npc_opinion" => {
                let opinion_change = arguments
                    .get("opinion_change")
                    .and_then(|v| v.as_str())
                    .unwrap_or("changed opinion");
                let reason = arguments
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("interaction");
                format!("NPC becomes {} toward player ({})", opinion_change, reason)
            }
            "transfer_item" => {
                let item_name = arguments
                    .get("item_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("an item");
                format!("Transfer '{}' between characters", item_name)
            }
            "add_condition" => {
                let condition_name = arguments
                    .get("condition_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("a condition");
                let duration = arguments
                    .get("duration")
                    .and_then(|v| v.as_str())
                    .unwrap_or("until removed");
                format!("Add condition '{}' ({})", condition_name, duration)
            }
            "remove_condition" => {
                let condition_name = arguments
                    .get("condition_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("a condition");
                format!("Remove condition '{}'", condition_name)
            }
            "update_character_stat" => {
                let stat_name = arguments
                    .get("stat_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("stat");
                let delta = arguments.get("delta").and_then(|v| v.as_i64()).unwrap_or(0);
                let change = if delta >= 0 {
                    format!("+{}", delta)
                } else {
                    delta.to_string()
                };
                format!("Update {} by {}", stat_name, change)
            }
            _ => format!("Unknown tool: {}", name),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InfoImportance {
    Minor,
    Major,
    Critical,
}

impl InfoImportance {
    pub fn as_str(&self) -> &'static str {
        match self {
            InfoImportance::Minor => "minor",
            InfoImportance::Major => "major",
            InfoImportance::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationshipChange {
    Improve,
    Worsen,
}

impl RelationshipChange {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationshipChange::Improve => "improve",
            RelationshipChange::Worsen => "worsen",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeAmount {
    Slight,
    Moderate,
    Significant,
}

impl ChangeAmount {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeAmount::Slight => "slight",
            ChangeAmount::Moderate => "moderate",
            ChangeAmount::Significant => "significant",
        }
    }
}

fn capitalize_first(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn truncate_str(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        value.to_string()
    } else {
        format!("{}...", &value[..max_len - 3])
    }
}
