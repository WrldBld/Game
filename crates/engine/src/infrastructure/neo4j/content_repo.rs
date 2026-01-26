//! Neo4j content repository implementation.
//!
//! Skills are stored as nodes and linked to worlds:
//! - `(World)-[:CONTAINS_SKILL]->(Skill)`

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Row};
use wrldbldr_domain::{Skill, SkillCategory, SkillId, Stat, WorldId};

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ContentRepo, RepoError};

/// Repository for content operations.
pub struct Neo4jContentRepo {
    graph: Neo4jGraph,
}

impl Neo4jContentRepo {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }

    fn row_to_skill(&self, row: Row) -> Result<Skill, RepoError> {
        let node: neo4rs::Node = row.get("s").map_err(|e| RepoError::database("query", e))?;

        let id: SkillId = parse_typed_id(&node, "id")
            .map_err(|e| RepoError::database("query", format!("Failed to parse SkillId: {}", e)))?;
        let world_id: WorldId = parse_typed_id(&node, "world_id").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to parse world_id for Skill {}: {}", id, e),
            )
        })?;
        let name: String = node.get("name").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get 'name' for Skill {}: {}", id, e),
            )
        })?;
        let description: String = node.get_string_or("description", "");
        let category_str = node.get_string_or("category", "");
        let category: SkillCategory = if category_str.is_empty() {
            return Err(RepoError::database(
                "parse",
                format!("Missing required field 'category' for skill: {}", id),
            ));
        } else {
            category_str.parse().map_err(|e| {
                RepoError::database(
                    "parse",
                    format!("Invalid SkillCategory '{}': {}", category_str, e),
                )
            })?
        };
        let base_attribute: Option<Stat> = node
            .get_optional_string("base_attribute")
            .and_then(|s| s.parse().ok());
        let is_custom = node.get_bool_or("is_custom", false);
        let is_hidden = node.get_bool_or("is_hidden", false);
        let order_num = node.get_i64_or("order_num", 0);

        Ok(Skill {
            id,
            world_id,
            name,
            description,
            category,
            base_attribute,
            is_custom,
            is_hidden,
            order: order_num as u32,
        })
    }
}

#[async_trait]
impl ContentRepo for Neo4jContentRepo {
    async fn get_skill(&self, id: SkillId) -> Result<Option<Skill>, RepoError> {
        let q = query("MATCH (s:Skill {id: $id}) RETURN s").param("id", id.to_string());

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
            Ok(Some(self.row_to_skill(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save_skill(&self, skill: &Skill) -> Result<(), RepoError> {
        let q = query(
            "MERGE (s:Skill {id: $id})
            SET s.world_id = $world_id,
                s.name = $name,
                s.description = $description,
                s.category = $category,
                s.base_attribute = $base_attribute,
                s.is_custom = $is_custom,
                s.is_hidden = $is_hidden,
                s.order_num = $order_num
            WITH s
            MATCH (w:World {id: $world_id})
            MERGE (w)-[:CONTAINS_SKILL]->(s)
            RETURN s.id as id",
        )
        .param("id", skill.id.to_string())
        .param("world_id", skill.world_id.to_string())
        .param("name", skill.name.to_string())
        .param("description", skill.description.to_string())
        .param("category", skill.category.to_string())
        .param(
            "base_attribute",
            skill
                .base_attribute
                .map(|s| s.to_string())
                .unwrap_or_default(),
        )
        .param("is_custom", skill.is_custom)
        .param("is_hidden", skill.is_hidden)
        .param("order_num", skill.order as i64);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Saved skill: {}", skill.name);
        Ok(())
    }

    async fn delete_skill(&self, id: SkillId) -> Result<(), RepoError> {
        let q = query("MATCH (s:Skill {id: $id}) DETACH DELETE s").param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        Ok(())
    }

    async fn list_skills_in_world(&self, world_id: WorldId) -> Result<Vec<Skill>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_SKILL]->(s:Skill)
            RETURN s
            ORDER BY s.order_num",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut skills = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            skills.push(self.row_to_skill(row)?);
        }

        Ok(skills)
    }
}
