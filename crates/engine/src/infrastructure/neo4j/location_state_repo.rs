//! Neo4j location state repository implementation.

use std::sync::Arc;

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Row};
use wrldbldr_domain::{
    value_objects::{Description, StateName},
    *,
};

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ClockPort, LocationStateRepo, RepoError};

/// Repository for LocationState operations.
pub struct Neo4jLocationStateRepo {
    graph: Neo4jGraph,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jLocationStateRepo {
    pub fn new(graph: Neo4jGraph, clock: Arc<dyn ClockPort>) -> Self {
        Self { graph, clock }
    }

    fn row_to_state(&self, row: Row) -> Result<LocationState, RepoError> {
        let node: neo4rs::Node = row.get("s").map_err(|e| RepoError::database("query", e))?;
        let fallback = self.clock.now();

        let id: LocationStateId = parse_typed_id(&node, "id").map_err(|e| {
            RepoError::database("query", format!("Failed to parse LocationStateId: {}", e))
        })?;
        let location_id: LocationId = parse_typed_id(&node, "location_id").map_err(|e| {
            RepoError::database(
                "query",
                format!(
                    "Failed to parse location_id for LocationState {}: {}",
                    id, e
                ),
            )
        })?;
        let world_id: WorldId = parse_typed_id(&node, "world_id").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to parse world_id for LocationState {}: {}", id, e),
            )
        })?;
        let name_str: String = node.get("name").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get 'name' for LocationState {}: {}", id, e),
            )
        })?;
        let name = StateName::new(&name_str).map_err(|e| {
            RepoError::database(
                "parse",
                format!(
                    "Failed to parse name '{}' for LocationState {}: {}",
                    name_str, id, e
                ),
            )
        })?;
        let description =
            Description::new(node.get_string_or("description", "")).unwrap_or_default();

        let backdrop_override: Option<AssetPath> = node
            .get_optional_string("backdrop_override")
            .map(AssetPath::new)
            .transpose()
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Failed to parse backdrop_override for LocationState {}: {}",
                        id, e
                    ),
                )
            })?;
        let atmosphere_override: Option<Atmosphere> = node
            .get_optional_string("atmosphere_override")
            .map(Atmosphere::new)
            .transpose()
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Failed to parse atmosphere_override for LocationState {}: {}",
                        id, e
                    ),
                )
            })?;
        let ambient_sound: Option<AssetPath> = node
            .get_optional_string("ambient_sound")
            .map(AssetPath::new)
            .transpose()
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Failed to parse ambient_sound for LocationState {}: {}",
                        id, e
                    ),
                )
            })?;
        let map_overlay: Option<AssetPath> = node
            .get_optional_string("map_overlay")
            .map(AssetPath::new)
            .transpose()
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Failed to parse map_overlay for LocationState {}: {}",
                        id, e
                    ),
                )
            })?;

        // Parse activation rules from JSON - fail-fast on invalid JSON
        let activation_rules_str =
            node.get_optional_string("activation_rules")
                .ok_or_else(|| {
                    RepoError::database(
                        "query",
                        format!("Missing activation_rules for LocationState {}", id),
                    )
                })?;
        let activation_rules: Vec<ActivationRule> = serde_json::from_str(&activation_rules_str)
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Invalid activation_rules JSON for LocationState {}: {} (value: '{}')",
                        id, e, activation_rules_str
                    ),
                )
            })?;

        let activation_logic_str =
            node.get_optional_string("activation_logic")
                .ok_or_else(|| {
                    RepoError::database(
                        "query",
                        format!("Missing activation_logic for LocationState {}", id),
                    )
                })?;
        let activation_logic: ActivationLogic = serde_json::from_str(&activation_logic_str)
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Invalid ActivationLogic JSON for LocationState {}: {} (value: '{}')",
                        id, e, activation_logic_str
                    ),
                )
            })?;

        let priority: i32 = node.get_i64_or("priority", 0) as i32;
        let is_default: bool = node.get_bool_or("is_default", false);
        let generation_prompt: Option<String> = node.get_optional_string("generation_prompt");
        let workflow_id: Option<String> = node.get_optional_string("workflow_id");
        let created_at = node.get_datetime_or("created_at", fallback);
        let updated_at = node.get_datetime_or("updated_at", fallback);

        Ok(LocationState::from_storage(
            id,
            location_id,
            world_id,
            name,
            description,
            backdrop_override,
            atmosphere_override,
            ambient_sound,
            map_overlay,
            activation_rules,
            activation_logic,
            priority,
            is_default,
            generation_prompt,
            workflow_id,
            created_at,
            updated_at,
        ))
    }
}

