//! Neo4j interaction repository implementation.
//!
//! Interactions are stored as nodes and linked to scenes:
//! - `(InteractionTemplate)-[:BELONGS_TO_SCENE]->(Scene)`

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Row};
use wrldbldr_domain::{
    InteractionCondition, InteractionId, InteractionTarget, InteractionTemplate, InteractionType,
    SceneId,
};

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{InteractionRepo, RepoError};

/// Repository for Interaction operations.
pub struct Neo4jInteractionRepo {
    graph: Neo4jGraph,
}

impl Neo4jInteractionRepo {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }

    fn row_to_interaction(&self, row: Row) -> Result<InteractionTemplate, RepoError> {
        let node: neo4rs::Node = row.get("i").map_err(|e| RepoError::database("query", e))?;

        let id: InteractionId = parse_typed_id(&node, "id").map_err(|e| {
            RepoError::database("query", format!("Failed to parse InteractionId: {}", e))
        })?;
        let scene_id: SceneId = parse_typed_id(&node, "scene_id").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to parse scene_id for Interaction {}: {}", id, e),
            )
        })?;
        let name: String = node.get("name").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get 'name' for Interaction {}: {}", id, e),
            )
        })?;
        let prompt_hints: String = node.get_string_or("prompt_hints", "");
        let is_available = node.get_bool_or("is_available", true);
        let order_num = node.get_i64_or("order_num", 0);

        let interaction_type_str =
            node.get_optional_string("interaction_type")
                .ok_or_else(|| {
                    RepoError::database(
                        "query",
                        format!("Missing interaction_type for Interaction {}", id),
                    )
                })?;
        let interaction_type: InteractionType = serde_json::from_str(&interaction_type_str)
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Invalid interaction_type JSON for Interaction {}: {} (value: '{}')",
                        id, e, interaction_type_str
                    ),
                )
            })?;

        let target_str = node.get_optional_string("target").ok_or_else(|| {
            RepoError::database("query", format!("Missing target for Interaction {}", id))
        })?;
        let target: InteractionTarget = serde_json::from_str(&target_str).map_err(|e| {
            RepoError::database(
                "parse",
                format!(
                    "Invalid target JSON for Interaction {}: {} (value: '{}')",
                    id, e, target_str
                ),
            )
        })?;

        let allowed_tools_str = node.get_optional_string("allowed_tools").ok_or_else(|| {
            RepoError::database(
                "query",
                format!("Missing allowed_tools for Interaction {}", id),
            )
        })?;
        let allowed_tools: Vec<String> = serde_json::from_str(&allowed_tools_str).map_err(|e| {
            RepoError::database(
                "parse",
                format!(
                    "Invalid allowed_tools JSON for Interaction {}: {} (value: '{}')",
                    id, e, allowed_tools_str
                ),
            )
        })?;

        let conditions_str = node.get_optional_string("conditions").ok_or_else(|| {
            RepoError::database(
                "query",
                format!("Missing conditions for Interaction {}", id),
            )
        })?;
        let conditions: Vec<InteractionCondition> =
            serde_json::from_str(&conditions_str).map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Invalid conditions JSON for Interaction {}: {} (value: '{}')",
                        id, e, conditions_str
                    ),
                )
            })?;

        Ok(InteractionTemplate::from_storage(
            id,
            scene_id,
            name,
            interaction_type,
            target,
            prompt_hints,
            allowed_tools,
            conditions,
            is_available,
            order_num as u32,
        ))
    }
}

#[async_trait]
impl InteractionRepo for Neo4jInteractionRepo {
    async fn get(&self, id: InteractionId) -> Result<Option<InteractionTemplate>, RepoError> {
        let q =
            query("MATCH (i:InteractionTemplate {id: $id}) RETURN i").param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            Ok(Some(self.row_to_interaction(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, interaction: &InteractionTemplate) -> Result<(), RepoError> {
        let interaction_type_json = serde_json::to_string(interaction.interaction_type())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let target_json = serde_json::to_string(interaction.target())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let allowed_tools_json = serde_json::to_string(interaction.allowed_tools())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let conditions_json = serde_json::to_string(interaction.conditions())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        let q = query(
            "MATCH (s:Scene {id: $scene_id})
            MERGE (i:InteractionTemplate {id: $id})
            SET i.scene_id = $scene_id,
                i.name = $name,
                i.interaction_type = $interaction_type,
                i.target = $target,
                i.prompt_hints = $prompt_hints,
                i.allowed_tools = $allowed_tools,
                i.conditions = $conditions,
                i.is_available = $is_available,
                i.order_num = $order_num
            MERGE (i)-[:BELONGS_TO_SCENE]->(s)
            RETURN i.id as id",
        )
        .param("id", interaction.id().to_string())
        .param("scene_id", interaction.scene_id().to_string())
        .param("name", interaction.name().to_string())
        .param("interaction_type", interaction_type_json)
        .param("target", target_json)
        .param("prompt_hints", interaction.prompt_hints().to_string())
        .param("allowed_tools", allowed_tools_json)
        .param("conditions", conditions_json)
        .param("is_available", interaction.is_available())
        .param("order_num", interaction.order() as i64);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Saved interaction: {}", interaction.name());
        Ok(())
    }

    async fn delete(&self, id: InteractionId) -> Result<(), RepoError> {
        let q = query("MATCH (i:InteractionTemplate {id: $id}) DETACH DELETE i")
            .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        Ok(())
    }

    async fn list_for_scene(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<InteractionTemplate>, RepoError> {
        let q = query(
            "MATCH (s:Scene {id: $scene_id})<-[:BELONGS_TO_SCENE]-(i:InteractionTemplate)
            RETURN i
            ORDER BY i.order_num",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut interactions = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            interactions.push(self.row_to_interaction(row)?);
        }

        Ok(interactions)
    }
}
