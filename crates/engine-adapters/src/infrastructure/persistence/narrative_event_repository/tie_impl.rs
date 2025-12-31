//! Tie/edge relationship implementation for NarrativeEventTiePort
//!
//! Handles TIED_TO_SCENE, TIED_TO_LOCATION, and BELONGS_TO_ACT edge relationships.

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;
use uuid::Uuid;

use super::Neo4jNarrativeEventRepository;
use wrldbldr_domain::{ActId, LocationId, NarrativeEventId, SceneId};
use wrldbldr_engine_ports::outbound::NarrativeEventTiePort;

#[async_trait]
impl NarrativeEventTiePort for Neo4jNarrativeEventRepository {
    // =========================================================================
    // TIED_TO_SCENE Edge Methods
    // =========================================================================

    /// Tie event to a scene (creates TIED_TO_SCENE edge)
    async fn tie_to_scene(&self, event_id: NarrativeEventId, scene_id: SceneId) -> Result<bool> {
        // First remove any existing scene tie, then create the new one
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})
            OPTIONAL MATCH (e)-[old:TIED_TO_SCENE]->()
            DELETE old
            WITH e
            MATCH (s:Scene {id: $scene_id})
            CREATE (e)-[:TIED_TO_SCENE]->(s)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the scene this event is tied to (if any)
    async fn get_tied_scene(&self, event_id: NarrativeEventId) -> Result<Option<SceneId>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[:TIED_TO_SCENE]->(s:Scene)
            RETURN s.id as scene_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let scene_id_str: String = row.get("scene_id")?;
            Ok(Some(SceneId::from(Uuid::parse_str(&scene_id_str)?)))
        } else {
            Ok(None)
        }
    }

    /// Remove scene tie (deletes TIED_TO_SCENE edge)
    async fn untie_from_scene(&self, event_id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:TIED_TO_SCENE]->()
            DELETE r
            RETURN count(*) as deleted",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    // =========================================================================
    // TIED_TO_LOCATION Edge Methods
    // =========================================================================

    /// Tie event to a location (creates TIED_TO_LOCATION edge)
    async fn tie_to_location(
        &self,
        event_id: NarrativeEventId,
        location_id: LocationId,
    ) -> Result<bool> {
        // First remove any existing location tie, then create the new one
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})
            OPTIONAL MATCH (e)-[old:TIED_TO_LOCATION]->()
            DELETE old
            WITH e
            MATCH (l:Location {id: $location_id})
            CREATE (e)-[:TIED_TO_LOCATION]->(l)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the location this event is tied to (if any)
    async fn get_tied_location(&self, event_id: NarrativeEventId) -> Result<Option<LocationId>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[:TIED_TO_LOCATION]->(l:Location)
            RETURN l.id as location_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let location_id_str: String = row.get("location_id")?;
            Ok(Some(LocationId::from(Uuid::parse_str(&location_id_str)?)))
        } else {
            Ok(None)
        }
    }

    /// Remove location tie (deletes TIED_TO_LOCATION edge)
    async fn untie_from_location(&self, event_id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:TIED_TO_LOCATION]->()
            DELETE r
            RETURN count(*) as deleted",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    // =========================================================================
    // BELONGS_TO_ACT Edge Methods
    // =========================================================================

    /// Assign event to an act (creates BELONGS_TO_ACT edge)
    async fn assign_to_act(&self, event_id: NarrativeEventId, act_id: ActId) -> Result<bool> {
        // First remove any existing act assignment, then create the new one
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})
            OPTIONAL MATCH (e)-[old:BELONGS_TO_ACT]->()
            DELETE old
            WITH e
            MATCH (a:Act {id: $act_id})
            CREATE (e)-[:BELONGS_TO_ACT]->(a)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("act_id", act_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the act this event belongs to (if any)
    async fn get_act(&self, event_id: NarrativeEventId) -> Result<Option<ActId>> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[:BELONGS_TO_ACT]->(a:Act)
            RETURN a.id as act_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let act_id_str: String = row.get("act_id")?;
            Ok(Some(ActId::from(Uuid::parse_str(&act_id_str)?)))
        } else {
            Ok(None)
        }
    }

    /// Remove act assignment (deletes BELONGS_TO_ACT edge)
    async fn unassign_from_act(&self, event_id: NarrativeEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:NarrativeEvent {id: $event_id})-[r:BELONGS_TO_ACT]->()
            DELETE r
            RETURN count(*) as deleted",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }
}
