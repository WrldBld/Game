//! Data transfer objects
//!
//! DTOs are used to transfer data between layers. The application layer
//! provides these types so that presentation doesn't need to import
//! directly from infrastructure.

pub mod player_action;
pub mod player_events;
pub mod requests;
pub mod session_dto;
pub mod session_types;
pub mod settings;
pub mod world_snapshot;

// Re-export action DTOs
pub use player_action::{PlayerAction, PlayerActionType};

// Re-export player events (domain-friendly server message representation)
pub use player_events::PlayerEvent;

// Re-export session DTOs
pub use session_dto::AppConnectionStatus;

// Re-export Engine snapshot contracts (application-owned).
pub use world_snapshot::{
    // Challenge types
    ChallengeData,
    ChallengeDifficulty,
    ChallengeOutcomes,
    ChallengeType,
    CreateNarrativeEventRequest,
    DiceSystem,
    FieldType,
    FieldValue,
    InventoryItemData,
    // Inventory types (Phase 23B)
    ItemData,
    NarrativeEventData,
    Outcome,
    // Rule system types (re-exported from protocol/domain)
    RuleSystemConfig,
    RuleSystemPresetDetails,
    RuleSystemType,
    // Rule system extension traits (UI-specific methods)
    RuleSystemTypeExt,
    RuleSystemVariant,
    RuleSystemVariantExt,
    SectionLayout,
    // Session snapshot types (simplified format from Engine)
    SessionWorldSnapshot,
    SheetField,
    SheetSection,
    // Character sheet types
    SheetTemplate,
    SkillCategory,
    // Skill types
    SkillData,
    StatDefinition,
    // Story arc types
    StoryEventData,
    StoryEventTypeData,
    SuccessComparison,
};

// Re-export settings DTOs
pub use settings::{
    AppSettings, ContextBudgetConfig, SettingsFieldMetadata, SettingsMetadataResponse,
};

// Re-export request DTOs
pub use requests::{
    ChangeArchetypeRequest, CreateChallengeRequest, CreateCharacterRequest, CreateWorldRequest,
    SuggestionContext, UpdateChallengeRequest, UpdateCharacterRequest,
};

// Re-export session types (application-owned DTOs for GameConnectionPort)
pub use session_types::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecision, DiceInput,
    DirectorialContext, NpcMotivationData, ParticipantRole,
};

// Re-export player event types for UI components
pub use player_events::{
    ActantialViewData,
    ChallengeSuggestionInfo,
    // Scene & character display
    CharacterData,
    CharacterPosition,
    // Connection
    ConnectedUser,
    // Dialogue
    DialogueChoice,
    EntityChangedData,
    // Game time
    GameTime,
    GoalData as PlayerEventGoalData,
    // Interactions & items
    InteractionData,
    JoinError,
    NarrativeEventSuggestionInfo,
    // Navigation
    NavigationData,
    NavigationExit,
    NavigationTarget,
    NpcDispositionData,
    // NPCs & staging
    NpcPresenceData,
    NpcPresentInfo,
    OutcomeBranchData,
    OutcomeDetailData,
    PreviousStagingInfo,
    // Approval & challenges
    ProposedToolInfo,
    RegionData,
    RegionItemData,
    ResponseResult,
    SceneData,
    SplitPartyLocation,
    StagedNpcInfo,
    WaitingPcInfo,
    // Actantial model (from player_events for UI)
    WantData as PlayerEventWantData,
    WantTargetData,
    WorldRole,
};

// ARCHITECTURE EXCEPTION: [APPROVED 2025-12-28]
// Re-export actantial model types from protocol (facade pattern for UI layer).
// These are value objects/enums used across layers. Creating app-layer equivalents
// for all 9 types would require 18+ From impls with no benefit. The protocol types
// are stable and serialization-ready.
// See: docs/plans/HEXAGONAL_GAP_REMEDIATION_PLAN.md Appendix B
pub use wrldbldr_protocol::{
    ActantialActorData, ActantialRoleData, ActorTypeData, GoalData, NpcActantialContextData,
    SocialRelationData, WantData, WantTargetTypeData, WantVisibilityData,
};

// NOTE: Infrastructure asset loader now depends inward on these DTOs.
