//! Translates ServerMessage from protocol to PlayerEvent for application layer
//!
//! This module provides the translation layer between the protocol's ServerMessage
//! and the application layer's PlayerEvent. This follows hexagonal architecture
//! by ensuring the application layer doesn't depend on protocol types.
//!
//! # Design
//!
//! All 65 ServerMessage variants are explicitly handled - none fall through to Raw.
//! The Raw variant is available for future extensibility but currently unused.
//!
//! # Type Consolidation (Phase 3 Remediation)
//!
//! Many types are now shared between protocol and player-ports via re-exports.
//! This simplifies translation - types with exact field matches are passed through
//! directly without conversion. Only types that intentionally differ (String vs
//! typed enums) require translation functions.

use crate::ports::outbound::player_events::{
    ActantialViewData, ConnectedUser, EntityChangedData, JoinError, PlayerEvent, ResponseResult,
    WantData, WantTargetData, WorldRole,
};
// Note: Types like SceneData, CharacterData, GameTime, etc. are now re-exported from
// protocol in player-ports, so no translation is needed - they're the same type.
use wrldbldr_shared::ServerMessage;

/// Translate a ServerMessage into a PlayerEvent
///
/// This function handles all ServerMessage variants, converting them to
/// domain-friendly PlayerEvent types for the application layer.
pub fn translate(msg: ServerMessage) -> PlayerEvent {
    match msg {
        // =====================================================================
        // Connection Events
        // =====================================================================
        ServerMessage::WorldJoined {
            world_id,
            snapshot,
            connected_users,
            your_role,
            your_pc,
        } => PlayerEvent::WorldJoined {
            world_id,
            snapshot,
            connected_users: connected_users
                .into_iter()
                .map(translate_connected_user)
                .collect(),
            your_role: WorldRole(format!("{:?}", your_role)),
            your_pc,
        },

        ServerMessage::WorldJoinFailed { world_id, error } => PlayerEvent::WorldJoinFailed {
            world_id,
            error: JoinError {
                code: format!("{:?}", error),
                message: format!("{:?}", error),
            },
        },

        ServerMessage::UserJoined {
            user_id,
            username,
            role,
            pc,
        } => PlayerEvent::UserJoined {
            user_id,
            username,
            role: WorldRole(format!("{:?}", role)),
            pc,
        },

        ServerMessage::UserLeft { user_id } => PlayerEvent::UserLeft { user_id },

        ServerMessage::Pong => PlayerEvent::Pong,

        // =====================================================================
        // Scene & Navigation Events
        // =====================================================================
        // These types are now shared (re-exported from protocol), so direct assignment works
        ServerMessage::SceneUpdate {
            scene,
            characters,
            interactions,
        } => PlayerEvent::SceneUpdate {
            scene,
            characters,
            interactions,
        },

        ServerMessage::SceneChanged {
            pc_id,
            region,
            npcs_present,
            navigation,
            region_items,
        } => PlayerEvent::SceneChanged {
            pc_id,
            region,
            npcs_present,
            navigation,
            region_items,
        },

        ServerMessage::PcSelected {
            pc_id,
            pc_name,
            location_id,
            region_id,
        } => PlayerEvent::PcSelected {
            pc_id,
            pc_name,
            location_id,
            region_id,
        },

        ServerMessage::MovementBlocked { pc_id, reason } => {
            PlayerEvent::MovementBlocked { pc_id, reason }
        }

        ServerMessage::SplitPartyNotification {
            location_count,
            locations,
        } => PlayerEvent::SplitPartyNotification {
            location_count,
            locations, // Direct assignment - same type now
        },

        // =====================================================================
        // Action & Queue Events
        // =====================================================================
        ServerMessage::ActionReceived {
            action_id,
            player_id,
            action_type,
        } => PlayerEvent::ActionReceived {
            action_id,
            player_id,
            action_type,
        },

        ServerMessage::ActionQueued {
            action_id,
            player_name,
            action_type,
            queue_depth,
        } => PlayerEvent::ActionQueued {
            action_id,
            player_name,
            action_type,
            queue_depth,
        },

        ServerMessage::LLMProcessing { action_id } => PlayerEvent::LLMProcessing { action_id },

        ServerMessage::QueueStatus {
            player_actions_pending,
            llm_requests_pending,
            llm_requests_processing,
            approvals_pending,
        } => PlayerEvent::QueueStatus {
            player_actions_pending,
            llm_requests_pending,
            llm_requests_processing,
            approvals_pending,
        },

        // =====================================================================
        // Dialogue Events
        // =====================================================================
        ServerMessage::ConversationStarted {
            conversation_id,
            npc_id,
            npc_name,
            npc_disposition,
        } => PlayerEvent::ConversationStarted {
            conversation_id,
            npc_id,
            npc_name,
            npc_disposition,
        },

        ServerMessage::DialogueResponse {
            speaker_id,
            speaker_name,
            text,
            choices,
            conversation_id,
        } => PlayerEvent::DialogueResponse {
            speaker_id,
            speaker_name,
            text,
            choices, // Direct assignment - same type now
            conversation_id,
        },

        ServerMessage::ConversationEnded {
            npc_id,
            npc_name,
            pc_id,
            summary,
            conversation_id,
        } => PlayerEvent::ConversationEnded {
            npc_id,
            npc_name,
            pc_id,
            summary,
            conversation_id,
        },

        ServerMessage::ResponseApproved {
            npc_dialogue,
            executed_tools,
        } => PlayerEvent::ResponseApproved {
            npc_dialogue,
            executed_tools,
        },

        // =====================================================================
        // Approval Events
        // =====================================================================
        ServerMessage::ApprovalRequired {
            request_id,
            npc_name,
            proposed_dialogue,
            internal_reasoning,
            proposed_tools,
            challenge_suggestion,
            narrative_event_suggestion,
        } => PlayerEvent::ApprovalRequired {
            request_id,
            npc_name,
            proposed_dialogue,
            internal_reasoning,
            proposed_tools, // Direct assignment - same type now
            challenge_suggestion,
            narrative_event_suggestion,
        },

        // =====================================================================
        // Challenge Events
        // =====================================================================
        ServerMessage::ChallengePrompt {
            challenge_id,
            challenge_name,
            skill_name,
            difficulty_display,
            description,
            character_modifier,
            suggested_dice,
            rule_system_hint,
        } => PlayerEvent::ChallengePrompt {
            challenge_id,
            challenge_name,
            skill_name,
            difficulty_display,
            description,
            character_modifier,
            suggested_dice,
            rule_system_hint,
        },

        ServerMessage::ChallengeResolved {
            challenge_id,
            challenge_name,
            character_name,
            roll,
            modifier,
            total,
            outcome,
            outcome_description,
            roll_breakdown,
            individual_rolls,
        } => PlayerEvent::ChallengeResolved {
            challenge_id,
            challenge_name,
            character_name,
            roll,
            modifier,
            total,
            outcome,
            outcome_description,
            roll_breakdown,
            individual_rolls,
        },

        ServerMessage::ChallengeRollSubmitted {
            challenge_id,
            challenge_name,
            roll,
            modifier,
            total,
            outcome_type,
            status,
        } => PlayerEvent::ChallengeRollSubmitted {
            challenge_id,
            challenge_name,
            roll,
            modifier,
            total,
            outcome_type,
            status,
        },

        ServerMessage::ChallengeOutcomePending {
            resolution_id,
            challenge_id,
            challenge_name,
            character_id,
            character_name,
            roll,
            modifier,
            total,
            outcome_type,
            outcome_description,
            outcome_triggers,
            roll_breakdown,
        } => PlayerEvent::ChallengeOutcomePending {
            resolution_id,
            challenge_id,
            challenge_name,
            character_id,
            character_name,
            roll,
            modifier,
            total,
            outcome_type,
            outcome_description,
            outcome_triggers, // Direct assignment - same type now
            roll_breakdown,
        },

        ServerMessage::OutcomeRegenerated {
            request_id,
            outcome_type,
            new_outcome,
        } => PlayerEvent::OutcomeRegenerated {
            request_id,
            outcome_type,
            new_outcome, // Direct assignment - same type now
        },

        ServerMessage::ChallengeDiscarded { request_id } => {
            PlayerEvent::ChallengeDiscarded { request_id }
        }

        ServerMessage::AdHocChallengeCreated {
            challenge_id,
            challenge_name,
            target_pc_id,
        } => PlayerEvent::AdHocChallengeCreated {
            challenge_id,
            challenge_name,
            target_pc_id,
        },

        ServerMessage::OutcomeSuggestionReady {
            resolution_id,
            suggestions,
        } => PlayerEvent::OutcomeSuggestionReady {
            resolution_id,
            suggestions,
        },

        ServerMessage::OutcomeBranchesReady {
            resolution_id,
            outcome_type,
            branches,
        } => PlayerEvent::OutcomeBranchesReady {
            resolution_id,
            outcome_type,
            branches, // Direct assignment - same type now
        },

        // =====================================================================
        // Narrative Events
        // =====================================================================
        ServerMessage::NarrativeEventTriggered {
            event_id,
            event_name,
            outcome_description,
            scene_direction,
        } => PlayerEvent::NarrativeEventTriggered {
            event_id,
            event_name,
            outcome_description,
            scene_direction,
        },

        ServerMessage::ApproachEvent {
            npc_id,
            npc_name,
            npc_sprite,
            description,
            reveal,
        } => PlayerEvent::ApproachEvent {
            npc_id,
            npc_name,
            npc_sprite,
            description,
            reveal,
        },

        ServerMessage::LocationEvent {
            region_id,
            description,
        } => PlayerEvent::LocationEvent {
            region_id,
            description,
        },

        ServerMessage::NpcLocationShared {
            npc_id,
            npc_name,
            region_name,
            notes,
        } => PlayerEvent::NpcLocationShared {
            npc_id,
            npc_name,
            region_name,
            notes,
        },

        // =====================================================================
        // Staging Events
        // =====================================================================
        ServerMessage::StagingApprovalRequired {
            request_id,
            region_id,
            region_name,
            location_id,
            location_name,
            game_time,
            previous_staging,
            rule_based_npcs,
            llm_based_npcs,
            default_ttl_hours,
            waiting_pcs,
            resolved_visual_state,
            available_location_states,
            available_region_states,
        } => PlayerEvent::StagingApprovalRequired {
            request_id,
            region_id,
            region_name,
            location_id,
            location_name,
            game_time,        // Direct assignment - same type now
            previous_staging, // Direct assignment - same type now
            rule_based_npcs,  // Direct assignment - same type now
            llm_based_npcs,
            default_ttl_hours,
            waiting_pcs, // Direct assignment - same type now
            resolved_visual_state,
            available_location_states,
            available_region_states,
        },

        ServerMessage::StagingPending {
            region_id,
            region_name,
            timeout_seconds,
        } => PlayerEvent::StagingPending {
            region_id,
            region_name,
            timeout_seconds,
        },

        ServerMessage::StagingReady {
            region_id,
            npcs_present,
            visual_state,
        } => PlayerEvent::StagingReady {
            region_id,
            npcs_present, // Direct assignment - same type now
            visual_state,
        },

        ServerMessage::StagingRegenerated {
            request_id,
            llm_based_npcs,
        } => PlayerEvent::StagingRegenerated {
            request_id,
            llm_based_npcs, // Direct assignment - same type now
        },

        ServerMessage::StagingTimedOut {
            region_id,
            region_name,
        } => PlayerEvent::StagingTimedOut {
            region_id,
            region_name,
        },

        // =====================================================================
        // Lore Events
        // =====================================================================
        ServerMessage::LoreDiscovered {
            character_id,
            lore,
            discovered_chunk_ids,
            discovery_source,
        } => PlayerEvent::LoreDiscovered {
            character_id,
            lore,
            discovered_chunk_ids,
            discovery_source,
        },

        ServerMessage::LoreRevoked {
            character_id,
            lore_id,
        } => PlayerEvent::LoreRevoked {
            character_id,
            lore_id,
        },

        ServerMessage::LoreUpdated { lore } => PlayerEvent::LoreUpdated { lore },

        ServerMessage::CharacterLoreResponse {
            character_id,
            known_lore,
        } => PlayerEvent::CharacterLoreResponse {
            character_id,
            known_lore,
        },

        // =====================================================================
        // Inventory Events
        // =====================================================================
        ServerMessage::ItemEquipped {
            pc_id,
            item_id,
            item_name,
        } => PlayerEvent::ItemEquipped {
            pc_id,
            item_id,
            item_name,
        },

        ServerMessage::ItemUnequipped {
            pc_id,
            item_id,
            item_name,
        } => PlayerEvent::ItemUnequipped {
            pc_id,
            item_id,
            item_name,
        },

        ServerMessage::ItemDropped {
            pc_id,
            item_id,
            item_name,
            quantity,
        } => PlayerEvent::ItemDropped {
            pc_id,
            item_id,
            item_name,
            quantity,
        },

        ServerMessage::ItemPickedUp {
            pc_id,
            item_id,
            item_name,
        } => PlayerEvent::ItemPickedUp {
            pc_id,
            item_id,
            item_name,
        },

        ServerMessage::InventoryUpdated { pc_id } => PlayerEvent::InventoryUpdated { pc_id },

        // =====================================================================
        // Character Events
        // =====================================================================
        ServerMessage::CharacterStatUpdated {
            character_id,
            character_name,
            stat_name,
            old_value,
            new_value,
            delta,
            source,
        } => PlayerEvent::CharacterStatUpdated {
            character_id,
            character_name,
            stat_name,
            old_value,
            new_value,
            delta,
            source,
        },

        ServerMessage::NpcDispositionChanged {
            npc_id,
            npc_name,
            pc_id,
            disposition,
            relationship,
            reason,
        } => PlayerEvent::NpcDispositionChanged {
            npc_id,
            npc_name,
            pc_id,
            disposition,
            relationship,
            reason,
        },

        ServerMessage::NpcMoodChanged {
            npc_id,
            npc_name,
            old_mood,
            new_mood,
            reason,
            region_id,
        } => PlayerEvent::NpcMoodChanged {
            npc_id,
            npc_name,
            old_mood,
            new_mood,
            reason,
            region_id,
        },

        ServerMessage::NpcDispositionsResponse {
            pc_id,
            dispositions,
        } => PlayerEvent::NpcDispositionsResponse {
            pc_id,
            dispositions, // Direct assignment - same type now
        },

        // =====================================================================
        // Actantial Model Events
        // =====================================================================
        ServerMessage::NpcWantCreated { npc_id, want } => PlayerEvent::NpcWantCreated {
            npc_id,
            want: translate_want_data(want),
        },

        ServerMessage::NpcWantUpdated { npc_id, want } => PlayerEvent::NpcWantUpdated {
            npc_id,
            want: translate_want_data(want),
        },

        ServerMessage::NpcWantDeleted { npc_id, want_id } => {
            PlayerEvent::NpcWantDeleted { npc_id, want_id }
        }

        ServerMessage::WantTargetSet { want_id, target } => PlayerEvent::WantTargetSet {
            want_id,
            target: translate_want_target_data(target),
        },

        ServerMessage::WantTargetRemoved { want_id } => PlayerEvent::WantTargetRemoved { want_id },

        ServerMessage::ActantialViewAdded { npc_id, view } => PlayerEvent::ActantialViewAdded {
            npc_id,
            view: translate_actantial_view_data(view),
        },

        ServerMessage::ActantialViewRemoved {
            npc_id,
            want_id,
            target_id,
            role,
        } => PlayerEvent::ActantialViewRemoved {
            npc_id,
            want_id,
            target_id,
            role: format!("{:?}", role),
        },

        ServerMessage::NpcActantialContextResponse { npc_id, context } => {
            PlayerEvent::NpcActantialContextResponse {
                npc_id,
                // Serialize the complex context to JSON for now
                context: serde_json::to_value(context).unwrap_or(serde_json::Value::Null),
            }
        }

        ServerMessage::WorldGoalsResponse { world_id, goals } => PlayerEvent::WorldGoalsResponse {
            world_id,
            goals, // Direct assignment - same type now
        },

        ServerMessage::GoalCreated { world_id, goal } => PlayerEvent::GoalCreated {
            world_id,
            goal, // Direct assignment - same type now
        },

        ServerMessage::GoalUpdated { goal } => PlayerEvent::GoalUpdated {
            goal, // Direct assignment - same type now
        },

        ServerMessage::GoalDeleted { goal_id } => PlayerEvent::GoalDeleted { goal_id },

        ServerMessage::DeflectionSuggestions {
            npc_id,
            want_id,
            suggestions,
        } => PlayerEvent::DeflectionSuggestions {
            npc_id,
            want_id,
            suggestions,
        },

        ServerMessage::TellsSuggestions {
            npc_id,
            want_id,
            suggestions,
        } => PlayerEvent::TellsSuggestions {
            npc_id,
            want_id,
            suggestions,
        },

        ServerMessage::WantDescriptionSuggestions {
            npc_id,
            suggestions,
        } => PlayerEvent::WantDescriptionSuggestions {
            npc_id,
            suggestions,
        },

        ServerMessage::ActantialReasonSuggestions {
            npc_id,
            want_id,
            target_id,
            role,
            suggestions,
        } => PlayerEvent::ActantialReasonSuggestions {
            npc_id,
            want_id,
            target_id,
            role: format!("{:?}", role),
            suggestions,
        },

        // =====================================================================
        // Generation Events
        // =====================================================================
        ServerMessage::GenerationQueued {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            position,
        } => PlayerEvent::GenerationQueued {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            position,
        },

        ServerMessage::GenerationProgress { batch_id, progress } => {
            PlayerEvent::GenerationProgress { batch_id, progress }
        }

        ServerMessage::GenerationComplete {
            batch_id,
            asset_count,
        } => PlayerEvent::GenerationComplete {
            batch_id,
            asset_count,
        },

        ServerMessage::GenerationFailed { batch_id, error } => {
            PlayerEvent::GenerationFailed { batch_id, error }
        }

        ServerMessage::SuggestionQueued {
            request_id,
            field_type,
            entity_id,
        } => PlayerEvent::SuggestionQueued {
            request_id,
            field_type,
            entity_id,
        },

        ServerMessage::SuggestionProgress { request_id, status } => {
            PlayerEvent::SuggestionProgress { request_id, status }
        }

        ServerMessage::SuggestionComplete {
            request_id,
            suggestions,
        } => PlayerEvent::SuggestionComplete {
            request_id,
            suggestions,
        },

        ServerMessage::SuggestionFailed { request_id, error } => {
            PlayerEvent::SuggestionFailed { request_id, error }
        }

        ServerMessage::ComfyUIStateChanged {
            state,
            message,
            retry_in_seconds,
        } => PlayerEvent::ComfyUIStateChanged {
            state,
            message,
            retry_in_seconds,
        },

        // =====================================================================
        // Time Events
        // =====================================================================
        ServerMessage::GameTimeUpdated { game_time } => PlayerEvent::GameTimeUpdated {
            game_time, // Direct assignment - same type now
        },

        ServerMessage::GameTimeAdvanced { data } => PlayerEvent::GameTimeAdvanced {
            previous_time: data.previous_time,
            new_time: data.new_time,
            seconds_advanced: data.seconds_advanced,
            reason: data.reason,
            period_changed: data.period_changed,
            new_period: data.new_period,
        },

        ServerMessage::TimeSuggestion { data } => PlayerEvent::TimeSuggestion {
            suggestion_id: data.suggestion_id,
            pc_id: data.pc_id,
            pc_name: data.pc_name,
            action_type: data.action_type,
            action_description: data.action_description,
            suggested_seconds: data.suggested_seconds,
            current_time: data.current_time,
            resulting_time: data.resulting_time,
            period_change: data.period_change,
        },

        ServerMessage::TimeModeChanged { world_id, mode } => PlayerEvent::TimeModeChanged {
            world_id,
            mode: format!("{:?}", mode).to_lowercase(),
        },

        ServerMessage::GameTimePaused { world_id, paused } => {
            PlayerEvent::GameTimePaused { world_id, paused }
        }

        ServerMessage::TimeConfigUpdated { world_id, config } => PlayerEvent::TimeConfigUpdated {
            world_id,
            mode: format!("{:?}", config.mode).to_lowercase(),
            show_time_to_players: config.show_time_to_players,
        },

        // =====================================================================
        // Request/Response Events
        // =====================================================================
        ServerMessage::Response { request_id, result } => PlayerEvent::Response {
            request_id,
            result: translate_response_result(result),
        },

        ServerMessage::EntityChanged(data) => {
            PlayerEvent::EntityChanged(translate_entity_changed_data(data))
        }

        ServerMessage::SpectateTargetChanged { pc_id, pc_name } => {
            PlayerEvent::SpectateTargetChanged { pc_id, pc_name }
        }

        // =====================================================================
        // Error Events
        // =====================================================================
        ServerMessage::Error { code, message } => PlayerEvent::Error { code, message },

        // Unknown message types for forward compatibility - ignore silently
        ServerMessage::Unknown => PlayerEvent::Error {
            code: "UNKNOWN_MESSAGE".to_string(),
            message: "Unknown server message type".to_string(),
        },
    }
}

