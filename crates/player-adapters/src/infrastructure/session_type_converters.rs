//! Converters between ports layer DTOs and protocol types
//!
//! This module implements the adapters layer's responsibility of converting
//! between internal application types and external protocol types for wire transfer.
//!
//! Primary conversions are via `From` trait implementations in player-ports/session_types.rs.
//! Free functions here are for backwards compatibility and specialized conversions.

use wrldbldr_player_ports::session_types as app;
use wrldbldr_protocol::{
    AdHocOutcomes, ApprovalDecision, ChallengeOutcomeDecisionData, DiceInputType,
    DirectorialContext, NpcMotivationData, ParticipantRole, WorldRole,
};

// Re-export protocol types used by callers
pub use wrldbldr_protocol::ApprovedNpcInfo;

// =============================================================================
// ParticipantRole Conversions
// =============================================================================

pub fn participant_role_to_proto(role: app::ParticipantRole) -> ParticipantRole {
    role.into()
}

pub fn participant_role_from_proto(role: ParticipantRole) -> app::ParticipantRole {
    role.into()
}

/// Convert ParticipantRole to WorldRole
///
/// This conversion is used when joining a world to determine the user's
/// role-based permissions (Player, DM, Spectator).
pub fn participant_role_to_world_role(role: ParticipantRole) -> WorldRole {
    match role {
        ParticipantRole::DungeonMaster => WorldRole::Dm,
        ParticipantRole::Player => WorldRole::Player,
        ParticipantRole::Spectator | ParticipantRole::Unknown => WorldRole::Spectator,
    }
}

// =============================================================================
// DiceInput Conversions
// =============================================================================

pub fn dice_input_to_proto(input: app::DiceInput) -> DiceInputType {
    input.into()
}

pub fn dice_input_from_proto(input: DiceInputType) -> app::DiceInput {
    input.into()
}

// =============================================================================
// ApprovalDecision Conversions
// =============================================================================

pub fn approval_decision_to_proto(decision: app::ApprovalDecision) -> ApprovalDecision {
    decision.into()
}

pub fn approval_decision_from_proto(decision: ApprovalDecision) -> app::ApprovalDecision {
    decision.into()
}

// =============================================================================
// DirectorialContext Conversions
// =============================================================================

pub fn npc_motivation_to_proto(data: app::NpcMotivationData) -> NpcMotivationData {
    data.into()
}

pub fn npc_motivation_from_proto(data: NpcMotivationData) -> app::NpcMotivationData {
    data.into()
}

pub fn directorial_context_to_proto(ctx: app::DirectorialContext) -> DirectorialContext {
    ctx.into()
}

pub fn directorial_context_from_proto(ctx: DirectorialContext) -> app::DirectorialContext {
    ctx.into()
}

// =============================================================================
// ApprovedNpcInfo Conversions
// =============================================================================

pub fn approved_npc_info_to_proto(info: app::ApprovedNpcInfo) -> ApprovedNpcInfo {
    info.into()
}

pub fn approved_npc_info_from_proto(info: ApprovedNpcInfo) -> app::ApprovedNpcInfo {
    info.into()
}

// =============================================================================
// AdHocOutcomes Conversions
// =============================================================================

pub fn adhoc_outcomes_to_proto(outcomes: app::AdHocOutcomes) -> AdHocOutcomes {
    outcomes.into()
}

pub fn adhoc_outcomes_from_proto(outcomes: AdHocOutcomes) -> app::AdHocOutcomes {
    outcomes.into()
}

// =============================================================================
// ChallengeOutcomeDecision Conversions
// =============================================================================

pub fn challenge_outcome_decision_to_proto(
    decision: app::ChallengeOutcomeDecision,
) -> ChallengeOutcomeDecisionData {
    decision.into()
}

pub fn challenge_outcome_decision_from_proto(
    decision: ChallengeOutcomeDecisionData,
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
            participant_role_to_world_role(ParticipantRole::DungeonMaster),
            WorldRole::Dm
        );
        assert_eq!(
            participant_role_to_world_role(ParticipantRole::Player),
            WorldRole::Player
        );
        assert_eq!(
            participant_role_to_world_role(ParticipantRole::Spectator),
            WorldRole::Spectator
        );
    }
}
