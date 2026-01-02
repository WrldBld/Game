//! StoryEventCrudPort implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::query;

use super::common::row_to_story_event;
use super::stored_types::StoredStoryEventType;
use super::Neo4jStoryEventRepository;
use wrldbldr_domain::entities::StoryEvent;
use wrldbldr_domain::{StoryEventId, WorldId};
use wrldbldr_engine_ports::outbound::StoryEventCrudPort;

#[async_trait]
impl StoryEventCrudPort for Neo4jStoryEventRepository {
    /// Create a new story event
    ///
    /// NOTE: Session, location, scene, involved characters, triggered_by, and recorded_challenge
    /// associations are now stored as graph edges and must be created separately using the
    /// edge methods after calling create().
    async fn create(&self, event: &StoryEvent) -> Result<()> {
        let stored_event_type: StoredStoryEventType = (&event.event_type).into();
        let event_type_json = serde_json::to_string(&stored_event_type)?;
        let tags_json = serde_json::to_string(&event.tags)?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (e:StoryEvent {
                id: $id,
                world_id: $world_id,
                event_type_json: $event_type_json,
                timestamp: $timestamp,
                game_time: $game_time,
                summary: $summary,
                is_hidden: $is_hidden,
                tags_json: $tags_json
            })
            CREATE (w)-[:HAS_STORY_EVENT]->(e)
            RETURN e.id as id",
        )
        .param("id", event.id.to_string())
        .param("world_id", event.world_id.to_string())
        .param("event_type_json", event_type_json)
        .param("timestamp", event.timestamp.to_rfc3339())
        .param("game_time", event.game_time.clone().unwrap_or_default())
        .param("summary", event.summary.clone())
        .param("is_hidden", event.is_hidden)
        .param("tags_json", tags_json);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created story event: {}", event.id);

        // NOTE: Session, location, scene, involved characters, triggered_by, and
        // recorded_challenge edges should be created separately using the edge methods

        Ok(())
    }

    /// Get a story event by ID
    async fn get(&self, id: StoryEventId) -> Result<Option<StoryEvent>> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            RETURN e",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_story_event(row, self.clock.now())?))
        } else {
            Ok(None)
        }
    }

    /// Update story event summary (DM editing)
    async fn update_summary(&self, id: StoryEventId, summary: &str) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            SET e.summary = $summary
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("summary", summary);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Update event visibility
    async fn set_hidden(&self, id: StoryEventId, is_hidden: bool) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            SET e.is_hidden = $is_hidden
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("is_hidden", is_hidden);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Update event tags
    async fn update_tags(&self, id: StoryEventId, tags: Vec<String>) -> Result<bool> {
        let tags_json = serde_json::to_string(&tags)?;
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            SET e.tags_json = $tags_json
            RETURN e.id as id",
        )
        .param("id", id.to_string())
        .param("tags_json", tags_json);

        let mut result = self.connection.graph().execute(q).await?;
        Ok(result.next().await?.is_some())
    }

    /// Delete a story event (rarely used - events are usually immutable)
    async fn delete(&self, id: StoryEventId) -> Result<bool> {
        let q = query(
            "MATCH (e:StoryEvent {id: $id})
            DETACH DELETE e
            RETURN count(*) as deleted",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted")?;
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    /// Count events for a world
    async fn count_by_world(&self, world_id: WorldId) -> Result<u64> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
            RETURN count(e) as count",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            Ok(count as u64)
        } else {
            Ok(0)
        }
    }
}
