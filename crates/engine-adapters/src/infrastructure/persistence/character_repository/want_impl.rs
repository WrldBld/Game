//! CharacterWantPort implementation

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;
use wrldbldr_common::datetime::parse_datetime_or;
use wrldbldr_domain::entities::{CharacterWant, Want, WantVisibility};
use wrldbldr_domain::value_objects::WantTarget;
use wrldbldr_domain::{CharacterId, WantId};
use wrldbldr_engine_ports::outbound::CharacterWantPort;

use super::super::converters::row_to_want;
use super::Neo4jCharacterRepository;

impl Neo4jCharacterRepository {
    /// Create a want and attach it to a character
    pub(crate) async fn create_want_impl(
        &self,
        character_id: CharacterId,
        want: &Want,
        priority: u32,
    ) -> Result<()> {
        let tells_json = serde_json::to_string(&want.tells)?;
        let visibility_str = match want.visibility {
            WantVisibility::Known => "Known",
            WantVisibility::Suspected => "Suspected",
            WantVisibility::Hidden => "Hidden",
        };

        let q = query(
            "MATCH (c:Character {id: $character_id})
            CREATE (w:Want {
                id: $id,
                description: $description,
                intensity: $intensity,
                visibility: $visibility,
                created_at: $created_at,
                deflection_behavior: $deflection_behavior,
                tells: $tells
            })
            CREATE (c)-[:HAS_WANT {
                priority: $priority,
                acquired_at: $acquired_at
            }]->(w)
            RETURN w.id as id",
        )
        .param("character_id", character_id.to_string())
        .param("id", want.id.to_string())
        .param("description", want.description.clone())
        .param("intensity", want.intensity as f64)
        .param("visibility", visibility_str)
        .param("created_at", want.created_at.to_rfc3339())
        .param(
            "deflection_behavior",
            want.deflection_behavior.clone().unwrap_or_default(),
        )
        .param("tells", tells_json)
        .param("priority", priority as i64)
        .param("acquired_at", self.clock.now_rfc3339());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Created want for character {}: {}",
            character_id,
            want.description
        );
        Ok(())
    }

    /// Get all wants for a character
    pub(crate) async fn get_wants_impl(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<CharacterWant>> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[r:HAS_WANT]->(w:Want)
            RETURN w, r.priority as priority, r.acquired_at as acquired_at
            ORDER BY r.priority",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut wants = Vec::new();

        while let Some(row) = result.next().await? {
            let want = row_to_want(&row, self.clock.now())?;
            let priority: i64 = row.get("priority")?;
            let acquired_at_str: String = row.get("acquired_at")?;
            let acquired_at = parse_datetime_or(&acquired_at_str, self.clock.now());

            wants.push(CharacterWant {
                want,
                priority: priority as u32,
                acquired_at,
            });
        }

        Ok(wants)
    }

    /// Update a want
    pub(crate) async fn update_want_impl(&self, want: &Want) -> Result<()> {
        let tells_json = serde_json::to_string(&want.tells)?;
        let visibility_str = match want.visibility {
            WantVisibility::Known => "Known",
            WantVisibility::Suspected => "Suspected",
            WantVisibility::Hidden => "Hidden",
        };

        let q = query(
            "MATCH (w:Want {id: $id})
            SET w.description = $description,
                w.intensity = $intensity,
                w.visibility = $visibility,
                w.deflection_behavior = $deflection_behavior,
                w.tells = $tells
            RETURN w.id as id",
        )
        .param("id", want.id.to_string())
        .param("description", want.description.clone())
        .param("intensity", want.intensity as f64)
        .param("visibility", visibility_str)
        .param(
            "deflection_behavior",
            want.deflection_behavior.clone().unwrap_or_default(),
        )
        .param("tells", tells_json);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Delete a want
    pub(crate) async fn delete_want_impl(&self, want_id: WantId) -> Result<()> {
        let q = query(
            "MATCH (w:Want {id: $id})
            DETACH DELETE w",
        )
        .param("id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Set a want's target (creates TARGETS edge)
    pub(crate) async fn set_want_target_impl(
        &self,
        want_id: WantId,
        target_id: &str,
        target_type: &str,
    ) -> Result<()> {
        // First remove any existing target
        self.remove_want_target_impl(want_id).await?;

        // Create the new TARGETS edge based on target type
        let cypher = match target_type {
            "Character" => {
                "MATCH (w:Want {id: $want_id}), (t:Character {id: $target_id})
                CREATE (w)-[:TARGETS]->(t)"
            }
            "Item" => {
                "MATCH (w:Want {id: $want_id}), (t:Item {id: $target_id})
                CREATE (w)-[:TARGETS]->(t)"
            }
            "Goal" => {
                "MATCH (w:Want {id: $want_id}), (t:Goal {id: $target_id})
                CREATE (w)-[:TARGETS]->(t)"
            }
            _ => return Err(anyhow::anyhow!("Invalid target type: {}", target_type)),
        };

        let q = query(cypher)
            .param("want_id", want_id.to_string())
            .param("target_id", target_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Remove a want's target
    pub(crate) async fn remove_want_target_impl(&self, want_id: WantId) -> Result<()> {
        let q = query(
            "MATCH (w:Want {id: $id})-[r:TARGETS]->()
            DELETE r",
        )
        .param("id", want_id.to_string());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get the resolved target of a want
    pub(crate) async fn get_want_target_impl(&self, want_id: WantId) -> Result<Option<WantTarget>> {
        // Query for TARGETS edge to any of the possible target types
        let q = query(
            "MATCH (w:Want {id: $want_id})-[:TARGETS]->(target)
            RETURN labels(target) as labels, target.id as id, target.name as name,
                   target.description as description",
        )
        .param("want_id", want_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let labels: Vec<String> = row.get("labels")?;
            let id_str: String = row.get("id")?;
            let name: String = row.get("name")?;
            let description: Option<String> = row
                .get("description")
                .ok()
                .filter(|s: &String| !s.is_empty());

            let id = uuid::Uuid::parse_str(&id_str)?;

            // Determine target type from labels
            if labels.contains(&"Character".to_string()) {
                Ok(Some(WantTarget::Character { id, name }))
            } else if labels.contains(&"Item".to_string()) {
                Ok(Some(WantTarget::Item { id, name }))
            } else if labels.contains(&"Goal".to_string()) {
                Ok(Some(WantTarget::Goal {
                    id,
                    name,
                    description,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl CharacterWantPort for Neo4jCharacterRepository {
    async fn create_want(
        &self,
        character_id: CharacterId,
        want: &Want,
        priority: u32,
    ) -> Result<()> {
        self.create_want_impl(character_id, want, priority).await
    }

    async fn get_wants(&self, character_id: CharacterId) -> Result<Vec<CharacterWant>> {
        self.get_wants_impl(character_id).await
    }

    async fn update_want(&self, want: &Want) -> Result<()> {
        self.update_want_impl(want).await
    }

    async fn delete_want(&self, want_id: WantId) -> Result<()> {
        self.delete_want_impl(want_id).await
    }

    async fn set_want_target(
        &self,
        want_id: WantId,
        target_id: String,
        target_type: String,
    ) -> Result<()> {
        self.set_want_target_impl(want_id, &target_id, &target_type)
            .await
    }

    async fn remove_want_target(&self, want_id: WantId) -> Result<()> {
        self.remove_want_target_impl(want_id).await
    }

    async fn get_want_target(&self, want_id: WantId) -> Result<Option<WantTarget>> {
        self.get_want_target_impl(want_id).await
    }
}
