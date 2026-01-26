//! Story event use cases.

use std::sync::Arc;

use serde::Serialize;
use uuid::Uuid;

use crate::infrastructure::ports::RepoError;
use crate::use_cases::narrative_operations::NarrativeOps;
use wrldbldr_domain::{StoryEventId, StoryEventType, WorldId};

// =============================================================================
// Domain Result Types
// =============================================================================

/// Summary of a story event.
#[derive(Debug, Clone, Serialize)]
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
    narrative: Arc<NarrativeOps>,
}

impl StoryEventOps {
    pub fn new(narrative: Arc<NarrativeOps>) -> Self {
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
        let event = self
            .narrative
            .get_story_event(event_id)
            .await?
            .ok_or(StoryEventError::NotFound)?;

        // Rebuild event with updated fields using from_storage
        let new_summary = summary.unwrap_or_else(|| event.summary().to_string());
        let new_tags: Vec<wrldbldr_domain::Tag> = match tags {
            Some(tag_strings) => tag_strings
                .into_iter()
                .filter_map(|s| wrldbldr_domain::Tag::new(&s).ok())
                .collect(),
            None => event.tags().to_vec(),
        };

        let event = wrldbldr_domain::StoryEvent::from_storage(
            event.id(),
            event.world_id(),
            event.event_type().clone(),
            event.timestamp(),
            event.game_time().map(|s| s.to_string()),
            new_summary,
            event.is_hidden(),
            new_tags,
        );

        self.narrative.save_story_event(&event).await?;
        Ok(story_event_to_summary(event))
    }

    pub async fn create_dm_marker(
        &self,
        world_id: WorldId,
        title: String,
        content: Option<String>,
    ) -> Result<Uuid, StoryEventError> {
        let now = self.narrative.now();
        let event = wrldbldr_domain::StoryEvent::new(
            world_id,
            StoryEventType::DmMarker {
                title: title.clone(),
                note: content.unwrap_or_default(),
                importance: wrldbldr_domain::MarkerImportance::Notable,
                marker_type: wrldbldr_domain::DmMarkerType::Note,
            },
            now,
        )
        .with_summary("DM Marker");

        self.narrative.save_story_event(&event).await?;
        Ok(Uuid::from(event.id()))
    }

    pub async fn set_visibility(
        &self,
        event_id: StoryEventId,
        visible: bool,
    ) -> Result<StoryEventSummary, StoryEventError> {
        let event = self
            .narrative
            .get_story_event(event_id)
            .await?
            .ok_or(StoryEventError::NotFound)?;

        // Rebuild event with updated visibility using from_storage
        let event = wrldbldr_domain::StoryEvent::from_storage(
            event.id(),
            event.world_id(),
            event.event_type().clone(),
            event.timestamp(),
            event.game_time().map(|s| s.to_string()),
            event.summary().to_string(),
            !visible, // is_hidden = !visible
            event.tags().to_vec(),
        );

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
        id: event.id().to_string(),
        world_id: event.world_id().to_string(),
        scene_id: None,
        location_id: None,
        event_type: event.event_type().clone(),
        timestamp: event.timestamp().to_rfc3339(),
        game_time: event.game_time().map(|s| s.to_string()),
        summary: event.summary().to_string(),
        involved_characters: Vec::new(),
        is_hidden: event.is_hidden(),
        tags: event.tags().iter().map(|t| t.to_string()).collect(),
        triggered_by: None,
        type_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::clock::FixedClock;
    use crate::infrastructure::ports::{
        ClockPort, MockChallengeRepo, MockCharacterRepo, MockFlagRepo, MockLocationRepo,
        MockNarrativeRepo, MockObservationRepo, MockPlayerCharacterRepo, MockSceneRepo,
        MockWorldRepo,
    };
    use chrono::TimeZone;
    use std::sync::Arc;

    fn fixed_time() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    fn create_test_story_event(world_id: WorldId) -> wrldbldr_domain::StoryEvent {
        wrldbldr_domain::StoryEvent::new(
            world_id,
            StoryEventType::DmMarker {
                title: "Test marker".to_string(),
                note: "Test note".to_string(),
                importance: wrldbldr_domain::MarkerImportance::Notable,
                marker_type: wrldbldr_domain::DmMarkerType::Note,
            },
            fixed_time(),
        )
        .with_summary("Test summary")
    }

    fn create_narrative_ops(narrative_repo: MockNarrativeRepo) -> Arc<NarrativeOps> {
        let location_repo: Arc<dyn crate::infrastructure::ports::LocationRepo> =
            Arc::new(MockLocationRepo::new());
        let world_repo: Arc<dyn crate::infrastructure::ports::WorldRepo> =
            Arc::new(MockWorldRepo::new());
        let player_character_repo: Arc<dyn crate::infrastructure::ports::PlayerCharacterRepo> =
            Arc::new(MockPlayerCharacterRepo::new());
        let character_repo: Arc<dyn crate::infrastructure::ports::CharacterRepo> =
            Arc::new(MockCharacterRepo::new());
        let observation_repo: Arc<dyn crate::infrastructure::ports::ObservationRepo> =
            Arc::new(MockObservationRepo::new());
        let challenge_repo: Arc<dyn crate::infrastructure::ports::ChallengeRepo> =
            Arc::new(MockChallengeRepo::new());
        let flag_repo: Arc<dyn crate::infrastructure::ports::FlagRepo> =
            Arc::new(MockFlagRepo::new());
        let scene_repo: Arc<dyn crate::infrastructure::ports::SceneRepo> =
            Arc::new(MockSceneRepo::new());
        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(fixed_time()));

        Arc::new(NarrativeOps::new(
            Arc::new(narrative_repo),
            location_repo,
            world_repo,
            player_character_repo,
            character_repo,
            observation_repo,
            challenge_repo,
            flag_repo,
            scene_repo,
            clock,
        ))
    }

