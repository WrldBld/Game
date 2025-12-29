//! Approval type converters between application DTOs and protocol types
//!
//! This module provides conversion functions for bidirectional conversion
//! between app-layer approval DTOs and wire-format protocol types.
//!
//! Note: We use standalone functions instead of `From` implementations
//! because of Rust's orphan rules - we cannot implement traits for types
//! that are both defined in external crates.

use wrldbldr_engine_app::application::dto as app;
use wrldbldr_protocol as proto;

// =============================================================================
// ProposedToolInfo conversions
// =============================================================================

/// Convert app ProposedToolInfo to protocol ProposedToolInfo
pub fn app_tool_to_proto(app: app::ProposedToolInfo) -> proto::ProposedToolInfo {
    proto::ProposedToolInfo {
        id: app.id,
        name: app.name,
        description: app.description,
        arguments: app.arguments,
    }
}

/// Convert protocol ProposedToolInfo to app ProposedToolInfo
pub fn proto_tool_to_app(proto: proto::ProposedToolInfo) -> app::ProposedToolInfo {
    app::ProposedToolInfo {
        id: proto.id,
        name: proto.name,
        description: proto.description,
        arguments: proto.arguments,
    }
}

// =============================================================================
// ChallengeSuggestionOutcomes conversions
// =============================================================================

/// Convert app ChallengeSuggestionOutcomes to protocol ChallengeSuggestionOutcomes
pub fn app_outcomes_to_proto(
    app: app::ChallengeSuggestionOutcomes,
) -> proto::ChallengeSuggestionOutcomes {
    proto::ChallengeSuggestionOutcomes {
        success: app.success,
        failure: app.failure,
        critical_success: app.critical_success,
        critical_failure: app.critical_failure,
    }
}

/// Convert protocol ChallengeSuggestionOutcomes to app ChallengeSuggestionOutcomes
pub fn proto_outcomes_to_app(
    proto: proto::ChallengeSuggestionOutcomes,
) -> app::ChallengeSuggestionOutcomes {
    app::ChallengeSuggestionOutcomes {
        success: proto.success,
        failure: proto.failure,
        critical_success: proto.critical_success,
        critical_failure: proto.critical_failure,
    }
}

// =============================================================================
// ChallengeSuggestionInfo conversions
// =============================================================================

/// Convert app ChallengeSuggestionInfo to protocol ChallengeSuggestionInfo
pub fn app_challenge_to_proto(app: app::ChallengeSuggestionInfo) -> proto::ChallengeSuggestionInfo {
    proto::ChallengeSuggestionInfo {
        challenge_id: app.challenge_id,
        challenge_name: app.challenge_name,
        skill_name: app.skill_name,
        difficulty_display: app.difficulty_display,
        confidence: app.confidence,
        reasoning: app.reasoning,
        target_pc_id: app.target_pc_id,
        outcomes: app.outcomes.map(app_outcomes_to_proto),
    }
}

/// Convert protocol ChallengeSuggestionInfo to app ChallengeSuggestionInfo
pub fn proto_challenge_to_app(
    proto: proto::ChallengeSuggestionInfo,
) -> app::ChallengeSuggestionInfo {
    app::ChallengeSuggestionInfo {
        challenge_id: proto.challenge_id,
        challenge_name: proto.challenge_name,
        skill_name: proto.skill_name,
        difficulty_display: proto.difficulty_display,
        confidence: proto.confidence,
        reasoning: proto.reasoning,
        target_pc_id: proto.target_pc_id,
        outcomes: proto.outcomes.map(proto_outcomes_to_app),
    }
}

// =============================================================================
// NarrativeEventSuggestionInfo conversions
// =============================================================================

/// Convert app NarrativeEventSuggestionInfo to protocol NarrativeEventSuggestionInfo
pub fn app_narrative_to_proto(
    app: app::NarrativeEventSuggestionInfo,
) -> proto::NarrativeEventSuggestionInfo {
    proto::NarrativeEventSuggestionInfo {
        event_id: app.event_id,
        event_name: app.event_name,
        description: app.description,
        scene_direction: app.scene_direction,
        confidence: app.confidence,
        reasoning: app.reasoning,
        matched_triggers: app.matched_triggers,
        suggested_outcome: app.suggested_outcome,
    }
}

