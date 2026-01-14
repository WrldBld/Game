//! Tool definition builder for LLM function calling.
//!
//! Converts game tools to LLM ToolDefinition format for NPC response generation.
//! The LLM can suggest these tools, which then require DM approval before execution.

use crate::infrastructure::ports::ToolDefinition;
use serde_json::json;

/// Build all game tool definitions for LLM function calling.
///
/// Returns a list of ToolDefinition that can be passed to `generate_with_tools()`.
/// These tools allow NPCs to suggest game actions that require DM approval.
pub fn build_game_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        build_give_item_tool(),
        build_reveal_info_tool(),
        build_change_relationship_tool(),
        build_trigger_event_tool(),
        build_modify_npc_motivation_tool(),
        build_modify_character_description_tool(),
        build_modify_npc_opinion_tool(),
        build_transfer_item_tool(),
        build_add_condition_tool(),
        build_remove_condition_tool(),
        build_update_character_stat_tool(),
    ]
}

fn build_give_item_tool() -> ToolDefinition {
    ToolDefinition {
        name: "give_item".to_string(),
        description: "Give an item to the player character. Use when the NPC wants to give, offer, or hand over an item.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "item_name": {
                    "type": "string",
                    "description": "The name of the item being given"
                },
                "description": {
                    "type": "string",
                    "description": "A brief description of the item"
                }
            },
            "required": ["item_name", "description"]
        }),
    }
}

fn build_reveal_info_tool() -> ToolDefinition {
    ToolDefinition {
        name: "reveal_info".to_string(),
        description: "Reveal plot-relevant information to the player. Use when sharing lore, quest details, character secrets, or location information.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "info_type": {
                    "type": "string",
                    "enum": ["lore", "quest", "character", "location"],
                    "description": "Category of information being revealed"
                },
                "content": {
                    "type": "string",
                    "description": "The information being revealed"
                },
                "importance": {
                    "type": "string",
                    "enum": ["minor", "major", "critical"],
                    "description": "How critical this information is to the plot"
                }
            },
            "required": ["info_type", "content", "importance"]
        }),
    }
}

fn build_change_relationship_tool() -> ToolDefinition {
    ToolDefinition {
        name: "change_relationship".to_string(),
        description: "Modify the relationship between this NPC and the player character. Use when trust is gained or lost through interaction.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "change": {
                    "type": "string",
                    "enum": ["improve", "worsen"],
                    "description": "Direction of relationship change"
                },
                "amount": {
                    "type": "string",
                    "enum": ["slight", "moderate", "significant"],
                    "description": "Magnitude of the change"
                },
                "reason": {
                    "type": "string",
                    "description": "Why the relationship is changing"
                }
            },
            "required": ["change", "amount", "reason"]
        }),
    }
}

fn build_trigger_event_tool() -> ToolDefinition {
    ToolDefinition {
        name: "trigger_event".to_string(),
        description: "Trigger a game event or narrative beat. Use sparingly for significant moments like combat starting, discoveries, or environmental changes.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "event_type": {
                    "type": "string",
                    "enum": ["combat", "discovery", "social", "environmental"],
                    "description": "Type of event being triggered"
                },
                "description": {
                    "type": "string",
                    "description": "What happens in this event"
                }
            },
            "required": ["event_type", "description"]
        }),
    }
}

fn build_modify_npc_motivation_tool() -> ToolDefinition {
    ToolDefinition {
        name: "modify_npc_motivation".to_string(),
        description: "Modify an NPC's motivation based on events. Use when an NPC's goals or desires change due to player actions.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "npc_id": {
                    "type": "string",
                    "description": "The ID of the NPC whose motivation is changing"
                },
                "motivation_type": {
                    "type": "string",
                    "enum": ["goal", "fear", "desire", "secret"],
                    "description": "Type of motivation being modified"
                },
                "new_value": {
                    "type": "string",
                    "description": "The new motivation value"
                },
                "reason": {
                    "type": "string",
                    "description": "Why the motivation is changing"
                }
            },
            "required": ["npc_id", "motivation_type", "new_value", "reason"]
        }),
    }
}

fn build_modify_character_description_tool() -> ToolDefinition {
    ToolDefinition {
        name: "modify_character_description".to_string(),
        description: "Modify a character's description. Use when appearance, personality, or backstory details are revealed or changed.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "character_id": {
                    "type": "string",
                    "description": "The ID of the character being modified"
                },
                "change_type": {
                    "type": "string",
                    "enum": ["appearance", "personality", "backstory"],
                    "description": "Type of description change"
                },
                "description": {
                    "type": "string",
                    "description": "The new or updated description"
                }
            },
            "required": ["character_id", "change_type", "description"]
        }),
    }
}

