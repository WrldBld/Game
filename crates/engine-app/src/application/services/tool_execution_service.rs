//! Tool Execution Service - Executes approved tool calls to modify game state
//!
//! This service handles the execution of game tools that have been approved by the DM.
//! It modifies game state and logs actions.
//!
//! TODO: This service was refactored to remove SessionManagementPort dependency.
//! Tool execution now returns results that callers can use to update state and
//! broadcast messages via WorldConnectionPort.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use wrldbldr_domain::value_objects::{ChangeAmount, GameTool, InfoImportance, RelationshipChange};

/// Result of executing a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    /// Whether the tool executed successfully
    pub success: bool,
    /// Human-readable description of what happened
    pub description: String,
    /// List of state changes that occurred (for broadcasting)
    pub state_changes: Vec<StateChange>,
}

/// Individual state changes caused by tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StateChange {
    /// An item was added to a character's inventory
    ItemAdded {
        character: String,
        item: String,
    },
    /// Information was revealed to the player
    InfoRevealed {
        info: String,
    },
    /// A relationship sentiment was changed
    RelationshipChanged {
        from: String,
        to: String,
        delta: i32,
    },
    /// An event was triggered
    EventTriggered {
        name: String,
    },
    /// An NPC's motivation was modified
    NpcMotivationChanged {
        npc_id: String,
        motivation_type: String,
        new_value: String,
        reason: String,
    },
    /// A character's description was updated
    CharacterDescriptionUpdated {
        character_id: String,
        change_type: String,
        description: String,
    },
    /// An NPC's opinion of a PC changed
    NpcOpinionChanged {
        npc_id: String,
        target_pc_id: String,
        opinion_change: String,
        reason: String,
    },
    /// An item was transferred between characters
    ItemTransferred {
        from_id: String,
        to_id: String,
        item_name: String,
    },
    /// A condition was added to a character
    ConditionAdded {
        character_id: String,
        condition_name: String,
        description: String,
        duration: Option<String>,
    },
    /// A condition was removed from a character
    ConditionRemoved {
        character_id: String,
        condition_name: String,
    },
    /// A character's stat was updated
    CharacterStatUpdated {
        character_id: String,
        stat_name: String,
        delta: i32,
    },
}

/// Errors that can occur during tool execution
#[derive(Debug, thiserror::Error)]
pub enum ToolExecutionError {
    /// Target character was not found in the session
    #[error("Character not found: {0}")]
    CharacterNotFound(String),

    /// Invalid tool parameters
    #[error("Invalid tool parameters: {0}")]
    InvalidParameters(String),

    /// Internal error during execution
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

/// Service for executing approved game tools
pub struct ToolExecutionService;

impl ToolExecutionService {
    /// Create a new tool execution service
    pub fn new() -> Self {
        Self
    }

    /// Execute an approved tool call
    ///
    /// # Arguments
    ///
    /// * `tool` - The game tool to execute
    ///
    /// # Returns
    ///
    /// A `ToolExecutionResult` describing what happened, or a `ToolExecutionError` if execution failed.
    /// The caller is responsible for logging to conversation history and broadcasting state changes.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use wrldbldr_engine::domain::GameTool;
    /// use wrldbldr_engine::application::services::ToolExecutionService;
    ///
    /// let service = ToolExecutionService::new();
    /// let tool = GameTool::GiveItem {
    ///     item_name: "Mysterious Key".to_string(),
    ///     description: "An ornate bronze key".to_string(),
    /// };
    ///
    /// let result = service.execute_tool(&tool).await?;
    /// assert!(result.success);
    /// // Caller should log result.description to conversation history
    /// ```
    #[instrument(skip(self))]
    pub async fn execute_tool(
        &self,
        tool: &GameTool,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        match tool {
            GameTool::GiveItem { item_name, description } => {
                self.execute_give_item(item_name, description).await
            }
            GameTool::RevealInfo {
                info_type,
                content,
                importance,
            } => {
                self.execute_reveal_info(info_type, content, importance).await
            }
            GameTool::ChangeRelationship { change, amount, reason } => {
                self.execute_change_relationship(change, amount, reason).await
            }
            GameTool::TriggerEvent {
                event_type,
                description,
            } => {
                self.execute_trigger_event(event_type, description).await
            }
            GameTool::ModifyNpcMotivation {
                npc_id,
                motivation_type,
                new_value,
                reason,
            } => {
                self.execute_modify_npc_motivation(npc_id, motivation_type, new_value, reason).await
            }
            GameTool::ModifyCharacterDescription {
                character_id,
                change_type,
                description,
            } => {
                self.execute_modify_character_description(character_id, change_type, description).await
            }
            GameTool::ModifyNpcOpinion {
                npc_id,
                target_pc_id,
                opinion_change,
                reason,
            } => {
                self.execute_modify_npc_opinion(npc_id, target_pc_id, opinion_change, reason).await
            }
            GameTool::TransferItem {
                from_id,
                to_id,
                item_name,
            } => {
                self.execute_transfer_item(from_id, to_id, item_name).await
            }
            GameTool::AddCondition {
                character_id,
                condition_name,
                description,
                duration,
            } => {
                self.execute_add_condition(character_id, condition_name, description, duration.as_deref()).await
            }
            GameTool::RemoveCondition {
                character_id,
                condition_name,
            } => {
                self.execute_remove_condition(character_id, condition_name).await
            }
            GameTool::UpdateCharacterStat {
                character_id,
                stat_name,
                delta,
            } => {
                self.execute_update_character_stat(character_id, stat_name, *delta).await
            }
        }
    }

