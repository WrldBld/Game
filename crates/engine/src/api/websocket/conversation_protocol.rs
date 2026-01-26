// Protocol conversion helpers for conversation types
//!
//! These methods convert domain types to wire format types.
//! This module lives in the API layer to comply with ADR-011,
//! which requires protocol conversions to happen at the API boundary.

use wrldbldr_shared::messages::{
    ConversationFullDetails as ProtocolConversationFullDetails,
    ConversationInfo as ProtocolConversationInfo,
    ConversationParticipant as ProtocolConversationParticipant,
    DialogueTurn as ProtocolDialogueTurn, LocationContext as ProtocolLocationContext,
    ParticipantType as ProtocolParticipantType, SceneContext as ProtocolSceneContext,
};

// Use case DTOs (not infrastructure types)
use crate::use_cases::conversation::{
    ActiveConversationSummary, ConversationDetailResult, DialogueTurnDetail, LocationSummary,
    ParticipantDetail, ParticipantType as UseCaseParticipantType, SceneSummary,
};

impl ActiveConversationSummary {
    /// Convert to protocol message type.
    /// This handles conversion from use case DTOs to protocol format.
    pub fn to_protocol(&self) -> ProtocolConversationInfo {
        ProtocolConversationInfo {
            conversation_id: self.id.to_string(),
            topic_hint: self.topic_hint.clone(),
            started_at: self
                .started_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            last_updated_at: self
                .last_updated_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            is_active: self.is_active,
            participants: vec![
                ProtocolConversationParticipant {
                    id: self.pc_id.to_string(),
                    name: self.pc_name.clone(),
                    participant_type: ProtocolParticipantType::Pc,
                    turn_count: (self.turn_count + 1) / 2, // Approximate split
                    last_spoke_at: Some(
                        self.last_updated_at
                            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    ),
                    want: None,
                    relationship: None,
                },
                ProtocolConversationParticipant {
                    id: self.npc_id.to_string(),
                    name: self.npc_name.clone(),
                    participant_type: ProtocolParticipantType::Npc,
                    turn_count: self.turn_count / 2,
                    last_spoke_at: Some(
                        self.last_updated_at
                            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    ),
                    want: None,
                    relationship: None,
                },
            ],
            location: self.location.as_ref().map(|l| l.to_protocol()),
            scene: self.scene.as_ref().map(|s| s.to_protocol()),
            turn_count: self.turn_count,
            pending_approval: self.pending_approval,
        }
    }
}

impl ConversationDetailResult {
    /// Convert to protocol message type.
    pub fn to_protocol(&self) -> ProtocolConversationFullDetails {
        let recent_turns = self.recent_turns.iter().map(|t| t.to_protocol()).collect();

        ProtocolConversationFullDetails {
            conversation_id: self.conversation.id.to_string(),
            topic_hint: self.conversation.topic_hint.clone(),
            started_at: self
                .conversation
                .started_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            last_updated_at: self
                .conversation
                .last_updated_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            is_active: self.conversation.is_active,
            participants: self.participants.iter().map(|p| p.to_protocol()).collect(),
            location: self.conversation.location.as_ref().map(|l| l.to_protocol()),
            scene: self.conversation.scene.as_ref().map(|s| s.to_protocol()),
            turn_count: self.conversation.turn_count,
            pending_approval: self.conversation.pending_approval,
            recent_turns,
        }
    }
}

impl ParticipantDetail {
    /// Convert to protocol message type.
    pub fn to_protocol(&self) -> ProtocolConversationParticipant {
        ProtocolConversationParticipant {
            id: self.character_id.to_string(),
            name: self.name.clone(),
            participant_type: match self.participant_type {
                UseCaseParticipantType::Pc => ProtocolParticipantType::Pc,
                UseCaseParticipantType::Npc => ProtocolParticipantType::Npc,
            },
            turn_count: self.turn_count,
            last_spoke_at: self
                .last_spoke_at
                .as_ref()
                .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
            want: self.want.clone(),
            relationship: self.relationship.clone(),
        }
    }
}

impl DialogueTurnDetail {
    /// Convert to protocol message type.
    pub fn to_protocol(&self) -> ProtocolDialogueTurn {
        ProtocolDialogueTurn {
            speaker_name: self.speaker_name.clone(),
            text: self.text.clone(),
            timestamp: self
                .timestamp
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            is_dm_override: self.is_dm_override,
        }
    }
}

impl LocationSummary {
    /// Convert to protocol message type.
    pub fn to_protocol(&self) -> ProtocolLocationContext {
        ProtocolLocationContext {
            location_id: self.location_id.to_string(),
            location_name: self.location_name.clone(),
            region_name: self.region_name.clone(),
        }
    }
}

impl SceneSummary {
    /// Convert to protocol message type.
    pub fn to_protocol(&self) -> ProtocolSceneContext {
        ProtocolSceneContext {
            scene_id: self.scene_id.to_string(),
            scene_name: self.scene_name.clone(),
        }
    }
}
