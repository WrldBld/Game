//! Approval type converters between domain types and protocol types
//!
//! This module provides conversion functions for bidirectional conversion
//! between domain-layer approval types and wire-format protocol types.
//!
//! Note: We use standalone functions instead of `From` implementations
//! because of Rust's orphan rules - we cannot implement traits for types
//! that are both defined in external crates.

use wrldbldr_domain::value_objects::{
    ChallengeSuggestion, ChallengeSuggestionOutcomes, DmApprovalDecision, NarrativeEventSuggestion,
    ProposedTool,
};
use wrldbldr_protocol as proto;

// =============================================================================
// ProposedTool conversions
// =============================================================================

/// Convert domain ProposedTool to protocol ProposedToolInfo
pub fn domain_tool_to_proto(domain: &ProposedTool) -> proto::ProposedToolInfo {
    proto::ProposedToolInfo {
        id: domain.id.clone(),
        name: domain.name.clone(),
        description: domain.description.clone(),
        arguments: domain.arguments.clone(),
    }
}

/// Convert protocol ProposedToolInfo to domain ProposedTool
pub fn proto_tool_to_domain(proto: proto::ProposedToolInfo) -> ProposedTool {
    ProposedTool {
        id: proto.id,
        name: proto.name,
        description: proto.description,
        arguments: proto.arguments,
    }
}

/// Convert a slice of domain ProposedTool to Vec of protocol ProposedToolInfo
pub fn domain_tools_to_proto(tools: &[ProposedTool]) -> Vec<proto::ProposedToolInfo> {
    tools.iter().map(domain_tool_to_proto).collect()
}

/// Convert a Vec of protocol ProposedToolInfo to Vec of domain ProposedTool
pub fn proto_tools_to_domain(tools: Vec<proto::ProposedToolInfo>) -> Vec<ProposedTool> {
    tools.into_iter().map(proto_tool_to_domain).collect()
}

// =============================================================================
// ChallengeSuggestionOutcomes conversions
// =============================================================================

/// Convert domain ChallengeSuggestionOutcomes to protocol ChallengeSuggestionOutcomes
pub fn domain_outcomes_to_proto(
    domain: &ChallengeSuggestionOutcomes,
) -> proto::ChallengeSuggestionOutcomes {
    proto::ChallengeSuggestionOutcomes {
        success: domain.success.clone(),
        failure: domain.failure.clone(),
        critical_success: domain.critical_success.clone(),
        critical_failure: domain.critical_failure.clone(),
    }
}

/// Convert protocol ChallengeSuggestionOutcomes to domain ChallengeSuggestionOutcomes
pub fn proto_outcomes_to_domain(
    proto: proto::ChallengeSuggestionOutcomes,
) -> ChallengeSuggestionOutcomes {
    ChallengeSuggestionOutcomes {
        success: proto.success,
        failure: proto.failure,
        critical_success: proto.critical_success,
        critical_failure: proto.critical_failure,
    }
}

// =============================================================================
// ChallengeSuggestion conversions
// =============================================================================

/// Convert domain ChallengeSuggestion to protocol ChallengeSuggestionInfo
pub fn domain_challenge_to_proto(domain: &ChallengeSuggestion) -> proto::ChallengeSuggestionInfo {
    proto::ChallengeSuggestionInfo {
        challenge_id: domain.challenge_id.clone(),
        challenge_name: domain.challenge_name.clone(),
        skill_name: domain.skill_name.clone(),
        difficulty_display: domain.difficulty_display.clone(),
        confidence: domain.confidence.clone(),
        reasoning: domain.reasoning.clone(),
        target_pc_id: domain.target_pc_id.map(|id| id.to_string()),
        outcomes: domain.outcomes.as_ref().map(domain_outcomes_to_proto),
    }
}

/// Convert protocol ChallengeSuggestionInfo to domain ChallengeSuggestion
pub fn proto_challenge_to_domain(proto: proto::ChallengeSuggestionInfo) -> ChallengeSuggestion {
    ChallengeSuggestion {
        challenge_id: proto.challenge_id,
        challenge_name: proto.challenge_name,
        skill_name: proto.skill_name,
        difficulty_display: proto.difficulty_display,
        confidence: proto.confidence,
        reasoning: proto.reasoning,
        target_pc_id: proto
            .target_pc_id
            .and_then(|s| uuid::Uuid::parse_str(&s).ok())
            .map(wrldbldr_domain::PlayerCharacterId::from_uuid),
        outcomes: proto.outcomes.map(proto_outcomes_to_domain),
    }
}

