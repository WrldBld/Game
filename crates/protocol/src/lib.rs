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

pub mod app_events;
pub mod messages;
pub mod requests;
pub mod responses;
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
    RegionItemData,
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
    // NPC Mood types (P1.4)
    NpcMoodData,
    // Actantial Model types (P1.5)
    WantData,
    WantTargetData,
    CreateWantData,
    UpdateWantData,
    ActantialActorData,
    ActantialViewData,
    NpcActantialContextData,
    SocialViewsData,
    SocialRelationData,
    GoalData,
    CreateGoalData,
    UpdateGoalData,
    WantVisibilityData,
    ActorTypeData,
    ActantialRoleData,
    WantTargetTypeData,
};

// =============================================================================
// App Events
// =============================================================================
pub use app_events::AppEvent;

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
    // Monomyth stages
    MonomythStage,
    // Participant roles
    ParticipantRole,
};

// =============================================================================
// Request Types (WebSocket Request/Response Pattern)
// =============================================================================
pub use requests::{
    // Main payload enum
    RequestPayload,
    // Create data types
    ChangeArchetypeData,
    CreateActData,
    CreateChallengeData,
    CreateCharacterData,
    CreateDmMarkerData,
    CreateEventChainData,
    CreateInteractionData,
    CreateLocationConnectionData,
    CreateLocationData,
    CreateNarrativeEventData,
    CreateObservationData,
    CreatePlayerCharacterData,
    CreateRegionConnectionData,
    CreateRegionData,
    CreateRelationshipData,
    CreateSceneData,
    CreateSkillData,
    CreateWorldData,
    // Update data types
    UpdateChallengeData,
    UpdateCharacterData,
    UpdateEventChainData,
    UpdateInteractionData,
    UpdateLocationData,
    UpdateNarrativeEventData,
    UpdatePlayerCharacterData,
    UpdateRegionData,
    UpdateSceneData,
    UpdateSkillData,
    UpdateStoryEventData,
    UpdateWorldData,
};

// =============================================================================
// Response Types (WebSocket Request/Response Pattern)
// =============================================================================
pub use responses::{
    // Response result
    ResponseResult,
    ErrorCode,
    // Entity change broadcasts
    EntityChangedData,
    EntityType,
    ChangeType,
    // World connection types
    WorldRole,
    ConnectedUser,
    JoinError,
};