fn build_modify_npc_opinion_tool() -> ToolDefinition {
    ToolDefinition {
        name: "modify_npc_opinion".to_string(),
        description: "Modify an NPC's opinion of a specific player character. Use when an NPC forms or changes their view of a player.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "npc_id": {
                    "type": "string",
                    "description": "The ID of the NPC whose opinion is changing"
                },
                "target_pc_id": {
                    "type": "string",
                    "description": "The ID of the player character being judged"
                },
                "opinion_change": {
                    "type": "string",
                    "description": "How the opinion changes (e.g., 'more trusting', 'suspicious', 'impressed')"
                },
                "reason": {
                    "type": "string",
                    "description": "Why the opinion is changing"
                }
            },
            "required": ["npc_id", "target_pc_id", "opinion_change", "reason"]
        }),
    }
}

fn build_transfer_item_tool() -> ToolDefinition {
    ToolDefinition {
        name: "transfer_item".to_string(),
        description: "Transfer an item between characters. Use when an NPC takes, steals, or receives an item from another character.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "from_id": {
                    "type": "string",
                    "description": "The ID of the character giving the item"
                },
                "to_id": {
                    "type": "string",
                    "description": "The ID of the character receiving the item"
                },
                "item_name": {
                    "type": "string",
                    "description": "The name of the item being transferred"
                }
            },
            "required": ["from_id", "to_id", "item_name"]
        }),
    }
}

fn build_add_condition_tool() -> ToolDefinition {
    ToolDefinition {
        name: "add_condition".to_string(),
        description: "Add a condition to a character. Use for status effects like poisoned, frightened, blessed, etc.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "character_id": {
                    "type": "string",
                    "description": "The ID of the character receiving the condition"
                },
                "condition_name": {
                    "type": "string",
                    "description": "Name of the condition (e.g., 'Poisoned', 'Frightened', 'Blessed')"
                },
                "description": {
                    "type": "string",
                    "description": "Description of the condition's effects"
                },
                "duration": {
                    "type": "string",
                    "description": "Duration of the condition (e.g., '1 hour', 'until rest', 'permanent')"
                }
            },
            "required": ["character_id", "condition_name", "description"]
        }),
    }
}

fn build_remove_condition_tool() -> ToolDefinition {
    ToolDefinition {
        name: "remove_condition".to_string(),
        description: "Remove a condition from a character. Use when healing, curing, or ending a status effect.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "character_id": {
                    "type": "string",
                    "description": "The ID of the character losing the condition"
                },
                "condition_name": {
                    "type": "string",
                    "description": "Name of the condition to remove"
                }
            },
            "required": ["character_id", "condition_name"]
        }),
    }
}

fn build_update_character_stat_tool() -> ToolDefinition {
    ToolDefinition {
        name: "update_character_stat".to_string(),
        description: "Update a character's stat value. Use for health changes, resource spending, or stat modifications.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "character_id": {
                    "type": "string",
                    "description": "The ID of the character whose stat is changing"
                },
                "stat_name": {
                    "type": "string",
                    "description": "Name of the stat (e.g., 'health', 'mana', 'gold')"
                },
                "delta": {
                    "type": "integer",
                    "description": "The change amount (positive to increase, negative to decrease)"
                }
            },
            "required": ["character_id", "stat_name", "delta"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builds_all_tools() {
        let tools = build_game_tool_definitions();
        assert_eq!(tools.len(), 11);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"give_item"));
        assert!(names.contains(&"reveal_info"));
        assert!(names.contains(&"change_relationship"));
        assert!(names.contains(&"trigger_event"));
        assert!(names.contains(&"modify_npc_motivation"));
        assert!(names.contains(&"modify_character_description"));
        assert!(names.contains(&"modify_npc_opinion"));
        assert!(names.contains(&"transfer_item"));
        assert!(names.contains(&"add_condition"));
        assert!(names.contains(&"remove_condition"));
        assert!(names.contains(&"update_character_stat"));
    }

    #[test]
    fn test_give_item_tool_schema() {
        let tool = build_give_item_tool();
        assert_eq!(tool.name, "give_item");
        assert!(tool.description.contains("Give an item"));

        let params = &tool.parameters;
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["item_name"].is_object());
        assert!(params["properties"]["description"].is_object());
        assert!(params["required"]
            .as_array()
            .unwrap()
            .contains(&json!("item_name")));
    }

    #[test]
    fn test_change_relationship_tool_has_enums() {
        let tool = build_change_relationship_tool();
        let change_enum = &tool.parameters["properties"]["change"]["enum"];
        assert!(change_enum.as_array().unwrap().contains(&json!("improve")));
        assert!(change_enum.as_array().unwrap().contains(&json!("worsen")));
    }
}
