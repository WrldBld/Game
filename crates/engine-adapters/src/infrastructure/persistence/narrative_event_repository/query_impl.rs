//! Query implementation for NarrativeEventQueryPort
//!
//! Provides query operations for finding events by their edge relationships.

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;

use super::common::row_to_narrative_event;
use super::Neo4jNarrativeEventRepository;
use wrldbldr_domain::entities::NarrativeEvent;
use wrldbldr_domain::{ActId, CharacterId, LocationId, SceneId};
use wrldbldr_engine_ports::outbound::NarrativeEventQueryPort;

#[async_trait]
impl NarrativeEventQueryPort for Neo4jNarrativeEventRepository {
    /// List events tied to a specific scene
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent)-[:TIED_TO_SCENE]->(s:Scene {id: $scene_id})
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List events tied to a specific location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent)-[:TIED_TO_LOCATION]->(l:Location {id: $location_id})
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List events belonging to a specific act
    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent)-[:BELONGS_TO_ACT]->(a:Act {id: $act_id})
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("act_id", act_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }

    /// List events featuring a specific NPC
    async fn list_by_featured_npc(&self, character_id: CharacterId) -> Result<Vec<NarrativeEvent>> {
        let q = query(
            "MATCH (e:NarrativeEvent)-[:FEATURES_NPC]->(c:Character {id: $character_id})
            RETURN e
            ORDER BY e.priority DESC, e.name",
        )
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_narrative_event(row)?);
        }

        Ok(events)
    }
}
