//! Tool call parsing and validation

use super::ProposedToolCall;

/// Convert LLM ToolCall format to ProposedToolCall
pub fn parse_tool_calls_from_response(
    tool_calls: &[wrldbldr_engine_ports::outbound::ToolCall],
) -> Vec<ProposedToolCall> {
    tool_calls
        .iter()
        .map(|tc| {
            let description = generate_tool_description(&tc.name, &tc.arguments);
            ProposedToolCall {
                tool_name: tc.name.clone(),
                arguments: tc.arguments.clone(),
                description,
            }
        })
        .collect()
}

/// Generate a human-readable description of a tool call
pub fn generate_tool_description(name: &str, arguments: &serde_json::Value) -> String {
    match name {
        "give_item" => {
            let item = arguments
                .get("item_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown item");
            format!("Give '{}' to the player", item)
        }
        "reveal_info" => {
            let info_type = arguments
                .get("info_type")
                .and_then(|v| v.as_str())
                .unwrap_or("information");
            let importance = arguments
                .get("importance")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
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
                .unwrap_or("slightly");
            format!("{} relationship {} with player", change, amount)
        }
        "trigger_event" => {
            let event_type = arguments
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("event");
            format!("Trigger {} event", event_type)
        }
        "modify_npc_motivation" => {
            let npc_id = arguments
                .get("npc_id")
                .and_then(|v| v.as_str())
                .unwrap_or("NPC");
            let motivation_type = arguments
                .get("motivation_type")
                .and_then(|v| v.as_str())
                .unwrap_or("motivation");
            format!("Modify {}'s {}", npc_id, motivation_type)
        }
        "modify_character_description" => {
            let character_id = arguments
                .get("character_id")
                .and_then(|v| v.as_str())
                .unwrap_or("character");
            let change_type = arguments
                .get("change_type")
                .and_then(|v| v.as_str())
                .unwrap_or("description");
            format!("Update {}'s {}", character_id, change_type)
        }
        "modify_npc_opinion" => {
            let npc_id = arguments
                .get("npc_id")
                .and_then(|v| v.as_str())
                .unwrap_or("NPC");
            let target_pc_id = arguments
                .get("target_pc_id")
                .and_then(|v| v.as_str())
                .unwrap_or("PC");
            format!("Change {}'s opinion of {}", npc_id, target_pc_id)
        }
        "transfer_item" => {
            let item = arguments
                .get("item_name")
                .and_then(|v| v.as_str())
                .unwrap_or("item");
            let from = arguments
                .get("from_id")
                .and_then(|v| v.as_str())
                .unwrap_or("giver");
            let to = arguments
                .get("to_id")
                .and_then(|v| v.as_str())
                .unwrap_or("receiver");
            format!("Transfer '{}' from {} to {}", item, from, to)
        }
        "add_condition" => {
            let character_id = arguments
                .get("character_id")
                .and_then(|v| v.as_str())
                .unwrap_or("character");
            let condition = arguments
                .get("condition_name")
                .and_then(|v| v.as_str())
                .unwrap_or("condition");
            format!("Add '{}' condition to {}", condition, character_id)
        }
        "remove_condition" => {
            let character_id = arguments
                .get("character_id")
                .and_then(|v| v.as_str())
                .unwrap_or("character");
            let condition = arguments
                .get("condition_name")
                .and_then(|v| v.as_str())
                .unwrap_or("condition");
            format!("Remove '{}' condition from {}", condition, character_id)
        }
        "update_character_stat" => {
            let character_id = arguments
                .get("character_id")
                .and_then(|v| v.as_str())
                .unwrap_or("character");
            let stat = arguments
                .get("stat_name")
                .and_then(|v| v.as_str())
                .unwrap_or("stat");
            let delta = arguments.get("delta").and_then(|v| v.as_i64()).unwrap_or(0);
            let sign = if delta >= 0 { "+" } else { "" };
            format!("Update {}'s {} by {}{}", character_id, stat, sign, delta)
        }
        _ => format!("Call {} with provided arguments", name),
    }
}
