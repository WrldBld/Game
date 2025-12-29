//! Converters between ports layer DTOs and protocol types
//!
//! This module implements the adapters layer's responsibility of converting
//! between internal application types and external protocol types for wire transfer.
//!
//! We use free functions instead of From traits due to orphan rules - we can't
//! implement From for types that aren't defined in this crate.

use wrldbldr_player_ports::session_types as app;
use wrldbldr_protocol as proto;

// =============================================================================
// ParticipantRole Conversions
// =============================================================================

pub fn participant_role_to_proto(role: app::ParticipantRole) -> proto::ParticipantRole {
    match role {
        app::ParticipantRole::Player => proto::ParticipantRole::Player,
        app::ParticipantRole::DungeonMaster => proto::ParticipantRole::DungeonMaster,
        app::ParticipantRole::Spectator => proto::ParticipantRole::Spectator,
    }
}

pub fn participant_role_from_proto(role: proto::ParticipantRole) -> app::ParticipantRole {
    match role {
        proto::ParticipantRole::Player => app::ParticipantRole::Player,
        proto::ParticipantRole::DungeonMaster => app::ParticipantRole::DungeonMaster,
        proto::ParticipantRole::Spectator => app::ParticipantRole::Spectator,
    }
}

/// Convert ParticipantRole to WorldRole
///
/// This conversion is used when joining a world to determine the user's
/// role-based permissions (Player, DM, Spectator).
pub fn participant_role_to_world_role(role: proto::ParticipantRole) -> proto::WorldRole {
    match role {
        proto::ParticipantRole::DungeonMaster => proto::WorldRole::Dm,
        proto::ParticipantRole::Player => proto::WorldRole::Player,
        proto::ParticipantRole::Spectator => proto::WorldRole::Spectator,
    }
}

// =============================================================================
// DiceInput Conversions
// =============================================================================

pub fn dice_input_to_proto(input: app::DiceInput) -> proto::DiceInputType {
    match input {
        app::DiceInput::Formula(formula) => proto::DiceInputType::Formula(formula),
        app::DiceInput::Manual(value) => proto::DiceInputType::Manual(value),
    }
}

pub fn dice_input_from_proto(input: proto::DiceInputType) -> app::DiceInput {
    match input {
        proto::DiceInputType::Formula(formula) => app::DiceInput::Formula(formula),
        proto::DiceInputType::Manual(value) => app::DiceInput::Manual(value),
    }
}

// =============================================================================
// ApprovalDecision Conversions
// =============================================================================

pub fn approval_decision_to_proto(decision: app::ApprovalDecision) -> proto::ApprovalDecision {
    match decision {
        app::ApprovalDecision::Accept => proto::ApprovalDecision::Accept,
        app::ApprovalDecision::AcceptWithRecipients { item_recipients } => {
            proto::ApprovalDecision::AcceptWithRecipients { item_recipients }
        }
        app::ApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => proto::ApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        },
        app::ApprovalDecision::Reject { feedback } => {
            proto::ApprovalDecision::Reject { feedback }
        }
        app::ApprovalDecision::TakeOver { dm_response } => {
            proto::ApprovalDecision::TakeOver { dm_response }
        }
    }
}

pub fn approval_decision_from_proto(decision: proto::ApprovalDecision) -> app::ApprovalDecision {
    match decision {
        proto::ApprovalDecision::Accept => app::ApprovalDecision::Accept,
        proto::ApprovalDecision::AcceptWithRecipients { item_recipients } => {
            app::ApprovalDecision::AcceptWithRecipients { item_recipients }
        }
        proto::ApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => app::ApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        },
        proto::ApprovalDecision::Reject { feedback } => {
            app::ApprovalDecision::Reject { feedback }
        }
        proto::ApprovalDecision::TakeOver { dm_response } => {
            app::ApprovalDecision::TakeOver { dm_response }
        }
    }
}

// =============================================================================
// DirectorialContext Conversions
// =============================================================================

pub fn npc_motivation_to_proto(data: app::NpcMotivationData) -> proto::NpcMotivationData {
    proto::NpcMotivationData {
        character_id: data.character_id,
        emotional_guidance: data.emotional_guidance,
        immediate_goal: data.immediate_goal,
        secret_agenda: data.secret_agenda,
    }
}

pub fn npc_motivation_from_proto(data: proto::NpcMotivationData) -> app::NpcMotivationData {
    app::NpcMotivationData {
        character_id: data.character_id,
        emotional_guidance: data.emotional_guidance,
        immediate_goal: data.immediate_goal,
        secret_agenda: data.secret_agenda,
    }
}

pub fn directorial_context_to_proto(ctx: app::DirectorialContext) -> proto::DirectorialContext {
    proto::DirectorialContext {
        scene_notes: ctx.scene_notes,
        tone: ctx.tone,
        npc_motivations: ctx.npc_motivations.into_iter().map(npc_motivation_to_proto).collect(),
        forbidden_topics: ctx.forbidden_topics,
    }
}

