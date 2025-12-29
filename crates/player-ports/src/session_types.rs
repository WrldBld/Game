//! Session-related DTOs for GameConnectionPort
//!
//! These types are owned by the ports layer and define the contract
//! between the application layer and the adapters layer.
//!
//! `From` trait implementations enable idiomatic conversions at adapter boundaries:
//! ```ignore
//! let local_role: ParticipantRole = proto_role.into();
//! let proto_role: proto::ParticipantRole = local_role.into();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wrldbldr_protocol as proto;

/// Role of a participant in a game session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantRole {
    Player,
    DungeonMaster,
    Spectator,
}

/// Type of dice input for challenge rolls
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiceInput {
    /// Dice formula (e.g., "1d20+5")
    Formula(String),
    /// Manual entry with result value
    Manual(i32),
}

/// DM's approval decision
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum ApprovalDecision {
    /// Accept all proposed tools with default recipients
    Accept,
    /// Accept with item recipient selection
    AcceptWithRecipients {
        /// For give_item tools: maps tool_id -> recipient PC IDs
        /// Empty list means "don't give this item"
        item_recipients: HashMap<String, Vec<String>>,
    },
    /// Accept with modifications to dialogue and/or tool selection
    AcceptWithModification {
        modified_dialogue: String,
        approved_tools: Vec<String>,
        rejected_tools: Vec<String>,
        /// For give_item tools: maps tool_id -> recipient PC IDs
        /// Empty list means "don't give this item"
        #[serde(default)]
        item_recipients: HashMap<String, Vec<String>>,
    },
    /// Reject the proposal
    Reject { feedback: String },
    /// DM takes over the response
    TakeOver { dm_response: String },
}

/// Directorial context for scene guidance
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DirectorialContext {
    pub scene_notes: String,
    pub tone: String,
    pub npc_motivations: Vec<NpcMotivationData>,
    pub forbidden_topics: Vec<String>,
}

/// NPC motivation data for directorial context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcMotivationData {
    pub character_id: String,
    /// Free-form emotional guidance for the NPC (e.g., "Conflicted about revealing secrets")
    pub emotional_guidance: String,
    pub immediate_goal: String,
    pub secret_agenda: Option<String>,
}

/// Approved NPC info for staging
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovedNpcInfo {
    pub character_id: String,
    pub is_present: bool,
    /// Optional override reasoning (if DM modified)
    #[serde(default)]
    pub reasoning: Option<String>,
    /// When true, NPC is present but hidden from players
    #[serde(default)]
    pub is_hidden_from_players: bool,
}

/// Ad-hoc challenge outcomes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdHocOutcomes {
    pub success: String,
    pub failure: String,
    #[serde(default)]
    pub critical_success: Option<String>,
    #[serde(default)]
    pub critical_failure: Option<String>,
}

/// Challenge outcome decision from DM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ChallengeOutcomeDecision {
    /// Accept the outcome as-is
    Accept,
    /// Edit the outcome description
    Edit { modified_description: String },
    /// Request LLM to suggest alternatives
    Suggest {
        #[serde(default)]
        guidance: Option<String>,
    },
}

// =============================================================================
// From Trait Implementations: proto -> session_types
// =============================================================================

impl From<proto::ParticipantRole> for ParticipantRole {
    fn from(proto: proto::ParticipantRole) -> Self {
        match proto {
            proto::ParticipantRole::Player => Self::Player,
            proto::ParticipantRole::DungeonMaster => Self::DungeonMaster,
            // Unknown falls back to Spectator (least privileged)
            proto::ParticipantRole::Spectator | proto::ParticipantRole::Unknown => Self::Spectator,
        }
    }
}

impl From<proto::DiceInputType> for DiceInput {
    fn from(proto: proto::DiceInputType) -> Self {
        match proto {
            proto::DiceInputType::Formula(formula) => Self::Formula(formula),
            proto::DiceInputType::Manual(value) => Self::Manual(value),
            // Unknown falls back to Manual(0)
            proto::DiceInputType::Unknown => Self::Manual(0),
        }
    }
}

