//! Converters between ports layer DTOs and protocol types
//!
//! This module implements the adapters layer's responsibility of converting
//! between internal application types and external protocol types for wire transfer.
//!
//! Primary conversions are via `From` trait implementations in player-ports/session_types.rs.
//! Free functions here are for backwards compatibility and specialized conversions.

use wrldbldr_player_ports::session_types as app;
use wrldbldr_protocol as proto;

// =============================================================================
// ParticipantRole Conversions
// =============================================================================

pub fn participant_role_to_proto(role: app::ParticipantRole) -> proto::ParticipantRole {
    role.into()
}

pub fn participant_role_from_proto(role: proto::ParticipantRole) -> app::ParticipantRole {
    role.into()
}

/// Convert ParticipantRole to WorldRole
///
/// This conversion is used when joining a world to determine the user's
/// role-based permissions (Player, DM, Spectator).
pub fn participant_role_to_world_role(role: proto::ParticipantRole) -> proto::WorldRole {
    match role {
        proto::ParticipantRole::DungeonMaster => proto::WorldRole::Dm,
        proto::ParticipantRole::Player => proto::WorldRole::Player,
        proto::ParticipantRole::Spectator | proto::ParticipantRole::Unknown => {
            proto::WorldRole::Spectator
        }
    }
}

// =============================================================================
// DiceInput Conversions
// =============================================================================

pub fn dice_input_to_proto(input: app::DiceInput) -> proto::DiceInputType {
    input.into()
}

pub fn dice_input_from_proto(input: proto::DiceInputType) -> app::DiceInput {
    input.into()
}

// =============================================================================
// ApprovalDecision Conversions
// =============================================================================

pub fn approval_decision_to_proto(decision: app::ApprovalDecision) -> proto::ApprovalDecision {
    decision.into()
}

pub fn approval_decision_from_proto(decision: proto::ApprovalDecision) -> app::ApprovalDecision {
    decision.into()
}

// =============================================================================
// DirectorialContext Conversions
// =============================================================================

pub fn npc_motivation_to_proto(data: app::NpcMotivationData) -> proto::NpcMotivationData {
    data.into()
}

pub fn npc_motivation_from_proto(data: proto::NpcMotivationData) -> app::NpcMotivationData {
    data.into()
}

pub fn directorial_context_to_proto(ctx: app::DirectorialContext) -> proto::DirectorialContext {
    ctx.into()
}

pub fn directorial_context_from_proto(ctx: proto::DirectorialContext) -> app::DirectorialContext {
    ctx.into()
}

// =============================================================================
// ApprovedNpcInfo Conversions
// =============================================================================

pub fn approved_npc_info_to_proto(info: app::ApprovedNpcInfo) -> proto::ApprovedNpcInfo {
    info.into()
}

pub fn approved_npc_info_from_proto(info: proto::ApprovedNpcInfo) -> app::ApprovedNpcInfo {
    info.into()
}

// =============================================================================
// AdHocOutcomes Conversions
// =============================================================================

pub fn adhoc_outcomes_to_proto(outcomes: app::AdHocOutcomes) -> proto::AdHocOutcomes {
    outcomes.into()
}

pub fn adhoc_outcomes_from_proto(outcomes: proto::AdHocOutcomes) -> app::AdHocOutcomes {
    outcomes.into()
}

// =============================================================================
// ChallengeOutcomeDecision Conversions
// =============================================================================

pub fn challenge_outcome_decision_to_proto(
    decision: app::ChallengeOutcomeDecision,
) -> proto::ChallengeOutcomeDecisionData {
    decision.into()
}

pub fn challenge_outcome_decision_from_proto(
    decision: proto::ChallengeOutcomeDecisionData,
) -> app::ChallengeOutcomeDecision {
    decision.into()
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

    #[test]
    fn test_participant_role_to_world_role() {
        assert_eq!(
            participant_role_to_world_role(proto::ParticipantRole::DungeonMaster),
            proto::WorldRole::Dm
        );
        assert_eq!(
            participant_role_to_world_role(proto::ParticipantRole::Player),
            proto::WorldRole::Player
        );
        assert_eq!(
            participant_role_to_world_role(proto::ParticipantRole::Spectator),
            proto::WorldRole::Spectator
        );
    }
}
