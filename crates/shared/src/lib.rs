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
//! 1. **Minimal dependencies** - Only serde, uuid, serde_json, and thiserror
//! 2. **No business logic** - Pure data types and serialization
//! 3. **WASM compatible** - Must compile for both native and wasm32 targets
//! 4. **No domain IDs** - use raw `uuid::Uuid` in DTOs

pub mod app_events;
pub mod character_sheet;
pub mod dto;
pub mod game_systems;
pub mod messages;
pub mod requests;
pub mod responses;
pub mod rule_system;
pub mod settings;
pub mod types;
pub mod workflow;

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
    // Conversation Management (DM Only)
    ConversationFullDetails,
    ConversationInfo,
    ConversationParticipant,
    CreateGoalData,
    CreateWantData,
    DialogueChoice,
    DialogueTurn,
    DiceInputType,
    // Session types
    DirectorialContext,
    GoalData,
    InteractionData,
    LocationContext,
    MapBoundsData,
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
    ParticipantType,
    PreviousStagingInfo,
    RegionData,
    RegionItemData,
    RegionListItemData,
    SceneContext,
    SceneData,
    ServerMessage,
    SocialRelationData,
    SocialViewsData,
    SplitPartyLocation,
    StagedNpcInfo,
    // Conversion error types
    UnknownChallengeOutcomeDecisionError,
    UpdateGoalData,
    UpdateWantData,
    WaitingPcInfo,
};

// =============================================================================
// Character Sheet Schema Types
// =============================================================================

pub use character_sheet::{
    AllocationSystem, BoostDefinition, BoostSource, CharacterSheetAssetPrompt,
    CharacterSheetFieldChange, CharacterSheetSchema, CharacterSheetUpdate, ComparisonOperator,
    ConditionLevel, CreationField, CreationStep, DerivationType, DerivationTypeLocation,
    DerivedField, DerivedFieldDefinition, DerivedFieldDefinitionLegacy, DerivedValueDefinition,
    DotPoolCategory, EntityRefType, FieldAlignment, FieldDefinition, FieldLayout,
    FieldLayoutDefinition, FieldValidation, FieldValidationRule, FieldVisibilityRule, LadderLabel,
    PointCost, ProficiencyOption, ResourceAllocation, ResourceColor, SchemaFieldType,
    SchemaSection, SchemaSelectOption, SchemaStatMapping, SchemaVariantConfig, SectionLayout,
    SectionType, StatArray, StatArrayOption, ValidationRuleType, VisibilityRule,
};

pub use crate::game_systems::{FilterField, FilterFieldType, FilterSchema};
pub use crate::workflow::{
    analyze_workflow, auto_detect_prompt_mappings, find_nodes_by_type, validate_workflow,
    InputDefault, InputType, PromptMapping, PromptMappingType, WorkflowAnalysis,
    WorkflowConfiguration, WorkflowInput, WorkflowSlot,
};
pub use wrldbldr_domain::types::{CharacterSheetValues, SheetValue};

// =============================================================================
// App Events
// =============================================================================
pub use app_events::AppEvent;

// =============================================================================
// Rule System Types
// =============================================================================
pub use rule_system::{
    // Narrative resolution types
    BladesPoolThresholds,
    // Core rule system types
    DiceSystem,
    DifficultyDescriptor,
    DifficultyLadder,
    EffectLevel,
    EffectTickConfig,
    LadderEntry,
    NarrativeDiceConfig,
    NarrativeDiceType,
    NarrativeResolutionConfig,
    NarrativeResolutionStyle,
    NarrativeThresholds,
    Position,
    PositionEffectConfig,
    RuleSystemConfig,
    RuleSystemType,
    RuleSystemVariant,
    StatDefinition,
    SuccessComparison,
};