// =============================================================================
// Helper Translation Functions
// =============================================================================
// Only types that intentionally differ between protocol and player-ports need
// translation. Types with exact field matches are passed through directly.

fn translate_connected_user(u: wrldbldr_shared::responses::ConnectedUser) -> ConnectedUser {
    ConnectedUser {
        user_id: u.user_id,
        username: u.username,
        role: format!("{:?}", u.role),
        pc_id: u.pc_id,
        connection_count: u.connection_count,
    }
}

fn translate_want_data(w: wrldbldr_shared::WantData) -> WantData {
    WantData {
        id: w.id,
        description: w.description,
        intensity: w.intensity,
        priority: w.priority,
        visibility: format!("{:?}", w.visibility),
        target: w.target.map(translate_want_target_data),
        deflection_behavior: w.deflection_behavior,
        tells: w.tells,
    }
}

fn translate_want_target_data(t: wrldbldr_shared::WantTargetData) -> WantTargetData {
    WantTargetData {
        id: t.id,
        name: t.name,
        target_type: format!("{:?}", t.target_type),
        description: t.description,
    }
}

fn translate_actantial_view_data(v: wrldbldr_shared::ActantialViewData) -> ActantialViewData {
    ActantialViewData {
        want_id: v.want_id,
        target_id: v.target_id,
        target_name: v.target_name,
        target_type: format!("{:?}", v.target_type),
        role: format!("{:?}", v.role),
        reason: v.reason,
    }
}

