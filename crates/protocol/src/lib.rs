//! WrldBldr Protocol - Shared types for Engine and Player communication
//!
//! This crate contains all types shared between the Engine (backend) and Player (frontend):
//! - Wire-format DTOs (REST + WebSocket)
//! - WebSocket message types (ClientMessage, ServerMessage)
//! - Rule system configuration types
//! - Shared enums and value objects
//!
//! # Design Principles
//!
//! 1. **Minimal dependencies** - Only serde, uuid, chrono, and serde_json
//! 2. **No business logic** - Pure data types and serialization
//! 3. **WASM compatible** - Must compile for both native and wasm32 targets
//! 4. **No domain IDs** - use raw `uuid::Uuid` in DTOs

pub mod messages;
pub mod rule_system;
pub mod types;

// =============================================================================
// WebSocket Message Types
// =============================================================================
pub use messages::{
    // Main message enums
    ClientMessage,
    ServerMessage,
    // Challenge types
    AdHocOutcomes,
    ChallengeOutcomeDecisionData,
    DiceInputType,
    OutcomeBranchData,
    OutcomeDetailData,
    // Navigation types
    NavigationData,
    NavigationExit,
    NavigationTarget,
    NpcPresenceData,
    RegionData,
    // Scene types
    CharacterData,
    CharacterPosition,
    DialogueChoice,
    InteractionData,
    SceneData,
    // Session types
    DirectorialContext,
    NpcMotivationData,
    ParticipantInfo,
    SplitPartyLocation,
    // Staging types
    ApprovedNpcInfo,
    NpcPresentInfo,
    PreviousStagingInfo,
    StagedNpcInfo,
    WaitingPcInfo,
};

// =============================================================================
// Rule System Types
// =============================================================================
pub use rule_system::{RuleSystemConfig, RuleSystemType, RuleSystemVariant};

// =============================================================================
// Shared Types
// =============================================================================
pub use types::{
    // Approval types
    ApprovalDecision,
    ChallengeSuggestionInfo,
    ChallengeSuggestionOutcomes,
    NarrativeEventSuggestionInfo,
    ProposedToolInfo,
    // Character archetypes
    CampbellArchetype,
    // Game time
    GameTime,
    TimeOfDay,
    // Monomyth stages
    MonomythStage,
    // Participant roles
    ParticipantRole,
};