// =============================================================================
// Shared Types
// =============================================================================
pub use types::{
    // Activation rules
    ActivationLogicData,
    ActivationRuleData,
    // Approval types
    ApprovalDecision,
    // Character archetypes
    CampbellArchetype,
    ChallengeSuggestionInfo,
    ChallengeSuggestionOutcomes,
    // Game time
    GameTime,
    GameTimeConfig,
    // Location/Region states
    LocationStateData,
    // Lore types
    LoreCategoryData,
    LoreChunkData,
    LoreData,
    LoreDiscoverySourceData,
    LoreKnowledgeData,
    LoreSummaryData,
    // Monomyth stages
    MonomythStage,
    NarrativeEventSuggestionInfo,
    // Participant roles
    ParticipantRole,
    ProposedToolInfo,
    RegionStateData,
    ResolvedStateInfoData,
    ResolvedVisualStateData,
    StateOptionData,
    TimeCostConfig,
    TimeFormat,
    TimeMode,
    TimeOfDayData,
    TimeSuggestionData,
    TimeSuggestionDecision,
    // Trigger schema types (for Visual Trigger Builder)
    TriggerCategory,
    TriggerFieldSchema,
    TriggerFieldType,
    TriggerLogicOption,
    TriggerSchema,
    TriggerTypeSchema,
    // Conversion error types
    UnknownTimeSuggestionDecisionError,
    VisualStateSourceData,
};

// =============================================================================
// DTOs
// =============================================================================
pub use dto::{
    // Rule System parsing
    parse_system_type,
    parse_variant,
    // Workflow parsing
    parse_workflow_slot,
    // NOTE: workflow_config_to_*_response_dto functions moved to engine-adapters
    // Workflow DTOs
    AnalyzeWorkflowRequestDto,
    CreateWorkflowConfigRequestDto,
    // Export DTOs
    ExportQueryDto,
    // Asset DTOs
    GalleryAssetResponseDto,
    GenerateAssetRequestDto,
    GenerationBatchResponseDto,
    ImportWorkflowsRequestDto,
    ImportWorkflowsResponseDto,
    InputDefaultDto,
    InputTypeDto,
    // NPC Disposition
    NpcDispositionStateDto,
    PromptMappingDto,
    PromptMappingTypeDto,
    // Rule System DTOs
    RuleSystemPresetDetailsDto,
    RuleSystemPresetSummaryDto,
    RuleSystemSummaryDto,
    RuleSystemTypeDetailsDto,
    SelectFromBatchRequestDto,
    TestWorkflowRequestDto,
    TestWorkflowResponseDto,
    UpdateAssetLabelRequestDto,
    UpdateWorkflowDefaultsRequestDto,
    UploadAssetRequestDto,
    WorkflowAnalysisDto,
    WorkflowAnalysisResponseDto,
    WorkflowConfigExportDto,
    WorkflowConfigFullResponseDto,
    WorkflowConfigResponseDto,
    WorkflowInputDto,
    WorkflowSlotCategoryDto,
    WorkflowSlotStatusDto,
    WorkflowSlotsResponseDto,
};

// =============================================================================
// Request Types (WebSocket Request/Response Pattern)
// =============================================================================
pub use requests::{
    act::ActRequest,
    actantial::ActantialRequest,
    ai::AiRequest,
    challenge::ChallengeRequest,
    character::CharacterRequest,
    character_sheet::{CharacterSheetRequest, FieldUpdateData, GameSystemInfo},
    event_chain::EventChainRequest,
    expression::ExpressionRequest,
    generation::GenerationRequest,
    goal::GoalRequest,
    interaction::InteractionRequest,
    items::ItemsRequest,
    location::LocationRequest,
    lore::LoreRequest,
    narrative_event::NarrativeEventRequest,
    npc::NpcRequest,
    observation::ObservationRequest,
    player_character::PlayerCharacterRequest,
    region::RegionRequest,
    relationship::RelationshipRequest,
    scene::SceneRequest,
    skill::SkillRequest,
    stat::AddModifierData,
    stat::StatRequest,
    story_event::StoryEventRequest,
    time::TimeRequest,
    visual_state::{
        CreateVisualStateRequest, DeleteVisualStateRequest, GenerateVisualStateRequest,
        GeneratedVisualStateData, GetVisualStateCatalogRequest, GetVisualStateDetailsRequest,
        SetActiveVisualStateRequest, UpdateVisualStateRequest, VisualStateCatalogData,
        VisualStateRequest, VisualStateType,
    },
    want::WantRequest,
    world::WorldRequest,
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