impl From<proto::ApprovalDecision> for ApprovalDecision {
    fn from(proto: proto::ApprovalDecision) -> Self {
        match proto {
            proto::ApprovalDecision::Accept => Self::Accept,
            proto::ApprovalDecision::AcceptWithRecipients { item_recipients } => {
                Self::AcceptWithRecipients { item_recipients }
            }
            proto::ApprovalDecision::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                rejected_tools,
                item_recipients,
            } => Self::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                rejected_tools,
                item_recipients,
            },
            proto::ApprovalDecision::Reject { feedback } => Self::Reject { feedback },
            proto::ApprovalDecision::TakeOver { dm_response } => Self::TakeOver { dm_response },
            // Unknown falls back to Reject with explanation
            proto::ApprovalDecision::Unknown => Self::Reject {
                feedback: "Unknown approval decision received".to_string(),
            },
        }
    }
}

impl From<proto::NpcMotivationData> for NpcMotivationData {
    fn from(proto: proto::NpcMotivationData) -> Self {
        Self {
            character_id: proto.character_id,
            emotional_guidance: proto.emotional_guidance,
            immediate_goal: proto.immediate_goal,
            secret_agenda: proto.secret_agenda,
        }
    }
}

impl From<proto::DirectorialContext> for DirectorialContext {
    fn from(proto: proto::DirectorialContext) -> Self {
        Self {
            scene_notes: proto.scene_notes,
            tone: proto.tone,
            npc_motivations: proto.npc_motivations.into_iter().map(Into::into).collect(),
            forbidden_topics: proto.forbidden_topics,
        }
    }
}

impl From<proto::ApprovedNpcInfo> for ApprovedNpcInfo {
    fn from(proto: proto::ApprovedNpcInfo) -> Self {
        Self {
            character_id: proto.character_id,
            is_present: proto.is_present,
            reasoning: proto.reasoning,
            is_hidden_from_players: proto.is_hidden_from_players,
        }
    }
}

impl From<proto::AdHocOutcomes> for AdHocOutcomes {
    fn from(proto: proto::AdHocOutcomes) -> Self {
        Self {
            success: proto.success,
            failure: proto.failure,
            critical_success: proto.critical_success,
            critical_failure: proto.critical_failure,
        }
    }
}

impl From<proto::ChallengeOutcomeDecisionData> for ChallengeOutcomeDecision {
    fn from(proto: proto::ChallengeOutcomeDecisionData) -> Self {
        match proto {
            proto::ChallengeOutcomeDecisionData::Accept => Self::Accept,
            proto::ChallengeOutcomeDecisionData::Edit {
                modified_description,
            } => Self::Edit {
                modified_description,
            },
            proto::ChallengeOutcomeDecisionData::Suggest { guidance } => Self::Suggest { guidance },
            // Unknown falls back to Accept
            proto::ChallengeOutcomeDecisionData::Unknown => Self::Accept,
        }
    }
}

// =============================================================================
// From Trait Implementations: session_types -> proto
// =============================================================================

impl From<ParticipantRole> for proto::ParticipantRole {
    fn from(local: ParticipantRole) -> Self {
        match local {
            ParticipantRole::Player => Self::Player,
            ParticipantRole::DungeonMaster => Self::DungeonMaster,
            ParticipantRole::Spectator => Self::Spectator,
        }
    }
}

impl From<DiceInput> for proto::DiceInputType {
    fn from(local: DiceInput) -> Self {
        match local {
            DiceInput::Formula(formula) => Self::Formula(formula),
            DiceInput::Manual(value) => Self::Manual(value),
        }
    }
}

impl From<ApprovalDecision> for proto::ApprovalDecision {
    fn from(local: ApprovalDecision) -> Self {
        match local {
            ApprovalDecision::Accept => Self::Accept,
            ApprovalDecision::AcceptWithRecipients { item_recipients } => {
                Self::AcceptWithRecipients { item_recipients }
            }
            ApprovalDecision::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                rejected_tools,
                item_recipients,
            } => Self::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                rejected_tools,
                item_recipients,
            },
            ApprovalDecision::Reject { feedback } => Self::Reject { feedback },
            ApprovalDecision::TakeOver { dm_response } => Self::TakeOver { dm_response },
        }
    }
}

