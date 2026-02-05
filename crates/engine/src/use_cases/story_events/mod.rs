//! Story event use cases.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::infrastructure::ports::{ClockPort, NarrativeRepo, RepoError, StoryEventWithEdges};
use wrldbldr_domain::{
    ChallengeEventOutcome, CombatEventType, CombatOutcome, DmMarkerType, InfoType, ItemSource,
    MarkerImportance, StoryEventId, StoryEventInfoImportance, StoryEventType, StoryEventVisibility,
    WorldId,
};

// =============================================================================
// Domain Result Types
// =============================================================================

/// Summary of a story event (typed DTO with domain types).
///
/// This is a use case DTO that uses typed IDs and DateTime instead of
/// presentation strings. Protocol conversion happens at the API boundary.
#[derive(Debug, Clone)]
pub struct StoryEventSummary {
    pub id: StoryEventId,
    pub world_id: WorldId,
    pub event_type: StoryEventType,
    pub timestamp: DateTime<Utc>,
    pub game_time: Option<String>,
    pub summary: String,
    pub visibility: StoryEventVisibility,
    pub tags: Vec<String>,
    pub type_name: String,
    /// Scene ID from graph edge (optional, populated from repo)
    pub scene_id: Option<wrldbldr_domain::SceneId>,
    /// Location ID from graph edge (optional, populated from repo)
    pub location_id: Option<wrldbldr_domain::LocationId>,
    /// Character IDs involved in this event (from graph edges)
    pub involved_characters: Vec<wrldbldr_domain::CharacterId>,
    /// Triggering narrative event ID (from TRIGGERED_BY_NARRATIVE graph edge)
    pub triggered_by: Option<wrldbldr_domain::NarrativeEventId>,
}

impl StoryEventSummary {
    /// Convert to protocol representation (API layer DTO).
    pub fn to_protocol(&self) -> StoryEventProtocol {
        StoryEventProtocol {
            id: self.id.to_string(),
            world_id: self.world_id.to_string(),
            scene_id: self.scene_id.map(|id| id.to_string()),
            location_id: self.location_id.map(|id| id.to_string()),
            event_type: StoryEventTypeProtocol::from_domain(&self.event_type),
            timestamp: self.timestamp.to_rfc3339(),
            game_time: self.game_time.clone(),
            summary: self.summary.clone(),
            involved_characters: self
                .involved_characters
                .iter()
                .map(|id| id.to_string())
                .collect(),
            is_hidden: self.visibility.is_hidden(),
            tags: self.tags.clone(),
            triggered_by: self.triggered_by.map(|id| id.to_string()),
            type_name: self.type_name.clone(),
        }
    }
}