    mod story_event_ops {
        use super::*;

        #[tokio::test]
        async fn when_list_returns_events() {
            let world_id = WorldId::new();
            let event = create_test_story_event(world_id);

            let mut narrative_repo = MockNarrativeRepo::new();
            narrative_repo
                .expect_list_story_events()
                .withf(move |w, l| *w == world_id && *l == 10)
                .returning(move |_, _| Ok(vec![event.clone()]));

            let narrative = create_narrative_ops(narrative_repo);
            let ops = StoryEventOps::new(narrative);

            let result = ops.list(world_id, 10).await;
            assert!(result.is_ok());
            let events = result.unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].summary, "Test summary");
        }

        #[tokio::test]
        async fn when_get_not_found_returns_none() {
            let event_id = wrldbldr_domain::StoryEventId::new();

            let mut narrative_repo = MockNarrativeRepo::new();
            narrative_repo
                .expect_get_story_event()
                .withf(move |id| *id == event_id)
                .returning(|_| Ok(None));

            let narrative = create_narrative_ops(narrative_repo);
            let ops = StoryEventOps::new(narrative);

            let result = ops.get(event_id).await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        #[tokio::test]
        async fn when_update_succeeds() {
            let world_id = WorldId::new();
            let event = create_test_story_event(world_id);
            let event_id = event.id();

            let mut narrative_repo = MockNarrativeRepo::new();
            narrative_repo
                .expect_get_story_event()
                .withf(move |id| *id == event_id)
                .returning(move |_| Ok(Some(event.clone())));
            narrative_repo
                .expect_save_story_event()
                .returning(|_| Ok(()));

            let narrative = create_narrative_ops(narrative_repo);
            let ops = StoryEventOps::new(narrative);

            let result = ops
                .update(event_id, Some("Updated summary".to_string()), None)
                .await;
            assert!(result.is_ok());
            let updated = result.unwrap();
            assert_eq!(updated.summary, "Updated summary");
        }

        #[tokio::test]
        async fn when_update_not_found_returns_error() {
            let event_id = wrldbldr_domain::StoryEventId::new();

            let mut narrative_repo = MockNarrativeRepo::new();
            narrative_repo
                .expect_get_story_event()
                .withf(move |id| *id == event_id)
                .returning(|_| Ok(None));

            let narrative = create_narrative_ops(narrative_repo);
            let ops = StoryEventOps::new(narrative);

            let result = ops
                .update(event_id, Some("Updated summary".to_string()), None)
                .await;
            assert!(matches!(result, Err(StoryEventError::NotFound)));
        }
    }
}
