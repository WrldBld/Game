//! Storage DTOs for StoryEventType
//!
//! These types have serde derives for JSON persistence in Neo4j.
//! They provide a stable serialization format separate from domain types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::parse_uuid_or_nil;
use wrldbldr_domain::entities::{
    ChallengeEventOutcome, CombatEventType, CombatOutcome, DmMarkerType, InfoType, ItemSource,
    MarkerImportance, StoryEventInfoImportance, StoryEventType,
};
use wrldbldr_domain::{
    ChallengeId, CharacterId, LocationId, NarrativeEventId, SceneId,
};

// ============================================================================
// Storage DTOs for StoryEventType
// These types have serde derives for JSON persistence in Neo4j
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum StoredStoryEventType {
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
        combat_type: StoredCombatEventType,
        participants: Vec<String>,
        enemies: Vec<String>,
        outcome: Option<StoredCombatOutcome>,
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
        outcome: StoredChallengeEventOutcome,
    },
    ItemAcquired {
        item_name: String,
        item_description: Option<String>,
        character_id: String,
        source: StoredItemSource,
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
        info_type: StoredInfoType,
        title: String,
        content: String,
        source: Option<String>,
        importance: StoredInfoImportance,
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
        importance: StoredMarkerImportance,
        marker_type: StoredDmMarkerType,
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
        data: serde_json::Value,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum StoredCombatEventType {
    Started,
    RoundCompleted,
    CharacterDefeated,
    CharacterFled,
    Ended,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum StoredCombatOutcome {
    Victory,
    Defeat,
    Fled,
    Negotiated,
    Draw,
    Interrupted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum StoredChallengeEventOutcome {
    CriticalSuccess,
    Success,
    PartialSuccess,
    Failure,
    CriticalFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source_type")]
pub(super) enum StoredItemSource {
    Found { location: String },
    Purchased { from: String, cost: Option<String> },
    Gifted { from: String },
    Looted { from: String },
    Crafted,
    Reward { for_what: String },
    Stolen { from: String },
    Custom { description: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum StoredInfoType {
    Lore,
    Quest,
    Character,
    Location,
    Item,
    Secret,
    Rumor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum StoredInfoImportance {
    Minor,
    Notable,
    Major,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum StoredMarkerImportance {
    Minor,
    Notable,
    Major,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum StoredDmMarkerType {
    Note,
    PlotPoint,
    CharacterMoment,
    WorldEvent,
    PlayerDecision,
    Foreshadowing,
    Callback,
    Custom,
}

// ============================================================================
// Conversion from domain to stored types (serialization)
// ============================================================================

impl From<&StoryEventType> for StoredStoryEventType {
    fn from(e: &StoryEventType) -> Self {
        match e {
            StoryEventType::LocationChange {
                from_location,
                to_location,
                character_id,
                travel_method,
            } => StoredStoryEventType::LocationChange {
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
            } => StoredStoryEventType::DialogueExchange {
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
            } => StoredStoryEventType::CombatEvent {
                combat_type: (*combat_type).into(),
                participants: participants.iter().map(|id| id.to_string()).collect(),
                enemies: enemies.clone(),
                outcome: outcome.map(|o| o.into()),
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
            } => StoredStoryEventType::ChallengeAttempted {
                challenge_id: challenge_id.map(|id| id.to_string()),
                challenge_name: challenge_name.clone(),
                character_id: character_id.to_string(),
                skill_used: skill_used.clone(),
                difficulty: difficulty.clone(),
                roll_result: *roll_result,
                modifier: *modifier,
                outcome: (*outcome).into(),
            },
            StoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id,
                source,
                quantity,
            } => StoredStoryEventType::ItemAcquired {
                item_name: item_name.clone(),
                item_description: item_description.clone(),
                character_id: character_id.to_string(),
                source: source.into(),
                quantity: *quantity,
            },
            StoryEventType::ItemTransferred {
                item_name,
                from_character,
                to_character,
                quantity,
                reason,
            } => StoredStoryEventType::ItemTransferred {
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
            } => StoredStoryEventType::ItemUsed {
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
            } => StoredStoryEventType::RelationshipChanged {
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
            } => StoredStoryEventType::SceneTransition {
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
            } => StoredStoryEventType::InformationRevealed {
                info_type: (*info_type).into(),
                title: title.clone(),
                content: content.clone(),
                source: source.map(|id| id.to_string()),
                importance: (*importance).into(),
                persist_to_journal: *persist_to_journal,
            },
            StoryEventType::NpcAction {
                npc_id,
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            } => StoredStoryEventType::NpcAction {
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
            } => StoredStoryEventType::DmMarker {
                title: title.clone(),
                note: note.clone(),
                importance: (*importance).into(),
                marker_type: (*marker_type).into(),
            },
            StoryEventType::NarrativeEventTriggered {
                narrative_event_id,
                narrative_event_name,
                outcome_branch,
                effects_applied,
            } => StoredStoryEventType::NarrativeEventTriggered {
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
            } => StoredStoryEventType::StatModified {
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
            } => StoredStoryEventType::FlagChanged {
                flag_name: flag_name.clone(),
                new_value: *new_value,
                reason: reason.clone(),
            },
            StoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            } => StoredStoryEventType::SessionStarted {
                session_number: *session_number,
                session_name: session_name.clone(),
                players_present: players_present.clone(),
            },
            StoryEventType::SessionEnded {
                duration_minutes,
                summary,
            } => StoredStoryEventType::SessionEnded {
                duration_minutes: *duration_minutes,
                summary: summary.clone(),
            },
            StoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            } => StoredStoryEventType::Custom {
                event_subtype: event_subtype.clone(),
                title: title.clone(),
                description: description.clone(),
                data: data.clone(),
            },
        }
    }
}

impl From<CombatEventType> for StoredCombatEventType {
    fn from(c: CombatEventType) -> Self {
        match c {
            CombatEventType::Started => StoredCombatEventType::Started,
            CombatEventType::RoundCompleted => StoredCombatEventType::RoundCompleted,
            CombatEventType::CharacterDefeated => StoredCombatEventType::CharacterDefeated,
            CombatEventType::CharacterFled => StoredCombatEventType::CharacterFled,
            CombatEventType::Ended => StoredCombatEventType::Ended,
        }
    }
}

impl From<CombatOutcome> for StoredCombatOutcome {
    fn from(c: CombatOutcome) -> Self {
        match c {
            CombatOutcome::Victory => StoredCombatOutcome::Victory,
            CombatOutcome::Defeat => StoredCombatOutcome::Defeat,
            CombatOutcome::Fled => StoredCombatOutcome::Fled,
            CombatOutcome::Negotiated => StoredCombatOutcome::Negotiated,
            CombatOutcome::Draw => StoredCombatOutcome::Draw,
            CombatOutcome::Interrupted => StoredCombatOutcome::Interrupted,
        }
    }
}

impl From<ChallengeEventOutcome> for StoredChallengeEventOutcome {
    fn from(c: ChallengeEventOutcome) -> Self {
        match c {
            ChallengeEventOutcome::CriticalSuccess => StoredChallengeEventOutcome::CriticalSuccess,
            ChallengeEventOutcome::Success => StoredChallengeEventOutcome::Success,
            ChallengeEventOutcome::PartialSuccess => StoredChallengeEventOutcome::PartialSuccess,
            ChallengeEventOutcome::Failure => StoredChallengeEventOutcome::Failure,
            ChallengeEventOutcome::CriticalFailure => StoredChallengeEventOutcome::CriticalFailure,
        }
    }
}

impl From<&ItemSource> for StoredItemSource {
    fn from(s: &ItemSource) -> Self {
        match s {
            ItemSource::Found { location } => StoredItemSource::Found {
                location: location.clone(),
            },
            ItemSource::Purchased { from, cost } => StoredItemSource::Purchased {
                from: from.clone(),
                cost: cost.clone(),
            },
            ItemSource::Gifted { from } => StoredItemSource::Gifted {
                from: from.to_string(),
            },
            ItemSource::Looted { from } => StoredItemSource::Looted { from: from.clone() },
            ItemSource::Crafted => StoredItemSource::Crafted,
            ItemSource::Reward { for_what } => StoredItemSource::Reward {
                for_what: for_what.clone(),
            },
            ItemSource::Stolen { from } => StoredItemSource::Stolen { from: from.clone() },
            ItemSource::Custom { description } => StoredItemSource::Custom {
                description: description.clone(),
            },
        }
    }
}

impl From<InfoType> for StoredInfoType {
    fn from(i: InfoType) -> Self {
        match i {
            InfoType::Lore => StoredInfoType::Lore,
            InfoType::Quest => StoredInfoType::Quest,
            InfoType::Character => StoredInfoType::Character,
            InfoType::Location => StoredInfoType::Location,
            InfoType::Item => StoredInfoType::Item,
            InfoType::Secret => StoredInfoType::Secret,
            InfoType::Rumor => StoredInfoType::Rumor,
        }
    }
}

impl From<StoryEventInfoImportance> for StoredInfoImportance {
    fn from(i: StoryEventInfoImportance) -> Self {
        match i {
            StoryEventInfoImportance::Minor => StoredInfoImportance::Minor,
            StoryEventInfoImportance::Notable => StoredInfoImportance::Notable,
            StoryEventInfoImportance::Major => StoredInfoImportance::Major,
            StoryEventInfoImportance::Critical => StoredInfoImportance::Critical,
        }
    }
}

impl From<MarkerImportance> for StoredMarkerImportance {
    fn from(m: MarkerImportance) -> Self {
        match m {
            MarkerImportance::Minor => StoredMarkerImportance::Minor,
            MarkerImportance::Notable => StoredMarkerImportance::Notable,
            MarkerImportance::Major => StoredMarkerImportance::Major,
            MarkerImportance::Critical => StoredMarkerImportance::Critical,
        }
    }
}

impl From<DmMarkerType> for StoredDmMarkerType {
    fn from(d: DmMarkerType) -> Self {
        match d {
            DmMarkerType::Note => StoredDmMarkerType::Note,
            DmMarkerType::PlotPoint => StoredDmMarkerType::PlotPoint,
            DmMarkerType::CharacterMoment => StoredDmMarkerType::CharacterMoment,
            DmMarkerType::WorldEvent => StoredDmMarkerType::WorldEvent,
            DmMarkerType::PlayerDecision => StoredDmMarkerType::PlayerDecision,
            DmMarkerType::Foreshadowing => StoredDmMarkerType::Foreshadowing,
            DmMarkerType::Callback => StoredDmMarkerType::Callback,
            DmMarkerType::Custom => StoredDmMarkerType::Custom,
        }
    }
}

// ============================================================================
// Conversion from stored to domain types (deserialization)
// ============================================================================

impl From<StoredStoryEventType> for StoryEventType {
    fn from(s: StoredStoryEventType) -> Self {
        match s {
            StoredStoryEventType::LocationChange {
                from_location,
                to_location,
                character_id,
                travel_method,
            } => StoryEventType::LocationChange {
                from_location: from_location
                    .and_then(|id| Uuid::parse_str(&id).ok().map(LocationId::from)),
                to_location: LocationId::from(parse_uuid_or_nil(
                    &to_location,
                    "StoryEventType::LocationChange.to_location",
                )),
                character_id: CharacterId::from(parse_uuid_or_nil(
                    &character_id,
                    "StoryEventType::LocationChange.character_id",
                )),
                travel_method,
            },
            StoredStoryEventType::DialogueExchange {
                npc_id,
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            } => StoryEventType::DialogueExchange {
                npc_id: CharacterId::from(parse_uuid_or_nil(
                    &npc_id,
                    "StoryEventType::DialogueExchange.npc_id",
                )),
                npc_name,
                player_dialogue,
                npc_response,
                topics_discussed,
                tone,
            },
            StoredStoryEventType::CombatEvent {
                combat_type,
                participants,
                enemies,
                outcome,
                location_id,
                rounds,
            } => StoryEventType::CombatEvent {
                combat_type: combat_type.into(),
                participants: participants
                    .into_iter()
                    .filter_map(|id| Uuid::parse_str(&id).ok().map(CharacterId::from))
                    .collect(),
                enemies,
                outcome: outcome.map(|o| o.into()),
                location_id: LocationId::from(parse_uuid_or_nil(
                    &location_id,
                    "StoryEventType::CombatEvent.location_id",
                )),
                rounds,
            },
            StoredStoryEventType::ChallengeAttempted {
                challenge_id,
                challenge_name,
                character_id,
                skill_used,
                difficulty,
                roll_result,
                modifier,
                outcome,
            } => StoryEventType::ChallengeAttempted {
                challenge_id: challenge_id
                    .and_then(|id| Uuid::parse_str(&id).ok().map(ChallengeId::from)),
                challenge_name,
                character_id: CharacterId::from(parse_uuid_or_nil(
                    &character_id,
                    "StoryEventType::ChallengeAttempted.character_id",
                )),
                skill_used,
                difficulty,
                roll_result,
                modifier,
                outcome: outcome.into(),
            },
            StoredStoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id,
                source,
                quantity,
            } => StoryEventType::ItemAcquired {
                item_name,
                item_description,
                character_id: CharacterId::from(parse_uuid_or_nil(
                    &character_id,
                    "StoryEventType::ItemAcquired.character_id",
                )),
                source: source.into(),
                quantity,
            },
            StoredStoryEventType::ItemTransferred {
                item_name,
                from_character,
                to_character,
                quantity,
                reason,
            } => StoryEventType::ItemTransferred {
                item_name,
                from_character: from_character
                    .and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
                to_character: CharacterId::from(parse_uuid_or_nil(
                    &to_character,
                    "StoryEventType::ItemTransferred.to_character",
                )),
                quantity,
                reason,
            },
            StoredStoryEventType::ItemUsed {
                item_name,
                character_id,
                target,
                effect,
                consumed,
            } => StoryEventType::ItemUsed {
                item_name,
                character_id: CharacterId::from(parse_uuid_or_nil(
                    &character_id,
                    "StoryEventType::ItemUsed.character_id",
                )),
                target,
                effect,
                consumed,
            },
            StoredStoryEventType::RelationshipChanged {
                from_character,
                to_character,
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            } => StoryEventType::RelationshipChanged {
                from_character: CharacterId::from(parse_uuid_or_nil(
                    &from_character,
                    "StoryEventType::RelationshipChanged.from_character",
                )),
                to_character: CharacterId::from(parse_uuid_or_nil(
                    &to_character,
                    "StoryEventType::RelationshipChanged.to_character",
                )),
                previous_sentiment,
                new_sentiment,
                sentiment_change,
                reason,
            },
            StoredStoryEventType::SceneTransition {
                from_scene,
                to_scene,
                from_scene_name,
                to_scene_name,
                trigger_reason,
            } => StoryEventType::SceneTransition {
                from_scene: from_scene.and_then(|id| Uuid::parse_str(&id).ok().map(SceneId::from)),
                to_scene: SceneId::from(parse_uuid_or_nil(
                    &to_scene,
                    "StoryEventType::SceneTransition.to_scene",
                )),
                from_scene_name,
                to_scene_name,
                trigger_reason,
            },
            StoredStoryEventType::InformationRevealed {
                info_type,
                title,
                content,
                source,
                importance,
                persist_to_journal,
            } => StoryEventType::InformationRevealed {
                info_type: info_type.into(),
                title,
                content,
                source: source.and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
                importance: importance.into(),
                persist_to_journal,
            },
            StoredStoryEventType::NpcAction {
                npc_id,
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            } => StoryEventType::NpcAction {
                npc_id: CharacterId::from(parse_uuid_or_nil(
                    &npc_id,
                    "StoryEventType::NpcAction.npc_id",
                )),
                npc_name,
                action_type,
                description,
                dm_approved,
                dm_modified,
            },
            StoredStoryEventType::DmMarker {
                title,
                note,
                importance,
                marker_type,
            } => StoryEventType::DmMarker {
                title,
                note,
                importance: importance.into(),
                marker_type: marker_type.into(),
            },
            StoredStoryEventType::NarrativeEventTriggered {
                narrative_event_id,
                narrative_event_name,
                outcome_branch,
                effects_applied,
            } => StoryEventType::NarrativeEventTriggered {
                narrative_event_id: NarrativeEventId::from(parse_uuid_or_nil(
                    &narrative_event_id,
                    "StoryEventType::NarrativeEventTriggered.narrative_event_id",
                )),
                narrative_event_name,
                outcome_branch,
                effects_applied,
            },
            StoredStoryEventType::StatModified {
                character_id,
                stat_name,
                previous_value,
                new_value,
                reason,
            } => StoryEventType::StatModified {
                character_id: CharacterId::from(parse_uuid_or_nil(
                    &character_id,
                    "StoryEventType::StatModified.character_id",
                )),
                stat_name,
                previous_value,
                new_value,
                reason,
            },
            StoredStoryEventType::FlagChanged {
                flag_name,
                new_value,
                reason,
            } => StoryEventType::FlagChanged {
                flag_name,
                new_value,
                reason,
            },
            StoredStoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            } => StoryEventType::SessionStarted {
                session_number,
                session_name,
                players_present,
            },
            StoredStoryEventType::SessionEnded {
                duration_minutes,
                summary,
            } => StoryEventType::SessionEnded {
                duration_minutes,
                summary,
            },
            StoredStoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            } => StoryEventType::Custom {
                event_subtype,
                title,
                description,
                data,
            },
        }
    }
}

impl From<StoredCombatEventType> for CombatEventType {
    fn from(s: StoredCombatEventType) -> Self {
        match s {
            StoredCombatEventType::Started => CombatEventType::Started,
            StoredCombatEventType::RoundCompleted => CombatEventType::RoundCompleted,
            StoredCombatEventType::CharacterDefeated => CombatEventType::CharacterDefeated,
            StoredCombatEventType::CharacterFled => CombatEventType::CharacterFled,
            StoredCombatEventType::Ended => CombatEventType::Ended,
        }
    }
}

impl From<StoredCombatOutcome> for CombatOutcome {
    fn from(s: StoredCombatOutcome) -> Self {
        match s {
            StoredCombatOutcome::Victory => CombatOutcome::Victory,
            StoredCombatOutcome::Defeat => CombatOutcome::Defeat,
            StoredCombatOutcome::Fled => CombatOutcome::Fled,
            StoredCombatOutcome::Negotiated => CombatOutcome::Negotiated,
            StoredCombatOutcome::Draw => CombatOutcome::Draw,
            StoredCombatOutcome::Interrupted => CombatOutcome::Interrupted,
        }
    }
}

impl From<StoredChallengeEventOutcome> for ChallengeEventOutcome {
    fn from(s: StoredChallengeEventOutcome) -> Self {
        match s {
            StoredChallengeEventOutcome::CriticalSuccess => ChallengeEventOutcome::CriticalSuccess,
            StoredChallengeEventOutcome::Success => ChallengeEventOutcome::Success,
            StoredChallengeEventOutcome::PartialSuccess => ChallengeEventOutcome::PartialSuccess,
            StoredChallengeEventOutcome::Failure => ChallengeEventOutcome::Failure,
            StoredChallengeEventOutcome::CriticalFailure => ChallengeEventOutcome::CriticalFailure,
        }
    }
}

impl From<StoredItemSource> for ItemSource {
    fn from(s: StoredItemSource) -> Self {
        match s {
            StoredItemSource::Found { location } => ItemSource::Found { location },
            StoredItemSource::Purchased { from, cost } => ItemSource::Purchased { from, cost },
            StoredItemSource::Gifted { from } => ItemSource::Gifted {
                from: CharacterId::from(parse_uuid_or_nil(&from, "StoredItemSource::Gifted.from")),
            },
            StoredItemSource::Looted { from } => ItemSource::Looted { from },
            StoredItemSource::Crafted => ItemSource::Crafted,
            StoredItemSource::Reward { for_what } => ItemSource::Reward { for_what },
            StoredItemSource::Stolen { from } => ItemSource::Stolen { from },
            StoredItemSource::Custom { description } => ItemSource::Custom { description },
        }
    }
}

impl From<StoredInfoType> for InfoType {
    fn from(s: StoredInfoType) -> Self {
        match s {
            StoredInfoType::Lore => InfoType::Lore,
            StoredInfoType::Quest => InfoType::Quest,
            StoredInfoType::Character => InfoType::Character,
            StoredInfoType::Location => InfoType::Location,
            StoredInfoType::Item => InfoType::Item,
            StoredInfoType::Secret => InfoType::Secret,
            StoredInfoType::Rumor => InfoType::Rumor,
        }
    }
}

impl From<StoredInfoImportance> for StoryEventInfoImportance {
    fn from(s: StoredInfoImportance) -> Self {
        match s {
            StoredInfoImportance::Minor => StoryEventInfoImportance::Minor,
            StoredInfoImportance::Notable => StoryEventInfoImportance::Notable,
            StoredInfoImportance::Major => StoryEventInfoImportance::Major,
            StoredInfoImportance::Critical => StoryEventInfoImportance::Critical,
        }
    }
}

impl From<StoredMarkerImportance> for MarkerImportance {
    fn from(s: StoredMarkerImportance) -> Self {
        match s {
            StoredMarkerImportance::Minor => MarkerImportance::Minor,
            StoredMarkerImportance::Notable => MarkerImportance::Notable,
            StoredMarkerImportance::Major => MarkerImportance::Major,
            StoredMarkerImportance::Critical => MarkerImportance::Critical,
        }
    }
}

impl From<StoredDmMarkerType> for DmMarkerType {
    fn from(s: StoredDmMarkerType) -> Self {
        match s {
            StoredDmMarkerType::Note => DmMarkerType::Note,
            StoredDmMarkerType::PlotPoint => DmMarkerType::PlotPoint,
            StoredDmMarkerType::CharacterMoment => DmMarkerType::CharacterMoment,
            StoredDmMarkerType::WorldEvent => DmMarkerType::WorldEvent,
            StoredDmMarkerType::PlayerDecision => DmMarkerType::PlayerDecision,
            StoredDmMarkerType::Foreshadowing => DmMarkerType::Foreshadowing,
            StoredDmMarkerType::Callback => DmMarkerType::Callback,
            StoredDmMarkerType::Custom => DmMarkerType::Custom,
        }
    }
}