/// Protocol representation of a story event (for API responses).
///
/// Matches the client's expected format with internally tagged event_type
/// and string fields for IDs and subtypes.
#[derive(Debug, Clone, Serialize)]
pub struct StoryEventProtocol {
    pub id: String,
    pub world_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
    pub event_type: StoryEventTypeProtocol,
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

/// Protocol representation of story event type (internally tagged, string fields).
///
/// This matches the client's expected format (StoryEventTypeData) with:
/// - `#[serde(tag = "type")]` for internal tagging
/// - String fields for IDs and subtypes instead of typed IDs/enums
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum StoryEventTypeProtocol {
    LocationChange {
        from_location: Option<String>,
        to_location: String,
        character_id: String,
        travel_method: Option<String>,
    },
    DialogueExchange {
        npc_id: String,
        npc_name: String,
        player_dialogue: String,
        npc_response: String,
        topics_discussed: Vec<String>,
        tone: Option<String>,
    },
    CombatEvent {
        combat_type: String,
        participants: Vec<String>,
        enemies: Vec<String>,
        outcome: Option<String>,
        location_id: String,
        rounds: Option<u32>,
    },
    ChallengeAttempted {
        challenge_id: Option<String>,
        challenge_name: String,
        character_id: String,
        skill_used: Option<String>,
        difficulty: Option<String>,
        roll_result: Option<i32>,
        modifier: Option<i32>,
        outcome: String,
    },
    ItemAcquired {
        item_name: String,
        item_description: Option<String>,
        character_id: String,
        source: String,
        quantity: u32,
    },
    ItemTransferred {
        item_name: String,
        from_character: Option<String>,
        to_character: String,
        quantity: u32,
        reason: Option<String>,
    },
    ItemUsed {
        item_name: String,
        character_id: String,
        target: Option<String>,
        effect: String,
        consumed: bool,
    },
    RelationshipChanged {
        from_character: String,
        to_character: String,
        previous_sentiment: Option<f32>,
        new_sentiment: f32,
        sentiment_change: f32,
        reason: String,
    },
    SceneTransition {
        from_scene: Option<String>,
        to_scene: String,
        from_scene_name: Option<String>,
        to_scene_name: String,
        trigger_reason: String,
    },
    InformationRevealed {
        info_type: String,
        title: String,
        content: String,
        source: Option<String>,
        importance: String,
        persist_to_journal: bool,
    },
    NpcAction {
        npc_id: String,
        npc_name: String,
        action_type: String,
        description: String,
        dm_approved: bool,
        dm_modified: bool,
    },
    DmMarker {
        title: String,
        note: String,
        importance: String,
        marker_type: String,
    },
    NarrativeEventTriggered {
        narrative_event_id: String,
        narrative_event_name: String,
        outcome_branch: Option<String>,
        effects_applied: Vec<String>,
    },
    StatModified {
        character_id: String,
        stat_name: String,
        previous_value: i32,
        new_value: i32,
        reason: String,
    },
    FlagChanged {
        flag_name: String,
        new_value: bool,
        reason: String,
    },
    SessionStarted {
        session_number: u32,
        session_name: Option<String>,
        players_present: Vec<String>,
    },
    SessionEnded {
        duration_minutes: u32,
        summary: String,
    },
    Custom {
        event_subtype: String,
        title: String,
        description: String,
    },
}

impl StoryEventTypeProtocol {
    /// Convert from domain StoryEventType to protocol format.
    pub fn from_domain(domain: &StoryEventType) -> Self {
        match domain {
            StoryEventType::LocationChange {
                from_location,
                to_location,
                character_id,
                travel_method,
            } => Self::LocationChange {
                from_location: from_location.map(|id| id.to_string()),
                to_location: to_location.to_string(),
                character_id: character_id.to_string(),
                travel_method: travel_method.clone(),
            },
            StoryEventType::DialogueExchange {
                npc_id,
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            } => Self::DialogueExchange {
                npc_id: npc_id.to_string(),
                npc_name: npc_name.clone(),
                player_dialogue: player_dialogue.clone(),
                npc_response: npc_response.clone(),
                topics_discussed: topics_discussed.clone(),
                tone: tone.clone(),
            },
            StoryEventType::CombatEvent {
                combat_type,
                participants,
                enemies,
                outcome,
                location_id,
                rounds,
            } => Self::CombatEvent {
                combat_type: combat_event_type_to_string(*combat_type),
                participants: participants.iter().map(|id| id.to_string()).collect(),
                enemies: enemies.clone(),
                outcome: outcome.map(combat_outcome_to_string),
                location_id: location_id.to_string(),
                rounds: *rounds,
            },
            StoryEventType::ChallengeAttempted {
                challenge_id,
                challenge_name,
                character_id,
                skill_used,
                difficulty,
                roll_result,
                modifier,
                outcome,
            } => Self::ChallengeAttempted {
                challenge_id: challenge_id.map(|id| id.to_string()),
                challenge_name: challenge_name.clone(),
                character_id: character_id.to_string(),
                skill_used: skill_used.clone(),
                difficulty: difficulty.clone(),
                roll_result: *roll_result,
                modifier: *modifier,
                outcome: challenge_event_outcome_to_string(*outcome),
            },
            StoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id,
                source,
                quantity,
            } => Self::ItemAcquired {
                item_name: item_name.clone(),
                item_description: item_description.clone(),
                character_id: character_id.to_string(),
                source: item_source_to_string(source),
                quantity: *quantity,
            },
            StoryEventType::ItemTransferred {
                item_name,
                from_character,
                to_character,
                quantity,
                reason,
            } => Self::ItemTransferred {
                item_name: item_name.clone(),
                from_character: from_character.map(|id| id.to_string()),
                to_character: to_character.to_string(),
                quantity: *quantity,
                reason: reason.clone(),
            },
            StoryEventType::ItemUsed {
                item_name,
                character_id,
                target,
                effect,
                consumed,
            } => Self::ItemUsed {
                item_name: item_name.clone(),
                character_id: character_id.to_string(),
                target: target.clone(),
                effect: effect.clone(),
                consumed: *consumed,
            },
            StoryEventType::RelationshipChanged {
                from_character,
                to_character,
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            } => Self::RelationshipChanged {
                from_character: from_character.to_string(),
                to_character: to_character.to_string(),
                previous_sentiment: *previous_sentiment,
                new_sentiment: *new_sentiment,
                sentiment_change: *sentiment_change,
                reason: reason.clone(),
            },
            StoryEventType::SceneTransition {
                from_scene,
                to_scene,
                from_scene_name,
                to_scene_name,
                trigger_reason,
            } => Self::SceneTransition {
                from_scene: from_scene.map(|id| id.to_string()),
                to_scene: to_scene.to_string(),
                from_scene_name: from_scene_name.clone(),
                to_scene_name: to_scene_name.clone(),
                trigger_reason: trigger_reason.clone(),
            },
            StoryEventType::InformationRevealed {
                info_type,
                title,
                content,
                source,
                importance,
                persist_to_journal,
            } => Self::InformationRevealed {
                info_type: info_type_to_string(*info_type),
                title: title.clone(),
                content: content.clone(),
                source: source.map(|id| id.to_string()),
                importance: info_importance_to_string(*importance),
                persist_to_journal: *persist_to_journal,
            },
            StoryEventType::NpcAction {
                npc_id,
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            } => Self::NpcAction {
                npc_id: npc_id.to_string(),
                npc_name: npc_name.clone(),
                action_type: action_type.clone(),
                description: description.clone(),
                dm_approved: *dm_approved,
                dm_modified: *dm_modified,
            },
            StoryEventType::DmMarker {
                title,
                note,
                importance,
                marker_type,
            } => Self::DmMarker {
                title: title.clone(),
                note: note.clone(),
                importance: marker_importance_to_string(*importance),
                marker_type: dm_marker_type_to_string(*marker_type),
            },
            StoryEventType::NarrativeEventTriggered {
                narrative_event_id,
                narrative_event_name,
                outcome_branch,
                effects_applied,
            } => Self::NarrativeEventTriggered {
                narrative_event_id: narrative_event_id.to_string(),
                narrative_event_name: narrative_event_name.clone(),
                outcome_branch: outcome_branch.clone(),
                effects_applied: effects_applied.clone(),
            },
            StoryEventType::StatModified {
                character_id,
                stat_name,
                previous_value,
                new_value,
                reason,
            } => Self::StatModified {
                character_id: character_id.to_string(),
                stat_name: stat_name.clone(),
                previous_value: *previous_value,
                new_value: *new_value,
                reason: reason.clone(),
            },
            StoryEventType::FlagChanged {
                flag_name,
                new_value,
                reason,
            } => Self::FlagChanged {
                flag_name: flag_name.clone(),
                new_value: *new_value,
                reason: reason.clone(),
            },
            StoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            } => Self::SessionStarted {
                session_number: *session_number,
                session_name: session_name.clone(),
                players_present: players_present.clone(),
            },
            StoryEventType::SessionEnded {
                duration_minutes,
                summary,
            } => Self::SessionEnded {
                duration_minutes: *duration_minutes,
                summary: summary.clone(),
            },
            StoryEventType::Custom {
                event_subtype,
                title,
                description,
                data: _,
            } => Self::Custom {
                event_subtype: event_subtype.clone(),
                title: title.clone(),
                description: description.clone(),
            },
        }
    }
}

