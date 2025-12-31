//! Want repository implementation for Neo4j
//!
//! Wants are desires that characters have. They can target Characters, Items, or Goals.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;

use super::connection::Neo4jConnection;
use super::converters::row_to_want;
use wrldbldr_domain::entities::Want;
use wrldbldr_domain::WantId;
use wrldbldr_engine_ports::outbound::{ClockPort, WantRepositoryPort};

/// Repository for Want operations
pub struct Neo4jWantRepository {
    connection: Neo4jConnection,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jWantRepository {
    pub fn new(connection: Neo4jConnection, clock: Arc<dyn ClockPort>) -> Self {
        Self { connection, clock }
    }

    /// Get a want by ID
    pub async fn get(&self, id: WantId) -> Result<Option<Want>> {
        let q = query(
            "MATCH (w:Want {id: $id})
            RETURN w",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_want(&row, self.clock.now())?))
        } else {
            Ok(None)
        }
    }

    /// Get the target of a want (returns type and ID)
    pub async fn get_target(&self, want_id: WantId) -> Result<Option<(String, String)>> {
        let q = query(
            "MATCH (w:Want {id: $want_id})-[:TARGETS]->(target)
            RETURN labels(target) as labels, target.id as target_id",
        )
        .param("want_id", want_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let labels: Vec<String> = row.get("labels")?;
            let target_id: String = row.get("target_id")?;

            // Determine target type from labels
            let target_type = if labels.contains(&"Character".to_string()) {
                "Character"
            } else if labels.contains(&"Item".to_string()) {
                "Item"
            } else if labels.contains(&"Goal".to_string()) {
                "Goal"
            } else {
                "Unknown"
            };

            Ok(Some((target_type.to_string(), target_id)))
        } else {
            Ok(None)
        }
    }
}

// =============================================================================
// WantRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl WantRepositoryPort for Neo4jWantRepository {
    async fn get(&self, id: WantId) -> Result<Option<Want>> {
        Neo4jWantRepository::get(self, id).await
    }

    async fn get_target(&self, want_id: WantId) -> Result<Option<(String, String)>> {
        Neo4jWantRepository::get_target(self, want_id).await
    }
}