pub fn directorial_context_from_proto(ctx: proto::DirectorialContext) -> app::DirectorialContext {
    app::DirectorialContext {
        scene_notes: ctx.scene_notes,
        tone: ctx.tone,
        npc_motivations: ctx.npc_motivations.into_iter().map(npc_motivation_from_proto).collect(),
        forbidden_topics: ctx.forbidden_topics,
    }
}

// =============================================================================
// ApprovedNpcInfo Conversions
// =============================================================================

pub fn approved_npc_info_to_proto(info: app::ApprovedNpcInfo) -> proto::ApprovedNpcInfo {
    proto::ApprovedNpcInfo {
        character_id: info.character_id,
        is_present: info.is_present,
        reasoning: info.reasoning,
        is_hidden_from_players: info.is_hidden_from_players,
    }
}

pub fn approved_npc_info_from_proto(info: proto::ApprovedNpcInfo) -> app::ApprovedNpcInfo {
    app::ApprovedNpcInfo {
        character_id: info.character_id,
        is_present: info.is_present,
        reasoning: info.reasoning,
        is_hidden_from_players: info.is_hidden_from_players,
    }
}

// =============================================================================
// AdHocOutcomes Conversions
// =============================================================================

pub fn adhoc_outcomes_to_proto(outcomes: app::AdHocOutcomes) -> proto::AdHocOutcomes {
    proto::AdHocOutcomes {
        success: outcomes.success,
        failure: outcomes.failure,
        critical_success: outcomes.critical_success,
        critical_failure: outcomes.critical_failure,
    }
}

pub fn adhoc_outcomes_from_proto(outcomes: proto::AdHocOutcomes) -> app::AdHocOutcomes {
    app::AdHocOutcomes {
        success: outcomes.success,
        failure: outcomes.failure,
        critical_success: outcomes.critical_success,
        critical_failure: outcomes.critical_failure,
    }
}

// =============================================================================
// ChallengeOutcomeDecision Conversions
// =============================================================================

pub fn challenge_outcome_decision_to_proto(decision: app::ChallengeOutcomeDecision) -> proto::ChallengeOutcomeDecisionData {
    match decision {
        app::ChallengeOutcomeDecision::Accept => proto::ChallengeOutcomeDecisionData::Accept,
        app::ChallengeOutcomeDecision::Edit { modified_description } => {
            proto::ChallengeOutcomeDecisionData::Edit { modified_description }
        }
        app::ChallengeOutcomeDecision::Suggest { guidance } => {
            proto::ChallengeOutcomeDecisionData::Suggest { guidance }
        }
    }
}

pub fn challenge_outcome_decision_from_proto(decision: proto::ChallengeOutcomeDecisionData) -> app::ChallengeOutcomeDecision {
    match decision {
        proto::ChallengeOutcomeDecisionData::Accept => app::ChallengeOutcomeDecision::Accept,
        proto::ChallengeOutcomeDecisionData::Edit { modified_description } => {
            app::ChallengeOutcomeDecision::Edit { modified_description }
        }
        proto::ChallengeOutcomeDecisionData::Suggest { guidance } => {
            app::ChallengeOutcomeDecision::Suggest { guidance }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_participant_role_roundtrip() {
        let roles = [
            app::ParticipantRole::Player,
            app::ParticipantRole::DungeonMaster,
            app::ParticipantRole::Spectator,
        ];
        
        for role in roles {
            let proto_role = participant_role_to_proto(role);
            let back = participant_role_from_proto(proto_role);
            assert_eq!(role, back);
        }
    }

    #[test]
    fn test_dice_input_roundtrip() {
        let inputs = [
            app::DiceInput::Formula("1d20+5".to_string()),
            app::DiceInput::Manual(15),
        ];
        
        for input in inputs {
            let proto_input = dice_input_to_proto(input.clone());
            let back = dice_input_from_proto(proto_input);
            assert_eq!(input, back);
        }
    }

    #[test]
    fn test_approval_decision_roundtrip() {
        let decisions = [
            app::ApprovalDecision::Accept,
            app::ApprovalDecision::AcceptWithRecipients {
                item_recipients: HashMap::from([("tool_1".to_string(), vec!["pc_1".to_string()])]),
            },
            app::ApprovalDecision::Reject {
                feedback: "Too powerful".to_string(),
            },
        ];
        
        for decision in decisions {
            let proto_decision = approval_decision_to_proto(decision.clone());
            let back = approval_decision_from_proto(proto_decision);
            assert_eq!(decision, back);
        }
    }

    #[test]
    fn test_challenge_outcome_decision_roundtrip() {
        let decisions = [
            app::ChallengeOutcomeDecision::Accept,
            app::ChallengeOutcomeDecision::Edit {
                modified_description: "New description".to_string(),
            },
            app::ChallengeOutcomeDecision::Suggest {
                guidance: Some("Make it more dramatic".to_string()),
            },
        ];
        
        for decision in decisions {
            let proto_decision = challenge_outcome_decision_to_proto(decision.clone());
            let back = challenge_outcome_decision_from_proto(proto_decision);
            assert_eq!(decision, back);
        }
    }
}
