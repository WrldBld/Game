//! StoryEventEdgePort implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;
use uuid::Uuid;

use super::Neo4jStoryEventRepository;
use wrldbldr_domain::entities::InvolvedCharacter;
use wrldbldr_domain::{
    ChallengeId, CharacterId, LocationId, NarrativeEventId, SceneId, StoryEventId,
};
use wrldbldr_engine_ports::outbound::StoryEventEdgePort;

#[async_trait]
impl StoryEventEdgePort for Neo4jStoryEventRepository {
    /// Set the location where event occurred (creates OCCURRED_AT edge)
    async fn set_location(&self, event_id: StoryEventId, location_id: LocationId) -> Result<bool> {
        // First remove any existing location edge
        let remove_q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:OCCURRED_AT]->(:Location)
            DELETE r",
        )
        .param("event_id", event_id.to_string());
        let _ = self.connection.graph().run(remove_q).await;

        // Create new edge
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (l:Location {id: $location_id})
            CREATE (e)-[:OCCURRED_AT]->(l)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the location where event occurred
    async fn get_location(&self, event_id: StoryEventId) -> Result<Option<LocationId>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[:OCCURRED_AT]->(l:Location)
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

    /// Remove location association (deletes OCCURRED_AT edge)
    async fn remove_location(&self, event_id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:OCCURRED_AT]->(:Location)
            DELETE r
            RETURN count(r) as deleted",
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
    // OCCURRED_IN_SCENE Edge Methods
    // =========================================================================

    /// Set the scene where event occurred (creates OCCURRED_IN_SCENE edge)
    async fn set_scene(&self, event_id: StoryEventId, scene_id: SceneId) -> Result<bool> {
        // First remove any existing scene edge
        let remove_q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:OCCURRED_IN_SCENE]->(:Scene)
            DELETE r",
        )
        .param("event_id", event_id.to_string());
        let _ = self.connection.graph().run(remove_q).await;

        // Create new edge
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (s:Scene {id: $scene_id})
            CREATE (e)-[:OCCURRED_IN_SCENE]->(s)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the scene where event occurred
    async fn get_scene(&self, event_id: StoryEventId) -> Result<Option<SceneId>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[:OCCURRED_IN_SCENE]->(s:Scene)
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

    /// Remove scene association (deletes OCCURRED_IN_SCENE edge)
    async fn remove_scene(&self, event_id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:OCCURRED_IN_SCENE]->(:Scene)
            DELETE r
            RETURN count(r) as deleted",
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
    // INVOLVES Edge Methods
    // =========================================================================

    /// Add an involved character (creates INVOLVES edge with role)
    async fn add_involved_character(
        &self,
        event_id: StoryEventId,
        involved: InvolvedCharacter,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (c:Character {id: $character_id})
            MERGE (e)-[r:INVOLVES]->(c)
            SET r.role = $role
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("character_id", involved.character_id.to_string())
        .param("role", involved.role);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get all involved characters for an event
    async fn get_involved_characters(
        &self,
        event_id: StoryEventId,
    ) -> Result<Vec<InvolvedCharacter>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:INVOLVES]->(c:Character)
            RETURN c.id as character_id, r.role as role",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut involved = Vec::new();

        while let Some(row) = result.next().await? {
            let char_id_str: String = row.get("character_id")?;
            let role: String = row.get("role").unwrap_or_else(|_| "Actor".to_string());
            involved.push(InvolvedCharacter {
                character_id: CharacterId::from(Uuid::parse_str(&char_id_str)?),
                role,
            });
        }

        Ok(involved)
    }

    /// Remove an involved character (deletes INVOLVES edge)
    async fn remove_involved_character(
        &self,
        event_id: StoryEventId,
        character_id: CharacterId,
    ) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:INVOLVES]->(c:Character {id: $character_id})
            DELETE r
            RETURN count(r) as deleted",
        )
        .param("event_id", event_id.to_string())
        .param("character_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    // =========================================================================
    // TRIGGERED_BY_NARRATIVE Edge Methods
    // =========================================================================

    /// Set the narrative event that triggered this story event
    async fn set_triggered_by(
        &self,
        event_id: StoryEventId,
        narrative_event_id: NarrativeEventId,
    ) -> Result<bool> {
        // First remove any existing triggered_by edge
        let remove_q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:TRIGGERED_BY_NARRATIVE]->(:NarrativeEvent)
            DELETE r",
        )
        .param("event_id", event_id.to_string());
        let _ = self.connection.graph().run(remove_q).await;

        // Create new edge
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (n:NarrativeEvent {id: $narrative_event_id})
            CREATE (e)-[:TRIGGERED_BY_NARRATIVE]->(n)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("narrative_event_id", narrative_event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the narrative event that triggered this story event
    async fn get_triggered_by(&self, event_id: StoryEventId) -> Result<Option<NarrativeEventId>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[:TRIGGERED_BY_NARRATIVE]->(n:NarrativeEvent)
            RETURN n.id as narrative_event_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let ne_id_str: String = row.get("narrative_event_id")?;
            Ok(Some(NarrativeEventId::from(Uuid::parse_str(&ne_id_str)?)))
        } else {
            Ok(None)
        }
    }

    /// Remove the triggered_by association
    async fn remove_triggered_by(&self, event_id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:TRIGGERED_BY_NARRATIVE]->(:NarrativeEvent)
            DELETE r
            RETURN count(r) as deleted",
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
    // RECORDS_CHALLENGE Edge Methods
    // =========================================================================

    /// Set the challenge this event records (creates RECORDS_CHALLENGE edge)
    async fn set_recorded_challenge(
        &self,
        event_id: StoryEventId,
        challenge_id: ChallengeId,
    ) -> Result<bool> {
        // First remove any existing recorded_challenge edge
        let remove_q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:RECORDS_CHALLENGE]->(:Challenge)
            DELETE r",
        )
        .param("event_id", event_id.to_string());
        let _ = self.connection.graph().run(remove_q).await;

        // Create new edge
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id}), (c:Challenge {id: $challenge_id})
            CREATE (e)-[:RECORDS_CHALLENGE]->(c)
            RETURN e.id as id",
        )
        .param("event_id", event_id.to_string())
        .param("challenge_id", challenge_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Get the challenge this event records
    async fn get_recorded_challenge(&self, event_id: StoryEventId) -> Result<Option<ChallengeId>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[:RECORDS_CHALLENGE]->(c:Challenge)
            RETURN c.id as challenge_id",
        )
        .param("event_id", event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let challenge_id_str: String = row.get("challenge_id")?;
            Ok(Some(ChallengeId::from(Uuid::parse_str(&challenge_id_str)?)))
        } else {
            Ok(None)
        }
    }

    /// Remove the recorded challenge association
    async fn remove_recorded_challenge(&self, event_id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $event_id})-[r:RECORDS_CHALLENGE]->(:Challenge)
            DELETE r
            RETURN count(r) as deleted",
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
