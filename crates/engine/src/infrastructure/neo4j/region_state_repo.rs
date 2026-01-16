//! Neo4j region state repository implementation.

use std::sync::Arc;

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Row};
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ClockPort, RegionStateRepo, RepoError};

/// Repository for RegionState operations.
pub struct Neo4jRegionStateRepo {
    graph: Neo4jGraph,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jRegionStateRepo {
    pub fn new(graph: Neo4jGraph, clock: Arc<dyn ClockPort>) -> Self {
        Self { graph, clock }
    }

    fn row_to_state(&self, row: Row) -> Result<RegionState, RepoError> {
        let node: neo4rs::Node = row.get("s").map_err(|e| RepoError::database("query", e))?;
        let fallback = self.clock.now();

        let id: RegionStateId =
            parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
        let region_id: RegionId =
            parse_typed_id(&node, "region_id").map_err(|e| RepoError::database("query", e))?;
        let location_id: LocationId =
            parse_typed_id(&node, "location_id").map_err(|e| RepoError::database("query", e))?;
        let world_id: WorldId =
            parse_typed_id(&node, "world_id").map_err(|e| RepoError::database("query", e))?;
        let name: String = node
            .get("name")
            .map_err(|e| RepoError::database("query", e))?;
        let description: String = node.get_string_or("description", "");

        let backdrop_override: Option<String> = node.get_optional_string("backdrop_override");
        let atmosphere_override: Option<String> = node.get_optional_string("atmosphere_override");
        let ambient_sound: Option<String> = node.get_optional_string("ambient_sound");

        // Parse activation rules from JSON
        let activation_rules: Vec<ActivationRule> = node
            .get_optional_string("activation_rules")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let activation_logic: ActivationLogic = node
            .get_optional_string("activation_logic")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(ActivationLogic::All);

        let priority: i32 = node.get_i64_or("priority", 0) as i32;
        let is_default: bool = node.get_bool_or("is_default", false);
        let created_at = node.get_datetime_or("created_at", fallback);
        let updated_at = node.get_datetime_or("updated_at", fallback);

        Ok(RegionState::from_parts(
            id,
            region_id,
            location_id,
            world_id,
            name,
            description,
            backdrop_override,
            atmosphere_override,
            ambient_sound,
            activation_rules,
            activation_logic,
            priority,
            is_default,
            created_at,
            updated_at,
        ))
    }
}

#[async_trait]
impl RegionStateRepo for Neo4jRegionStateRepo {
    async fn get(&self, id: RegionStateId) -> Result<Option<RegionState>, RepoError> {
        let q = query("MATCH (s:RegionState {id: $id}) RETURN s").param("id", id.to_string());

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

    async fn save(&self, state: &RegionState) -> Result<(), RepoError> {
        let activation_rules_json = serde_json::to_string(state.activation_rules())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let activation_logic_json = serde_json::to_string(&state.activation_logic())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        let q = query(
            "MERGE (s:RegionState {id: $id})
            SET s.region_id = $region_id,
                s.location_id = $location_id,
                s.world_id = $world_id,
                s.name = $name,
                s.description = $description,
                s.backdrop_override = $backdrop_override,
                s.atmosphere_override = $atmosphere_override,
                s.ambient_sound = $ambient_sound,
                s.activation_rules = $activation_rules,
                s.activation_logic = $activation_logic,
                s.priority = $priority,
                s.is_default = $is_default,
                s.created_at = $created_at,
                s.updated_at = $updated_at
            WITH s
            MATCH (r:Region {id: $region_id})
            MERGE (r)-[:HAS_STATE]->(s)
            RETURN s.id as id",
        )
        .param("id", state.id().to_string())
        .param("region_id", state.region_id().to_string())
        .param("location_id", state.location_id().to_string())
        .param("world_id", state.world_id().to_string())
        .param("name", state.name().to_string())
        .param("description", state.description().to_string())
        .param(
            "backdrop_override",
            state.backdrop_override().unwrap_or_default().to_string(),
        )
        .param(
            "atmosphere_override",
            state.atmosphere_override().unwrap_or_default().to_string(),
        )
        .param(
            "ambient_sound",
            state.ambient_sound().unwrap_or_default().to_string(),
        )
        .param("activation_rules", activation_rules_json)
        .param("activation_logic", activation_logic_json)
        .param("priority", state.priority() as i64)
        .param("is_default", state.is_default())
        .param("created_at", state.created_at().to_rfc3339())
        .param("updated_at", state.updated_at().to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Saved region state: {}", state.name());
        Ok(())
    }

    async fn delete(&self, id: RegionStateId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (s:RegionState {id: $id})
            DETACH DELETE s",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Deleted region state: {}", id);
        Ok(())
    }

    async fn list_for_region(&self, region_id: RegionId) -> Result<Vec<RegionState>, RepoError> {
        let q = query(
            "MATCH (s:RegionState {region_id: $region_id})
            RETURN s ORDER BY s.priority DESC, s.name",
        )
        .param("region_id", region_id.to_string());

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

    async fn get_default(&self, region_id: RegionId) -> Result<Option<RegionState>, RepoError> {
        let q = query(
            "MATCH (s:RegionState {region_id: $region_id, is_default: true})
            RETURN s LIMIT 1",
        )
        .param("region_id", region_id.to_string());

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
        region_id: RegionId,
        state_id: RegionStateId,
    ) -> Result<(), RepoError> {
        // Match region and target state FIRST to ensure they exist,
        // then delete old relationship and create new one atomically.
        // This prevents leaving the region without an active state if the target doesn't exist.
        let q = query(
            "MATCH (r:Region {id: $region_id})
            MATCH (s:RegionState {id: $state_id})
            OPTIONAL MATCH (r)-[old:ACTIVE_STATE]->(:RegionState)
            DELETE old
            CREATE (r)-[:ACTIVE_STATE]->(s)
            RETURN r.id as region_id",
        )
        .param("region_id", region_id.to_string())
        .param("state_id", state_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Check if the query matched anything (region and state both exist)
        if result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
            .is_none()
        {
            return Err(RepoError::not_found("Entity", "unknown"));
        }

        tracing::debug!(
            "Set active region state {} for region {}",
            state_id,
            region_id
        );
        Ok(())
    }

    async fn get_active(&self, region_id: RegionId) -> Result<Option<RegionState>, RepoError> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[:ACTIVE_STATE]->(s:RegionState)
            RETURN s",
        )
        .param("region_id", region_id.to_string());

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

    async fn clear_active(&self, region_id: RegionId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[rel:ACTIVE_STATE]->(:RegionState)
            DELETE rel",
        )
        .param("region_id", region_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Cleared active region state for region {}", region_id);
        Ok(())
    }
}
