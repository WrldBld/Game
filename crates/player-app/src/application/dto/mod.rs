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
pub mod world_snapshot;
pub mod settings;

// Re-export action DTOs
pub use player_action::{PlayerAction, PlayerActionType};

// Re-export player events (domain-friendly server message representation)
pub use player_events::PlayerEvent;

// Re-export session DTOs
pub use session_dto::AppConnectionStatus;


// Re-export Engine snapshot contracts (application-owned).
pub use world_snapshot::{
    // Rule system types (re-exported from protocol/domain)
    RuleSystemConfig, RuleSystemPresetDetails, RuleSystemType, RuleSystemVariant,
    StatDefinition, DiceSystem, SuccessComparison,
    // Rule system extension traits (UI-specific methods)
    RuleSystemTypeExt, RuleSystemVariantExt,
    // Skill types
    SkillData, SkillCategory,
    // Character sheet types
    SheetTemplate, SheetSection, SheetField, SectionLayout,
    FieldType, FieldValue,
    // Challenge types
    ChallengeData, ChallengeType, ChallengeDifficulty,
    ChallengeOutcomes, Outcome,
    // Story arc types
    StoryEventData, StoryEventTypeData,
    NarrativeEventData, CreateNarrativeEventRequest,
    // Session snapshot types (simplified format from Engine)
    SessionWorldSnapshot,
    // Inventory types (Phase 23B)
    ItemData, InventoryItemData,
};

// Re-export settings DTOs
pub use settings::{AppSettings, ContextBudgetConfig, SettingsFieldMetadata, SettingsMetadataResponse};

// Re-export session types (application-owned DTOs for GameConnectionPort)
pub use session_types::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecision,
    DiceInput, DirectorialContext, NpcMotivationData, ParticipantRole,
};

// Re-export player event types for UI components
pub use player_events::{
    // Scene & character display
    CharacterData, CharacterPosition, SceneData, RegionData,
    // Dialogue
    DialogueChoice,
    // Navigation
    NavigationData, NavigationTarget, NavigationExit,
    // Interactions & items
    InteractionData, RegionItemData,
    // NPCs & staging
    NpcPresenceData, SplitPartyLocation, StagedNpcInfo, PreviousStagingInfo,
    WaitingPcInfo, NpcPresentInfo, NpcDispositionData,
    // Approval & challenges
    ProposedToolInfo, ChallengeSuggestionInfo, NarrativeEventSuggestionInfo,
    OutcomeDetailData, OutcomeBranchData,
    // Game time
    GameTime,
    // Connection
    ConnectedUser, WorldRole, JoinError, EntityChangedData, ResponseResult,
    // Actantial model (from player_events for UI)
    WantData as PlayerEventWantData, WantTargetData, ActantialViewData, GoalData as PlayerEventGoalData,
};

// Re-export actantial model types from protocol (Phase P5: facade pattern for UI layer)
// These are used by motivations_tab and other actantial-related components.
pub use wrldbldr_protocol::{
    WantVisibilityData, ActantialRoleData, WantTargetTypeData,
    NpcActantialContextData, WantData, GoalData,
    ActantialActorData, ActorTypeData, SocialRelationData,
};

// NOTE: Infrastructure asset loader now depends inward on these DTOs.