    /// Execute GiveItem tool - adds item to character inventory
    #[instrument(skip(self))]
    async fn execute_give_item(
        &self,
        item_name: &str,
        description: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "Gave '{}' to player: {}",
            item_name, description
        );

        debug!("Item transfer: {}", description_msg);

        let state_change = StateChange::ItemAdded {
            character: "Player".to_string(),
            item: item_name.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute RevealInfo tool - marks information as known to player
    #[instrument(skip(self))]
    async fn execute_reveal_info(
        &self,
        info_type: &str,
        content: &str,
        importance: &InfoImportance,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "Revealed {} {} information",
            importance.as_str(),
            info_type
        );

        debug!("Info revealed: {} - {}", info_type, content);

        let state_change = StateChange::InfoRevealed {
            info: format!("[{}] {}", info_type, content),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute ChangeRelationship tool - updates relationship sentiment
    #[instrument(skip(self))]
    async fn execute_change_relationship(
        &self,
        change: &RelationshipChange,
        amount: &ChangeAmount,
        reason: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        // Calculate sentiment delta based on amount
        let delta = match amount {
            ChangeAmount::Slight => 10,
            ChangeAmount::Moderate => 25,
            ChangeAmount::Significant => 50,
        };

        // Apply sign based on improvement/worsening
        let signed_delta = match change {
            RelationshipChange::Improve => delta,
            RelationshipChange::Worsen => -delta,
        };

        let change_str = match change {
            RelationshipChange::Improve => "Improve",
            RelationshipChange::Worsen => "Worsen",
        };

        let description_msg = format!(
            "{} relationship {} with player (reason: {})",
            change_str,
            amount.as_str(),
            reason
        );

        debug!(
            "Relationship change: {} (delta: {})",
            description_msg, signed_delta
        );

        let state_change = StateChange::RelationshipChanged {
            from: "NPC".to_string(),
            to: "Player".to_string(),
            delta: signed_delta,
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute TriggerEvent tool - logs and triggers an event
    #[instrument(skip(self))]
    async fn execute_trigger_event(
        &self,
        event_type: &str,
        description: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!("Triggered {} event: {}", event_type, description);

        info!("Event triggered: {}", description_msg);

        let state_change = StateChange::EventTriggered {
            name: format!("{}: {}", event_type, description),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute ModifyNpcMotivation tool - updates an NPC's motivation
    #[instrument(skip(self))]
    async fn execute_modify_npc_motivation(
        &self,
        npc_id: &str,
        motivation_type: &str,
        new_value: &str,
        reason: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "NPC {} motivation '{}' changed to '{}' ({})",
            npc_id, motivation_type, new_value, reason
        );

        info!("NPC motivation changed: {}", description_msg);

        let state_change = StateChange::NpcMotivationChanged {
            npc_id: npc_id.to_string(),
            motivation_type: motivation_type.to_string(),
            new_value: new_value.to_string(),
            reason: reason.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute ModifyCharacterDescription tool - updates a character's description
    #[instrument(skip(self))]
    async fn execute_modify_character_description(
        &self,
        character_id: &str,
        change_type: &str,
        description: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "Character {} {} updated: {}",
            character_id, change_type, description
        );

        info!("Character description updated: {}", description_msg);

        let state_change = StateChange::CharacterDescriptionUpdated {
            character_id: character_id.to_string(),
            change_type: change_type.to_string(),
            description: description.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute ModifyNpcOpinion tool - changes an NPC's opinion of a PC
    #[instrument(skip(self))]
    async fn execute_modify_npc_opinion(
        &self,
        npc_id: &str,
        target_pc_id: &str,
        opinion_change: &str,
        reason: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "NPC {} now {} toward PC {} ({})",
            npc_id, opinion_change, target_pc_id, reason
        );

        info!("NPC opinion changed: {}", description_msg);

        let state_change = StateChange::NpcOpinionChanged {
            npc_id: npc_id.to_string(),
            target_pc_id: target_pc_id.to_string(),
            opinion_change: opinion_change.to_string(),
            reason: reason.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute TransferItem tool - transfers an item between characters
    #[instrument(skip(self))]
    async fn execute_transfer_item(
        &self,
        from_id: &str,
        to_id: &str,
        item_name: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "'{}' transferred from {} to {}",
            item_name, from_id, to_id
        );

        info!("Item transferred: {}", description_msg);

        let state_change = StateChange::ItemTransferred {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            item_name: item_name.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute AddCondition tool - adds a condition to a character
    #[instrument(skip(self))]
    async fn execute_add_condition(
        &self,
        character_id: &str,
        condition_name: &str,
        description: &str,
        duration: Option<&str>,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let dur_str = duration.unwrap_or("permanent");
        let description_msg = format!(
            "Condition '{}' added to {} ({}): {}",
            condition_name, character_id, dur_str, description
        );

        info!("Condition added: {}", description_msg);

        let state_change = StateChange::ConditionAdded {
            character_id: character_id.to_string(),
            condition_name: condition_name.to_string(),
            description: description.to_string(),
            duration: duration.map(|s| s.to_string()),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute RemoveCondition tool - removes a condition from a character
    #[instrument(skip(self))]
    async fn execute_remove_condition(
        &self,
        character_id: &str,
        condition_name: &str,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let description_msg = format!(
            "Condition '{}' removed from {}",
            condition_name, character_id
        );

        info!("Condition removed: {}", description_msg);

        let state_change = StateChange::ConditionRemoved {
            character_id: character_id.to_string(),
            condition_name: condition_name.to_string(),
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }

    /// Execute UpdateCharacterStat tool - updates a character's stat
    #[instrument(skip(self))]
    async fn execute_update_character_stat(
        &self,
        character_id: &str,
        stat_name: &str,
        delta: i32,
    ) -> Result<ToolExecutionResult, ToolExecutionError> {
        let change_str = if delta >= 0 {
            format!("+{}", delta)
        } else {
            format!("{}", delta)
        };

        let description_msg = format!(
            "{} {} changed by {}",
            character_id, stat_name, change_str
        );

        info!("Character stat updated: {}", description_msg);

        let state_change = StateChange::CharacterStatUpdated {
            character_id: character_id.to_string(),
            stat_name: stat_name.to_string(),
            delta,
        };

        Ok(ToolExecutionResult {
            success: true,
            description: description_msg,
            state_changes: vec![state_change],
        })
    }
}

impl Default for ToolExecutionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_give_item() {
        let service = ToolExecutionService::new();
        let tool = GameTool::GiveItem {
            item_name: "Mysterious Key".to_string(),
            description: "An ornate bronze key".to_string(),
        };

        let result = service.execute_tool(&tool).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("Mysterious Key"));
        assert_eq!(result.state_changes.len(), 1);
        assert!(matches!(
            &result.state_changes[0],
            StateChange::ItemAdded { item, .. } if item == "Mysterious Key"
        ));
    }

    #[tokio::test]
    async fn test_execute_reveal_info_minor() {
        let service = ToolExecutionService::new();
        let tool = GameTool::RevealInfo {
            info_type: "lore".to_string(),
            content: "The ancient civilization was destroyed".to_string(),
            importance: InfoImportance::Minor,
        };

        let result = service.execute_tool(&tool).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("minor"));
        assert_eq!(result.state_changes.len(), 1);
    }

    #[tokio::test]
    async fn test_execute_reveal_info_critical() {
        let service = ToolExecutionService::new();
        let tool = GameTool::RevealInfo {
            info_type: "quest".to_string(),
            content: "Your father is alive!".to_string(),
            importance: InfoImportance::Critical,
        };

        let result = service.execute_tool(&tool).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("critical"));
        assert!(matches!(
            &result.state_changes[0],
            StateChange::InfoRevealed { info } if info.contains("Your father is alive!")
        ));
    }

    #[tokio::test]
    async fn test_execute_relationship_improve_slight() {
        let service = ToolExecutionService::new();
        let tool = GameTool::ChangeRelationship {
            change: RelationshipChange::Improve,
            amount: ChangeAmount::Slight,
            reason: "Good conversation".to_string(),
        };

        let result = service.execute_tool(&tool).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("Improve"));
        assert!(result.description.contains("slight"));
        assert_eq!(result.state_changes.len(), 1);

        if let StateChange::RelationshipChanged { delta, .. } = &result.state_changes[0] {
            assert_eq!(*delta, 10);
        } else {
            panic!("Expected RelationshipChanged");
        }
    }

    #[tokio::test]
    async fn test_execute_relationship_improve_moderate() {
        let service = ToolExecutionService::new();
        let tool = GameTool::ChangeRelationship {
            change: RelationshipChange::Improve,
            amount: ChangeAmount::Moderate,
            reason: "Great help".to_string(),
        };

        let result = service.execute_tool(&tool).await.unwrap();

        assert!(result.success);

        if let StateChange::RelationshipChanged { delta, .. } = &result.state_changes[0] {
            assert_eq!(*delta, 25);
        } else {
            panic!("Expected RelationshipChanged");
        }
    }

    #[tokio::test]
    async fn test_execute_relationship_improve_significant() {
        let service = ToolExecutionService::new();
        let tool = GameTool::ChangeRelationship {
            change: RelationshipChange::Improve,
            amount: ChangeAmount::Significant,
            reason: "Saved their life".to_string(),
        };

        let result = service.execute_tool(&tool).await.unwrap();

        assert!(result.success);

        if let StateChange::RelationshipChanged { delta, .. } = &result.state_changes[0] {
            assert_eq!(*delta, 50);
        } else {
            panic!("Expected RelationshipChanged");
        }
    }

    #[tokio::test]
    async fn test_execute_relationship_worsen() {
        let service = ToolExecutionService::new();
        let tool = GameTool::ChangeRelationship {
            change: RelationshipChange::Worsen,
            amount: ChangeAmount::Significant,
            reason: "Betrayal".to_string(),
        };

        let result = service.execute_tool(&tool).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("Worsen"));

        if let StateChange::RelationshipChanged { delta, .. } = &result.state_changes[0] {
            assert_eq!(*delta, -50);
        } else {
            panic!("Expected RelationshipChanged");
        }
    }

    #[tokio::test]
    async fn test_execute_trigger_event() {
        let service = ToolExecutionService::new();
        let tool = GameTool::TriggerEvent {
            event_type: "combat".to_string(),
            description: "A group of bandits appears!".to_string(),
        };

        let result = service.execute_tool(&tool).await.unwrap();

        assert!(result.success);
        assert!(result.description.contains("combat"));
        assert!(result.description.contains("bandits"));
        assert_eq!(result.state_changes.len(), 1);
        assert!(matches!(
            &result.state_changes[0],
            StateChange::EventTriggered { .. }
        ));
    }

    #[tokio::test]
    async fn test_multiple_tools_sequence() {
        let service = ToolExecutionService::new();

        // Execute multiple tools in sequence
        let tool1 = GameTool::GiveItem {
            item_name: "Sword".to_string(),
            description: "A sharp blade".to_string(),
        };
        let result1 = service.execute_tool(&tool1).await.unwrap();
        assert!(result1.success);

        let tool2 = GameTool::RevealInfo {
            info_type: "quest".to_string(),
            content: "Find the dragon".to_string(),
            importance: InfoImportance::Major,
        };
        let result2 = service.execute_tool(&tool2).await.unwrap();
        assert!(result2.success);

        let tool3 = GameTool::ChangeRelationship {
            change: RelationshipChange::Improve,
            amount: ChangeAmount::Moderate,
            reason: "Helping out".to_string(),
        };
        let result3 = service.execute_tool(&tool3).await.unwrap();
        assert!(result3.success);
    }
}
