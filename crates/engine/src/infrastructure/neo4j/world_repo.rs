//! Neo4j world repository implementation.

use std::sync::Arc;

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Row};
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ClockPort, RepoError, WorldRepo};

/// Repository for World aggregate operations.
pub struct Neo4jWorldRepo {
    graph: Neo4jGraph,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jWorldRepo {
    pub fn new(graph: Neo4jGraph, clock: Arc<dyn ClockPort>) -> Self {
        Self { graph, clock }
    }

    fn row_to_world(&self, row: Row) -> Result<World, RepoError> {
        let node: neo4rs::Node = row
            .get("w")
            .map_err(|e| RepoError::database("query", format!("Failed to get 'w' node: {}", e)))?;
        let fallback = self.clock.now();

        let id: WorldId = parse_typed_id(&node, "id")
            .map_err(|e| RepoError::database("query", format!("Failed to parse WorldId: {}", e)))?;
        let name: String = node.get("name").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get 'name' for World {}: {}", id, e),
            )
        })?;
        let description: String = node.get("description").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get 'description' for World {}: {}", id, e),
            )
        })?;
        let rule_system: RuleSystemConfig = node.get_json("rule_system").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get 'rule_system' for World {}: {}", id, e),
            )
        })?;
        let created_at = node.get_datetime_or("created_at", fallback);
        let updated_at = node.get_datetime_or("updated_at", fallback);

        // GameTime fields - use defaults for backwards compatibility
        let game_time_paused = node.get_bool_or("game_time_paused", true);

        // Parse game time as total minutes since epoch
        // For backwards compatibility: if stored as DateTime string, default to 0
        // New storage format uses game_time_minutes (i64)
        let total_minutes = node.get_i64_or("game_time_minutes", 0);
        let mut game_time = GameTime::from_minutes(total_minutes);
        game_time.set_paused(game_time_paused);

        // Parse time config or use defaults
        let time_config: GameTimeConfig = node
            .get_optional_string("time_config")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let name = WorldName::new(&name)
            .map_err(|e| RepoError::database("query", format!("Invalid world name: {}", e)))?;
        let description = Description::new(&description)
            .map_err(|e| RepoError::database("query", format!("Invalid description: {}", e)))?;

        Ok(World::new(name, created_at)
            .with_id(id)
            .with_description(description)
            .with_rule_system(rule_system)
            .with_game_time(game_time)
            .with_time_config(time_config)
            .with_created_at(created_at)
            .with_updated_at(updated_at))
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
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            Ok(Some(self.row_to_world(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, world: &World) -> Result<(), RepoError> {
        let rule_system_json = serde_json::to_string(world.rule_system())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let time_config_json = serde_json::to_string(world.time_config())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        // MERGE to handle both create and update
        let q = query(
            "MERGE (w:World {id: $id})
            SET w.name = $name,
                w.description = $description,
                w.rule_system = $rule_system,
                w.game_time_minutes = $game_time_minutes,
                w.game_time_paused = $game_time_paused,
                w.time_config = $time_config,
                w.created_at = $created_at,
                w.updated_at = $updated_at
            RETURN w.id as id",
        )
        .param("id", world.id().to_string())
        .param("name", world.name().as_str().to_owned())
        .param("description", world.description().as_str().to_owned())
        .param("rule_system", rule_system_json)
        .param("game_time_minutes", world.game_time().total_minutes())
        .param("game_time_paused", world.game_time().is_paused())
        .param("time_config", time_config_json)
        .param("created_at", world.created_at().to_rfc3339())
        .param("updated_at", world.updated_at().to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Saved world: {}", world.name().as_str());
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<World>, RepoError> {
        let q = query("MATCH (w:World) RETURN w ORDER BY w.name");

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut worlds = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            worlds.push(self.row_to_world(row)?);
        }

        Ok(worlds)
    }

    async fn delete(&self, id: WorldId) -> Result<(), RepoError> {
        // Delete world entities in order to respect dependencies.
        // Uses explicit node types to avoid deleting unrelated data.
        let world_id_str = id.to_string();

        // 1. Delete Stagings (depend on Regions)
        let q1 = query(
            "MATCH (s:Staging {world_id: $id})
            DETACH DELETE s",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q1)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 2. Delete NarrativeEvents, EventChains, StoryEvents
        let q2 = query(
            "MATCH (e:NarrativeEvent {world_id: $id})
            DETACH DELETE e",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q2)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let q2b = query(
            "MATCH (ec:EventChain {world_id: $id})
            DETACH DELETE ec",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q2b)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let q2c = query(
            "MATCH (se:StoryEvent {world_id: $id})
            DETACH DELETE se",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q2c)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 3. Delete Scenes
        let q3 = query(
            "MATCH (s:Scene {world_id: $id})
            DETACH DELETE s",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q3)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 4. Delete Challenges
        let q4 = query(
            "MATCH (c:Challenge {world_id: $id})
            DETACH DELETE c",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q4)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 5. Delete RegionStates, then Regions
        let q5a = query(
            "MATCH (rs:RegionState)-[:STATE_OF]->(r:Region)-[:WITHIN]->(l:Location {world_id: $id})
            DETACH DELETE rs",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q5a)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let q5b = query(
            "MATCH (r:Region)-[:WITHIN]->(l:Location {world_id: $id})
            DETACH DELETE r",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q5b)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 6. Delete LocationStates, then Locations
        let q6a = query(
            "MATCH (ls:LocationState)-[:STATE_OF]->(l:Location {world_id: $id})
            DETACH DELETE ls",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q6a)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let q6b = query(
            "MATCH (l:Location {world_id: $id})
            DETACH DELETE l",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q6b)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 7. Delete Character relationships (Wants, NpcDispositions), then Characters
        let q7a = query(
            "MATCH (w:Want)-[:WANTS]->(c:Character {world_id: $id})
            DETACH DELETE w",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q7a)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let q7b = query(
            "MATCH (c:Character {world_id: $id})
            DETACH DELETE c",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q7b)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 8. Delete PlayerCharacters
        let q8 = query(
            "MATCH (pc:PlayerCharacter {world_id: $id})
            DETACH DELETE pc",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q8)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 9. Delete Items
        let q9 = query(
            "MATCH (i:Item {world_id: $id})
            DETACH DELETE i",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q9)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 10. Delete Acts
        let q10 = query(
            "MATCH (a:Act {world_id: $id})
            DETACH DELETE a",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q10)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 11. Delete Lore and LoreChunks
        let q11a = query(
            "MATCH (lc:LoreChunk)-[:CHUNK_OF]->(l:Lore {world_id: $id})
            DETACH DELETE lc",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q11a)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let q11b = query(
            "MATCH (l:Lore {world_id: $id})
            DETACH DELETE l",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q11b)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 12. Delete Observations
        let q12 = query(
            "MATCH (o:Observation)-[:OBSERVED_BY]->(pc:PlayerCharacter {world_id: $id})
            DETACH DELETE o",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q12)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 13. Delete Flags
        let q13 = query(
            "MATCH (f:Flag)-[:FLAG_OF]->(w:World {id: $id})
            DETACH DELETE f",
        )
        .param("id", world_id_str.clone());
        self.graph
            .run(q13)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // 14. Finally delete the World itself
        let q14 = query(
            "MATCH (w:World {id: $id})
            DETACH DELETE w",
        )
        .param("id", world_id_str);
        self.graph
            .run(q14)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::info!(world_id = %id, "Deleted world and all related entities");
        Ok(())
    }
}
