//! Tool call extractor for DM approval workflow.
//!
//! Converts LLM ToolCall responses to ProposedTool structs for DM review.
//! Each proposed tool gets a human-readable description for the DM to understand.

use crate::infrastructure::ports::ToolCall;
use wrldbldr_domain::{GameTool, ProposedTool};

/// Extract proposed tools from LLM tool calls.
///
/// Converts each ToolCall from the LLM response into a ProposedTool
/// suitable for DM review in the approval queue.
pub fn extract_proposed_tools(tool_calls: Vec<ToolCall>) -> Vec<ProposedTool> {
    tool_calls
        .into_iter()
        .map(|tc| ProposedTool {
            id: tc.id,
            name: tc.name.clone(),
            // Use consolidated description logic from domain
            description: GameTool::describe_from_json(&tc.name, &tc.arguments),
            arguments: tc.arguments,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_give_item() {
        let tool_calls = vec![ToolCall {
            id: "call_123".to_string(),
            name: "give_item".to_string(),
            arguments: json!({
                "item_name": "Healing Potion",
                "description": "A red potion that restores health"
            }),
        }];

        let proposed = extract_proposed_tools(tool_calls);
        assert_eq!(proposed.len(), 1);
        assert_eq!(proposed[0].name, "give_item");
        assert_eq!(
            proposed[0].description,
            "Give 'Healing Potion' to the player"
        );
        assert_eq!(proposed[0].arguments["item_name"], "Healing Potion");
    }

    #[test]
    fn test_extract_change_relationship() {
        let tool_calls = vec![ToolCall {
            id: "call_456".to_string(),
            name: "change_relationship".to_string(),
            arguments: json!({
                "change": "improve",
                "amount": "moderate",
                "reason": "helped with quest"
            }),
        }];

        let proposed = extract_proposed_tools(tool_calls);
        assert_eq!(proposed.len(), 1);
        assert!(proposed[0].description.contains("Improve"));
        assert!(proposed[0].description.contains("moderate"));
        assert!(proposed[0].description.contains("helped with quest"));
    }

    #[test]
    fn test_extract_multiple_tools() {
        let tool_calls = vec![
            ToolCall {
                id: "call_1".to_string(),
                name: "give_item".to_string(),
                arguments: json!({ "item_name": "Key", "description": "A brass key" }),
            },
            ToolCall {
                id: "call_2".to_string(),
                name: "reveal_info".to_string(),
                arguments: json!({
                    "info_type": "quest",
                    "content": "Find the temple",
                    "importance": "major"
                }),
            },
        ];

        let proposed = extract_proposed_tools(tool_calls);
        assert_eq!(proposed.len(), 2);
        assert_eq!(proposed[0].id, "call_1");
        assert_eq!(proposed[1].id, "call_2");
    }

    #[test]
    fn test_extract_update_stat_positive() {
        let tool_calls = vec![ToolCall {
            id: "call_789".to_string(),
            name: "update_character_stat".to_string(),
            arguments: json!({
                "character_id": "pc_001",
                "stat_name": "gold",
                "delta": 50
            }),
        }];

        let proposed = extract_proposed_tools(tool_calls);
        assert!(proposed[0].description.contains("+50"));
    }

    #[test]
    fn test_extract_update_stat_negative() {
        let tool_calls = vec![ToolCall {
            id: "call_789".to_string(),
            name: "update_character_stat".to_string(),
            arguments: json!({
                "character_id": "pc_001",
                "stat_name": "health",
                "delta": -10
            }),
        }];

        let proposed = extract_proposed_tools(tool_calls);
        assert!(proposed[0].description.contains("-10"));
    }

    #[test]
    fn test_empty_tool_calls() {
        let proposed = extract_proposed_tools(vec![]);
        assert!(proposed.is_empty());
    }

    #[test]
    fn test_unknown_tool() {
        let tool_calls = vec![ToolCall {
            id: "call_unknown".to_string(),
            name: "unknown_tool".to_string(),
            arguments: json!({}),
        }];

        let proposed = extract_proposed_tools(tool_calls);
        assert!(proposed[0].description.contains("Unknown tool"));
    }
}