impl From<NpcMotivationData> for proto::NpcMotivationData {
    fn from(local: NpcMotivationData) -> Self {
        Self {
            character_id: local.character_id,
            emotional_guidance: local.emotional_guidance,
            immediate_goal: local.immediate_goal,
            secret_agenda: local.secret_agenda,
        }
    }
}

impl From<DirectorialContext> for proto::DirectorialContext {
    fn from(local: DirectorialContext) -> Self {
        Self {
            scene_notes: local.scene_notes,
            tone: local.tone,
            npc_motivations: local.npc_motivations.into_iter().map(Into::into).collect(),
            forbidden_topics: local.forbidden_topics,
        }
    }
}

impl From<ApprovedNpcInfo> for proto::ApprovedNpcInfo {
    fn from(local: ApprovedNpcInfo) -> Self {
        Self {
            character_id: local.character_id,
            is_present: local.is_present,
            reasoning: local.reasoning,
            is_hidden_from_players: local.is_hidden_from_players,
        }
    }
}

impl From<AdHocOutcomes> for proto::AdHocOutcomes {
    fn from(local: AdHocOutcomes) -> Self {
        Self {
            success: local.success,
            failure: local.failure,
            critical_success: local.critical_success,
            critical_failure: local.critical_failure,
        }
    }
}

