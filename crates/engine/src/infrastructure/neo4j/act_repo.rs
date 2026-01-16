//! Neo4j act repository implementation.
//!
//! Acts are stored as nodes and linked to worlds:
//! - `(World)-[:CONTAINS_ACT]->(Act)`

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Row};
use wrldbldr_domain::{Act, ActId, MonomythStage, WorldId};

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ActRepo, RepoError};

/// Repository for Act operations.
pub struct Neo4jActRepo {
    graph: Neo4jGraph,
}

impl Neo4jActRepo {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }

    fn row_to_act(&self, row: Row) -> Result<Act, RepoError> {
        let node: neo4rs::Node = row.get("a").map_err(|e| RepoError::database("query", e))?;

        let id: ActId = parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
        let world_id: WorldId =
            parse_typed_id(&node, "world_id").map_err(|e| RepoError::database("query", e))?;
        let name: String = node
            .get("name")
            .map_err(|e| RepoError::database("query", e))?;
        let stage_str = node.get_string_or("stage", MonomythStage::Unknown.as_str());
        let stage = stage_str
            .parse::<MonomythStage>()
            .unwrap_or(MonomythStage::Unknown);
        let description: String = node.get_string_or("description", "");
        let order_num = node.get_i64_or("order_num", 0);

        Ok(Act {
            id,
            world_id,
            name,
            stage,
            description,
            order: order_num as u32,
        })
    }
}

#[async_trait]
impl ActRepo for Neo4jActRepo {
    async fn get(&self, id: ActId) -> Result<Option<Act>, RepoError> {
        let q = query("MATCH (a:Act {id: $id}) RETURN a").param("id", id.to_string());

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
            Ok(Some(self.row_to_act(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, act: &Act) -> Result<(), RepoError> {
        let q = query(
            "MERGE (a:Act {id: $id})
            SET a.world_id = $world_id,
                a.name = $name,
                a.stage = $stage,
                a.description = $description,
                a.order_num = $order_num
            WITH a
            MATCH (w:World {id: $world_id})
            MERGE (w)-[:CONTAINS_ACT]->(a)
            RETURN a.id as id",
        )
        .param("id", act.id.to_string())
        .param("world_id", act.world_id.to_string())
        .param("name", act.name.clone())
        .param("stage", act.stage.to_string())
        .param("description", act.description.clone())
        .param("order_num", act.order as i64);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Saved act: {}", act.name);
        Ok(())
    }

    async fn delete(&self, id: ActId) -> Result<(), RepoError> {
        let q = query("MATCH (a:Act {id: $id}) DETACH DELETE a").param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        Ok(())
    }

    async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<Act>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_ACT]->(a:Act)
            RETURN a
            ORDER BY a.order_num",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut acts = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            acts.push(self.row_to_act(row)?);
        }

        Ok(acts)
    }
}