/// Convert protocol NarrativeEventSuggestionInfo to app NarrativeEventSuggestionInfo
pub fn proto_narrative_to_app(
    proto: proto::NarrativeEventSuggestionInfo,
) -> app::NarrativeEventSuggestionInfo {
    app::NarrativeEventSuggestionInfo {
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

// =============================================================================
// DmApprovalDecision <-> ApprovalDecision conversions
// =============================================================================

/// Convert app DmApprovalDecision to protocol ApprovalDecision
pub fn app_decision_to_proto(app: app::DmApprovalDecision) -> proto::ApprovalDecision {
    match app {
        app::DmApprovalDecision::Accept => proto::ApprovalDecision::Accept,
        app::DmApprovalDecision::AcceptWithRecipients { item_recipients } => {
            proto::ApprovalDecision::AcceptWithRecipients { item_recipients }
        }
        app::DmApprovalDecision::AcceptWithModification {
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
        app::DmApprovalDecision::Reject { feedback } => {
            proto::ApprovalDecision::Reject { feedback }
        }
        app::DmApprovalDecision::TakeOver { dm_response } => {
            proto::ApprovalDecision::TakeOver { dm_response }
        }
    }
}

/// Convert protocol ApprovalDecision to app DmApprovalDecision
pub fn proto_decision_to_app(proto: proto::ApprovalDecision) -> app::DmApprovalDecision {
    match proto {
        proto::ApprovalDecision::Accept => app::DmApprovalDecision::Accept,
        proto::ApprovalDecision::AcceptWithRecipients { item_recipients } => {
            app::DmApprovalDecision::AcceptWithRecipients { item_recipients }
        }
        proto::ApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        } => app::DmApprovalDecision::AcceptWithModification {
            modified_dialogue,
            approved_tools,
            rejected_tools,
            item_recipients,
        },
        proto::ApprovalDecision::Reject { feedback } => {
            app::DmApprovalDecision::Reject { feedback }
        }
        proto::ApprovalDecision::TakeOver { dm_response } => {
            app::DmApprovalDecision::TakeOver { dm_response }
        }
        proto::ApprovalDecision::Unknown => {
            // Default unknown to Reject with explanation
            app::DmApprovalDecision::Reject {
                feedback: "Unknown approval decision received".to_string(),
            }
        }
    }
}

// =============================================================================
// Convenience conversion functions for Vec and Option types
// =============================================================================

/// Convert a Vec of app ProposedToolInfo to protocol ProposedToolInfo
pub fn app_tools_to_proto(tools: Vec<app::ProposedToolInfo>) -> Vec<proto::ProposedToolInfo> {
    tools.into_iter().map(app_tool_to_proto).collect()
}

/// Convert a Vec of protocol ProposedToolInfo to app ProposedToolInfo
pub fn proto_tools_to_app(tools: Vec<proto::ProposedToolInfo>) -> Vec<app::ProposedToolInfo> {
    tools.into_iter().map(proto_tool_to_app).collect()
}

/// Convert Option<app::ChallengeSuggestionInfo> to Option<proto::ChallengeSuggestionInfo>
pub fn app_challenge_suggestion_to_proto(
    suggestion: Option<app::ChallengeSuggestionInfo>,
) -> Option<proto::ChallengeSuggestionInfo> {
    suggestion.map(app_challenge_to_proto)
}

/// Convert Option<proto::ChallengeSuggestionInfo> to Option<app::ChallengeSuggestionInfo>
pub fn proto_challenge_suggestion_to_app(
    suggestion: Option<proto::ChallengeSuggestionInfo>,
) -> Option<app::ChallengeSuggestionInfo> {
    suggestion.map(proto_challenge_to_app)
}

/// Convert Option<app::NarrativeEventSuggestionInfo> to Option<proto::NarrativeEventSuggestionInfo>
pub fn app_narrative_suggestion_to_proto(
    suggestion: Option<app::NarrativeEventSuggestionInfo>,
) -> Option<proto::NarrativeEventSuggestionInfo> {
    suggestion.map(app_narrative_to_proto)
}

/// Convert Option<proto::NarrativeEventSuggestionInfo> to Option<app::NarrativeEventSuggestionInfo>
pub fn proto_narrative_suggestion_to_app(
    suggestion: Option<proto::NarrativeEventSuggestionInfo>,
) -> Option<app::NarrativeEventSuggestionInfo> {
    suggestion.map(proto_narrative_to_app)
}

// =============================================================================
// Domain ProposedTool conversions
// =============================================================================

/// Convert domain ProposedTool to protocol ProposedToolInfo
pub fn domain_tool_to_proto(
    domain: &wrldbldr_domain::value_objects::ProposedTool,
) -> proto::ProposedToolInfo {
    proto::ProposedToolInfo {
        id: domain.id.clone(),
        name: domain.name.clone(),
        description: domain.description.clone(),
        arguments: domain.arguments.clone(),
    }
}

/// Convert a slice of domain ProposedTool to Vec of protocol ProposedToolInfo
pub fn domain_tools_to_proto(
    tools: &[wrldbldr_domain::value_objects::ProposedTool],
) -> Vec<proto::ProposedToolInfo> {
    tools.iter().map(domain_tool_to_proto).collect()
}

// =============================================================================
// Domain ChallengeSuggestion and NarrativeEventSuggestion conversions
// =============================================================================

/// Convert domain ChallengeSuggestionOutcomes to protocol ChallengeSuggestionOutcomes
pub fn domain_outcomes_to_proto(
    domain: &wrldbldr_domain::value_objects::ChallengeSuggestionOutcomes,
) -> proto::ChallengeSuggestionOutcomes {
    proto::ChallengeSuggestionOutcomes {
        success: domain.success.clone(),
        failure: domain.failure.clone(),
        critical_success: domain.critical_success.clone(),
        critical_failure: domain.critical_failure.clone(),
    }
}

/// Convert domain ChallengeSuggestion to protocol ChallengeSuggestionInfo
pub fn domain_challenge_to_proto(
    domain: &wrldbldr_domain::value_objects::ChallengeSuggestion,
) -> proto::ChallengeSuggestionInfo {
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

/// Convert Option<domain ChallengeSuggestion> to Option<proto::ChallengeSuggestionInfo>
pub fn domain_challenge_suggestion_to_proto(
    suggestion: Option<&wrldbldr_domain::value_objects::ChallengeSuggestion>,
) -> Option<proto::ChallengeSuggestionInfo> {
    suggestion.map(domain_challenge_to_proto)
}

/// Convert domain NarrativeEventSuggestion to protocol NarrativeEventSuggestionInfo
pub fn domain_narrative_to_proto(
    domain: &wrldbldr_domain::value_objects::NarrativeEventSuggestion,
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

/// Convert Option<domain NarrativeEventSuggestion> to Option<proto::NarrativeEventSuggestionInfo>
pub fn domain_narrative_suggestion_to_proto(
    suggestion: Option<&wrldbldr_domain::value_objects::NarrativeEventSuggestion>,
) -> Option<proto::NarrativeEventSuggestionInfo> {
    suggestion.map(domain_narrative_to_proto)
}
