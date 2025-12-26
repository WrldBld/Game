//! Want repository implementation for Neo4j
//!
//! Wants are desires that characters have. They can target Characters, Items, or Goals.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use wrldbldr_domain::entities::{Want, WantVisibility};
use wrldbldr_domain::WantId;
use wrldbldr_engine_ports::outbound::WantRepositoryPort;

/// Repository for Want operations
pub struct Neo4jWantRepository {
    connection: Neo4jConnection,
}

impl Neo4jWantRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
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
            Ok(Some(row_to_want(&row)?))
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

/// Convert a Neo4j row to a Want
fn row_to_want(row: &Row) -> Result<Want> {
    let node: neo4rs::Node = row.get("w")?;

    let id_str: String = node.get("id")?;
    let description: String = node.get("description")?;
    let intensity: f64 = node.get("intensity")?;
    let created_at_str: String = node.get("created_at")?;

    // Handle visibility - try new field first, fall back to legacy known_to_player
    let visibility = if let Ok(vis_str) = node.get::<String>("visibility") {
        match vis_str.as_str() {
            "Known" => WantVisibility::Known,
            "Suspected" => WantVisibility::Suspected,
            _ => WantVisibility::Hidden,
        }
    } else {
        // Legacy: convert from known_to_player bool
        let known_to_player: bool = node.get("known_to_player").unwrap_or(false);
        WantVisibility::from_known_to_player(known_to_player)
    };

    // Behavioral guidance fields (optional)
    let deflection_behavior: Option<String> = node
        .get("deflection_behavior")
        .ok()
        .filter(|s: &String| !s.is_empty());
    let tells_json: String = node.get("tells").unwrap_or_else(|_| "[]".to_string());
    let tells: Vec<String> = serde_json::from_str(&tells_json).unwrap_or_default();

    let id = uuid::Uuid::parse_str(&id_str)?;
    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(Want {
        id: WantId::from_uuid(id),
        description,
        intensity: intensity as f32,
        visibility,
        created_at,
        deflection_behavior,
        tells,
    })
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
