//! Common helpers for StoryEvent repository operations

use anyhow::Result;
use neo4rs::Row;

use super::super::neo4j_helpers::{parse_typed_id, NodeExt};
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

    let id: StoryEventId = parse_typed_id(&node, "id")?;
    let world_id: WorldId = parse_typed_id(&node, "world_id")?;
    let event_type_json: String = node.get("event_type_json")?;
    let timestamp = node.get_datetime_or("timestamp", chrono::Utc::now());
    let game_time = node.get_optional_string("game_time");
    let summary: String = node.get("summary")?;
    let is_hidden = node.get_bool_or("is_hidden", false);
    let tags: Vec<String> = node.get_json_or_default("tags_json");

    // Deserialize to stored type, then convert to domain type
    let stored_event_type: StoredStoryEventType = serde_json::from_str(&event_type_json)?;
    let event_type: StoryEventType = stored_event_type.into();

    Ok(StoryEvent {
        id,
        world_id,
        // NOTE: scene_id stored as OCCURRED_IN_SCENE edge
        // NOTE: location_id stored as OCCURRED_AT edge
        event_type,
        timestamp,
        game_time,
        summary,
        // NOTE: involved_characters now stored as INVOLVES edges
        is_hidden,
        tags,
        // NOTE: triggered_by now stored as TRIGGERED_BY_NARRATIVE edge
    })
}