impl From<ChallengeOutcomeDecision> for proto::ChallengeOutcomeDecisionData {
    fn from(local: ChallengeOutcomeDecision) -> Self {
        match local {
            ChallengeOutcomeDecision::Accept => Self::Accept,
            ChallengeOutcomeDecision::Edit {
                modified_description,
            } => Self::Edit {
                modified_description,
            },
            ChallengeOutcomeDecision::Suggest { guidance } => Self::Suggest { guidance },
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_participant_role_roundtrip() {
        let roles = [
            ParticipantRole::Player,
            ParticipantRole::DungeonMaster,
            ParticipantRole::Spectator,
        ];

        for role in roles {
            let proto_role: proto::ParticipantRole = role.into();
            let back: ParticipantRole = proto_role.into();
            assert_eq!(role, back);
        }
    }

    #[test]
    fn test_participant_role_unknown_fallback() {
        let unknown: ParticipantRole = proto::ParticipantRole::Unknown.into();
        assert_eq!(unknown, ParticipantRole::Spectator);
    }

    #[test]
    fn test_dice_input_roundtrip() {
        let inputs = [
            DiceInput::Formula("1d20+5".to_string()),
            DiceInput::Manual(15),
        ];

        for input in inputs {
            let proto_input: proto::DiceInputType = input.clone().into();
            let back: DiceInput = proto_input.into();
            assert_eq!(input, back);
        }
    }

    #[test]
    fn test_dice_input_unknown_fallback() {
        let unknown: DiceInput = proto::DiceInputType::Unknown.into();
        assert_eq!(unknown, DiceInput::Manual(0));
    }

    #[test]
    fn test_approval_decision_roundtrip() {
        let decisions = [
            ApprovalDecision::Accept,
            ApprovalDecision::AcceptWithRecipients {
                item_recipients: HashMap::from([("tool_1".to_string(), vec!["pc_1".to_string()])]),
            },
            ApprovalDecision::AcceptWithModification {
                modified_dialogue: "Modified response".to_string(),
                approved_tools: vec!["tool_1".to_string()],
                rejected_tools: vec!["tool_2".to_string()],
                item_recipients: HashMap::new(),
            },
            ApprovalDecision::Reject {
                feedback: "Too powerful".to_string(),
            },
            ApprovalDecision::TakeOver {
                dm_response: "DM takes over".to_string(),
            },
        ];

        for decision in decisions {
            let proto_decision: proto::ApprovalDecision = decision.clone().into();
            let back: ApprovalDecision = proto_decision.into();
            assert_eq!(decision, back);
        }
    }

    #[test]
    fn test_approval_decision_unknown_fallback() {
        let unknown: ApprovalDecision = proto::ApprovalDecision::Unknown.into();
        match unknown {
            ApprovalDecision::Reject { feedback } => {
                assert!(feedback.contains("Unknown"));
            }
            _ => panic!("Expected Reject for unknown"),
        }
    }

    #[test]
    fn test_npc_motivation_roundtrip() {
        let motivation = NpcMotivationData {
            character_id: "npc-123".to_string(),
            emotional_guidance: "Conflicted".to_string(),
            immediate_goal: "Find the artifact".to_string(),
            secret_agenda: Some("Betray the party".to_string()),
        };

        let proto_motivation: proto::NpcMotivationData = motivation.clone().into();
        let back: NpcMotivationData = proto_motivation.into();
        assert_eq!(motivation, back);
    }

    #[test]
    fn test_directorial_context_roundtrip() {
        let context = DirectorialContext {
            scene_notes: "Tense scene".to_string(),
            tone: "Dark".to_string(),
            npc_motivations: vec![NpcMotivationData {
                character_id: "npc-123".to_string(),
                emotional_guidance: "Suspicious".to_string(),
                immediate_goal: "Guard the entrance".to_string(),
                secret_agenda: None,
            }],
            forbidden_topics: vec!["meta-gaming".to_string()],
        };

        let proto_context: proto::DirectorialContext = context.clone().into();
        let back: DirectorialContext = proto_context.into();
        assert_eq!(context, back);
    }

    #[test]
    fn test_approved_npc_info_roundtrip() {
        let infos = [
            ApprovedNpcInfo {
                character_id: "npc-123".to_string(),
                is_present: true,
                reasoning: Some("Plot requirement".to_string()),
                is_hidden_from_players: false,
            },
            ApprovedNpcInfo {
                character_id: "npc-456".to_string(),
                is_present: true,
                reasoning: None,
                is_hidden_from_players: true,
            },
        ];

        for info in infos {
            let proto_info: proto::ApprovedNpcInfo = info.clone().into();
            let back: ApprovedNpcInfo = proto_info.into();
            assert_eq!(info, back);
        }
    }

    #[test]
    fn test_adhoc_outcomes_roundtrip() {
        let outcomes = [
            AdHocOutcomes {
                success: "You succeed!".to_string(),
                failure: "You fail!".to_string(),
                critical_success: Some("Critical success!".to_string()),
                critical_failure: Some("Critical failure!".to_string()),
            },
            AdHocOutcomes {
                success: "Minor success".to_string(),
                failure: "Minor failure".to_string(),
                critical_success: None,
                critical_failure: None,
            },
        ];

        for outcome in outcomes {
            let proto_outcome: proto::AdHocOutcomes = outcome.clone().into();
            let back: AdHocOutcomes = proto_outcome.into();
            assert_eq!(outcome, back);
        }
    }

    #[test]
    fn test_challenge_outcome_decision_roundtrip() {
        let decisions = [
            ChallengeOutcomeDecision::Accept,
            ChallengeOutcomeDecision::Edit {
                modified_description: "New description".to_string(),
            },
            ChallengeOutcomeDecision::Suggest {
                guidance: Some("Make it more dramatic".to_string()),
            },
            ChallengeOutcomeDecision::Suggest { guidance: None },
        ];

        for decision in decisions {
            let proto_decision: proto::ChallengeOutcomeDecisionData = decision.clone().into();
            let back: ChallengeOutcomeDecision = proto_decision.into();
            assert_eq!(decision, back);
        }
    }

    #[test]
    fn test_challenge_outcome_decision_unknown_fallback() {
        let unknown: ChallengeOutcomeDecision = proto::ChallengeOutcomeDecisionData::Unknown.into();
        assert_eq!(unknown, ChallengeOutcomeDecision::Accept);
    }
}
