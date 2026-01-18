//! Tool executor for approved LLM tool calls.
//!
//! When the DM approves tools proposed by the LLM, this module executes them
//! to actually affect game state (give items, change relationships, etc.).

use std::sync::Arc;

use wrldbldr_domain::{CharacterId, PlayerCharacterId};

use crate::infrastructure::ports::{CharacterRepo, ItemRepo, PlayerCharacterRepo, RepoError};
use crate::queue_types::ProposedTool;
use crate::use_cases::inventory::GiveItem;

/// Errors that can occur during tool execution.
#[derive(Debug, thiserror::Error)]
pub enum ToolExecutionError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),

    #[error("Inventory error: {0}")]
    Inventory(#[from] crate::use_cases::inventory::InventoryError),

    #[error("Tool not found in proposed tools: {0}")]
    ToolNotFound(String),

    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

/// Result of executing a single tool.
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    /// Tool ID that was executed
    pub tool_id: String,
    /// Human-readable description of what happened
    pub description: String,
}

/// Executor for approved LLM tool calls.
///
/// Takes the proposed tools from an approval request and executes the ones
/// that were approved by the DM.
pub struct ToolExecutor {
    give_item: Arc<GiveItem>,
    character_repo: Arc<dyn CharacterRepo>,
}

impl ToolExecutor {
    pub fn new(
        item_repo: Arc<dyn ItemRepo>,
        pc_repo: Arc<dyn PlayerCharacterRepo>,
        character_repo: Arc<dyn CharacterRepo>,
    ) -> Self {
        Self {
            give_item: Arc::new(GiveItem::new(item_repo, pc_repo)),
            character_repo,
        }
    }

    /// Execute all approved tools.
    ///
    /// # Arguments
    /// * `approved_tool_ids` - IDs of tools the DM approved
    /// * `proposed_tools` - All tools proposed in the original approval request
    /// * `pc_id` - The player character involved (recipient for items, etc.)
    /// * `npc_id` - The NPC who proposed the tools (for relationship changes)
    ///
    /// # Returns
    /// Results for each successfully executed tool. Tools that fail are logged but don't
    /// stop execution of other tools.
    pub async fn execute_approved(
        &self,
        approved_tool_ids: &[String],
        proposed_tools: &[ProposedTool],
        pc_id: Option<PlayerCharacterId>,
        npc_id: Option<CharacterId>,
    ) -> Vec<ToolExecutionResult> {
        let mut results = Vec::new();

        for tool_id in approved_tool_ids {
            // Find the proposed tool by ID
            let Some(tool) = proposed_tools.iter().find(|t| &t.id == tool_id) else {
                tracing::warn!(tool_id = %tool_id, "Approved tool not found in proposed tools");
                continue;
            };

            match self.execute_tool(tool, pc_id, npc_id).await {
                Ok(result) => {
                    tracing::info!(
                        tool_id = %tool_id,
                        tool_name = %tool.name,
                        "Tool executed successfully"
                    );
                    results.push(result);
                }
                Err(e) => {
                    tracing::error!(
                        tool_id = %tool_id,
                        tool_name = %tool.name,
                        error = %e,
                        "Failed to execute approved tool"
                    );
                    // Continue with other tools
                }
            }
        }

        results
    }

    /// Execute a single tool.
    async fn execute_tool(
        &self,
        tool: &ProposedTool,
        pc_id: Option<PlayerCharacterId>,
        npc_id: Option<CharacterId>,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        match tool.name.as_str() {
            "give_item" => self.execute_give_item(tool, pc_id).await,
            "change_relationship" => self.execute_change_relationship(tool, npc_id, pc_id).await,
            "change_disposition" => self.execute_change_disposition(tool, npc_id).await,
            "change_mood" => self.execute_change_mood(tool, npc_id).await,
            "reveal_info" => self.execute_reveal_info(tool).await,
            "trigger_event" => self.execute_trigger_event(tool).await,
            "update_character_stat" => self.execute_update_character_stat(tool).await,
            _ => {
                tracing::warn!(tool_name = %tool.name, "Unknown tool type - skipping execution");
                Ok(ToolExecutionResult {
                    tool_id: tool.id.clone(),
                    description: format!("Unknown tool '{}' - no action taken", tool.name),
                })
            }
        }
    }

