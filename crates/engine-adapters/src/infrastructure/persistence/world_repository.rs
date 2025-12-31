//! World repository implementation for Neo4j

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use super::neo4j_helpers::{parse_typed_id, NodeExt};
use wrldbldr_domain::entities::{Act, MonomythStage, World};
use wrldbldr_domain::value_objects::RuleSystemConfig;
use wrldbldr_domain::{ActId, GameTime, WorldId};
use wrldbldr_engine_ports::outbound::{ClockPort, WorldRepositoryPort};

/// Repository for World aggregate operations
pub struct Neo4jWorldRepository {
    connection: Neo4jConnection,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jWorldRepository {
    pub fn new(connection: Neo4jConnection, clock: Arc<dyn ClockPort>) -> Self {
        Self { connection, clock }
    }

    /// Create a new world
    pub async fn create(&self, world: &World) -> Result<()> {
        // Domain RuleSystemConfig now has serde derives, serialize directly
        let rule_system_json = serde_json::to_string(&world.rule_system)?;

        let q = query(
            "CREATE (w:World {
                id: $id,
                name: $name,
                description: $description,
                rule_system: $rule_system,
                game_time: $game_time,
                game_time_paused: $game_time_paused,
                created_at: $created_at,
                updated_at: $updated_at
            })
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

        self.connection.graph().run(q).await?;
        tracing::debug!("Created world: {}", world.name);
        Ok(())
    }

    /// Get a world by ID
    pub async fn get(&self, id: WorldId) -> Result<Option<World>> {
        let q = query(
            "MATCH (w:World {id: $id})
            RETURN w",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(self.row_to_world(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all worlds
    pub async fn list(&self) -> Result<Vec<World>> {
        let q = query(
            "MATCH (w:World)
            RETURN w
            ORDER BY w.name",
        );

        let mut result = self.connection.graph().execute(q).await?;
        let mut worlds = Vec::new();

        while let Some(row) = result.next().await? {
            worlds.push(self.row_to_world(row)?);
        }

        Ok(worlds)
    }

    /// Update a world
    pub async fn update(&self, world: &World) -> Result<()> {
        // Domain RuleSystemConfig now has serde derives, serialize directly
        let rule_system_json = serde_json::to_string(&world.rule_system)?;

        let q = query(
            "MATCH (w:World {id: $id})
            SET w.name = $name,
                w.description = $description,
                w.rule_system = $rule_system,
                w.game_time = $game_time,
                w.game_time_paused = $game_time_paused,
                w.updated_at = $updated_at
            RETURN w.id as id",
        )
        .param("id", world.id.to_string())
        .param("name", world.name.clone())
        .param("description", world.description.clone())
        .param("rule_system", rule_system_json)
        .param("game_time", world.game_time.current().to_rfc3339())
        .param("game_time_paused", world.game_time.is_paused())
        .param("updated_at", world.updated_at.to_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated world: {}", world.name);
        Ok(())
    }

    /// Delete a world and all its contents
    pub async fn delete(&self, id: WorldId) -> Result<()> {
        // Delete all related entities first (cascading delete)
        let q = query(
            "MATCH (w:World {id: $id})
            OPTIONAL MATCH (w)-[*]->(related)
            DETACH DELETE related, w",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted world: {}", id);
        Ok(())
    }

    /// Create an act within a world
    pub async fn create_act(&self, act: &Act) -> Result<()> {
        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (a:Act {
                id: $id,
                world_id: $world_id,
                name: $name,
                stage: $stage,
                description: $description,
                order_num: $order_num
            })
            CREATE (w)-[:CONTAINS_ACT]->(a)
            RETURN a.id as id",
        )
        .param("id", act.id.to_string())
        .param("world_id", act.world_id.to_string())
        .param("name", act.name.clone())
        .param("stage", format!("{:?}", act.stage))
        .param("description", act.description.clone())
        .param("order_num", act.order as i64);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created act: {}", act.name);
        Ok(())
    }

    /// Get acts for a world
    pub async fn get_acts(&self, world_id: WorldId) -> Result<Vec<Act>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_ACT]->(a:Act)
            RETURN a
            ORDER BY a.order_num",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut acts = Vec::new();

        while let Some(row) = result.next().await? {
            acts.push(row_to_act(row)?);
        }

        Ok(acts)
    }
}

impl Neo4jWorldRepository {
    fn row_to_world(&self, row: Row) -> Result<World> {
        let node: neo4rs::Node = row.get("w")?;
        let fallback = self.clock.now();

        let id: WorldId = parse_typed_id(&node, "id")?;
        let name: String = node.get("name")?;
        let description: String = node.get("description")?;
        let rule_system: RuleSystemConfig = node.get_json("rule_system")?;
        let created_at = node.get_datetime_or("created_at", fallback);
        let updated_at = node.get_datetime_or("updated_at", fallback);

        // GameTime fields - use defaults for backwards compatibility with existing DBs
        let game_time_paused = node.get_bool_or("game_time_paused", true);

        // Parse game time or create new with current time via injected clock
        let mut game_time = if let Some(gt_str) = node.get_optional_string("game_time") {
            let dt = node.get_datetime_or("game_time", fallback);
            let _ = gt_str; // used get_optional_string to check existence
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

fn row_to_act(row: Row) -> Result<Act> {
    let node: neo4rs::Node = row.get("a")?;

    let id: ActId = parse_typed_id(&node, "id")?;
    let world_id: WorldId = parse_typed_id(&node, "world_id")?;
    let name: String = node.get("name")?;
    let stage_str: String = node.get("stage")?;
    let description: String = node.get("description")?;
    let order_num = node.get_i64_or("order_num", 0);

    let stage: MonomythStage = stage_str.parse().unwrap_or_default();

    Ok(Act {
        id,
        world_id,
        name,
        stage,
        description,
        order: order_num as u32,
    })
}

// =============================================================================
// WorldRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl WorldRepositoryPort for Neo4jWorldRepository {
    async fn create(&self, world: &World) -> Result<()> {
        Neo4jWorldRepository::create(self, world).await
    }

    async fn get(&self, id: WorldId) -> Result<Option<World>> {
        Neo4jWorldRepository::get(self, id).await
    }

    async fn list(&self) -> Result<Vec<World>> {
        Neo4jWorldRepository::list(self).await
    }

    async fn update(&self, world: &World) -> Result<()> {
        Neo4jWorldRepository::update(self, world).await
    }

    async fn delete(&self, id: WorldId) -> Result<()> {
        Neo4jWorldRepository::delete(self, id).await
    }

    async fn create_act(&self, act: &Act) -> Result<()> {
        Neo4jWorldRepository::create_act(self, act).await
    }

    async fn get_acts(&self, world_id: WorldId) -> Result<Vec<Act>> {
        Neo4jWorldRepository::get_acts(self, world_id).await
    }
}
