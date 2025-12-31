//! Storage DTOs for NarrativeTrigger and EventOutcome
//!
//! These types have serde derives for JSON persistence in Neo4j.
//! They handle conversion to/from domain types, including UUID string parsing.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::parse_uuid_or_nil;
use wrldbldr_domain::entities::{
    ChainedEvent, EventEffect, EventOutcome, NarrativeTrigger, NarrativeTriggerType,
    OutcomeCondition,
};
use wrldbldr_domain::{ChallengeId, CharacterId, LocationId, NarrativeEventId, SceneId};

// ============================================================================
// Stored Types (Serializable DTOs)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredNarrativeTrigger {
    pub trigger_type: StoredNarrativeTriggerType,
    pub description: String,
    pub is_required: bool,
    pub trigger_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum StoredNarrativeTriggerType {
    NpcAction {
        npc_id: String,
        npc_name: String,
        action_keywords: Vec<String>,
        action_description: String,
    },
    PlayerEntersLocation {
        location_id: String,
        location_name: String,
    },
    TimeAtLocation {
        location_id: String,
        location_name: String,
        time_context: String,
    },
    DialogueTopic {
        keywords: Vec<String>,
        with_npc: Option<String>,
        npc_name: Option<String>,
    },
    ChallengeCompleted {
        challenge_id: String,
        challenge_name: String,
        requires_success: Option<bool>,
    },
    RelationshipThreshold {
        character_id: String,
        character_name: String,
        with_character: String,
        with_character_name: String,
        min_sentiment: Option<f32>,
        max_sentiment: Option<f32>,
    },
    HasItem {
        item_name: String,
        quantity: Option<u32>,
    },
    MissingItem {
        item_name: String,
    },
    EventCompleted {
        event_id: String,
        event_name: String,
        outcome_name: Option<String>,
    },
    TurnCount {
        turns: u32,
        since_event: Option<String>,
    },
    FlagSet {
        flag_name: String,
    },
    FlagNotSet {
        flag_name: String,
    },
    StatThreshold {
        character_id: String,
        stat_name: String,
        min_value: Option<i32>,
        max_value: Option<i32>,
    },
    CombatResult {
        victory: Option<bool>,
        involved_npc: Option<String>,
    },
    Custom {
        description: String,
        llm_evaluation: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredEventOutcome {
    pub name: String,
    pub label: String,
    pub description: String,
    pub condition: Option<StoredOutcomeCondition>,
    pub effects: Vec<StoredEventEffect>,
    pub chain_events: Vec<StoredChainedEvent>,
    pub timeline_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum StoredOutcomeCondition {
    DmChoice,
    ChallengeResult {
        challenge_id: Option<String>,
        success_required: bool,
    },
    CombatResult {
        victory_required: bool,
    },
    DialogueChoice {
        keywords: Vec<String>,
    },
    PlayerAction {
        action_keywords: Vec<String>,
    },
    HasItem {
        item_name: String,
    },
    Custom {
        description: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum StoredEventEffect {
    ModifyRelationship {
        from_character: String,
        from_name: String,
        to_character: String,
        to_name: String,
        sentiment_change: f32,
        reason: String,
    },
    GiveItem {
        item_name: String,
        item_description: Option<String>,
        quantity: u32,
    },
    TakeItem {
        item_name: String,
        quantity: u32,
    },
    RevealInformation {
        info_type: String,
        title: String,
        content: String,
        persist_to_journal: bool,
    },
    SetFlag {
        flag_name: String,
        value: bool,
    },
    EnableChallenge {
        challenge_id: String,
        challenge_name: String,
    },
    DisableChallenge {
        challenge_id: String,
        challenge_name: String,
    },
    EnableEvent {
        event_id: String,
        event_name: String,
    },
    DisableEvent {
        event_id: String,
        event_name: String,
    },
    TriggerScene {
        scene_id: String,
        scene_name: String,
    },
    StartCombat {
        participants: Vec<String>,
        participant_names: Vec<String>,
        combat_description: String,
    },
    ModifyStat {
        character_id: String,
        character_name: String,
        stat_name: String,
        modifier: i32,
    },
    AddReward {
        reward_type: String,
        amount: i32,
        description: String,
    },
    Custom {
        description: String,
        requires_dm_action: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredChainedEvent {
    pub event_id: String,
    pub event_name: String,
    pub delay_turns: u32,
    pub additional_trigger: Option<Box<StoredNarrativeTriggerType>>,
    pub chain_reason: Option<String>,
}

// ============================================================================
// Domain -> Stored Conversions (Serialization)
// ============================================================================

impl From<&NarrativeTrigger> for StoredNarrativeTrigger {
    fn from(t: &NarrativeTrigger) -> Self {
        Self {
            trigger_type: StoredNarrativeTriggerType::from(&t.trigger_type),
            description: t.description.clone(),
            is_required: t.is_required,
            trigger_id: t.trigger_id.clone(),
        }
    }
}

impl From<&NarrativeTriggerType> for StoredNarrativeTriggerType {
    fn from(t: &NarrativeTriggerType) -> Self {
        match t {
            NarrativeTriggerType::NpcAction {
                npc_id,
                npc_name,
                action_keywords,
                action_description,
            } => StoredNarrativeTriggerType::NpcAction {
                npc_id: npc_id.to_string(),
                npc_name: npc_name.clone(),
                action_keywords: action_keywords.clone(),
                action_description: action_description.clone(),
            },
            NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name,
            } => StoredNarrativeTriggerType::PlayerEntersLocation {
                location_id: location_id.to_string(),
                location_name: location_name.clone(),
            },
            NarrativeTriggerType::TimeAtLocation {
                location_id,
                location_name,
                time_context,
            } => StoredNarrativeTriggerType::TimeAtLocation {
                location_id: location_id.to_string(),
                location_name: location_name.clone(),
                time_context: time_context.clone(),
            },
            NarrativeTriggerType::DialogueTopic {
                keywords,
                with_npc,
                npc_name,
            } => StoredNarrativeTriggerType::DialogueTopic {
                keywords: keywords.clone(),
                with_npc: with_npc.as_ref().map(|id| id.to_string()),
                npc_name: npc_name.clone(),
            },
            NarrativeTriggerType::ChallengeCompleted {
                challenge_id,
                challenge_name,
                requires_success,
            } => StoredNarrativeTriggerType::ChallengeCompleted {
                challenge_id: challenge_id.to_string(),
                challenge_name: challenge_name.clone(),
                requires_success: *requires_success,
            },
            NarrativeTriggerType::RelationshipThreshold {
                character_id,
                character_name,
                with_character,
                with_character_name,
                min_sentiment,
                max_sentiment,
            } => StoredNarrativeTriggerType::RelationshipThreshold {
                character_id: character_id.to_string(),
                character_name: character_name.clone(),
                with_character: with_character.to_string(),
                with_character_name: with_character_name.clone(),
                min_sentiment: *min_sentiment,
                max_sentiment: *max_sentiment,
            },
            NarrativeTriggerType::HasItem {
                item_name,
                quantity,
            } => StoredNarrativeTriggerType::HasItem {
                item_name: item_name.clone(),
                quantity: *quantity,
            },
            NarrativeTriggerType::MissingItem { item_name } => {
                StoredNarrativeTriggerType::MissingItem {
                    item_name: item_name.clone(),
                }
            }
            NarrativeTriggerType::EventCompleted {
                event_id,
                event_name,
                outcome_name,
            } => StoredNarrativeTriggerType::EventCompleted {
                event_id: event_id.to_string(),
                event_name: event_name.clone(),
                outcome_name: outcome_name.clone(),
            },
            NarrativeTriggerType::TurnCount { turns, since_event } => {
                StoredNarrativeTriggerType::TurnCount {
                    turns: *turns,
                    since_event: since_event.as_ref().map(|id| id.to_string()),
                }
            }
            NarrativeTriggerType::FlagSet { flag_name } => StoredNarrativeTriggerType::FlagSet {
                flag_name: flag_name.clone(),
            },
            NarrativeTriggerType::FlagNotSet { flag_name } => {
                StoredNarrativeTriggerType::FlagNotSet {
                    flag_name: flag_name.clone(),
                }
            }
            NarrativeTriggerType::StatThreshold {
                character_id,
                stat_name,
                min_value,
                max_value,
            } => StoredNarrativeTriggerType::StatThreshold {
                character_id: character_id.to_string(),
                stat_name: stat_name.clone(),
                min_value: *min_value,
                max_value: *max_value,
            },
            NarrativeTriggerType::CombatResult {
                victory,
                involved_npc,
            } => StoredNarrativeTriggerType::CombatResult {
                victory: *victory,
                involved_npc: involved_npc.as_ref().map(|id| id.to_string()),
            },
            NarrativeTriggerType::Custom {
                description,
                llm_evaluation,
            } => StoredNarrativeTriggerType::Custom {
                description: description.clone(),
                llm_evaluation: *llm_evaluation,
            },
        }
    }
}

impl From<&EventOutcome> for StoredEventOutcome {
    fn from(o: &EventOutcome) -> Self {
        Self {
            name: o.name.clone(),
            label: o.label.clone(),
            description: o.description.clone(),
            condition: o.condition.as_ref().map(StoredOutcomeCondition::from),
            effects: o.effects.iter().map(StoredEventEffect::from).collect(),
            chain_events: o
                .chain_events
                .iter()
                .map(StoredChainedEvent::from)
                .collect(),
            timeline_summary: o.timeline_summary.clone(),
        }
    }
}

impl From<&OutcomeCondition> for StoredOutcomeCondition {
    fn from(c: &OutcomeCondition) -> Self {
        match c {
            OutcomeCondition::DmChoice => StoredOutcomeCondition::DmChoice,
            OutcomeCondition::ChallengeResult {
                challenge_id,
                success_required,
            } => StoredOutcomeCondition::ChallengeResult {
                challenge_id: challenge_id.as_ref().map(|id| id.to_string()),
                success_required: *success_required,
            },
            OutcomeCondition::CombatResult { victory_required } => {
                StoredOutcomeCondition::CombatResult {
                    victory_required: *victory_required,
                }
            }
            OutcomeCondition::DialogueChoice { keywords } => {
                StoredOutcomeCondition::DialogueChoice {
                    keywords: keywords.clone(),
                }
            }
            OutcomeCondition::PlayerAction { action_keywords } => {
                StoredOutcomeCondition::PlayerAction {
                    action_keywords: action_keywords.clone(),
                }
            }
            OutcomeCondition::HasItem { item_name } => StoredOutcomeCondition::HasItem {
                item_name: item_name.clone(),
            },
            OutcomeCondition::Custom { description } => StoredOutcomeCondition::Custom {
                description: description.clone(),
            },
        }
    }
}

impl From<&EventEffect> for StoredEventEffect {
    fn from(e: &EventEffect) -> Self {
        match e {
            EventEffect::ModifyRelationship {
                from_character,
                from_name,
                to_character,
                to_name,
                sentiment_change,
                reason,
            } => StoredEventEffect::ModifyRelationship {
                from_character: from_character.to_string(),
                from_name: from_name.clone(),
                to_character: to_character.to_string(),
                to_name: to_name.clone(),
                sentiment_change: *sentiment_change,
                reason: reason.clone(),
            },
            EventEffect::GiveItem {
                item_name,
                item_description,
                quantity,
            } => StoredEventEffect::GiveItem {
                item_name: item_name.clone(),
                item_description: item_description.clone(),
                quantity: *quantity,
            },
            EventEffect::TakeItem {
                item_name,
                quantity,
            } => StoredEventEffect::TakeItem {
                item_name: item_name.clone(),
                quantity: *quantity,
            },
            EventEffect::RevealInformation {
                info_type,
                title,
                content,
                persist_to_journal,
            } => StoredEventEffect::RevealInformation {
                info_type: info_type.clone(),
                title: title.clone(),
                content: content.clone(),
                persist_to_journal: *persist_to_journal,
            },
            EventEffect::SetFlag { flag_name, value } => StoredEventEffect::SetFlag {
                flag_name: flag_name.clone(),
                value: *value,
            },
            EventEffect::EnableChallenge {
                challenge_id,
                challenge_name,
            } => StoredEventEffect::EnableChallenge {
                challenge_id: challenge_id.to_string(),
                challenge_name: challenge_name.clone(),
            },
            EventEffect::DisableChallenge {
                challenge_id,
                challenge_name,
            } => StoredEventEffect::DisableChallenge {
                challenge_id: challenge_id.to_string(),
                challenge_name: challenge_name.clone(),
            },
            EventEffect::EnableEvent {
                event_id,
                event_name,
            } => StoredEventEffect::EnableEvent {
                event_id: event_id.to_string(),
                event_name: event_name.clone(),
            },
            EventEffect::DisableEvent {
                event_id,
                event_name,
            } => StoredEventEffect::DisableEvent {
                event_id: event_id.to_string(),
                event_name: event_name.clone(),
            },
            EventEffect::TriggerScene {
                scene_id,
                scene_name,
            } => StoredEventEffect::TriggerScene {
                scene_id: scene_id.to_string(),
                scene_name: scene_name.clone(),
            },
            EventEffect::StartCombat {
                participants,
                participant_names,
                combat_description,
            } => StoredEventEffect::StartCombat {
                participants: participants.iter().map(|id| id.to_string()).collect(),
                participant_names: participant_names.clone(),
                combat_description: combat_description.clone(),
            },
            EventEffect::ModifyStat {
                character_id,
                character_name,
                stat_name,
                modifier,
            } => StoredEventEffect::ModifyStat {
                character_id: character_id.to_string(),
                character_name: character_name.clone(),
                stat_name: stat_name.clone(),
                modifier: *modifier,
            },
            EventEffect::AddReward {
                reward_type,
                amount,
                description,
            } => StoredEventEffect::AddReward {
                reward_type: reward_type.clone(),
                amount: *amount,
                description: description.clone(),
            },
            EventEffect::Custom {
                description,
                requires_dm_action,
            } => StoredEventEffect::Custom {
                description: description.clone(),
                requires_dm_action: *requires_dm_action,
            },
        }
    }
}

impl From<&ChainedEvent> for StoredChainedEvent {
    fn from(c: &ChainedEvent) -> Self {
        Self {
            event_id: c.event_id.to_string(),
            event_name: c.event_name.clone(),
            delay_turns: c.delay_turns,
            additional_trigger: c
                .additional_trigger
                .as_ref()
                .map(|t| Box::new(StoredNarrativeTriggerType::from(t))),
            chain_reason: c.chain_reason.clone(),
        }
    }
}

// ============================================================================
// Stored -> Domain Conversions (Deserialization)
// ============================================================================

impl From<StoredNarrativeTrigger> for NarrativeTrigger {
    fn from(s: StoredNarrativeTrigger) -> Self {
        Self {
            trigger_type: NarrativeTriggerType::from(s.trigger_type),
            description: s.description,
            is_required: s.is_required,
            trigger_id: s.trigger_id,
        }
    }
}

impl From<StoredNarrativeTriggerType> for NarrativeTriggerType {
    fn from(s: StoredNarrativeTriggerType) -> Self {
        match s {
            StoredNarrativeTriggerType::NpcAction {
                npc_id,
                npc_name,
                action_keywords,
                action_description,
            } => NarrativeTriggerType::NpcAction {
                npc_id: CharacterId::from(parse_uuid_or_nil(
                    &npc_id,
                    "NarrativeTriggerType::NpcAction.npc_id",
                )),
                npc_name,
                action_keywords,
                action_description,
            },
            StoredNarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name,
            } => NarrativeTriggerType::PlayerEntersLocation {
                location_id: LocationId::from(parse_uuid_or_nil(
                    &location_id,
                    "NarrativeTriggerType::PlayerEntersLocation.location_id",
                )),
                location_name,
            },
            StoredNarrativeTriggerType::TimeAtLocation {
                location_id,
                location_name,
                time_context,
            } => NarrativeTriggerType::TimeAtLocation {
                location_id: LocationId::from(parse_uuid_or_nil(
                    &location_id,
                    "NarrativeTriggerType::TimeAtLocation.location_id",
                )),
                location_name,
                time_context,
            },
            StoredNarrativeTriggerType::DialogueTopic {
                keywords,
                with_npc,
                npc_name,
            } => NarrativeTriggerType::DialogueTopic {
                keywords,
                with_npc: with_npc.and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
                npc_name,
            },
            StoredNarrativeTriggerType::ChallengeCompleted {
                challenge_id,
                challenge_name,
                requires_success,
            } => NarrativeTriggerType::ChallengeCompleted {
                challenge_id: ChallengeId::from(parse_uuid_or_nil(
                    &challenge_id,
                    "NarrativeTriggerType::ChallengeCompleted.challenge_id",
                )),
                challenge_name,
                requires_success,
            },
            StoredNarrativeTriggerType::RelationshipThreshold {
                character_id,
                character_name,
                with_character,
                with_character_name,
                min_sentiment,
                max_sentiment,
            } => NarrativeTriggerType::RelationshipThreshold {
                character_id: CharacterId::from(parse_uuid_or_nil(
                    &character_id,
                    "NarrativeTriggerType::RelationshipThreshold.character_id",
                )),
                character_name,
                with_character: CharacterId::from(parse_uuid_or_nil(
                    &with_character,
                    "NarrativeTriggerType::RelationshipThreshold.with_character",
                )),
                with_character_name,
                min_sentiment,
                max_sentiment,
            },
            StoredNarrativeTriggerType::HasItem {
                item_name,
                quantity,
            } => NarrativeTriggerType::HasItem {
                item_name,
                quantity,
            },
            StoredNarrativeTriggerType::MissingItem { item_name } => {
                NarrativeTriggerType::MissingItem { item_name }
            }
            StoredNarrativeTriggerType::EventCompleted {
                event_id,
                event_name,
                outcome_name,
            } => NarrativeTriggerType::EventCompleted {
                event_id: NarrativeEventId::from(parse_uuid_or_nil(
                    &event_id,
                    "NarrativeTriggerType::EventCompleted.event_id",
                )),
                event_name,
                outcome_name,
            },
            StoredNarrativeTriggerType::TurnCount { turns, since_event } => {
                NarrativeTriggerType::TurnCount {
                    turns,
                    since_event: since_event
                        .and_then(|id| Uuid::parse_str(&id).ok().map(NarrativeEventId::from)),
                }
            }
            StoredNarrativeTriggerType::FlagSet { flag_name } => {
                NarrativeTriggerType::FlagSet { flag_name }
            }
            StoredNarrativeTriggerType::FlagNotSet { flag_name } => {
                NarrativeTriggerType::FlagNotSet { flag_name }
            }
            StoredNarrativeTriggerType::StatThreshold {
                character_id,
                stat_name,
                min_value,
                max_value,
            } => NarrativeTriggerType::StatThreshold {
                character_id: CharacterId::from(parse_uuid_or_nil(
                    &character_id,
                    "NarrativeTriggerType::StatThreshold.character_id",
                )),
                stat_name,
                min_value,
                max_value,
            },
            StoredNarrativeTriggerType::CombatResult {
                victory,
                involved_npc,
            } => NarrativeTriggerType::CombatResult {
                victory,
                involved_npc: involved_npc
                    .and_then(|id| Uuid::parse_str(&id).ok().map(CharacterId::from)),
            },
            StoredNarrativeTriggerType::Custom {
                description,
                llm_evaluation,
            } => NarrativeTriggerType::Custom {
                description,
                llm_evaluation,
            },
        }
    }
}

impl From<StoredEventOutcome> for EventOutcome {
    fn from(s: StoredEventOutcome) -> Self {
        Self {
            name: s.name,
            label: s.label,
            description: s.description,
            condition: s.condition.map(OutcomeCondition::from),
            effects: s.effects.into_iter().map(EventEffect::from).collect(),
            chain_events: s.chain_events.into_iter().map(ChainedEvent::from).collect(),
            timeline_summary: s.timeline_summary,
        }
    }
}

impl From<StoredOutcomeCondition> for OutcomeCondition {
    fn from(s: StoredOutcomeCondition) -> Self {
        match s {
            StoredOutcomeCondition::DmChoice => OutcomeCondition::DmChoice,
            StoredOutcomeCondition::ChallengeResult {
                challenge_id,
                success_required,
            } => OutcomeCondition::ChallengeResult {
                challenge_id: challenge_id
                    .and_then(|id| Uuid::parse_str(&id).ok().map(ChallengeId::from)),
                success_required,
            },
            StoredOutcomeCondition::CombatResult { victory_required } => {
                OutcomeCondition::CombatResult { victory_required }
            }
            StoredOutcomeCondition::DialogueChoice { keywords } => {
                OutcomeCondition::DialogueChoice { keywords }
            }
            StoredOutcomeCondition::PlayerAction { action_keywords } => {
                OutcomeCondition::PlayerAction { action_keywords }
            }
            StoredOutcomeCondition::HasItem { item_name } => {
                OutcomeCondition::HasItem { item_name }
            }
            StoredOutcomeCondition::Custom { description } => {
                OutcomeCondition::Custom { description }
            }
        }
    }
}

impl From<StoredEventEffect> for EventEffect {
    fn from(s: StoredEventEffect) -> Self {
        match s {
            StoredEventEffect::ModifyRelationship {
                from_character,
                from_name,
                to_character,
                to_name,
                sentiment_change,
                reason,
            } => EventEffect::ModifyRelationship {
                from_character: CharacterId::from(parse_uuid_or_nil(
                    &from_character,
                    "EventEffect::ModifyRelationship.from_character",
                )),
                from_name,
                to_character: CharacterId::from(parse_uuid_or_nil(
                    &to_character,
                    "EventEffect::ModifyRelationship.to_character",
                )),
                to_name,
                sentiment_change,
                reason,
            },
            StoredEventEffect::GiveItem {
                item_name,
                item_description,
                quantity,
            } => EventEffect::GiveItem {
                item_name,
                item_description,
                quantity,
            },
            StoredEventEffect::TakeItem {
                item_name,
                quantity,
            } => EventEffect::TakeItem {
                item_name,
                quantity,
            },
            StoredEventEffect::RevealInformation {
                info_type,
                title,
                content,
                persist_to_journal,
            } => EventEffect::RevealInformation {
                info_type,
                title,
                content,
                persist_to_journal,
            },
            StoredEventEffect::SetFlag { flag_name, value } => {
                EventEffect::SetFlag { flag_name, value }
            }
            StoredEventEffect::EnableChallenge {
                challenge_id,
                challenge_name,
            } => EventEffect::EnableChallenge {
                challenge_id: ChallengeId::from(parse_uuid_or_nil(
                    &challenge_id,
                    "EventEffect::EnableChallenge.challenge_id",
                )),
                challenge_name,
            },
            StoredEventEffect::DisableChallenge {
                challenge_id,
                challenge_name,
            } => EventEffect::DisableChallenge {
                challenge_id: ChallengeId::from(parse_uuid_or_nil(
                    &challenge_id,
                    "EventEffect::DisableChallenge.challenge_id",
                )),
                challenge_name,
            },
            StoredEventEffect::EnableEvent {
                event_id,
                event_name,
            } => EventEffect::EnableEvent {
                event_id: NarrativeEventId::from(parse_uuid_or_nil(
                    &event_id,
                    "EventEffect::EnableEvent.event_id",
                )),
                event_name,
            },
            StoredEventEffect::DisableEvent {
                event_id,
                event_name,
            } => EventEffect::DisableEvent {
                event_id: NarrativeEventId::from(parse_uuid_or_nil(
                    &event_id,
                    "EventEffect::DisableEvent.event_id",
                )),
                event_name,
            },
            StoredEventEffect::TriggerScene {
                scene_id,
                scene_name,
            } => EventEffect::TriggerScene {
                scene_id: SceneId::from(parse_uuid_or_nil(
                    &scene_id,
                    "EventEffect::TriggerScene.scene_id",
                )),
                scene_name,
            },
            StoredEventEffect::StartCombat {
                participants,
                participant_names,
                combat_description,
            } => EventEffect::StartCombat {
                participants: participants
                    .into_iter()
                    .filter_map(|id| Uuid::parse_str(&id).ok().map(CharacterId::from))
                    .collect(),
                participant_names,
                combat_description,
            },
            StoredEventEffect::ModifyStat {
                character_id,
                character_name,
                stat_name,
                modifier,
            } => EventEffect::ModifyStat {
                character_id: CharacterId::from(parse_uuid_or_nil(
                    &character_id,
                    "EventEffect::ModifyStat.character_id",
                )),
                character_name,
                stat_name,
                modifier,
            },
            StoredEventEffect::AddReward {
                reward_type,
                amount,
                description,
            } => EventEffect::AddReward {
                reward_type,
                amount,
                description,
            },
            StoredEventEffect::Custom {
                description,
                requires_dm_action,
            } => EventEffect::Custom {
                description,
                requires_dm_action,
            },
        }
    }
}

impl From<StoredChainedEvent> for ChainedEvent {
    fn from(s: StoredChainedEvent) -> Self {
        Self {
            event_id: NarrativeEventId::from(parse_uuid_or_nil(
                &s.event_id,
                "ChainedEvent.event_id",
            )),
            event_name: s.event_name,
            delay_turns: s.delay_turns,
            additional_trigger: s.additional_trigger.map(|t| NarrativeTriggerType::from(*t)),
            chain_reason: s.chain_reason,
        }
    }
}
