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
use wrldbldr_protocol::{
    ApprovalDecision, ChallengeSuggestionInfo, ChallengeSuggestionOutcomes as ProtoOutcomes,
    NarrativeEventSuggestionInfo, ProposedToolInfo,
};

// =============================================================================
// ProposedTool conversions
// =============================================================================

/// Convert domain ProposedTool to protocol ProposedToolInfo
pub fn domain_tool_to_proto(domain: &ProposedTool) -> ProposedToolInfo {
    ProposedToolInfo {
        id: domain.id.clone(),
        name: domain.name.clone(),
        description: domain.description.clone(),
        arguments: domain.arguments.clone(),
    }
}

/// Convert protocol ProposedToolInfo to domain ProposedTool
pub fn proto_tool_to_domain(proto_tool: ProposedToolInfo) -> ProposedTool {
    ProposedTool {
        id: proto_tool.id,
        name: proto_tool.name,
        description: proto_tool.description,
        arguments: proto_tool.arguments,
    }
}

/// Convert a slice of domain ProposedTool to Vec of protocol ProposedToolInfo
pub fn domain_tools_to_proto(tools: &[ProposedTool]) -> Vec<ProposedToolInfo> {
    tools.iter().map(domain_tool_to_proto).collect()
}

/// Convert a Vec of protocol ProposedToolInfo to Vec of domain ProposedTool
pub fn proto_tools_to_domain(tools: Vec<ProposedToolInfo>) -> Vec<ProposedTool> {
    tools.into_iter().map(proto_tool_to_domain).collect()
}

// =============================================================================
// ChallengeSuggestionOutcomes conversions
// =============================================================================

/// Convert domain ChallengeSuggestionOutcomes to protocol ChallengeSuggestionOutcomes
pub fn domain_outcomes_to_proto(domain: &ChallengeSuggestionOutcomes) -> ProtoOutcomes {
    ProtoOutcomes {
        success: domain.success.clone(),
        failure: domain.failure.clone(),
        critical_success: domain.critical_success.clone(),
        critical_failure: domain.critical_failure.clone(),
    }
}

/// Convert protocol ChallengeSuggestionOutcomes to domain ChallengeSuggestionOutcomes
pub fn proto_outcomes_to_domain(proto_outcomes: ProtoOutcomes) -> ChallengeSuggestionOutcomes {
    ChallengeSuggestionOutcomes {
        success: proto_outcomes.success,
        failure: proto_outcomes.failure,
        critical_success: proto_outcomes.critical_success,
        critical_failure: proto_outcomes.critical_failure,
    }
}

// =============================================================================
// ChallengeSuggestion conversions
// =============================================================================

