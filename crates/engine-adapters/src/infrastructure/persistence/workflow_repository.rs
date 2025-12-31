//! Neo4j repository for workflow configurations

use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;

use super::neo4j_helpers::RowExt;
use crate::infrastructure::persistence::Neo4jConnection;
use wrldbldr_domain::entities::{InputDefault, PromptMapping, WorkflowConfiguration, WorkflowSlot};
use wrldbldr_domain::WorkflowConfigId;
use wrldbldr_engine_dto::persistence::{InputDefaultDto, PromptMappingDto};
use wrldbldr_engine_ports::outbound::{ClockPort, WorkflowRepositoryPort};

/// Repository for workflow configuration persistence
#[derive(Clone)]
pub struct Neo4jWorkflowRepository {
    connection: Neo4jConnection,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jWorkflowRepository {
    pub fn new(connection: Neo4jConnection, clock: Arc<dyn ClockPort>) -> Self {
        Self { connection, clock }
    }

    /// Create or update a workflow configuration
    pub async fn save(&self, config: &WorkflowConfiguration) -> Result<()> {
        let id = config.id.to_string();
        let slot = config.slot.as_str();
        let name = &config.name;
        let workflow_json = serde_json::to_string(&config.workflow_json)?;
        let prompt_mappings = serde_json::to_string(
            &config
                .prompt_mappings
                .iter()
                .cloned()
                .map(PromptMappingDto::from)
                .collect::<Vec<_>>(),
        )?;
        let input_defaults = serde_json::to_string(
            &config
                .input_defaults
                .iter()
                .cloned()
                .map(InputDefaultDto::from)
                .collect::<Vec<_>>(),
        )?;
        let locked_inputs = serde_json::to_string(&config.locked_inputs)?;
        let created_at = config.created_at.to_rfc3339();
        let updated_at = config.updated_at.to_rfc3339();

        let q = query(
            r#"
            MERGE (w:WorkflowConfiguration {slot: $slot})
            SET w.id = $id,
                w.name = $name,
                w.workflow_json = $workflow_json,
                w.prompt_mappings = $prompt_mappings,
                w.input_defaults = $input_defaults,
                w.locked_inputs = $locked_inputs,
                w.created_at = $created_at,
                w.updated_at = $updated_at
            RETURN w
            "#,
        )
        .param("id", id)
        .param("slot", slot)
        .param("name", name.clone())
        .param("workflow_json", workflow_json)
        .param("prompt_mappings", prompt_mappings)
        .param("input_defaults", input_defaults)
        .param("locked_inputs", locked_inputs)
        .param("created_at", created_at)
        .param("updated_at", updated_at);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get a workflow configuration by slot
    pub async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>> {
        let slot_str = slot.as_str();

        let q = query(
            r#"
            MATCH (w:WorkflowConfiguration {slot: $slot})
            RETURN w.id AS id,
                   w.slot AS slot,
                   w.name AS name,
                   w.workflow_json AS workflow_json,
                   w.prompt_mappings AS prompt_mappings,
                   w.input_defaults AS input_defaults,
                   w.locked_inputs AS locked_inputs,
                   w.created_at AS created_at,
                   w.updated_at AS updated_at
            "#,
        )
        .param("slot", slot_str);

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let config = self.row_to_config(&row)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// Get all workflow configurations
    pub async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>> {
        let q = query(
            r#"
            MATCH (w:WorkflowConfiguration)
            RETURN w.id AS id,
                   w.slot AS slot,
                   w.name AS name,
                   w.workflow_json AS workflow_json,
                   w.prompt_mappings AS prompt_mappings,
                   w.input_defaults AS input_defaults,
                   w.locked_inputs AS locked_inputs,
                   w.created_at AS created_at,
                   w.updated_at AS updated_at
            ORDER BY w.slot
            "#,
        );

        let mut result = self.connection.graph().execute(q).await?;
        let mut configs = Vec::new();

        while let Some(row) = result.next().await? {
            let config = self.row_to_config(&row)?;
            configs.push(config);
        }

        Ok(configs)
    }

    /// Delete a workflow configuration by slot
    pub async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool> {
        let slot_str = slot.as_str();

        let q = query(
            r#"
            MATCH (w:WorkflowConfiguration {slot: $slot})
            DELETE w
            RETURN count(w) AS deleted
            "#,
        )
        .param("slot", slot_str);

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted").unwrap_or(0);
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    /// Check which slots have configurations
    pub async fn get_configured_slots(&self) -> Result<Vec<WorkflowSlot>> {
        let q = query(
            r#"
            MATCH (w:WorkflowConfiguration)
            RETURN w.slot AS slot
            ORDER BY w.slot
            "#,
        );

        let mut result = self.connection.graph().execute(q).await?;
        let mut slots = Vec::new();

        while let Some(row) = result.next().await? {
            let slot_str: String = row.get("slot").unwrap_or_default();
            if let Ok(slot) = WorkflowSlot::from_str(&slot_str) {
                slots.push(slot);
            }
        }

        Ok(slots)
    }

    /// Convert a Neo4j row to a WorkflowConfiguration
    fn row_to_config(&self, row: &neo4rs::Row) -> Result<WorkflowConfiguration> {
        let id = row
            .get_uuid("id")
            .map(WorkflowConfigId::from)
            .unwrap_or_else(|_| WorkflowConfigId::new());

        let slot_str = row.get_string_or("slot", "");
        let slot = WorkflowSlot::from_str(&slot_str)
            .map_err(|e| anyhow::anyhow!("Invalid workflow slot: {}", e))?;

        let name = row.get_string_or("name", "");

        let workflow_json: serde_json::Value = row.get_json_or_default("workflow_json");

        let prompt_mappings: Vec<PromptMapping> = row
            .get_json_or_default::<Vec<PromptMappingDto>>("prompt_mappings")
            .into_iter()
            .map(Into::into)
            .collect();

        let input_defaults: Vec<InputDefault> = row
            .get_json_or_default::<Vec<InputDefaultDto>>("input_defaults")
            .into_iter()
            .map(Into::into)
            .collect();

        let locked_inputs: Vec<String> = row.get_json_or_default("locked_inputs");

        let created_at = row.get_datetime_or("created_at", self.clock.now());
        let updated_at = row.get_datetime_or("updated_at", self.clock.now());

        Ok(WorkflowConfiguration {
            id,
            slot,
            name,
            workflow_json,
            prompt_mappings,
            input_defaults,
            locked_inputs,
            created_at,
            updated_at,
        })
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

#[async_trait]
impl WorkflowRepositoryPort for Neo4jWorkflowRepository {
    async fn save(&self, config: &WorkflowConfiguration) -> Result<()> {
        self.save(config).await
    }

    async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>> {
        self.get_by_slot(slot).await
    }

    async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool> {
        self.delete_by_slot(slot).await
    }

    async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>> {
        self.list_all().await
    }
}
