// Game tools are defined for future LLM tool-calling workflows
#![allow(dead_code)]

//! Game tools used in LLM tool-call workflows.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use wrldbldr_domain::CharacterId;
use wrldbldr_domain::PlayerCharacterId;

/// Available tools/actions that an NPC can suggest via LLM
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

// =============================================================================
// ToolArgs - Typed arguments for proposed tools
// =============================================================================

/// Typed arguments for a proposed tool call.
///
/// This is the typed version of what gets parsed from LLM JSON.
/// It matches the structure of GameTool for type safety.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool")]
pub enum ToolArgs {
    /// Give an item to player
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

    /// Modify relationship between an NPC and player
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
        npc_id: CharacterId,
        motivation_type: String,
        new_value: String,
        reason: String,
    },

    /// Modify a character's description
    ModifyCharacterDescription {
        character_id: CharacterId,
        change_type: String,
        description: String,
    },

    /// Modify an NPC's opinion of a player character
    ModifyNpcOpinion {
        npc_id: CharacterId,
        target_pc_id: PlayerCharacterId,
        opinion_change: String,
        reason: String,
    },

    /// Transfer an item between characters
    TransferItem {
        from_id: CharacterId,
        to_id: CharacterId,
        item_name: String,
    },

    /// Add a condition to a character
    AddCondition {
        character_id: CharacterId,
        condition_name: String,
        description: String,
        duration: Option<String>,
    },

    /// Remove a condition from a character
    RemoveCondition {
        character_id: CharacterId,
        condition_name: String,
    },

    /// Update a character's stat value
    UpdateCharacterStat {
        character_id: CharacterId,
        stat_name: String,
        delta: i32,
    },

    /// Unknown tool for forward compatibility
    #[serde(other)]
    Unknown,
}

impl ToolArgs {
    /// Get the tool name (snake_case string matching tool type).
    pub fn tool_name(&self) -> &'static str {
        match self {
            ToolArgs::GiveItem { .. } => "give_item",
            ToolArgs::RevealInfo { .. } => "reveal_info",
            ToolArgs::ChangeRelationship { .. } => "change_relationship",
            ToolArgs::TriggerEvent { .. } => "trigger_event",
            ToolArgs::ModifyNpcMotivation { .. } => "modify_npc_motivation",
            ToolArgs::ModifyCharacterDescription { .. } => "modify_character_description",
            ToolArgs::ModifyNpcOpinion { .. } => "modify_npc_opinion",
            ToolArgs::TransferItem { .. } => "transfer_item",
            ToolArgs::AddCondition { .. } => "add_condition",
            ToolArgs::RemoveCondition { .. } => "remove_condition",
            ToolArgs::UpdateCharacterStat { .. } => "update_character_stat",
            ToolArgs::Unknown => "unknown",
        }
    }

    /// Get a human-readable description of this tool call.
    pub fn description(&self) -> String {
        match self {
            ToolArgs::GiveItem { item_name, .. } => {
                format!("Give '{}' to player", item_name)
            }
            ToolArgs::RevealInfo {
                info_type,
                importance,
                ..
            } => {
                format!("Reveal {} {} to player", importance.as_str(), info_type)
            }
            ToolArgs::ChangeRelationship {
                change,
                amount,
                reason,
                ..
            } => {
                format!(
                    "{} relationship {} with player ({})",
                    capitalize_first(change.as_str()),
                    amount.as_str(),
                    reason
                )
            }
            ToolArgs::TriggerEvent {
                event_type,
                description,
            } => {
                format!("Trigger {} event: {}", event_type, description)
            }
            ToolArgs::ModifyNpcMotivation {
                motivation_type,
                new_value,
                reason,
                ..
            } => {
                format!(
                    "Change NPC's {}: '{}' ({})",
                    motivation_type, new_value, reason
                )
            }
            ToolArgs::ModifyCharacterDescription {
                change_type,
                description,
                ..
            } => {
                format!(
                    "Update character's {}: {}",
                    change_type,
                    truncate_str(description, 50)
                )
            }
            ToolArgs::ModifyNpcOpinion {
                opinion_change,
                reason,
                ..
            } => {
                format!("NPC becomes {} toward player ({})", opinion_change, reason)
            }
            ToolArgs::TransferItem { item_name, .. } => {
                format!("Transfer '{}' between characters", item_name)
            }
            ToolArgs::AddCondition {
                condition_name,
                duration,
                ..
            } => {
                let duration = duration.as_deref().unwrap_or("until removed");
                format!("Add condition '{}' ({})", condition_name, duration)
            }
            ToolArgs::RemoveCondition { condition_name, .. } => {
                format!("Remove condition '{}'", condition_name)
            }
            ToolArgs::UpdateCharacterStat {
                stat_name, delta, ..
            } => {
                let change = if *delta >= 0 {
                    format!("+{}", delta)
                } else {
                    delta.to_string()
                };
                format!("Update {} by {}", stat_name, change)
            }
            ToolArgs::Unknown => "Unknown tool".to_string(),
        }
    }

    /// Convert to serde_json::Value for backward compatibility with queue storage.
    pub fn to_json_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// Parse from JSON string for validation.
    pub fn from_json_str(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

// Implement serde support for helper enums
impl Serialize for InfoImportance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for InfoImportance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "minor" => Ok(InfoImportance::Minor),
            "major" => Ok(InfoImportance::Major),
            "critical" => Ok(InfoImportance::Critical),
            _ => Err(serde::de::Error::custom(format!(
                "unknown importance: {}",
                s
            ))),
        }
    }
}

impl Serialize for RelationshipChange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RelationshipChange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "improve" => Ok(RelationshipChange::Improve),
            "worsen" => Ok(RelationshipChange::Worsen),
            _ => Err(serde::de::Error::custom(format!(
                "unknown relationship change: {}",
                s
            ))),
        }
    }
}

impl Serialize for ChangeAmount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ChangeAmount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "slight" => Ok(ChangeAmount::Slight),
            "moderate" => Ok(ChangeAmount::Moderate),
            "significant" => Ok(ChangeAmount::Significant),
            _ => Err(serde::de::Error::custom(format!(
                "unknown change amount: {}",
                s
            ))),
        }
    }
}
