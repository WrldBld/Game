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
pub mod dto;
pub mod messages;
pub mod requests;
pub mod responses;
pub mod rule_system;
pub mod types;

// =============================================================================
// WebSocket Message Types
// =============================================================================
pub use messages::{
    ActantialActorData,
    ActantialRoleData,
    ActantialViewData,
    ActorTypeData,
    // Challenge types
    AdHocOutcomes,
    // Staging types
    ApprovedNpcInfo,
    ChallengeOutcomeDecisionData,
    // Scene types
    CharacterData,
    CharacterPosition,
    // Main message enums
    ClientMessage,
    CreateGoalData,
    CreateWantData,
    DialogueChoice,
    DiceInputType,
    // Session types
    DirectorialContext,
    GoalData,
    InteractionData,
    // Navigation types
    NavigationData,
    NavigationExit,
    NavigationTarget,
    NpcActantialContextData,
    // NPC Disposition types (P1.4)
    NpcDispositionData,
    NpcMotivationData,
    NpcPresenceData,
    NpcPresentInfo,
    OutcomeBranchData,
    OutcomeDetailData,
    ParticipantInfo,
    PreviousStagingInfo,
    RegionData,
    RegionItemData,
    SceneData,
    ServerMessage,
    SocialRelationData,
    SocialViewsData,
    SplitPartyLocation,
    StagedNpcInfo,
    UpdateGoalData,
    UpdateWantData,
    WaitingPcInfo,
    // Actantial Model types (P1.5)
    WantData,
    WantTargetData,
    WantTargetTypeData,
    WantVisibilityData,
};

// =============================================================================
// App Events
// =============================================================================
pub use app_events::AppEvent;

// =============================================================================
// Rule System Types
// =============================================================================
pub use rule_system::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition,
    SuccessComparison,
};

// =============================================================================
// Shared Types
// =============================================================================
pub use types::{
    // Approval types
    ApprovalDecision,
    // Character archetypes
    CampbellArchetype,
    ChallengeSuggestionInfo,
    ChallengeSuggestionOutcomes,
    // Game time
    GameTime,
    // Monomyth stages
    MonomythStage,
    NarrativeEventSuggestionInfo,
    // Participant roles
    ParticipantRole,
    ProposedToolInfo,
};

// =============================================================================
// DTOs
// =============================================================================
pub use dto::{
    // Export DTOs
    ExportQueryDto,
    // Asset DTOs
    GalleryAssetResponseDto,
    GenerateAssetRequestDto,
    GenerationBatchResponseDto,
    // NPC Disposition
    NpcDispositionStateDto,
    // Workflow DTOs
    PromptMappingDto,
    PromptMappingTypeDto,
    SelectFromBatchRequestDto,
    UpdateAssetLabelRequestDto,
    UploadAssetRequestDto,
    WorkflowConfigResponseDto,
};

// =============================================================================
// Request Types (WebSocket Request/Response Pattern)
// =============================================================================
pub use requests::{
    // Create data types
    ChangeArchetypeData,
    CreateActData,
    CreateChallengeData,
    CreateCharacterData,
    CreateDmMarkerData,
    CreateEventChainData,
    CreateInteractionData,
    CreateItemData,
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
    // Main payload enum
    RequestPayload,
    // Suggestion types
    SuggestionContextData,
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
    ChangeType,
    ConnectedUser,
    // Entity change broadcasts
    EntityChangedData,
    EntityType,
    ErrorCode,
    JoinError,
    // Request error (client-side)
    RequestError,
    // Response result
    ResponseResult,
    // World connection types
    WorldRole,
};
