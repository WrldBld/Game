//! Neo4j world repository implementation.

use std::sync::Arc;

use async_trait::async_trait;
use neo4rs::{query, Graph, Row};
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ClockPort, RepoError, WorldRepo};

/// Repository for World aggregate operations.
pub struct Neo4jWorldRepo {
    graph: Graph,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jWorldRepo {
    pub fn new(graph: Graph, clock: Arc<dyn ClockPort>) -> Self {
        Self { graph, clock }
    }

    fn row_to_world(&self, row: Row) -> Result<World, RepoError> {
        let node: neo4rs::Node = row
            .get("w")
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let fallback = self.clock.now();

        let id: WorldId =
            parse_typed_id(&node, "id").map_err(|e| RepoError::Database(e.to_string()))?;
        let name: String = node
            .get("name")
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let description: String = node
            .get("description")
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let rule_system: RuleSystemConfig = node
            .get_json("rule_system")
            .map_err(|e| RepoError::Database(e.to_string()))?;
        let created_at = node.get_datetime_or("created_at", fallback);
        let updated_at = node.get_datetime_or("updated_at", fallback);

        // GameTime fields - use defaults for backwards compatibility
        let game_time_paused = node.get_bool_or("game_time_paused", true);

        // Parse game time or create new with current time via injected clock
        let mut game_time = if node.get_optional_string("game_time").is_some() {
            let dt = node.get_datetime_or("game_time", fallback);
            GameTime::starting_at(dt)
        } else {
            GameTime::new(fallback)
        };
        game_time.set_paused(game_time_paused);

        Ok(World {
            id,
            name,
            description,
            rule_system,
            game_time,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl WorldRepo for Neo4jWorldRepo {
    async fn get(&self, id: WorldId) -> Result<Option<World>, RepoError> {
        let q = query("MATCH (w:World {id: $id}) RETURN w").param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            Ok(Some(self.row_to_world(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, world: &World) -> Result<(), RepoError> {
        let rule_system_json = serde_json::to_string(&world.rule_system)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        // MERGE to handle both create and update
        let q = query(
            "MERGE (w:World {id: $id})
            SET w.name = $name,
                w.description = $description,
                w.rule_system = $rule_system,
                w.game_time = $game_time,
                w.game_time_paused = $game_time_paused,
                w.created_at = $created_at,
                w.updated_at = $updated_at
            RETURN w.id as id",
        )
        .param("id", world.id.to_string())
        .param("name", world.name.clone())
        .param("description", world.description.clone())
        .param("rule_system", rule_system_json)
        .param("game_time", world.game_time.current().to_rfc3339())
        .param("game_time_paused", world.game_time.is_paused())
        .param("created_at", world.created_at.to_rfc3339())
        .param("updated_at", world.updated_at.to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Saved world: {}", world.name);
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<World>, RepoError> {
        let q = query("MATCH (w:World) RETURN w ORDER BY w.name");

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut worlds = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            worlds.push(self.row_to_world(row)?);
        }

        Ok(worlds)
    }

    async fn delete(&self, id: WorldId) -> Result<(), RepoError> {
        // Delete all related entities first (cascading delete)
        let q = query(
            "MATCH (w:World {id: $id})
            OPTIONAL MATCH (w)-[*]->(related)
            DETACH DELETE related, w",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Deleted world: {}", id);
        Ok(())
    }
}
