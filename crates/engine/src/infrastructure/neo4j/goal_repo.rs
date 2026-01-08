//! Neo4j goal repository implementation.
//!
//! Handles Goal persistence and usage counts.

use async_trait::async_trait;
use neo4rs::{query, Graph};
use wrldbldr_domain::{Goal, GoalId, WorldId};

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{GoalDetails, GoalRepo, RepoError};

pub struct Neo4jGoalRepo {
    graph: Graph,
}

impl Neo4jGoalRepo {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }
}

#[async_trait]
impl GoalRepo for Neo4jGoalRepo {
    async fn get(&self, id: GoalId) -> Result<Option<GoalDetails>, RepoError> {
        let q = query(
            "MATCH (g:Goal {id: $id})
            OPTIONAL MATCH (w:Want)-[:TARGETS]->(g)
            RETURN g, count(w) as usage_count",
        )
        .param("id", id.to_string());

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
            let node: neo4rs::Node = row
                .get("g")
                .map_err(|e| RepoError::Database(e.to_string()))?;

            let goal_id: GoalId =
                parse_typed_id(&node, "id").map_err(|e| RepoError::Database(e.to_string()))?;
            let world_id: WorldId =
                parse_typed_id(&node, "world_id").map_err(|e| RepoError::Database(e.to_string()))?;
            let name: String = node.get_string_or("name", "");
            let description = node.get_optional_string("description");
            let usage_count: i64 = row.get("usage_count").unwrap_or(0);

            Ok(Some(GoalDetails {
                goal: Goal {
                    id: goal_id,
                    world_id,
                    name,
                    description,
                },
                usage_count: usage_count.max(0) as u32,
            }))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, goal: &Goal) -> Result<(), RepoError> {
        let q = query(
            "MERGE (g:Goal {id: $id})
            SET g.world_id = $world_id,
                g.name = $name,
                g.description = $description
            WITH g
            MATCH (w:World {id: $world_id})
            MERGE (w)-[:CONTAINS_GOAL]->(g)",
        )
        .param("id", goal.id.to_string())
        .param("world_id", goal.world_id.to_string())
        .param("name", goal.name.clone())
        .param("description", goal.description.clone().unwrap_or_default());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: GoalId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (g:Goal {id: $id})
            DETACH DELETE g",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        tracing::debug!("Deleted goal: {}", id);
        Ok(())
    }

    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<GoalDetails>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_GOAL]->(g:Goal)
            OPTIONAL MATCH (want:Want)-[:TARGETS]->(g)
            RETURN g, count(want) as usage_count
            ORDER BY g.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut goals = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?
        {
            let node: neo4rs::Node = row
                .get("g")
                .map_err(|e| RepoError::Database(e.to_string()))?;

            let goal_id: GoalId =
                parse_typed_id(&node, "id").map_err(|e| RepoError::Database(e.to_string()))?;
            let world_id: WorldId =
                parse_typed_id(&node, "world_id").map_err(|e| RepoError::Database(e.to_string()))?;
            let name: String = node.get_string_or("name", "");
            let description = node.get_optional_string("description");
            let usage_count: i64 = row.get("usage_count").unwrap_or(0);

            goals.push(GoalDetails {
                goal: Goal {
                    id: goal_id,
                    world_id,
                    name,
                    description,
                },
                usage_count: usage_count.max(0) as u32,
            });
        }

        Ok(goals)
    }
}