// Helper functions to convert domain enums to stable string representations

fn combat_event_type_to_string(ct: CombatEventType) -> String {
    match ct {
        CombatEventType::Started => "started".to_string(),
        CombatEventType::RoundCompleted => "roundCompleted".to_string(),
        CombatEventType::CharacterDefeated => "characterDefeated".to_string(),
        CombatEventType::CharacterFled => "characterFled".to_string(),
        CombatEventType::Ended => "ended".to_string(),
    }
}

fn combat_outcome_to_string(co: CombatOutcome) -> String {
    match co {
        CombatOutcome::Victory => "victory".to_string(),
        CombatOutcome::Defeat => "defeat".to_string(),
        CombatOutcome::Fled => "fled".to_string(),
        CombatOutcome::Negotiated => "negotiated".to_string(),
        CombatOutcome::Draw => "draw".to_string(),
        CombatOutcome::Interrupted => "interrupted".to_string(),
    }
}

fn challenge_event_outcome_to_string(outcome: ChallengeEventOutcome) -> String {
    match outcome {
        ChallengeEventOutcome::CriticalSuccess => "criticalSuccess".to_string(),
        ChallengeEventOutcome::Success => "success".to_string(),
        ChallengeEventOutcome::PartialSuccess => "partialSuccess".to_string(),
        ChallengeEventOutcome::Failure => "failure".to_string(),
        ChallengeEventOutcome::CriticalFailure => "criticalFailure".to_string(),
    }
}

