//! WebSocket protocol messages (Engine ↔ Player)
//!
//! This module re-exports wire-format DTOs from the shared protocol crate
//! and provides any Engine-specific extensions.

// Re-export all message types from protocol crate
pub use wrldbldr_protocol::{
    // Main message enums
    ClientMessage,
    ServerMessage,
    // Session types
    ParticipantInfo,
    ParticipantRole,
    DirectorialContext,
    NpcMotivationData,
    SplitPartyLocation,
    // Scene types
    SceneData,
    CharacterData,
    CharacterPosition,
    InteractionData,
    DialogueChoice,
    // Navigation types
    RegionData,
    NpcPresenceData,
    NavigationData,
    NavigationTarget,
    NavigationExit,
    // Challenge types
    DiceInputType,
    AdHocOutcomes,
    OutcomeDetailData,
    ChallengeOutcomeDecisionData,
    OutcomeBranchData,
    // Approval types
    ProposedToolInfo,
    ApprovalDecision,
    ChallengeSuggestionInfo,
    NarrativeEventSuggestionInfo,
    // Staging types
    StagedNpcInfo,
    PreviousStagingInfo,
    WaitingPcInfo,
    NpcPresentInfo,
    ApprovedNpcInfo,
};
