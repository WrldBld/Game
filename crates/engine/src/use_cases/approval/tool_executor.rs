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

    #[error("Unknown tool type: {0}")]
    UnknownTool(String),
}

/// Result of executing a single tool.
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    /// Tool ID that was executed
    pub tool_id: String,
    /// Human-readable description of what happened
    pub description: String,
    /// Stat update result (for update_character_stat tool)
    pub stat_update: Option<StatUpdate>,
}

/// Result of updating a character stat.
#[derive(Debug, Clone)]
pub struct StatUpdate {
    /// Character ID whose stat was updated
    pub character_id: PlayerCharacterId,
    /// Character name (for UI display)
    pub character_name: String,
    /// Stat name that was updated
    pub stat_name: String,
    /// Old value before update
    pub old_value: i32,
    /// New value after update
    pub new_value: i32,
    /// Delta that was applied (can be negative)
    pub delta: i32,
}

/// Executor for approved LLM tool calls.
///
/// Takes proposed tools from an approval request and executes ones
/// that were approved by the DM.
pub struct ToolExecutor {
    give_item: Arc<GiveItem>,
    character_repo: Arc<dyn CharacterRepo>,
    pc_repo: Arc<dyn PlayerCharacterRepo>,
}

impl ToolExecutor {
    pub fn new(
        item_repo: Arc<dyn ItemRepo>,
        pc_repo: Arc<dyn PlayerCharacterRepo>,
        character_repo: Arc<dyn CharacterRepo>,
    ) -> Self {
        Self {
            give_item: Arc::new(GiveItem::new(item_repo, pc_repo.clone())),
            character_repo,
            pc_repo,
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
        let tool_id = tool.id.clone();
        // Match on typed ToolArgs variant and validate it matches tool name
        match &tool.arguments {
            crate::game_tools::ToolArgs::GiveItem { item_name, description } => {
                if tool.name != "give_item" {
                    return Err(ToolExecutionError::InvalidArgument(format!(
                        "Tool name '{}' does not match arguments type GiveItem",
                        tool.name
                    )));
                }
                self.execute_give_item_typed(tool_id, pc_id, item_name, description).await
            }
            crate::game_tools::ToolArgs::RevealInfo {
                info_type,
                content,
                importance: _,
            } => {
                if tool.name != "reveal_info" {
                    return Err(ToolExecutionError::InvalidArgument(format!(
                        "Tool name '{}' does not match arguments type RevealInfo",
                        tool.name
                    )));
                }
                self.execute_reveal_info_typed(tool_id, info_type, content).await
            }
            crate::game_tools::ToolArgs::ChangeRelationship {
                change,
                amount,
                reason,
            } => {
                if tool.name != "change_relationship" {
                    return Err(ToolExecutionError::InvalidArgument(format!(
                        "Tool name '{}' does not match arguments type ChangeRelationship",
                        tool.name
                    )));
                }
                self.execute_change_relationship_typed(tool_id, npc_id, change, amount, reason).await
            }
            crate::game_tools::ToolArgs::TriggerEvent {
                event_type,
                description,
            } => {
                if tool.name != "trigger_event" {
                    return Err(ToolExecutionError::InvalidArgument(format!(
                        "Tool name '{}' does not match arguments type TriggerEvent",
                        tool.name
                    )));
                }
                self.execute_trigger_event_typed(tool_id, event_type, description).await
            }
            crate::game_tools::ToolArgs::UpdateCharacterStat {
                character_id,
                stat_name,
                delta,
            } => {
                if tool.name != "update_character_stat" {
                    return Err(ToolExecutionError::InvalidArgument(format!(
                        "Tool name '{}' does not match arguments type UpdateCharacterStat",
                        tool.name
                    )));
                }
                self.execute_update_character_stat_typed(tool_id, *character_id, stat_name, *delta).await
            }
            crate::game_tools::ToolArgs::Unknown => {
                tracing::error!(tool_name = %tool.name, "Unknown tool variant in ToolArgs");
                Err(ToolExecutionError::UnknownTool(tool.name.clone()))
            }
            _ => {
                tracing::warn!(tool_name = %tool.name, "Tool not supported by executor");
                Err(ToolExecutionError::UnknownTool(tool.name.clone()))
            }
        }
    }

    /// Execute give_item tool.
    async fn execute_give_item_typed(
        &self,
        tool_id: String,
        pc_id: Option<PlayerCharacterId>,
        item_name: &str,
        description: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let pc_id = pc_id.ok_or_else(|| {
            ToolExecutionError::MissingArgument("pc_id required for give_item".to_string())
        })?;

        self.give_item
            .execute(pc_id, item_name.to_string(), Some(description.to_string()))
            .await?;

        Ok(ToolExecutionResult {
            tool_id,
            description: format!("Gave '{}' to player", item_name),
            stat_update: None,
        })
    }

    /// Execute change_relationship tool.
    ///
    /// TODO: Implement proper relationship persistence when PC-NPC relationship
    /// tracking is fully designed. Currently logs the change for visibility.
    async fn execute_change_relationship_typed(
        &self,
        tool_id: String,
        npc_id: Option<CharacterId>,
        change: &crate::game_tools::RelationshipChange,
        amount: &crate::game_tools::ChangeAmount,
        reason: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        tracing::info!(
            npc_id = ?npc_id,
            change = %change.as_str(),
            amount = %amount.as_str(),
            reason = %reason,
            "Relationship change requested (persistence TBD)"
        );

        Ok(ToolExecutionResult {
            tool_id,
            description: format!(
                "Relationship {} {} ({})",
                change.as_str(),
                amount.as_str(),
                reason
            ),
            stat_update: None,
        })
    }

    /// Execute change_disposition tool.
    ///
    /// TODO: Implement proper disposition persistence when PC-specific NPC
    /// state tracking is designed. Currently logs the change for visibility.
    async fn execute_change_disposition(
        &self,
        tool_id: String,
        _tool: &ProposedTool,
        _npc_id: Option<CharacterId>,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        // Legacy tool - no longer supported
        tracing::warn!("change_disposition tool is deprecated and not supported");
        Ok(ToolExecutionResult {
            tool_id,
            description: "Disposition change not supported".to_string(),
            stat_update: None,
        })
    }

    /// Execute change_mood tool.
    ///
    /// TODO: Implement proper mood persistence when conversation-scoped NPC
    /// state tracking is designed. Currently logs the change for visibility.
    async fn execute_change_mood(
        &self,
        tool_id: String,
        _tool: &ProposedTool,
        _npc_id: Option<CharacterId>,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        // Legacy tool - no longer supported
        tracing::warn!("change_mood tool is deprecated and not supported");
        Ok(ToolExecutionResult {
            tool_id,
            description: "Mood change not supported".to_string(),
            stat_update: None,
        })
    }

    /// Execute reveal_info tool (currently just logs - info persistence TBD).
    async fn execute_reveal_info_typed(
        &self,
        tool_id: String,
        info_type: &str,
        content: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        tracing::info!(
            info_type = %info_type,
            content = %content,
            "Information revealed (not yet persisted)"
        );

        // TODO: Persist revealed information to lore system when implemented
        Ok(ToolExecutionResult {
            tool_id,
            description: format!("Revealed {} information", info_type),
            stat_update: None,
        })
    }

    /// Execute trigger_event tool (currently just logs - event triggering TBD).
    async fn execute_trigger_event_typed(
        &self,
        tool_id: String,
        event_type: &str,
        description: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        tracing::info!(
            event_type = %event_type,
            description = %description,
            "Event triggered (not yet integrated with narrative system)"
        );

        // TODO: Integrate with narrative event system when appropriate
        Ok(ToolExecutionResult {
            tool_id,
            description: format!("Triggered {} event: {}", event_type, description),
            stat_update: None,
        })
    }

    /// Execute update_character_stat tool.
    async fn execute_update_character_stat_typed(
        &self,
        tool_id: String,
        pc_id: PlayerCharacterId,
        stat_name: &str,
        delta: i32,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        // Get current PC to read current stat value
        let pc = self
            .pc_repo
            .get(pc_id)
            .await?
            .ok_or(ToolExecutionError::InvalidArgument(format!(
                "Player character not found: {}",
                pc_id
            )))?;

        let character_name = pc.name().to_string();

        // Apply stat update via PlayerCharacterRepo
        let stat_result = self
            .pc_repo
            .modify_stat(pc_id, stat_name, delta)
            .await?;

        let change_str = if delta >= 0 {
            format!("+{}", delta)
        } else {
            delta.to_string()
        };

        Ok(ToolExecutionResult {
            tool_id,
            description: format!(
                "Updated {} by {} (new value: {})",
                stat_name, change_str, stat_result.new_value
            ),
            stat_update: Some(StatUpdate {
                character_id: pc_id,
                character_name,
                stat_name: stat_name.to_string(),
                old_value: stat_result.old_value,
                new_value: stat_result.new_value,
                delta: stat_result.delta,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{MockCharacterRepo, MockItemRepo, MockPlayerCharacterRepo};

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
            stat_update: None,
        };
        assert_eq!(result.tool_id, "call_123");
        assert!(result.description.contains("Healing Potion"));
        assert!(result.stat_update.is_none());
    }

    #[test]
    fn test_stat_update_fields() {
        let pc_id = PlayerCharacterId::new();
        let stat_update = StatUpdate {
            character_id: pc_id,
            character_name: "Test PC".to_string(),
            stat_name: "STR".to_string(),
            old_value: 10,
            new_value: 12,
            delta: 2,
        };
        assert_eq!(stat_update.character_id, pc_id);
        assert_eq!(stat_update.character_name, "Test PC");
        assert_eq!(stat_update.stat_name, "STR");
        assert_eq!(stat_update.old_value, 10);
        assert_eq!(stat_update.new_value, 12);
        assert_eq!(stat_update.delta, 2);
    }

    #[tokio::test]
    async fn test_unknown_tool_returns_error() {
        let executor = mock_executor();
        let tool = ProposedTool {
            id: "call_unknown".to_string(),
            name: "unknown_tool".to_string(),
            description: "Unknown tool".to_string(),
            arguments: crate::game_tools::ToolArgs::Unknown,
        };

        let results = executor
            .execute_approved(&["call_unknown".to_string()], &[tool], None, None)
            .await;

        assert!(
            results.is_empty(),
            "Unknown tool should not return a result"
        );
    }

    #[tokio::test]
    async fn test_tool_not_in_proposed_is_skipped() {
        let executor = mock_executor();
        let tool = ProposedTool {
            id: "call_123".to_string(),
            name: "give_item".to_string(),
            description: "Give item".to_string(),
            arguments: crate::game_tools::ToolArgs::GiveItem {
                item_name: "Sword".to_string(),
                description: "Sharp blade".to_string(),
            },
        };

        // Approve a tool that doesn't exist in proposed_tools
        let results = executor
            .execute_approved(&["call_999".to_string()], &[tool], None, None)
            .await;

        assert!(results.is_empty());
    }
}