fn item_source_to_string(source: &ItemSource) -> String {
    match source {
        ItemSource::Found { .. } => "found".to_string(),
        ItemSource::Purchased { .. } => "purchased".to_string(),
        ItemSource::Gifted { .. } => "gifted".to_string(),
        ItemSource::Looted { .. } => "looted".to_string(),
        ItemSource::Crafted => "crafted".to_string(),
        ItemSource::Reward { .. } => "reward".to_string(),
        ItemSource::Stolen { .. } => "stolen".to_string(),
        ItemSource::Custom { .. } => "custom".to_string(),
    }
}

fn info_type_to_string(it: InfoType) -> String {
    match it {
        InfoType::Lore => "lore".to_string(),
        InfoType::Quest => "quest".to_string(),
        InfoType::Character => "character".to_string(),
        InfoType::Location => "location".to_string(),
        InfoType::Item => "item".to_string(),
        InfoType::Secret => "secret".to_string(),
        InfoType::Rumor => "rumor".to_string(),
    }
}

fn info_importance_to_string(ii: StoryEventInfoImportance) -> String {
    match ii {
        StoryEventInfoImportance::Minor => "minor".to_string(),
        StoryEventInfoImportance::Notable => "notable".to_string(),
        StoryEventInfoImportance::Major => "major".to_string(),
        StoryEventInfoImportance::Critical => "critical".to_string(),
    }
}

fn marker_importance_to_string(mi: MarkerImportance) -> String {
    match mi {
        MarkerImportance::Minor => "minor".to_string(),
        // Note: "normal" matches player UI expectations (not "notable")
        MarkerImportance::Notable => "normal".to_string(),
        MarkerImportance::Major => "major".to_string(),
        MarkerImportance::Critical => "critical".to_string(),
    }
}

fn dm_marker_type_to_string(mt: DmMarkerType) -> String {
    // Note: snake_case values match player UI expectations
    match mt {
        DmMarkerType::Note => "note".to_string(),
        DmMarkerType::PlotPoint => "plot_point".to_string(),
        DmMarkerType::CharacterMoment => "character_moment".to_string(),
        DmMarkerType::WorldEvent => "world_event".to_string(),
        DmMarkerType::PlayerDecision => "player_decision".to_string(),
        DmMarkerType::Foreshadowing => "foreshadowing".to_string(),
        DmMarkerType::Callback => "callback".to_string(),
        DmMarkerType::Custom => "custom".to_string(),
    }
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
    narrative_repo: Arc<dyn NarrativeRepo>,
    clock: Arc<dyn ClockPort>,
}

