//! StoryEventQueryPort implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;

use super::common::row_to_story_event;
use super::Neo4jStoryEventRepository;
use wrldbldr_domain::entities::StoryEvent;
use wrldbldr_domain::{ChallengeId, CharacterId, LocationId, NarrativeEventId, SceneId, WorldId};
use wrldbldr_engine_ports::outbound::StoryEventQueryPort;

#[async_trait]
impl StoryEventQueryPort for Neo4jStoryEventRepository {
    /// List story events for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List story events for a world with pagination
    async fn list_by_world_paginated(
        &self,
        world_id: WorldId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            RETURN e
            ORDER BY e.timestamp DESC
            SKIP $offset
            LIMIT $limit",
        )
        .param("world_id", world_id.to_string())
        .param("offset", offset as i64)
        .param("limit", limit as i64);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List visible (non-hidden) story events for a world
    async fn list_visible(&self, world_id: WorldId, limit: u32) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE e.is_hidden = false
            RETURN e
            ORDER BY e.timestamp DESC
            LIMIT $limit",
        )
        .param("world_id", world_id.to_string())
        .param("limit", limit as i64);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// Search story events by tags
    async fn search_by_tags(
        &self,
        world_id: WorldId,
        tags: Vec<String>,
    ) -> Result<Vec<StoryEvent>> {
        // Note: We store tags as JSON, so we search in the JSON string
        // A more efficient approach would be to store tags as separate nodes
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE ANY(tag IN $tags WHERE e.tags_json CONTAINS tag)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("world_id", world_id.to_string())
        .param("tags", tags);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// Search story events by text in summary
    async fn search_by_text(
        &self,
        world_id: WorldId,
        search_text: &str,
    ) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            WHERE toLower(e.summary) CONTAINS toLower($search_text)
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("world_id", world_id.to_string())
        .param("search_text", search_text);

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events involving a specific character (via INVOLVES edge)
    async fn list_by_character(&self, character_id: CharacterId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:INVOLVES]->(c:Character {id: $char_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("char_id", character_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events at a specific location (via OCCURRED_AT edge)
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:OCCURRED_AT]->(l:Location {id: $location_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events triggered by a specific narrative event
    async fn list_by_narrative_event(
        &self,
        narrative_event_id: NarrativeEventId,
    ) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:TRIGGERED_BY_NARRATIVE]->(n:NarrativeEvent {id: $narrative_event_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("narrative_event_id", narrative_event_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events recording a specific challenge
    async fn list_by_challenge(&self, challenge_id: ChallengeId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:RECORDS_CHALLENGE]->(c:Challenge {id: $challenge_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("challenge_id", challenge_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }

    /// List events that occurred in a specific scene (via OCCURRED_IN_SCENE edge)
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent)-[:OCCURRED_IN_SCENE]->(s:Scene {id: $scene_id})
            RETURN e
            ORDER BY e.timestamp DESC",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut events = Vec::new();

        while let Some(row) = result.next().await? {
            events.push(row_to_story_event(row)?);
        }

        Ok(events)
    }
}
