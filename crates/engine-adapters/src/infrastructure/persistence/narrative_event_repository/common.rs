//! Common helpers for NarrativeEvent repository operations

use anyhow::Result;
use chrono::{DateTime, Utc};
use neo4rs::Row;
use uuid::Uuid;

use super::stored_types::{StoredEventOutcome, StoredNarrativeTrigger};
use wrldbldr_domain::entities::{EventOutcome, NarrativeEvent, NarrativeTrigger, TriggerLogic};
use wrldbldr_domain::{NarrativeEventId, WorldId};

/// Convert a Neo4j row to a NarrativeEvent
///
/// NOTE: Scene/location/act associations and featured NPCs are now stored as graph edges
/// and must be fetched separately using the edge methods on the repository.
pub(super) fn row_to_narrative_event(row: Row) -> Result<NarrativeEvent> {
    let node: neo4rs::Node = row.get("e")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let tags_json: String = node.get("tags_json").unwrap_or_else(|_| "[]".to_string());
    let triggers_json: String = node
        .get("triggers_json")
        .unwrap_or_else(|_| "[]".to_string());
    let trigger_logic_str: String = node
        .get("trigger_logic")
        .unwrap_or_else(|_| "All".to_string());
    let scene_direction: String = node.get("scene_direction").unwrap_or_default();
    let suggested_opening: String = node.get("suggested_opening").unwrap_or_default();
    // NOTE: featured_npcs moved to FEATURES_NPC edges
    let outcomes_json: String = node
        .get("outcomes_json")
        .unwrap_or_else(|_| "[]".to_string());
    let default_outcome: String = node.get("default_outcome").unwrap_or_default();
    let is_active: bool = node.get("is_active").unwrap_or(true);
    let is_triggered: bool = node.get("is_triggered").unwrap_or(false);
    let triggered_at_str: String = node.get("triggered_at").unwrap_or_default();
    let selected_outcome: String = node.get("selected_outcome").unwrap_or_default();
    let is_repeatable: bool = node.get("is_repeatable").unwrap_or(false);
    let trigger_count: i64 = node.get("trigger_count").unwrap_or(0);
    let delay_turns: i64 = node.get("delay_turns").unwrap_or(0);
    let expires_after_turns: i64 = node.get("expires_after_turns").unwrap_or(-1);
    // NOTE: scene_id, location_id, act_id moved to graph edges
    let priority: i64 = node.get("priority").unwrap_or(0);
    let is_favorite: bool = node.get("is_favorite").unwrap_or(false);
    // NOTE: chain_id, chain_position moved to CONTAINS_EVENT edge
    let created_at_str: String = node.get("created_at")?;
    let updated_at_str: String = node.get("updated_at")?;

    let tags: Vec<String> = serde_json::from_str(&tags_json)?;
    // Deserialize to stored types, then convert to domain types
    let stored_triggers: Vec<StoredNarrativeTrigger> = serde_json::from_str(&triggers_json)?;
    let trigger_conditions: Vec<NarrativeTrigger> =
        stored_triggers.into_iter().map(|t| t.into()).collect();
    let stored_outcomes: Vec<StoredEventOutcome> = serde_json::from_str(&outcomes_json)?;
    let outcomes: Vec<EventOutcome> = stored_outcomes.into_iter().map(|o| o.into()).collect();

    let trigger_logic = match trigger_logic_str.as_str() {
        "Any" => TriggerLogic::Any,
        s if s.starts_with("AtLeast(") => {
            let n: u32 = s
                .trim_start_matches("AtLeast(")
                .trim_end_matches(')')
                .parse()
                .unwrap_or(1);
            TriggerLogic::AtLeast(n)
        }
        _ => TriggerLogic::All,
    };

    Ok(NarrativeEvent {
        id: NarrativeEventId::from(Uuid::parse_str(&id_str)?),
        world_id: WorldId::from(Uuid::parse_str(&world_id_str)?),
        name,
        description,
        tags,
        trigger_conditions,
        trigger_logic,
        scene_direction,
        suggested_opening: if suggested_opening.is_empty() {
            None
        } else {
            Some(suggested_opening)
        },
        // NOTE: featured_npcs now stored as FEATURES_NPC edges
        outcomes,
        default_outcome: if default_outcome.is_empty() {
            None
        } else {
            Some(default_outcome)
        },
        is_active,
        is_triggered,
        triggered_at: if triggered_at_str.is_empty() {
            None
        } else {
            DateTime::parse_from_rfc3339(&triggered_at_str)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        },
        selected_outcome: if selected_outcome.is_empty() {
            None
        } else {
            Some(selected_outcome)
        },
        is_repeatable,
        trigger_count: trigger_count as u32,
        delay_turns: delay_turns as u32,
        expires_after_turns: if expires_after_turns < 0 {
            None
        } else {
            Some(expires_after_turns as u32)
        },
        // NOTE: scene_id, location_id, act_id now stored as graph edges
        priority: priority as i32,
        is_favorite,
        // NOTE: chain_id, chain_position now stored as CONTAINS_EVENT edge
        created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc),
    })
}
