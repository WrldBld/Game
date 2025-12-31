//! Common helpers for StoryEvent repository operations

use anyhow::Result;
use chrono::{DateTime, Utc};
use neo4rs::Row;
use uuid::Uuid;

use super::stored_types::StoredStoryEventType;
use wrldbldr_domain::entities::{StoryEvent, StoryEventType};
use wrldbldr_domain::{StoryEventId, WorldId};

/// Convert a Neo4j row to a StoryEvent
///
/// NOTE: scene_id, location_id, involved_characters, and triggered_by
/// are stored as graph edges, not node properties. Use the edge query methods
/// to retrieve these associations.
pub(super) fn row_to_story_event(row: Row) -> Result<StoryEvent> {
    let node: neo4rs::Node = row.get("e")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let event_type_json: String = node.get("event_type_json")?;
    let timestamp_str: String = node.get("timestamp")?;
    let game_time: String = node.get("game_time").unwrap_or_default();
    let summary: String = node.get("summary")?;
    let is_hidden: bool = node.get("is_hidden").unwrap_or(false);
    let tags_json: String = node.get("tags_json").unwrap_or_else(|_| "[]".to_string());

    // Deserialize to stored type, then convert to domain type
    let stored_event_type: StoredStoryEventType = serde_json::from_str(&event_type_json)?;
    let event_type: StoryEventType = stored_event_type.into();
    let tags: Vec<String> = serde_json::from_str(&tags_json)?;

    Ok(StoryEvent {
        id: StoryEventId::from(Uuid::parse_str(&id_str)?),
        world_id: WorldId::from(Uuid::parse_str(&world_id_str)?),
        // NOTE: scene_id stored as OCCURRED_IN_SCENE edge
        // NOTE: location_id stored as OCCURRED_AT edge
        event_type,
        timestamp: DateTime::parse_from_rfc3339(&timestamp_str)?.with_timezone(&Utc),
        game_time: if game_time.is_empty() {
            None
        } else {
            Some(game_time)
        },
        summary,
        // NOTE: involved_characters now stored as INVOLVES edges
        is_hidden,
        tags,
        // NOTE: triggered_by now stored as TRIGGERED_BY_NARRATIVE edge
    })
}