/// Convert domain ChallengeSuggestion to protocol ChallengeSuggestionInfo
pub fn domain_challenge_to_proto(domain: &ChallengeSuggestion) -> ChallengeSuggestionInfo {
    ChallengeSuggestionInfo {
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
pub fn proto_challenge_to_domain(proto_challenge: ChallengeSuggestionInfo) -> ChallengeSuggestion {
    ChallengeSuggestion {
        challenge_id: proto_challenge.challenge_id,
        challenge_name: proto_challenge.challenge_name,
        skill_name: proto_challenge.skill_name,
        difficulty_display: proto_challenge.difficulty_display,
        confidence: proto_challenge.confidence,
        reasoning: proto_challenge.reasoning,
        target_pc_id: proto_challenge
            .target_pc_id
            .and_then(|s| uuid::Uuid::parse_str(&s).ok())
            .map(wrldbldr_domain::PlayerCharacterId::from_uuid),
        outcomes: proto_challenge.outcomes.map(proto_outcomes_to_domain),
    }
}

/// Convert Option<domain ChallengeSuggestion> to Option<ChallengeSuggestionInfo>
pub fn domain_challenge_suggestion_to_proto(
    suggestion: Option<&ChallengeSuggestion>,
) -> Option<ChallengeSuggestionInfo> {
    suggestion.map(domain_challenge_to_proto)
}

/// Convert Option<ChallengeSuggestionInfo> to Option<domain ChallengeSuggestion>
pub fn proto_challenge_suggestion_to_domain(
    suggestion: Option<ChallengeSuggestionInfo>,
) -> Option<ChallengeSuggestion> {
    suggestion.map(proto_challenge_to_domain)
}

// =============================================================================
// NarrativeEventSuggestion conversions
// =============================================================================

/// Convert domain NarrativeEventSuggestion to protocol NarrativeEventSuggestionInfo
pub fn domain_narrative_to_proto(
    domain: &NarrativeEventSuggestion,
) -> NarrativeEventSuggestionInfo {
    NarrativeEventSuggestionInfo {
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
    proto_narrative: NarrativeEventSuggestionInfo,
) -> NarrativeEventSuggestion {
    NarrativeEventSuggestion {
        event_id: proto_narrative.event_id,
        event_name: proto_narrative.event_name,
        description: proto_narrative.description,
        scene_direction: proto_narrative.scene_direction,
        confidence: proto_narrative.confidence,
        reasoning: proto_narrative.reasoning,
        matched_triggers: proto_narrative.matched_triggers,
        suggested_outcome: proto_narrative.suggested_outcome,
    }
}

/// Convert Option<domain NarrativeEventSuggestion> to Option<NarrativeEventSuggestionInfo>
pub fn domain_narrative_suggestion_to_proto(
    suggestion: Option<&NarrativeEventSuggestion>,
) -> Option<NarrativeEventSuggestionInfo> {
    suggestion.map(domain_narrative_to_proto)
}

/// Convert Option<NarrativeEventSuggestionInfo> to Option<domain NarrativeEventSuggestion>
pub fn proto_narrative_suggestion_to_domain(
    suggestion: Option<NarrativeEventSuggestionInfo>,
) -> Option<NarrativeEventSuggestion> {
    suggestion.map(proto_narrative_to_domain)
}

// =============================================================================
// DmApprovalDecision <-> ApprovalDecision conversions
// =============================================================================

/// Convert domain DmApprovalDecision to protocol ApprovalDecision
pub fn domain_decision_to_proto(domain: &DmApprovalDecision) -> ApprovalDecision {
    match domain {
        DmApprovalDecision::Accept => ApprovalDecision::Accept,
        DmApprovalDecision::AcceptWithRecipients { item_recipients } => {
            ApprovalDecision::AcceptWithRecipients {
                item_recipients: item_recipients.clone(),
            }
        }
        DmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => ApprovalDecision::AcceptWithModification {
            modified_dialogue: modified_dialogue.clone(),
            approved_tools: approved_tools.clone(),
            rejected_tools: rejected_tools.clone(),
            item_recipients: item_recipients.clone(),
        },
        DmApprovalDecision::Reject { feedback } => ApprovalDecision::Reject {
            feedback: feedback.clone(),
        },
        DmApprovalDecision::TakeOver { dm_response } => ApprovalDecision::TakeOver {
            dm_response: dm_response.clone(),
        },
    }
}

/// Convert protocol ApprovalDecision to domain DmApprovalDecision
pub fn proto_decision_to_domain(proto_decision: ApprovalDecision) -> DmApprovalDecision {
    match proto_decision {
        ApprovalDecision::Accept => DmApprovalDecision::Accept,
        ApprovalDecision::AcceptWithRecipients { item_recipients } => {
            DmApprovalDecision::AcceptWithRecipients { item_recipients }
        }
        ApprovalDecision::AcceptWithModification {
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
        ApprovalDecision::Reject { feedback } => DmApprovalDecision::Reject { feedback },
        ApprovalDecision::TakeOver { dm_response } => DmApprovalDecision::TakeOver { dm_response },
        ApprovalDecision::Unknown => {
            // Default unknown to Reject with explanation
            DmApprovalDecision::Reject {
                feedback: "Unknown approval decision received".to_string(),
            }
        }
    }
}