/// Convert Option<domain ChallengeSuggestion> to Option<proto::ChallengeSuggestionInfo>
pub fn domain_challenge_suggestion_to_proto(
    suggestion: Option<&ChallengeSuggestion>,
) -> Option<proto::ChallengeSuggestionInfo> {
    suggestion.map(domain_challenge_to_proto)
}

/// Convert Option<proto::ChallengeSuggestionInfo> to Option<domain ChallengeSuggestion>
pub fn proto_challenge_suggestion_to_domain(
    suggestion: Option<proto::ChallengeSuggestionInfo>,
) -> Option<ChallengeSuggestion> {
    suggestion.map(proto_challenge_to_domain)
}

// =============================================================================
// NarrativeEventSuggestion conversions
// =============================================================================

/// Convert domain NarrativeEventSuggestion to protocol NarrativeEventSuggestionInfo
pub fn domain_narrative_to_proto(
    domain: &NarrativeEventSuggestion,
) -> proto::NarrativeEventSuggestionInfo {
    proto::NarrativeEventSuggestionInfo {
        event_id: domain.event_id.clone(),
        event_name: domain.event_name.clone(),
        description: domain.description.clone(),
        scene_direction: domain.scene_direction.clone(),
        confidence: domain.confidence.clone(),
        reasoning: domain.reasoning.clone(),
        matched_triggers: domain.matched_triggers.clone(),
        suggested_outcome: domain.suggested_outcome.clone(),
    }
}

/// Convert protocol NarrativeEventSuggestionInfo to domain NarrativeEventSuggestion
pub fn proto_narrative_to_domain(
    proto: proto::NarrativeEventSuggestionInfo,
) -> NarrativeEventSuggestion {
    NarrativeEventSuggestion {
        event_id: proto.event_id,
        event_name: proto.event_name,
        description: proto.description,
        scene_direction: proto.scene_direction,
        confidence: proto.confidence,
        reasoning: proto.reasoning,
        matched_triggers: proto.matched_triggers,
        suggested_outcome: proto.suggested_outcome,
    }
}

/// Convert Option<domain NarrativeEventSuggestion> to Option<proto::NarrativeEventSuggestionInfo>
pub fn domain_narrative_suggestion_to_proto(
    suggestion: Option<&NarrativeEventSuggestion>,
) -> Option<proto::NarrativeEventSuggestionInfo> {
    suggestion.map(domain_narrative_to_proto)
}

/// Convert Option<proto::NarrativeEventSuggestionInfo> to Option<domain NarrativeEventSuggestion>
pub fn proto_narrative_suggestion_to_domain(
    suggestion: Option<proto::NarrativeEventSuggestionInfo>,
) -> Option<NarrativeEventSuggestion> {
    suggestion.map(proto_narrative_to_domain)
}

// =============================================================================
// DmApprovalDecision <-> ApprovalDecision conversions
// =============================================================================

/// Convert domain DmApprovalDecision to protocol ApprovalDecision
pub fn domain_decision_to_proto(domain: &DmApprovalDecision) -> proto::ApprovalDecision {
    match domain {
        DmApprovalDecision::Accept => proto::ApprovalDecision::Accept,
        DmApprovalDecision::AcceptWithRecipients { item_recipients } => {
            proto::ApprovalDecision::AcceptWithRecipients {
                item_recipients: item_recipients.clone(),
            }
        }
        DmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => proto::ApprovalDecision::AcceptWithModification {
            modified_dialogue: modified_dialogue.clone(),
            approved_tools: approved_tools.clone(),
            rejected_tools: rejected_tools.clone(),
            item_recipients: item_recipients.clone(),
        },
        DmApprovalDecision::Reject { feedback } => proto::ApprovalDecision::Reject {
            feedback: feedback.clone(),
        },
        DmApprovalDecision::TakeOver { dm_response } => proto::ApprovalDecision::TakeOver {
            dm_response: dm_response.clone(),
        },
    }
}

/// Convert protocol ApprovalDecision to domain DmApprovalDecision
pub fn proto_decision_to_domain(proto: proto::ApprovalDecision) -> DmApprovalDecision {
    match proto {
        proto::ApprovalDecision::Accept => DmApprovalDecision::Accept,
        proto::ApprovalDecision::AcceptWithRecipients { item_recipients } => {
            DmApprovalDecision::AcceptWithRecipients { item_recipients }
        }
        proto::ApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => DmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        },
        proto::ApprovalDecision::Reject { feedback } => DmApprovalDecision::Reject { feedback },
        proto::ApprovalDecision::TakeOver { dm_response } => {
            DmApprovalDecision::TakeOver { dm_response }
        }
        proto::ApprovalDecision::Unknown => {
            // Default unknown to Reject with explanation
            DmApprovalDecision::Reject {
                feedback: "Unknown approval decision received".to_string(),
            }
        }
    }
}