#[async_trait]
impl LocationStateRepo for Neo4jLocationStateRepo {
    async fn get(&self, id: LocationStateId) -> Result<Option<LocationState>, RepoError> {
        let q = query("MATCH (s:LocationState {id: $id}) RETURN s").param("id", id.to_string());

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
            Ok(Some(self.row_to_state(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, state: &LocationState) -> Result<(), RepoError> {
        let activation_rules_json = serde_json::to_string(state.activation_rules())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let activation_logic_json = serde_json::to_string(&state.activation_logic())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        let q = query(
            "MERGE (s:LocationState {id: $id})
            SET s.location_id = $location_id,
                s.world_id = $world_id,
                s.name = $name,
                s.description = $description,
                s.backdrop_override = $backdrop_override,
                s.atmosphere_override = $atmosphere_override,
                s.ambient_sound = $ambient_sound,
                s.map_overlay = $map_overlay,
                s.activation_rules = $activation_rules,
                s.activation_logic = $activation_logic,
                s.priority = $priority,
                s.is_default = $is_default,
                s.generation_prompt = $generation_prompt,
                s.workflow_id = $workflow_id,
                s.created_at = $created_at,
                s.updated_at = $updated_at
            WITH s
            MATCH (loc:Location {id: $location_id})
            MERGE (loc)-[:HAS_STATE]->(s)
            RETURN s.id as id",
        )
        .param("id", state.id().to_string())
        .param("location_id", state.location_id().to_string())
        .param("world_id", state.world_id().to_string())
        .param("name", state.name().to_string())
        .param("description", state.description().to_string())
        .param(
            "backdrop_override",
            state
                .backdrop_override()
                .map(|p| p.to_string())
                .unwrap_or_default(),
        )
        .param(
            "atmosphere_override",
            state
                .atmosphere_override()
                .map(|a| a.as_str().to_string())
                .unwrap_or_default(),
        )
        .param(
            "ambient_sound",
            state
                .ambient_sound()
                .map(|p| p.to_string())
                .unwrap_or_default(),
        )
        .param(
            "map_overlay",
            state
                .map_overlay()
                .map(|p| p.to_string())
                .unwrap_or_default(),
        )
        .param("activation_rules", activation_rules_json)
        .param("activation_logic", activation_logic_json)
        .param("priority", state.priority() as i64)
        .param("is_default", state.is_default())
        .param(
            "generation_prompt",
            state.generation_prompt().unwrap_or_default().to_string(),
        )
        .param("workflow_id", state.workflow_id().unwrap_or_default().to_string())
        .param("created_at", state.created_at().to_rfc3339())
        .param("updated_at", state.updated_at().to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Saved location state: {}", state.name());
        Ok(())
    }

    async fn delete(&self, id: LocationStateId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (s:LocationState {id: $id})
            DETACH DELETE s",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Deleted location state: {}", id);
        Ok(())
    }

    async fn list_for_location(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<LocationState>, RepoError> {
        let q = query(
            "MATCH (s:LocationState {location_id: $location_id})
            RETURN s ORDER BY s.priority DESC, s.name",
        )
        .param("location_id", location_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut states = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            states.push(self.row_to_state(row)?);
        }

        Ok(states)
    }

    async fn get_default(
        &self,
        location_id: LocationId,
    ) -> Result<Option<LocationState>, RepoError> {
        let q = query(
            "MATCH (s:LocationState {location_id: $location_id, is_default: true})
            RETURN s LIMIT 1",
        )
        .param("location_id", location_id.to_string());

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
            Ok(Some(self.row_to_state(row)?))
        } else {
            Ok(None)
        }
    }

    async fn set_active(
        &self,
        location_id: LocationId,
        state_id: LocationStateId,
    ) -> Result<(), RepoError> {
        // Match location and target state FIRST to ensure they exist,
        // then delete old relationship and create new one atomically.
        // This prevents leaving the location without an active state if the target doesn't exist.
        let q = query(
            "MATCH (loc:Location {id: $location_id})
            MATCH (s:LocationState {id: $state_id})
            OPTIONAL MATCH (loc)-[old:ACTIVE_STATE]->(:LocationState)
            DELETE old
            CREATE (loc)-[:ACTIVE_STATE]->(s)
            RETURN loc.id as location_id",
        )
        .param("location_id", location_id.to_string())
        .param("state_id", state_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Check if the query matched anything (location and state both exist)
        if result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
            .is_none()
        {
            tracing::warn!(
                location_id = %location_id,
                state_id = %state_id,
                "Cannot set active state: location or state not found"
            );
            return Err(RepoError::not_found(
                "LocationState",
                format!("location:{}/state:{}", location_id, state_id),
            ));
        }

        tracing::debug!(
            "Set active location state {} for location {}",
            state_id,
            location_id
        );
        Ok(())
    }

    async fn get_active(
        &self,
        location_id: LocationId,
    ) -> Result<Option<LocationState>, RepoError> {
        let q = query(
            "MATCH (loc:Location {id: $location_id})-[:ACTIVE_STATE]->(s:LocationState)
            RETURN s",
        )
        .param("location_id", location_id.to_string());

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
            Ok(Some(self.row_to_state(row)?))
        } else {
            Ok(None)
        }
    }

    async fn clear_active(&self, location_id: LocationId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (loc:Location {id: $location_id})-[r:ACTIVE_STATE]->(:LocationState)
            DELETE r",
        )
        .param("location_id", location_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Cleared active location state for location {}", location_id);
        Ok(())
    }
}