fn translate_response_result(r: wrldbldr_shared::responses::ResponseResult) -> ResponseResult {
    match r {
        wrldbldr_shared::responses::ResponseResult::Success { data } => ResponseResult {
            success: true,
            data,
            error_code: None,
            error_message: None,
            error_details: None,
        },
        wrldbldr_shared::responses::ResponseResult::Error {
            code,
            message,
            details,
        } => ResponseResult {
            success: false,
            data: None,
            error_code: Some(format!("{:?}", code)),
            error_message: Some(message),
            error_details: details,
        },
        wrldbldr_shared::responses::ResponseResult::Unknown => ResponseResult {
            success: false,
            data: None,
            error_code: Some("UNKNOWN".to_string()),
            error_message: Some("Unknown response type".to_string()),
            error_details: None,
        },
    }
}

fn translate_entity_changed_data(
    e: wrldbldr_shared::responses::EntityChangedData,
) -> EntityChangedData {
    EntityChangedData {
        entity_type: format!("{:?}", e.entity_type),
        entity_id: e.entity_id,
        change_type: format!("{:?}", e.change_type),
        data: e.data,
        world_id: e.world_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_shared::types::{GameTime, TimeAdvanceData, TimeSuggestionData};

    #[test]
    fn test_translate_pong() {
        let msg = ServerMessage::Pong;
        let event = translate(msg);
        assert!(matches!(event, PlayerEvent::Pong));
    }

    #[test]
    fn test_translate_error() {
        let msg = ServerMessage::Error {
            code: "TEST_ERROR".to_string(),
            message: "Test error message".to_string(),
        };
        let event = translate(msg);
        match event {
            PlayerEvent::Error { code, message } => {
                assert_eq!(code, "TEST_ERROR");
                assert_eq!(message, "Test error message");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_translate_user_left() {
        let msg = ServerMessage::UserLeft {
            user_id: "user-123".to_string(),
        };
        let event = translate(msg);
        match event {
            PlayerEvent::UserLeft { user_id } => {
                assert_eq!(user_id, "user-123");
            }
            _ => panic!("Expected UserLeft event"),
        }
    }

    #[test]
    fn test_translate_game_time_advanced() {
        let previous_time = GameTime::new(1, 8, 0, true);
        let new_time = GameTime::new(1, 9, 30, false);

        let msg = ServerMessage::GameTimeAdvanced {
            data: TimeAdvanceData {
                previous_time: previous_time.clone(),
                new_time: new_time.clone(),
                seconds_advanced: 90,
                reason: "Travel".to_string(),
                period_changed: true,
                new_period: Some("morning".to_string()),
            },
        };

        let event = translate(msg);
        match event {
            PlayerEvent::GameTimeAdvanced {
                previous_time: prev,
                new_time: next,
                seconds_advanced,
                reason,
                period_changed,
                new_period,
            } => {
                assert_eq!(prev, previous_time);
                assert_eq!(next, new_time);
                assert_eq!(seconds_advanced, 90);
                assert_eq!(reason, "Travel");
                assert!(period_changed);
                assert_eq!(new_period.as_deref(), Some("morning"));
            }
            _ => panic!("Expected GameTimeAdvanced event"),
        }
    }

    #[test]
    fn test_translate_time_suggestion() {
        let current_time = GameTime::new(1, 10, 0, true);
        let resulting_time = GameTime::new(1, 10, 10, true);

        let msg = ServerMessage::TimeSuggestion {
            data: TimeSuggestionData {
                suggestion_id: "sug-1".to_string(),
                pc_id: "pc-1".to_string(),
                pc_name: "Alice".to_string(),
                action_type: "conversation".to_string(),
                action_description: "Talk to merchant".to_string(),
                suggested_seconds: 10,
                current_time: current_time.clone(),
                resulting_time: resulting_time.clone(),
                period_change: None,
            },
        };

        let event = translate(msg);
        match event {
            PlayerEvent::TimeSuggestion {
                suggestion_id,
                pc_id,
                pc_name,
                action_type,
                action_description,
                suggested_seconds,
                current_time: cur,
                resulting_time: res,
                period_change,
            } => {
                assert_eq!(suggestion_id, "sug-1");
                assert_eq!(pc_id, "pc-1");
                assert_eq!(pc_name, "Alice");
                assert_eq!(action_type, "conversation");
                assert_eq!(action_description, "Talk to merchant");
                assert_eq!(suggested_seconds, 10);
                assert_eq!(cur, current_time);
                assert_eq!(res, resulting_time);
                assert_eq!(period_change, None);
            }
            _ => panic!("Expected TimeSuggestion event"),
        }
    }

    #[test]
    fn test_translate_staging_pending_ready() {
        let pending = ServerMessage::StagingPending {
            region_id: "region-1".to_string(),
            region_name: "Town".to_string(),
            timeout_seconds: 30,
        };

        let event = translate(pending);
        match event {
            PlayerEvent::StagingPending {
                region_id,
                region_name,
                timeout_seconds,
            } => {
                assert_eq!(region_id, "region-1");
                assert_eq!(region_name, "Town");
                assert_eq!(timeout_seconds, 30);
            }
            _ => panic!("Expected StagingPending event"),
        }

        let ready = ServerMessage::StagingReady {
            region_id: "region-1".to_string(),
            npcs_present: vec![wrldbldr_shared::NpcPresentInfo {
                character_id: "npc-1".to_string(),
                name: "Bob".to_string(),
                sprite_asset: Some("/sprite.png".to_string()),
                portrait_asset: None,
                is_hidden_from_players: false,
                mood: Some("calm".to_string()),
            }],
            visual_state: None,
        };

        let event = translate(ready);
        match event {
            PlayerEvent::StagingReady {
                region_id,
                npcs_present,
                visual_state,
            } => {
                assert_eq!(region_id, "region-1");
                assert_eq!(visual_state, None);
                assert_eq!(npcs_present.len(), 1);
                assert_eq!(npcs_present[0].character_id, "npc-1");
                assert_eq!(npcs_present[0].name, "Bob");
                assert_eq!(npcs_present[0].sprite_asset.as_deref(), Some("/sprite.png"));
                assert_eq!(npcs_present[0].mood.as_deref(), Some("calm"));
            }
            _ => panic!("Expected StagingReady event"),
        }
    }
}
