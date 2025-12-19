//! WebSocket message DTOs (application layer).
//!
//! Re-exports types from the shared protocol crate for Engine ↔ Player communication.
//! Type aliases maintain backward compatibility with existing Player code.

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
    // Scene types (with aliases below for backward compatibility)
    CharacterPosition,
    DialogueChoice,
    InteractionData,
    // Navigation types
    NavigationData,
    NavigationExit,
    NavigationTarget,
    NpcPresenceData,
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
};

// =============================================================================
// Type Aliases for Backward Compatibility
// =============================================================================

/// Alias: Player used `SceneSnapshot`, protocol uses `SceneData`
pub type SceneSnapshot = wrldbldr_protocol::SceneData;

/// Alias: Player used `SceneCharacterState`, protocol uses `CharacterData`
pub type SceneCharacterState = wrldbldr_protocol::CharacterData;

/// Alias: Player used `SceneRegionInfo`, protocol uses `RegionData`
pub type SceneRegionInfo = wrldbldr_protocol::RegionData;

/// Alias: Player used `ProposedTool`, protocol uses `ProposedToolInfo`
pub type ProposedTool = wrldbldr_protocol::ProposedToolInfo;
