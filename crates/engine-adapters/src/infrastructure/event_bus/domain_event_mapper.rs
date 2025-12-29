//! Domain Event Mapper - Maps between DomainEvent and AppEvent
//!
//! This module provides conversions between domain-layer events and the
//! wire/storage format (AppEvent from protocol). The adapter boundary is
//! responsible for this mapping.
//!
//! Note: We use free functions instead of From/TryFrom traits because of Rust's
//! orphan rule - we can't implement traits for types defined in other crates.

use wrldbldr_domain::{
    ChallengeId, CharacterId, DomainEvent, NarrativeEventId, StoryEventId, WorldId,
};
use wrldbldr_protocol::AppEvent;

/// Convert a DomainEvent to an AppEvent for wire/storage format
pub fn domain_event_to_app_event(event: DomainEvent) -> AppEvent {
    match event {
        DomainEvent::StoryEventCreated {
            story_event_id,
            world_id,
            event_type,
        } => AppEvent::StoryEventCreated {
            story_event_id: story_event_id.to_string(),
            world_id: world_id.to_string(),
            event_type,
        },
        DomainEvent::NarrativeEventTriggered {
            event_id,
            world_id,
            event_name,
            outcome_name,
            session_id,
        } => AppEvent::NarrativeEventTriggered {
            event_id: event_id.to_string(),
            world_id: world_id.to_string(),
            event_name,
            outcome_name,
            session_id,
        },
        DomainEvent::ChallengeResolved {
            challenge_id,
            challenge_name,
            world_id,
            character_id,
            success,
            roll,
            total,
            session_id,
        } => AppEvent::ChallengeResolved {
            challenge_id: challenge_id.map(|id| id.to_string()),
            challenge_name,
            world_id: world_id.to_string(),
            character_id: character_id.to_string(),
            success,
            roll,
            total,
            session_id,
        },
        DomainEvent::GenerationBatchQueued {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            position,
            session_id,
        } => AppEvent::GenerationBatchQueued {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            position,
            session_id,
        },
        DomainEvent::GenerationBatchProgress {
            batch_id,
            progress,
            session_id,
        } => AppEvent::GenerationBatchProgress {
            batch_id,
            // AppEvent uses u8 (0-100), DomainEvent uses f32 (0.0-1.0)
            progress: (progress * 100.0).clamp(0.0, 100.0) as u8,
            session_id,
        },
        DomainEvent::GenerationBatchCompleted {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            asset_count,
            session_id,
        } => AppEvent::GenerationBatchCompleted {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            asset_count,
            session_id,
        },
        DomainEvent::GenerationBatchFailed {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            error,
            session_id,
        } => AppEvent::GenerationBatchFailed {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            error,
            session_id,
        },
        DomainEvent::SuggestionQueued {
            request_id,
            field_type,
            entity_id,
            world_id,
        } => AppEvent::SuggestionQueued {
            request_id,
            field_type,
            entity_id,
            world_id: world_id.map(|id| id.to_string()),
        },
        DomainEvent::SuggestionProgress {
            request_id,
            status,
            world_id,
        } => AppEvent::SuggestionProgress {
            request_id,
            status,
            world_id: world_id.map(|id| id.to_string()),
        },
        DomainEvent::SuggestionCompleted {
            request_id,
            field_type,
            suggestions,
            world_id,
        } => AppEvent::SuggestionCompleted {
            request_id,
            field_type,
            suggestions,
            world_id: world_id.map(|id| id.to_string()),
        },
        DomainEvent::SuggestionFailed {
            request_id,
            field_type,
            error,
            world_id,
        } => AppEvent::SuggestionFailed {
            request_id,
            field_type,
            error,
            world_id: world_id.map(|id| id.to_string()),
        },
    }
}

/// Error type for AppEvent to DomainEvent conversion
#[derive(Debug, thiserror::Error)]
pub enum DomainEventConversionError {
    #[error("Failed to parse UUID '{0}': {1}")]
    UuidParseError(String, uuid::Error),
    #[error("Unknown event type cannot be converted to domain event")]
    UnknownEventType,
}