    /// Execute give_item tool.
    async fn execute_give_item(
        &self,
        tool: &ProposedTool,
        pc_id: Option<PlayerCharacterId>,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let pc_id = pc_id.ok_or_else(|| {
            ToolExecutionError::MissingArgument("pc_id required for give_item".to_string())
        })?;

        let item_name = tool
            .arguments
            .get("item_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolExecutionError::MissingArgument("item_name".to_string()))?;

        let description = tool
            .arguments
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        self.give_item
            .execute(pc_id, item_name.to_string(), description)
            .await?;

        Ok(ToolExecutionResult {
            tool_id: tool.id.clone(),
            description: format!("Gave '{}' to player", item_name),
        })
    }

    /// Execute change_relationship tool.
    ///
    /// TODO: Implement proper relationship persistence when PC-NPC relationship
    /// tracking is fully designed. Currently logs the change for visibility.
    async fn execute_change_relationship(
        &self,
        tool: &ProposedTool,
        npc_id: Option<CharacterId>,
        _pc_id: Option<PlayerCharacterId>,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let change = tool
            .arguments
            .get("change")
            .and_then(|v| v.as_str())
            .unwrap_or("improve");

        let amount = tool
            .arguments
            .get("amount")
            .and_then(|v| v.as_str())
            .unwrap_or("moderate");

        let reason = tool
            .arguments
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("interaction");

        tracing::info!(
            npc_id = ?npc_id,
            change = %change,
            amount = %amount,
            reason = %reason,
            "Relationship change requested (persistence TBD)"
        );

        Ok(ToolExecutionResult {
            tool_id: tool.id.clone(),
            description: format!(
                "Relationship {} {} ({})",
                if change == "improve" {
                    "improved"
                } else {
                    "worsened"
                },
                amount,
                reason
            ),
        })
    }

    /// Execute change_disposition tool.
    ///
    /// TODO: Implement proper disposition persistence when PC-specific NPC
    /// state tracking is designed. Currently logs the change for visibility.
    async fn execute_change_disposition(
        &self,
        tool: &ProposedTool,
        npc_id: Option<CharacterId>,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let new_disposition = tool
            .arguments
            .get("new_disposition")
            .and_then(|v| v.as_str())
            .unwrap_or("neutral");

        let reason = tool
            .arguments
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("interaction");

        tracing::info!(
            npc_id = ?npc_id,
            disposition = %new_disposition,
            reason = %reason,
            "Disposition change requested (persistence TBD)"
        );

        Ok(ToolExecutionResult {
            tool_id: tool.id.clone(),
            description: format!("Disposition changed to {} ({})", new_disposition, reason),
        })
    }

    /// Execute change_mood tool.
    ///
    /// TODO: Implement proper mood persistence when conversation-scoped NPC
    /// state tracking is designed. Currently logs the change for visibility.
    async fn execute_change_mood(
        &self,
        tool: &ProposedTool,
        npc_id: Option<CharacterId>,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let new_mood = tool
            .arguments
            .get("new_mood")
            .and_then(|v| v.as_str())
            .unwrap_or("calm");

        let reason = tool
            .arguments
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("interaction");

        tracing::info!(
            npc_id = ?npc_id,
            mood = %new_mood,
            reason = %reason,
            "Mood change requested (persistence TBD)"
        );

        Ok(ToolExecutionResult {
            tool_id: tool.id.clone(),
            description: format!("Mood changed to {} ({})", new_mood, reason),
        })
    }

    /// Execute reveal_info tool (currently just logs - info persistence TBD).
    async fn execute_reveal_info(
        &self,
        tool: &ProposedTool,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let info_type = tool
            .arguments
            .get("info_type")
            .and_then(|v| v.as_str())
            .unwrap_or("information");

        let content = tool
            .arguments
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let importance = tool
            .arguments
            .get("importance")
            .and_then(|v| v.as_str())
            .unwrap_or("minor");

        tracing::info!(
            info_type = %info_type,
            importance = %importance,
            content = %content,
            "Information revealed (not yet persisted)"
        );

        // TODO: Persist revealed information to lore system when implemented
        Ok(ToolExecutionResult {
            tool_id: tool.id.clone(),
            description: format!("Revealed {} {} information", importance, info_type),
        })
    }

    /// Execute trigger_event tool (currently just logs - event triggering TBD).
    async fn execute_trigger_event(
        &self,
        tool: &ProposedTool,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let event_type = tool
            .arguments
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("narrative");

        let description = tool
            .arguments
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("An event occurred");

        tracing::info!(
            event_type = %event_type,
            description = %description,
            "Event triggered (not yet integrated with narrative system)"
        );

        // TODO: Integrate with narrative event system when appropriate
        Ok(ToolExecutionResult {
            tool_id: tool.id.clone(),
            description: format!("Triggered {} event: {}", event_type, description),
        })
    }

    /// Execute update_character_stat tool (currently just logs - stat updates TBD).
    async fn execute_update_character_stat(
        &self,
        tool: &ProposedTool,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let stat_name = tool
            .arguments
            .get("stat_name")
            .and_then(|v| v.as_str())
            .unwrap_or("stat");

        let delta = tool
            .arguments
            .get("delta")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let character_id = tool
            .arguments
            .get("character_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        tracing::info!(
            character_id = %character_id,
            stat_name = %stat_name,
            delta = %delta,
            "Stat update requested (not yet implemented)"
        );

        // TODO: Implement stat updates via PlayerCharacterRepo when appropriate
        let change_str = if delta >= 0 {
            format!("+{}", delta)
        } else {
            delta.to_string()
        };

        Ok(ToolExecutionResult {
            tool_id: tool.id.clone(),
            description: format!("Updated {} by {}", stat_name, change_str),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{MockCharacterRepo, MockItemRepo, MockPlayerCharacterRepo};
    use serde_json::json;

    fn mock_executor() -> ToolExecutor {
        ToolExecutor::new(
            Arc::new(MockItemRepo::new()),
            Arc::new(MockPlayerCharacterRepo::new()),
            Arc::new(MockCharacterRepo::new()),
        )
    }

    #[test]
    fn test_tool_execution_result_creation() {
        let result = ToolExecutionResult {
            tool_id: "call_123".to_string(),
            description: "Gave 'Healing Potion' to player".to_string(),
        };
        assert_eq!(result.tool_id, "call_123");
        assert!(result.description.contains("Healing Potion"));
    }

    #[tokio::test]
    async fn test_unknown_tool_returns_ok_with_message() {
        let executor = mock_executor();
        let tool = ProposedTool {
            id: "call_unknown".to_string(),
            name: "unknown_tool".to_string(),
            description: "Unknown tool".to_string(),
            arguments: json!({}),
        };

        let results = executor
            .execute_approved(&["call_unknown".to_string()], &[tool], None, None)
            .await;

        assert_eq!(results.len(), 1);
        assert!(results[0].description.contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_tool_not_in_proposed_is_skipped() {
        let executor = mock_executor();
        let tool = ProposedTool {
            id: "call_123".to_string(),
            name: "give_item".to_string(),
            description: "Give item".to_string(),
            arguments: json!({"item_name": "Sword"}),
        };

        // Approve a tool that doesn't exist in proposed_tools
        let results = executor
            .execute_approved(&["call_999".to_string()], &[tool], None, None)
            .await;

        assert!(results.is_empty());
    }
}
