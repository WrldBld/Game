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

use wrldbldr_player_ports::outbound::player_events::{
    ActantialViewData, CharacterData, CharacterPosition, ConnectedUser, DialogueChoice,
    EntityChangedData, GameTime, GoalData, InteractionData, JoinError, NavigationData,
    NavigationExit, NavigationTarget, NpcDispositionData, NpcPresenceData, NpcPresentInfo,
    OutcomeBranchData, OutcomeDetailData, PlayerEvent, PreviousStagingInfo, RegionData,
    RegionItemData, ResponseResult, SceneData, SplitPartyLocation, StagedNpcInfo, WaitingPcInfo,
    WantData, WantTargetData, WorldRole,
};
// Note: ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, NarrativeEventSuggestionInfo,
// and ProposedToolInfo are now used directly from protocol (same types as in player-ports)
use wrldbldr_protocol::ServerMessage;

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
        ServerMessage::SceneUpdate {
            scene,
            characters,
            interactions,
        } => PlayerEvent::SceneUpdate {
            scene: translate_scene_data(scene),
            characters: characters
                .into_iter()
                .map(translate_character_data)
                .collect(),
            interactions: interactions
                .into_iter()
                .map(translate_interaction_data)
                .collect(),
        },

        ServerMessage::SceneChanged {
            pc_id,
            region,
            npcs_present,
            navigation,
            region_items,
        } => PlayerEvent::SceneChanged {
            pc_id,
            region: translate_region_data(region),
            npcs_present: npcs_present
                .into_iter()
                .map(translate_npc_presence_data)
                .collect(),
            navigation: translate_navigation_data(navigation),
            region_items: region_items
                .into_iter()
                .map(translate_region_item_data)
                .collect(),
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
            locations: locations
                .into_iter()
                .map(translate_split_party_location)
                .collect(),
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
        ServerMessage::DialogueResponse {
            speaker_id,
            speaker_name,
            text,
            choices,
        } => PlayerEvent::DialogueResponse {
            speaker_id,
            speaker_name,
            text,
            choices: choices.into_iter().map(translate_dialogue_choice).collect(),
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
            new_outcome: translate_outcome_detail_data(new_outcome),
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
            branches: branches
                .into_iter()
                .map(translate_outcome_branch_data)
                .collect(),
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
        } => PlayerEvent::StagingApprovalRequired {
            request_id,
            region_id,
            region_name,
            location_id,
            location_name,
            game_time: translate_game_time(game_time),
            previous_staging: previous_staging.map(translate_previous_staging_info),
            rule_based_npcs: rule_based_npcs
                .into_iter()
                .map(translate_staged_npc_info)
                .collect(),
            llm_based_npcs: llm_based_npcs
                .into_iter()
                .map(translate_staged_npc_info)
                .collect(),
            default_ttl_hours,
            waiting_pcs: waiting_pcs
                .into_iter()
                .map(translate_waiting_pc_info)
                .collect(),
        },

        ServerMessage::StagingPending {
            region_id,
            region_name,
        } => PlayerEvent::StagingPending {
            region_id,
            region_name,
        },

        ServerMessage::StagingReady {
            region_id,
            npcs_present,
        } => PlayerEvent::StagingReady {
            region_id,
            npcs_present: npcs_present
                .into_iter()
                .map(translate_npc_present_info)
                .collect(),
        },

        ServerMessage::StagingRegenerated {
            request_id,
            llm_based_npcs,
        } => PlayerEvent::StagingRegenerated {
            request_id,
            llm_based_npcs: llm_based_npcs
                .into_iter()
                .map(translate_staged_npc_info)
                .collect(),
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

        ServerMessage::NpcDispositionsResponse {
            pc_id,
            dispositions,
        } => PlayerEvent::NpcDispositionsResponse {
            pc_id,
            dispositions: dispositions
                .into_iter()
                .map(translate_npc_disposition_data)
                .collect(),
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
            goals: goals.into_iter().map(translate_goal_data).collect(),
        },

        ServerMessage::GoalCreated { world_id, goal } => PlayerEvent::GoalCreated {
            world_id,
            goal: translate_goal_data(goal),
        },

        ServerMessage::GoalUpdated { goal } => PlayerEvent::GoalUpdated {
            goal: translate_goal_data(goal),
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
            game_time: translate_game_time(game_time),
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

fn translate_connected_user(u: wrldbldr_protocol::responses::ConnectedUser) -> ConnectedUser {
    ConnectedUser {
        user_id: u.user_id,
        username: u.username,
        role: format!("{:?}", u.role),
        pc_id: u.pc_id,
        connection_count: u.connection_count,
    }
}

fn translate_scene_data(s: wrldbldr_protocol::SceneData) -> SceneData {
    SceneData {
        id: s.id,
        name: s.name,
        location_id: s.location_id,
        location_name: s.location_name,
        backdrop_asset: s.backdrop_asset,
        time_context: s.time_context,
        directorial_notes: s.directorial_notes,
    }
}

fn translate_character_data(c: wrldbldr_protocol::CharacterData) -> CharacterData {
    CharacterData {
        id: c.id,
        name: c.name,
        sprite_asset: c.sprite_asset,
        portrait_asset: c.portrait_asset,
        position: translate_character_position(c.position),
        is_speaking: c.is_speaking,
        emotion: c.emotion,
    }
}

fn translate_character_position(p: wrldbldr_protocol::CharacterPosition) -> CharacterPosition {
    match p {
        wrldbldr_protocol::CharacterPosition::Left => CharacterPosition::Left,
        wrldbldr_protocol::CharacterPosition::Center => CharacterPosition::Center,
        wrldbldr_protocol::CharacterPosition::Right => CharacterPosition::Right,
        wrldbldr_protocol::CharacterPosition::OffScreen
        | wrldbldr_protocol::CharacterPosition::Unknown => CharacterPosition::OffScreen,
    }
}

fn translate_interaction_data(i: wrldbldr_protocol::InteractionData) -> InteractionData {
    InteractionData {
        id: i.id,
        name: i.name,
        interaction_type: i.interaction_type,
        target_name: i.target_name,
        is_available: i.is_available,
    }
}

fn translate_dialogue_choice(c: wrldbldr_protocol::DialogueChoice) -> DialogueChoice {
    DialogueChoice {
        id: c.id,
        text: c.text,
        is_custom_input: c.is_custom_input,
    }
}

fn translate_region_data(r: wrldbldr_protocol::RegionData) -> RegionData {
    RegionData {
        id: r.id,
        name: r.name,
        location_id: r.location_id,
        location_name: r.location_name,
        backdrop_asset: r.backdrop_asset,
        atmosphere: r.atmosphere,
        map_asset: r.map_asset,
    }
}

fn translate_npc_presence_data(n: wrldbldr_protocol::NpcPresenceData) -> NpcPresenceData {
    NpcPresenceData {
        character_id: n.character_id,
        name: n.name,
        sprite_asset: n.sprite_asset,
        portrait_asset: n.portrait_asset,
    }
}

fn translate_navigation_data(n: wrldbldr_protocol::NavigationData) -> NavigationData {
    NavigationData {
        connected_regions: n
            .connected_regions
            .into_iter()
            .map(translate_navigation_target)
            .collect(),
        exits: n.exits.into_iter().map(translate_navigation_exit).collect(),
    }
}

fn translate_navigation_target(t: wrldbldr_protocol::NavigationTarget) -> NavigationTarget {
    NavigationTarget {
        region_id: t.region_id,
        name: t.name,
        is_locked: t.is_locked,
        lock_description: t.lock_description,
    }
}

fn translate_navigation_exit(e: wrldbldr_protocol::NavigationExit) -> NavigationExit {
    NavigationExit {
        location_id: e.location_id,
        location_name: e.location_name,
        arrival_region_id: e.arrival_region_id,
        description: e.description,
    }
}

fn translate_region_item_data(i: wrldbldr_protocol::RegionItemData) -> RegionItemData {
    RegionItemData {
        id: i.id,
        name: i.name,
        description: i.description,
        item_type: i.item_type,
    }
}

fn translate_split_party_location(l: wrldbldr_protocol::SplitPartyLocation) -> SplitPartyLocation {
    SplitPartyLocation {
        location_id: l.location_id,
        location_name: l.location_name,
        pc_count: l.pc_count,
        pc_names: l.pc_names,
    }
}

// NOTE: translate_proposed_tool_info, translate_challenge_suggestion_info, and
// translate_narrative_event_suggestion_info have been removed because player-ports
// now re-exports protocol types directly - no translation needed.

fn translate_outcome_detail_data(o: wrldbldr_protocol::OutcomeDetailData) -> OutcomeDetailData {
    OutcomeDetailData {
        flavor_text: o.flavor_text,
        scene_direction: o.scene_direction,
        proposed_tools: o.proposed_tools, // Direct assignment - same type now
    }
}

fn translate_outcome_branch_data(b: wrldbldr_protocol::OutcomeBranchData) -> OutcomeBranchData {
    OutcomeBranchData {
        id: b.id,
        title: b.title,
        description: b.description,
        effects: b.effects,
    }
}

fn translate_game_time(t: wrldbldr_protocol::types::GameTime) -> GameTime {
    GameTime {
        day: t.day,
        hour: t.hour,
        minute: t.minute,
        is_paused: t.is_paused,
    }
}

fn translate_previous_staging_info(
    p: wrldbldr_protocol::PreviousStagingInfo,
) -> PreviousStagingInfo {
    PreviousStagingInfo {
        staging_id: p.staging_id,
        approved_at: p.approved_at,
        npcs: p.npcs.into_iter().map(translate_staged_npc_info).collect(),
    }
}

fn translate_staged_npc_info(n: wrldbldr_protocol::StagedNpcInfo) -> StagedNpcInfo {
    StagedNpcInfo {
        character_id: n.character_id,
        name: n.name,
        sprite_asset: n.sprite_asset,
        portrait_asset: n.portrait_asset,
        is_present: n.is_present,
        reasoning: n.reasoning,
        is_hidden_from_players: n.is_hidden_from_players,
    }
}

fn translate_waiting_pc_info(w: wrldbldr_protocol::WaitingPcInfo) -> WaitingPcInfo {
    WaitingPcInfo {
        pc_id: w.pc_id,
        pc_name: w.pc_name,
        player_id: w.player_id,
    }
}

fn translate_npc_present_info(n: wrldbldr_protocol::NpcPresentInfo) -> NpcPresentInfo {
    NpcPresentInfo {
        character_id: n.character_id,
        name: n.name,
        sprite_asset: n.sprite_asset,
        portrait_asset: n.portrait_asset,
        is_hidden_from_players: n.is_hidden_from_players,
    }
}

fn translate_npc_disposition_data(d: wrldbldr_protocol::NpcDispositionData) -> NpcDispositionData {
    NpcDispositionData {
        npc_id: d.npc_id,
        npc_name: d.npc_name,
        disposition: d.disposition,
        relationship: d.relationship,
        sentiment: d.sentiment,
        last_reason: d.last_reason,
    }
}

fn translate_want_data(w: wrldbldr_protocol::WantData) -> WantData {
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

fn translate_want_target_data(t: wrldbldr_protocol::WantTargetData) -> WantTargetData {
    WantTargetData {
        id: t.id,
        name: t.name,
        target_type: format!("{:?}", t.target_type),
        description: t.description,
    }
}

fn translate_actantial_view_data(v: wrldbldr_protocol::ActantialViewData) -> ActantialViewData {
    ActantialViewData {
        want_id: v.want_id,
        target_id: v.target_id,
        target_name: v.target_name,
        target_type: format!("{:?}", v.target_type),
        role: format!("{:?}", v.role),
        reason: v.reason,
    }
}

fn translate_goal_data(g: wrldbldr_protocol::GoalData) -> GoalData {
    GoalData {
        id: g.id,
        name: g.name,
        description: g.description,
        usage_count: g.usage_count,
    }
}

fn translate_response_result(r: wrldbldr_protocol::responses::ResponseResult) -> ResponseResult {
    match r {
        wrldbldr_protocol::responses::ResponseResult::Success { data } => ResponseResult {
            success: true,
            data,
            error_code: None,
            error_message: None,
            error_details: None,
        },
        wrldbldr_protocol::responses::ResponseResult::Error {
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
        wrldbldr_protocol::responses::ResponseResult::Unknown => ResponseResult {
            success: false,
            data: None,
            error_code: Some("UNKNOWN".to_string()),
            error_message: Some("Unknown response type".to_string()),
            error_details: None,
        },
    }
}

fn translate_entity_changed_data(
    e: wrldbldr_protocol::responses::EntityChangedData,
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
}
