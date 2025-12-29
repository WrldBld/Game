//! Domain Events
//!
//! Coarse-grained events representing significant state changes in the domain.
//! These are the domain's internal events - they get mapped to protocol AppEvent
//! at the adapter boundary for persistence and cross-system communication.

use serde::{Deserialize, Serialize};

use crate::{ChallengeId, CharacterId, NarrativeEventId, StoryEventId, WorldId};

/// Domain event for significant state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DomainEvent {
    // Story & Narrative
    StoryEventCreated {
        story_event_id: StoryEventId,
        world_id: WorldId,
        event_type: String,
    },
    NarrativeEventTriggered {
        event_id: NarrativeEventId,
        world_id: WorldId,
        event_name: String,
        outcome_name: String,
        session_id: Option<String>,
    },

    // Challenge
    ChallengeResolved {
        challenge_id: Option<ChallengeId>,
        challenge_name: String,
        world_id: WorldId,
        character_id: CharacterId,
        success: bool,
        roll: Option<i32>,
        total: Option<i32>,
        session_id: Option<String>,
    },

    // Generation (Asset/Image)
    GenerationBatchQueued {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        position: u32,
        session_id: Option<String>,
    },
    GenerationBatchProgress {
        batch_id: String,
        progress: f32,
        session_id: Option<String>,
    },
    GenerationBatchCompleted {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        asset_count: u32,
        session_id: Option<String>,
    },
    GenerationBatchFailed {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        error: String,
        session_id: Option<String>,
    },

    // Suggestion (LLM Text)
    SuggestionQueued {
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
        world_id: Option<WorldId>,
    },
    SuggestionProgress {
        request_id: String,
        status: String,
        world_id: Option<WorldId>,
    },
    SuggestionCompleted {
        request_id: String,
        field_type: String,
        suggestions: Vec<String>,
        world_id: Option<WorldId>,
    },
    SuggestionFailed {
        request_id: String,
        field_type: String,
        error: String,
        world_id: Option<WorldId>,
    },
}

impl DomainEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::StoryEventCreated { .. } => "story_event_created",
            Self::NarrativeEventTriggered { .. } => "narrative_event_triggered",
            Self::ChallengeResolved { .. } => "challenge_resolved",
            Self::GenerationBatchQueued { .. } => "generation_batch_queued",
            Self::GenerationBatchProgress { .. } => "generation_batch_progress",
            Self::GenerationBatchCompleted { .. } => "generation_batch_completed",
            Self::GenerationBatchFailed { .. } => "generation_batch_failed",
            Self::SuggestionQueued { .. } => "suggestion_queued",
            Self::SuggestionProgress { .. } => "suggestion_progress",
            Self::SuggestionCompleted { .. } => "suggestion_completed",
            Self::SuggestionFailed { .. } => "suggestion_failed",
        }
    }

    pub fn world_id(&self) -> Option<WorldId> {
        match self {
            Self::StoryEventCreated { world_id, .. } => Some(*world_id),
            Self::NarrativeEventTriggered { world_id, .. } => Some(*world_id),
            Self::ChallengeResolved { world_id, .. } => Some(*world_id),
            Self::SuggestionQueued { world_id, .. } => *world_id,
            Self::SuggestionProgress { world_id, .. } => *world_id,
            Self::SuggestionCompleted { world_id, .. } => *world_id,
            Self::SuggestionFailed { world_id, .. } => *world_id,
            _ => None,
        }
    }
}
