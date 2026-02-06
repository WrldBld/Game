//! Neo4j implementation of WorkflowConfigRepo.

use async_trait::async_trait;
use neo4rs::query;

use crate::infrastructure::ports::{RepoError, WorkflowConfigRepo};
use crate::infrastructure::neo4j::Neo4jGraph;
use wrldbldr_shared::workflow::{WorkflowConfiguration, WorkflowSlot};

/// Neo4j implementation of WorkflowConfigRepo.
pub struct Neo4jWorkflowConfigRepo {
    graph: Neo4jGraph,
}

impl Neo4jWorkflowConfigRepo {
    /// Create a new Neo4jWorkflowConfigRepo.
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }

    /// Initialize the database schema (constraints and indexes).
    ///
    /// Should be called once at application startup.
    #[allow(dead_code)]
    pub async fn initialize_schema(&self) -> Result<(), RepoError> {
        // Create unique constraint on slot
        let constraint_query = query(
            "CREATE CONSTRAINT workflow_configuration_slot IF NOT EXISTS \
             FOR (wc:WorkflowConfiguration) REQUIRE wc.slot IS UNIQUE"
        );

        self.graph.execute(constraint_query).await.map_err(|e| {
            RepoError::database("initialize_schema", e.to_string())
        })?;

        Ok(())
    }
}

#[async_trait]
impl WorkflowConfigRepo for Neo4jWorkflowConfigRepo {
    async fn get_by_slot(
        &self,
        slot: WorkflowSlot,
    ) -> Result<Option<WorkflowConfiguration>, RepoError> {
        let query = query(
            "MATCH (wc:WorkflowConfiguration {slot: $slot}) \
             RETURN wc.id as id, \
                    wc.slot as slot, \
                    wc.name as name, \
                    wc.workflow_json as workflow_json, \
                    wc.prompt_mappings as prompt_mappings, \
                    wc.input_defaults as input_defaults, \
                    wc.locked_inputs as locked_inputs, \
                    wc.created_at as created_at, \
                    wc.updated_at as updated_at"
        )
        .param("slot", slot.as_str());

        let mut result = self.graph.execute(query).await.map_err(|e| {
            RepoError::database("get_by_slot", e.to_string())
        })?;

        if let Some(row) = result.next().await.map_err(|e| {
            RepoError::database("get_by_slot", e.to_string())
        })? {
            let config = row_to_workflow_config(row).map_err(|e| {
                RepoError::database("get_by_slot", e)
            })?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, config: &WorkflowConfiguration) -> Result<(), RepoError> {
        let prompt_mappings_json = serde_json::to_string(&config.prompt_mappings)
            .map_err(|e| RepoError::serialization(format!("Failed to serialize prompt_mappings: {}", e)))?;
        let input_defaults_json = serde_json::to_string(&config.input_defaults)
            .map_err(|e| RepoError::serialization(format!("Failed to serialize input_defaults: {}", e)))?;
        let locked_inputs_json = serde_json::to_string(&config.locked_inputs)
            .map_err(|e| RepoError::serialization(format!("Failed to serialize locked_inputs: {}", e)))?;

        let query = query(
            "MERGE (wc:WorkflowConfiguration {slot: $slot}) \
             SET wc.id = $id, \
                 wc.name = $name, \
                 wc.workflow_json = $workflow_json, \
                 wc.prompt_mappings = $prompt_mappings, \
                 wc.input_defaults = $input_defaults, \
                 wc.locked_inputs = $locked_inputs, \
                 wc.created_at = $created_at, \
                 wc.updated_at = $updated_at"
        )
        .param("slot", config.slot.as_str())
        .param("id", config.id.clone())
        .param("name", config.name.clone())
        .param("workflow_json", config.workflow_json.to_string())
        .param("prompt_mappings", prompt_mappings_json)
        .param("input_defaults", input_defaults_json)
        .param("locked_inputs", locked_inputs_json)
        .param("created_at", config.created_at.clone())
        .param("updated_at", config.updated_at.clone());

        self.graph.execute(query).await.map_err(|e| {
            RepoError::database("save", e.to_string())
        })?;

        Ok(())
    }

    async fn delete(&self, slot: WorkflowSlot) -> Result<(), RepoError> {
        let query = query(
            "MATCH (wc:WorkflowConfiguration {slot: $slot}) \
             DELETE wc"
        )
        .param("slot", slot.as_str());

        self.graph.execute(query).await.map_err(|e| {
            RepoError::database("delete", e.to_string())
        })?;

        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>, RepoError> {
        let query = query(
            "MATCH (wc:WorkflowConfiguration) \
             RETURN wc.id as id, \
                    wc.slot as slot, \
                    wc.name as name, \
                    wc.workflow_json as workflow_json, \
                    wc.prompt_mappings as prompt_mappings, \
                    wc.input_defaults as input_defaults, \
                    wc.locked_inputs as locked_inputs, \
                    wc.created_at as created_at, \
                    wc.updated_at as updated_at \
             ORDER BY wc.slot"
        );

        let mut result = self.graph.execute(query).await.map_err(|e| {
            RepoError::database("list_all", e.to_string())
        })?;

        let mut configs = Vec::new();

        while let Some(row) = result.next().await.map_err(|e| {
            RepoError::database("list_all", e.to_string())
        })? {
            let config = row_to_workflow_config(row).map_err(|e| {
                RepoError::database("list_all", e)
            })?;
            configs.push(config);
        }

        Ok(configs)
    }
}

/// Convert a Neo4j row to WorkflowConfiguration.
fn row_to_workflow_config(row: neo4rs::Row) -> Result<WorkflowConfiguration, String> {
    let id: String = row.get("id")
        .map_err(|e| format!("Missing id: {}", e))?;
    let slot_str: String = row.get("slot")
        .map_err(|e| format!("Missing slot: {}", e))?;
    let slot = slot_str.parse::<WorkflowSlot>()
        .map_err(|e| format!("Invalid slot: {}", e))?;
    let name: String = row.get("name")
        .map_err(|e| format!("Missing name: {}", e))?;
    let workflow_json_str: String = row.get("workflow_json")
        .map_err(|e| format!("Missing workflow_json: {}", e))?;
    let workflow_json: serde_json::Value = serde_json::from_str(&workflow_json_str)
        .map_err(|e| format!("Invalid workflow_json: {}", e))?;
    let prompt_mappings_str: String = row.get("prompt_mappings")
        .map_err(|e| format!("Missing prompt_mappings: {}", e))?;
    let prompt_mappings: Vec<wrldbldr_shared::workflow::PromptMapping> = serde_json::from_str(&prompt_mappings_str)
        .map_err(|e| format!("Invalid prompt_mappings: {}", e))?;
    let input_defaults_str: String = row.get("input_defaults")
        .map_err(|e| format!("Missing input_defaults: {}", e))?;
    let input_defaults: Vec<wrldbldr_shared::workflow::InputDefault> = serde_json::from_str(&input_defaults_str)
        .map_err(|e| format!("Invalid input_defaults: {}", e))?;
    let locked_inputs_str: String = row.get("locked_inputs")
        .map_err(|e| format!("Missing locked_inputs: {}", e))?;
    let locked_inputs: Vec<String> = serde_json::from_str(&locked_inputs_str)
        .map_err(|e| format!("Invalid locked_inputs: {}", e))?;
    let created_at: String = row.get("created_at")
        .map_err(|e| format!("Missing created_at: {}", e))?;
    let updated_at: String = row.get("updated_at")
        .map_err(|e| format!("Missing updated_at: {}", e))?;

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
