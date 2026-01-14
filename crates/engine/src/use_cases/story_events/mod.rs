//! Story event use cases.

use std::sync::Arc;

use serde::Serialize;
use uuid::Uuid;

use crate::infrastructure::ports::RepoError;
use crate::use_cases::narrative_operations::Narrative;
use wrldbldr_domain::{StoryEventId, StoryEventType, WorldId};

// =============================================================================
// Domain Result Types
// =============================================================================

/// Summary of a story event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryEventSummary {
    pub id: String,
    pub world_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
    pub event_type: StoryEventType,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_time: Option<String>,
    pub summary: String,
    pub involved_characters: Vec<String>,
    pub is_hidden: bool,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggered_by: Option<String>,
    pub type_name: String,
}

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
    ) -> Result<Vec<StoryEventSummary>, StoryEventError> {
        let events = self.narrative.list_story_events(world_id, limit).await?;
        Ok(events.into_iter().map(story_event_to_summary).collect())
    }

    pub async fn get(
        &self,
        event_id: StoryEventId,
    ) -> Result<Option<StoryEventSummary>, StoryEventError> {
        let event = self.narrative.get_story_event(event_id).await?;
        Ok(event.map(story_event_to_summary))
    }

    pub async fn update(
        &self,
        event_id: StoryEventId,
        summary: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<StoryEventSummary, StoryEventError> {
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
        Ok(story_event_to_summary(event))
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
    ) -> Result<StoryEventSummary, StoryEventError> {
        let mut event = self
            .narrative
            .get_story_event(event_id)
            .await?
            .ok_or(StoryEventError::NotFound)?;
        event.is_hidden = !visible;
        self.narrative.save_story_event(&event).await?;
        Ok(story_event_to_summary(event))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoryEventError {
    #[error("Story event not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

fn story_event_to_summary(event: wrldbldr_domain::StoryEvent) -> StoryEventSummary {
    let type_name = event.type_name().to_string();
    StoryEventSummary {
        id: event.id.to_string(),
        world_id: event.world_id.to_string(),
        scene_id: None,
        location_id: None,
        event_type: event.event_type,
        timestamp: event.timestamp.to_rfc3339(),
        game_time: event.game_time,
        summary: event.summary,
        involved_characters: Vec::new(),
        is_hidden: event.is_hidden,
        tags: event.tags,
        triggered_by: None,
        type_name,
    }
}
