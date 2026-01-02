//! Common helpers for NarrativeEvent repository operations

use anyhow::Result;
use chrono::{DateTime, Utc};
use neo4rs::Row;

use super::super::neo4j_helpers::{parse_typed_id, NodeExt};
use super::stored_types::{StoredEventOutcome, StoredNarrativeTrigger};
use wrldbldr_domain::entities::{EventOutcome, NarrativeEvent, NarrativeTrigger, TriggerLogic};
use wrldbldr_domain::{NarrativeEventId, WorldId};

/// Convert a Neo4j row to a NarrativeEvent
///
/// The `fallback` timestamp is used when datetime fields are missing from the database.
/// This should be obtained from `ClockPort::now()` by the caller.
///
/// NOTE: Scene/location/act associations and featured NPCs are now stored as graph edges
/// and must be fetched separately using the edge methods on the repository.
pub(super) fn row_to_narrative_event(row: Row, fallback: DateTime<Utc>) -> Result<NarrativeEvent> {
    let node: neo4rs::Node = row.get("e")?;

    // Required fields - use parse_typed_id for typed IDs
    let id: NarrativeEventId = parse_typed_id(&node, "id")?;
    let world_id: WorldId = parse_typed_id(&node, "world_id")?;
    let name: String = node.get("name")?;

    // Optional/defaulted string fields
    let description = node.get_string_or("description", "");
    let scene_direction = node.get_string_or("scene_direction", "");
    let suggested_opening = node.get_optional_string("suggested_opening");
    let default_outcome = node.get_optional_string("default_outcome");
    let selected_outcome = node.get_optional_string("selected_outcome");

    // JSON fields with defaults
    let tags: Vec<String> = node.get_json_or_default("tags_json");
    let stored_triggers: Vec<StoredNarrativeTrigger> = node.get_json_or_default("triggers_json");
    let stored_outcomes: Vec<StoredEventOutcome> = node.get_json_or_default("outcomes_json");
    let trigger_logic_str = node.get_string_or("trigger_logic", "All");

    // Boolean fields
    let is_active = node.get_bool_or("is_active", true);
    let is_triggered = node.get_bool_or("is_triggered", false);
    let is_repeatable = node.get_bool_or("is_repeatable", false);
    let is_favorite = node.get_bool_or("is_favorite", false);

    // Integer fields
    let trigger_count = node.get_i64_or("trigger_count", 0) as u32;
    let delay_turns = node.get_i64_or("delay_turns", 0) as u32;
    let priority = node.get_i64_or("priority", 0) as i32;

    // Optional positive integer (stored as -1 for None)
    let expires_after_turns = node.get_positive_i64("expires_after_turns");

    // Datetime fields - required ones use fallback, triggered_at is optional
    let created_at = node.get_datetime_or("created_at", fallback);
    let updated_at = node.get_datetime_or("updated_at", fallback);

    // triggered_at is optional - parse manually since get_datetime_or returns non-optional
    let triggered_at: Option<DateTime<Utc>> = node
        .get_optional_string("triggered_at")
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    // Convert stored types to domain types
    let trigger_conditions: Vec<NarrativeTrigger> =
        stored_triggers.into_iter().map(|t| t.into()).collect();
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
        id,
        world_id,
        name,
        description,
        tags,
        trigger_conditions,
        trigger_logic,
        scene_direction,
        suggested_opening,
        // NOTE: featured_npcs now stored as FEATURES_NPC edges
        outcomes,
        default_outcome,
        is_active,
        is_triggered,
        triggered_at,
        selected_outcome,
        is_repeatable,
        trigger_count,
        delay_turns,
        expires_after_turns,
        // NOTE: scene_id, location_id, act_id now stored as graph edges
        priority,
        is_favorite,
        // NOTE: chain_id, chain_position now stored as CONTAINS_EVENT edge
        created_at,
        updated_at,
    })
}
