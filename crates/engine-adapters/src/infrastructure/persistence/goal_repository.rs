//! Goal repository implementation for Neo4j
//!
//! Goals are abstract desire targets for the Actantial Model.
//! They represent intangible objectives like "Power", "Revenge", "Redemption".

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use super::neo4j_helpers::{parse_typed_id, NodeExt};
use wrldbldr_domain::entities::Goal;
use wrldbldr_domain::{GoalId, WorldId};
use wrldbldr_engine_ports::outbound::GoalRepositoryPort;

/// Repository for Goal operations
pub struct Neo4jGoalRepository {
    connection: Neo4jConnection,
}

impl Neo4jGoalRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new goal with CONTAINS_GOAL edge from world
    pub async fn create(&self, goal: &Goal) -> Result<()> {
        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (g:Goal {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description
            })
            CREATE (w)-[:CONTAINS_GOAL]->(g)
            RETURN g.id as id",
        )
        .param("id", goal.id.to_string())
        .param("world_id", goal.world_id.to_string())
        .param("name", goal.name.clone())
        .param("description", goal.description.clone().unwrap_or_default());

        self.connection.graph().run(q).await?;
        tracing::debug!("Created goal: {} ({})", goal.name, goal.id);
        Ok(())
    }

    /// Get a goal by ID
    pub async fn get(&self, id: GoalId) -> Result<Option<Goal>> {
        let q = query(
            "MATCH (g:Goal {id: $id})
            RETURN g",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_goal(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all goals for a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Goal>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_GOAL]->(g:Goal)
            RETURN g
            ORDER BY g.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut goals = Vec::new();

        while let Some(row) = result.next().await? {
            goals.push(row_to_goal(row)?);
        }

        Ok(goals)
    }

    /// Update a goal
    pub async fn update(&self, goal: &Goal) -> Result<()> {
        let q = query(
            "MATCH (g:Goal {id: $id})
            SET g.name = $name,
                g.description = $description
            RETURN g.id as id",
        )
        .param("id", goal.id.to_string())
        .param("name", goal.name.clone())
        .param("description", goal.description.clone().unwrap_or_default());

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated goal: {} ({})", goal.name, goal.id);
        Ok(())
    }

    /// Delete a goal (also removes CONTAINS_GOAL and any TARGETS edges)
    pub async fn delete(&self, id: GoalId) -> Result<()> {
        let q = query(
            "MATCH (g:Goal {id: $id})
            DETACH DELETE g",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted goal: {}", id);
        Ok(())
    }

    /// Find a goal by name within a world
    pub async fn find_by_name(&self, world_id: WorldId, name: &str) -> Result<Option<Goal>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_GOAL]->(g:Goal)
            WHERE toLower(g.name) = toLower($name)
            RETURN g",
        )
        .param("world_id", world_id.to_string())
        .param("name", name);

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_goal(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get the count of wants targeting this goal
    pub async fn get_targeting_want_count(&self, goal_id: GoalId) -> Result<u32> {
        let q = query(
            "MATCH (w:Want)-[:TARGETS]->(g:Goal {id: $id})
            RETURN count(w) as count",
        )
        .param("id", goal_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            Ok(count as u32)
        } else {
            Ok(0)
        }
    }
}

/// Convert a Neo4j row to a Goal
fn row_to_goal(row: Row) -> Result<Goal> {
    let node: neo4rs::Node = row.get("g")?;

    let id: GoalId = parse_typed_id(&node, "id")?;
    let world_id: WorldId = parse_typed_id(&node, "world_id")?;
    let name: String = node.get("name")?;
    let description = node.get_optional_string("description");

    Ok(Goal {
        id,
        world_id,
        name,
        description,
    })
}

// =============================================================================
// GoalRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl GoalRepositoryPort for Neo4jGoalRepository {
    async fn create(&self, goal: &Goal) -> Result<()> {
        Neo4jGoalRepository::create(self, goal).await
    }

    async fn get(&self, id: GoalId) -> Result<Option<Goal>> {
        Neo4jGoalRepository::get(self, id).await
    }

    async fn list(&self, world_id: WorldId) -> Result<Vec<Goal>> {
        Neo4jGoalRepository::list_by_world(self, world_id).await
    }

    async fn update(&self, goal: &Goal) -> Result<()> {
        Neo4jGoalRepository::update(self, goal).await
    }

    async fn delete(&self, id: GoalId) -> Result<()> {
        Neo4jGoalRepository::delete(self, id).await
    }
}