impl StoryEventOps {
    pub fn new(
        narrative_repo: Arc<dyn NarrativeRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            narrative_repo,
            clock,
        }
    }

    pub async fn list(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<StoryEventSummary>, StoryEventError> {
        let events = self
            .narrative_repo
            .list_story_events_with_edges(world_id, limit)
            .await?;
        Ok(events.into_iter().map(story_event_with_edges_to_summary).collect())
    }

    pub async fn get(
        &self,
        event_id: StoryEventId,
    ) -> Result<Option<StoryEventSummary>, StoryEventError> {
        let event = self
            .narrative_repo
            .get_story_event_with_edges(event_id)
            .await?;
        Ok(event.map(story_event_with_edges_to_summary))
    }

    pub async fn update(
        &self,
        event_id: StoryEventId,
        summary: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<StoryEventSummary, StoryEventError> {
        // Get event with edges to preserve edge-backed fields in the response
        let edges = self
            .narrative_repo
            .get_story_event_with_edges(event_id)
            .await?
            .ok_or(StoryEventError::NotFound)?;

        let event = &edges.event;

        // Rebuild event with updated fields using from_storage
        let new_summary = summary.unwrap_or_else(|| event.summary().to_string());
        let new_tags: Vec<wrldbldr_domain::Tag> = match tags {
            Some(tag_strings) => tag_strings
                .into_iter()
                .filter_map(|s| wrldbldr_domain::Tag::new(&s).ok())
                .collect(),
            None => event.tags().to_vec(),
        };

        let visibility = if event.is_hidden() {
            wrldbldr_domain::StoryEventVisibility::Hidden
        } else {
            wrldbldr_domain::StoryEventVisibility::Visible
        };

        let updated_event = wrldbldr_domain::StoryEvent::from_storage(
            event.id(),
            event.world_id(),
            event.event_type().clone(),
            event.timestamp(),
            event.game_time().map(|s| s.to_string()),
            new_summary,
            visibility,
            new_tags,
        );

        self.narrative_repo.save_story_event(&updated_event).await?;

        // Return summary with preserved edge data
        Ok(StoryEventSummary {
            id: updated_event.id(),
            world_id: updated_event.world_id(),
            event_type: updated_event.event_type().clone(),
            timestamp: updated_event.timestamp(),
            game_time: updated_event.game_time().map(|s| s.to_string()),
            summary: updated_event.summary().to_string(),
            visibility: updated_event.visibility(),
            tags: updated_event.tags().iter().map(|t| t.to_string()).collect(),
            type_name: updated_event.type_name().to_string(),
            // Preserve edge fields from the original query
            scene_id: edges.scene_id,
            location_id: edges.location_id,
            involved_characters: edges.involved_characters,
            triggered_by: edges.triggered_by,
        })
    }

    pub async fn create_dm_marker(
        &self,
        world_id: WorldId,
        title: String,
        content: Option<String>,
    ) -> Result<StoryEventId, StoryEventError> {
        let now = self.clock.now();
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
        .with_summary(&title);

        self.narrative_repo.save_story_event(&event).await?;
        Ok(event.id())
    }

    pub async fn set_visibility(
        &self,
        event_id: StoryEventId,
        visible: bool,
    ) -> Result<StoryEventSummary, StoryEventError> {
        // Get event with edges to preserve edge-backed fields in the response
        let edges = self
            .narrative_repo
            .get_story_event_with_edges(event_id)
            .await?
            .ok_or(StoryEventError::NotFound)?;

        let event = &edges.event;

        // Rebuild event with updated visibility using from_storage
        let visibility = if visible {
            wrldbldr_domain::StoryEventVisibility::Visible
        } else {
            wrldbldr_domain::StoryEventVisibility::Hidden
        };

        let updated_event = wrldbldr_domain::StoryEvent::from_storage(
            event.id(),
            event.world_id(),
            event.event_type().clone(),
            event.timestamp(),
            event.game_time().map(|s| s.to_string()),
            event.summary().to_string(),
            visibility,
            event.tags().to_vec(),
        );

        self.narrative_repo.save_story_event(&updated_event).await?;

        // Return summary with preserved edge data
        Ok(StoryEventSummary {
            id: updated_event.id(),
            world_id: updated_event.world_id(),
            event_type: updated_event.event_type().clone(),
            timestamp: updated_event.timestamp(),
            game_time: updated_event.game_time().map(|s| s.to_string()),
            summary: updated_event.summary().to_string(),
            visibility: updated_event.visibility(),
            tags: updated_event.tags().iter().map(|t| t.to_string()).collect(),
            type_name: updated_event.type_name().to_string(),
            // Preserve edge fields from the original query
            scene_id: edges.scene_id,
            location_id: edges.location_id,
            involved_characters: edges.involved_characters,
            triggered_by: edges.triggered_by,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoryEventError {
    #[error("Story event not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[allow(dead_code)]
fn story_event_to_summary(event: wrldbldr_domain::StoryEvent) -> StoryEventSummary {
    let type_name = event.type_name().to_string();
    StoryEventSummary {
        id: event.id(),
        world_id: event.world_id(),
        event_type: event.event_type().clone(),
        timestamp: event.timestamp(),
        game_time: event.game_time().map(|s| s.to_string()),
        summary: event.summary().to_string(),
        visibility: event.visibility(),
        tags: event.tags().iter().map(|t| t.to_string()).collect(),
        type_name,
        // Edge fields are populated from repo queries when available
        scene_id: None,
        location_id: None,
        involved_characters: Vec::new(),
        triggered_by: None,
    }
}

fn story_event_with_edges_to_summary(edges: StoryEventWithEdges) -> StoryEventSummary {
    let event = edges.event;
    let type_name = event.type_name().to_string();
    StoryEventSummary {
        id: event.id(),
        world_id: event.world_id(),
        event_type: event.event_type().clone(),
        timestamp: event.timestamp(),
        game_time: event.game_time().map(|s| s.to_string()),
        summary: event.summary().to_string(),
        visibility: event.visibility(),
        tags: event.tags().iter().map(|t| t.to_string()).collect(),
        type_name,
        // Edge fields populated from the repo query
        scene_id: edges.scene_id,
        location_id: edges.location_id,
        involved_characters: edges.involved_characters,
        triggered_by: edges.triggered_by,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::clock::FixedClock;
    use crate::infrastructure::ports::{ClockPort, MockNarrativeRepo};
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

    fn create_test_ops(narrative_repo: MockNarrativeRepo) -> Arc<StoryEventOps> {
        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(fixed_time()));
        Arc::new(StoryEventOps::new(Arc::new(narrative_repo), clock))
    }

    mod story_event_ops {
        use super::*;

        fn create_test_story_event_with_edges(
            world_id: WorldId,
        ) -> crate::infrastructure::ports::StoryEventWithEdges {
            let event = create_test_story_event(world_id);
            crate::infrastructure::ports::StoryEventWithEdges {
                event,
                scene_id: None,
                location_id: None,
                involved_characters: Vec::new(),
                triggered_by: None,
            }
        }

        #[tokio::test]
        async fn when_list_returns_events() {
            let world_id = WorldId::new();
            let event = create_test_story_event_with_edges(world_id);

            let mut narrative_repo = MockNarrativeRepo::new();
            narrative_repo
                .expect_list_story_events_with_edges()
                .withf(move |w, l| *w == world_id && *l == 10)
                .returning(move |_, _| Ok(vec![event.clone()]));

            let ops = create_test_ops(narrative_repo);

            let result = ops.list(world_id, 10).await;
            assert!(result.is_ok());
            let events = result.expect("result should be Ok");
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].summary, "Test summary");
        }

        #[tokio::test]
        async fn when_get_not_found_returns_none() {
            let event_id = wrldbldr_domain::StoryEventId::new();

            let mut narrative_repo = MockNarrativeRepo::new();
            narrative_repo
                .expect_get_story_event_with_edges()
                .withf(move |id| *id == event_id)
                .returning(|_| Ok(None));

            let ops = create_test_ops(narrative_repo);

            let result = ops.get(event_id).await;
            assert!(result.is_ok());
            assert!(result.expect("result should be Ok").is_none());
        }

        #[tokio::test]
        async fn when_update_succeeds() {
            let world_id = WorldId::new();
            let edges = create_test_story_event_with_edges(world_id);
            let event_id = edges.event.id();

            let mut narrative_repo = MockNarrativeRepo::new();
            narrative_repo
                .expect_get_story_event_with_edges()
                .withf(move |id| *id == event_id)
                .returning(move |_| Ok(Some(edges.clone())));
            narrative_repo
                .expect_save_story_event()
                .returning(|_| Ok(()));

            let ops = create_test_ops(narrative_repo);

            let result = ops
                .update(event_id, Some("Updated summary".to_string()), None)
                .await;
            assert!(result.is_ok());
            let updated = result.expect("result should be Ok");
            assert_eq!(updated.summary, "Updated summary");
        }

        #[tokio::test]
        async fn when_update_not_found_returns_error() {
            let event_id = wrldbldr_domain::StoryEventId::new();

            let mut narrative_repo = MockNarrativeRepo::new();
            narrative_repo
                .expect_get_story_event_with_edges()
                .withf(move |id| *id == event_id)
                .returning(|_| Ok(None));

            let ops = create_test_ops(narrative_repo);

            let result = ops
                .update(event_id, Some("Updated summary".to_string()), None)
                .await;
            assert!(matches!(result, Err(StoryEventError::NotFound)));
        }
    }
}
