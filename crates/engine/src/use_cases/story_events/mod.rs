//! Story event use cases.

use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::entities::Narrative;
use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::{StoryEventId, StoryEventType, WorldId};

/// Container for story event use cases.
pub struct StoryEventUseCases {
    pub ops: Arc<StoryEventOps>,
}

impl StoryEventUseCases {
    pub fn new(ops: Arc<StoryEventOps>) -> Self {
        Self { ops }
    }
}

/// Story event operations.
pub struct StoryEventOps {
    narrative: Arc<Narrative>,
}

impl StoryEventOps {
    pub fn new(narrative: Arc<Narrative>) -> Self {
        Self { narrative }
    }

    pub async fn list(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<Value>, StoryEventError> {
        let events = self.narrative.list_story_events(world_id, limit).await?;
        Ok(events.into_iter().map(story_event_to_json).collect())
    }

    pub async fn get(&self, event_id: StoryEventId) -> Result<Option<Value>, StoryEventError> {
        let event = self.narrative.get_story_event(event_id).await?;
        Ok(event.map(story_event_to_json))
    }

    pub async fn update(
        &self,
        event_id: StoryEventId,
        summary: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<Value, StoryEventError> {
        let mut event = self
            .narrative
            .get_story_event(event_id)
            .await?
            .ok_or(StoryEventError::NotFound)?;

        if let Some(summary) = summary {
            event.summary = summary;
        }
        if let Some(tags) = tags {
            event.tags = tags;
        }

        self.narrative.save_story_event(&event).await?;
        Ok(story_event_to_json(event))
    }

    pub async fn create_dm_marker(
        &self,
        world_id: WorldId,
        title: String,
        content: Option<String>,
    ) -> Result<Uuid, StoryEventError> {
        let now = chrono::Utc::now();
        let event = wrldbldr_domain::StoryEvent {
            id: StoryEventId::new(),
            world_id,
            event_type: StoryEventType::DmMarker {
                title: title.clone(),
                note: content.unwrap_or_default(),
                importance: wrldbldr_domain::MarkerImportance::Notable,
                marker_type: wrldbldr_domain::DmMarkerType::Note,
            },
            timestamp: now,
            game_time: None,
            summary: "DM Marker".to_string(),
            is_hidden: false,
            tags: Vec::new(),
        };

        self.narrative.save_story_event(&event).await?;
        Ok(Uuid::from(event.id))
    }

    pub async fn set_visibility(
        &self,
        event_id: StoryEventId,
        visible: bool,
    ) -> Result<Value, StoryEventError> {
        let mut event = self
            .narrative
            .get_story_event(event_id)
            .await?
            .ok_or(StoryEventError::NotFound)?;
        event.is_hidden = !visible;
        self.narrative.save_story_event(&event).await?;
        Ok(story_event_to_json(event))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoryEventError {
    #[error("Story event not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

fn story_event_to_json(event: wrldbldr_domain::StoryEvent) -> Value {
    let event_type = match &event.event_type {
        StoryEventType::LocationChange {
            from_location,
            to_location,
            character_id,
            travel_method,
        } => serde_json::json!({
            "type": "location_change",
            "from_location": from_location.as_ref().map(|id| id.to_string()),
            "to_location": to_location.to_string(),
            "character_id": character_id.to_string(),
            "travel_method": travel_method,
        }),

        StoryEventType::DialogueExchange {
            npc_id,
            npc_name,
            player_dialogue,
            npc_response,
            topics_discussed,
            tone,
        } => serde_json::json!({
            "type": "dialogue_exchange",
            "npc_id": npc_id.to_string(),
            "npc_name": npc_name,
            "player_dialogue": player_dialogue,
            "npc_response": npc_response,
            "topics_discussed": topics_discussed,
            "tone": tone,
        }),

        StoryEventType::DmMarker {
            title,
            note,
            importance,
            marker_type,
        } => serde_json::json!({
            "type": "dm_marker",
            "title": title,
            "note": note,
            "importance": format!("{:?}", importance),
            "marker_type": format!("{:?}", marker_type),
        }),

        StoryEventType::NarrativeEventTriggered {
            narrative_event_id,
            narrative_event_name,
            outcome_branch,
            effects_applied,
        } => serde_json::json!({
            "type": "narrative_event_triggered",
            "narrative_event_id": narrative_event_id.to_string(),
            "narrative_event_name": narrative_event_name,
            "outcome_branch": outcome_branch,
            "effects_applied": effects_applied,
        }),

        StoryEventType::SessionStarted {
            session_number,
            session_name,
            players_present,
        } => serde_json::json!({
            "type": "session_started",
            "session_number": session_number,
            "session_name": session_name,
            "players_present": players_present,
        }),

        StoryEventType::SessionEnded {
            duration_minutes,
            summary,
        } => serde_json::json!({
            "type": "session_ended",
            "duration_minutes": duration_minutes,
            "summary": summary,
        }),

        other => serde_json::json!({
            "type": "custom",
            "event_subtype": event.type_name(),
            "title": event.type_name(),
            "description": format!("{:?}", other),
        }),
    };

    serde_json::json!({
        "id": event.id.to_string(),
        "world_id": event.world_id.to_string(),
        "scene_id": serde_json::Value::Null,
        "location_id": serde_json::Value::Null,
        "event_type": event_type,
        "timestamp": event.timestamp.to_rfc3339(),
        "game_time": event.game_time,
        "summary": event.summary,
        "involved_characters": Vec::<String>::new(),
        "is_hidden": event.is_hidden,
        "tags": event.tags,
        "triggered_by": serde_json::Value::Null,
        "type_name": event.type_name(),
    })
}