/// Try to convert an AppEvent back to a DomainEvent
///
/// This is used when reading events from storage and converting them back
/// to domain events for processing.
pub fn app_event_to_domain_event(
    event: AppEvent,
) -> Result<DomainEvent, DomainEventConversionError> {
    match event {
        AppEvent::StoryEventCreated {
            story_event_id,
            world_id,
            event_type,
        } => Ok(DomainEvent::StoryEventCreated {
            story_event_id: parse_id::<StoryEventId>(&story_event_id)?,
            world_id: parse_id::<WorldId>(&world_id)?,
            event_type,
        }),
        AppEvent::NarrativeEventTriggered {
            event_id,
            world_id,
            event_name,
            outcome_name,
            session_id,
        } => Ok(DomainEvent::NarrativeEventTriggered {
            event_id: parse_id::<NarrativeEventId>(&event_id)?,
            world_id: parse_id::<WorldId>(&world_id)?,
            event_name,
            outcome_name,
            session_id,
        }),
        AppEvent::ChallengeResolved {
            challenge_id,
            challenge_name,
            world_id,
            character_id,
            success,
            roll,
            total,
            session_id,
        } => Ok(DomainEvent::ChallengeResolved {
            challenge_id: challenge_id
                .map(|id| parse_id::<ChallengeId>(&id))
                .transpose()?,
            challenge_name,
            world_id: parse_id::<WorldId>(&world_id)?,
            character_id: parse_id::<CharacterId>(&character_id)?,
            success,
            roll,
            total,
            session_id,
        }),
        AppEvent::GenerationBatchQueued {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            position,
            session_id,
        } => Ok(DomainEvent::GenerationBatchQueued {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            position,
            session_id,
        }),
        AppEvent::GenerationBatchProgress {
            batch_id,
            progress,
            session_id,
        } => Ok(DomainEvent::GenerationBatchProgress {
            batch_id,
            // Convert u8 (0-100) back to f32 (0.0-1.0)
            progress: progress as f32 / 100.0,
            session_id,
        }),
        AppEvent::GenerationBatchCompleted {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            asset_count,
            session_id,
        } => Ok(DomainEvent::GenerationBatchCompleted {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            asset_count,
            session_id,
        }),
        AppEvent::GenerationBatchFailed {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            error,
            session_id,
        } => Ok(DomainEvent::GenerationBatchFailed {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            error,
            session_id,
        }),
        AppEvent::SuggestionQueued {
            request_id,
            field_type,
            entity_id,
            world_id,
        } => Ok(DomainEvent::SuggestionQueued {
            request_id,
            field_type,
            entity_id,
            world_id: world_id.map(|id| parse_id::<WorldId>(&id)).transpose()?,
        }),
        AppEvent::SuggestionProgress {
            request_id,
            status,
            world_id,
        } => Ok(DomainEvent::SuggestionProgress {
            request_id,
            status,
            world_id: world_id.map(|id| parse_id::<WorldId>(&id)).transpose()?,
        }),
        AppEvent::SuggestionCompleted {
            request_id,
            field_type,
            suggestions,
            world_id,
        } => Ok(DomainEvent::SuggestionCompleted {
            request_id,
            field_type,
            suggestions,
            world_id: world_id.map(|id| parse_id::<WorldId>(&id)).transpose()?,
        }),
        AppEvent::SuggestionFailed {
            request_id,
            field_type,
            error,
            world_id,
        } => Ok(DomainEvent::SuggestionFailed {
            request_id,
            field_type,
            error,
            world_id: world_id.map(|id| parse_id::<WorldId>(&id)).transpose()?,
        }),
        // Unknown events cannot be converted to domain events
        AppEvent::Unknown => Err(DomainEventConversionError::UnknownEventType),
    }
}

/// Helper function to parse a string ID into a domain ID type
fn parse_id<T: From<uuid::Uuid>>(s: &str) -> Result<T, DomainEventConversionError> {
    let uuid: uuid::Uuid = s
        .parse()
        .map_err(|e| DomainEventConversionError::UuidParseError(s.to_string(), e))?;
    Ok(T::from(uuid))
}
