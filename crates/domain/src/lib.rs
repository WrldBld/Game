extern crate self as wrldbldr_domain;

// Merged from wrldbldr-domain-types crate
pub mod types;

// Merged from wrldbldr-common crate
pub mod common;

pub mod aggregates;
pub mod character_sheet;
pub mod entities;
pub mod error;
pub mod events;
pub mod game_systems;
pub mod game_time;
pub mod ids;
pub mod value_objects;

// Re-export all entities (explicit list in entities/mod.rs)
pub use entities::{
    default_skills_for_variant, AbilityUses, AcquiredFeat, AcquisitionMethod, Act, ActantialRole,
    ActantialView, ActiveFeature, AssetType, BackgroundFeature, BatchStatus, CastingTime,
    CastingTimeUnit, ChainStatus, ChainedEvent, Challenge, ChallengeEventOutcome,
    ChallengeLocationAvailability, ChallengeOutcomes, ChallengePrerequisite,
    ChallengeRegionAvailability, ChallengeType, ChallengeUnlock, Character, CharacterFeats,
    CharacterFeatures, CharacterIdentity, CharacterSpells, CharacterWant, ClassFeature, ClassLevel,
    CombatEventType, CombatOutcome, Difficulty, DifficultyDescriptor, DmMarkerType, DurationUnit,
    EntityType, EventChain, EventChainMembership, EventEffect, EventOutcome, Feat, FeatBenefit,
    FeaturedNpc, FeatureUses, FlagScope, FrequencyLevel, GalleryAsset, GameFlag, GenerationBatch,
    GenerationMetadata, GenerationRequest, Goal, GridMap, InfoType, InputDefault, InputType,
    InteractionCondition, InteractionRequirement, InteractionTarget, InteractionTargetType,
    InteractionTemplate, InteractionType, InventoryItem, InvolvedCharacter, Item, ItemSource,
    KnownSpell, Location, LocationConnection, LocationState, LocationStateSummary, LocationType,
    Lore, LoreCategory, LoreChunk, LoreDiscoverySource, LoreKnowledge, MapBounds, MarkerImportance,
    MaterialComponent, MonomythStage, NarrativeEvent, NarrativeTrigger, NarrativeTriggerType,
    NpcObservation, ObservationSummary, ObservationType, Outcome, OutcomeCondition, OutcomeTrigger,
    OutcomeType, PlayerCharacter, Prerequisite, PromptMapping, PromptMappingType, RacialTrait,
    RechargeType, Region, RegionConnection, RegionExit, RegionState, RegionStateSummary,
    ResolvedStateInfo, ResolvedVisualState, Scene, SceneCharacter, SceneCharacterRole,
    SceneCondition, Skill, SkillCategory, Spell, SpellComponents, SpellDuration, SpellLevel,
    SpellRange, SpellSlotPool, StagedNpc, Staging, StagingSource, StatBlock, StoryEvent,
    StoryEventInfoImportance, StoryEventType, TimeAdvanceResult, TimeContext, TriggerCondition,
    TriggerContext, TriggerEvaluation, TriggerLogic, TriggerType, UsesFormula, VisualStateSource,
    Want, WantTargetType, WantVisibility, WorkflowAnalysis, WorkflowConfiguration, WorkflowInput,
    WorkflowSlot, World,
    // Content types for compendium system
    ContentFilter, ContentItem, ContentSource, ContentType, SourceType,
};

pub use error::DomainError;
pub use events::DomainEvent;

// Re-export game system traits and types
pub use game_systems::{
    dnd5e_skill_ability, CalculationEngine, CasterType, CharacterSheetProvider,
    CompendiumProvider, ContentError, Dnd5eSystem, FilterField, FilterFieldType, FilterSchema,
    GameSystem, GameSystemRegistry, ProficiencyLevel, RestType, SpellcastingSystem,
};

// Re-export character sheet schema types
pub use character_sheet::{
    CharacterSheetData, CharacterSheetResponse, CharacterSheetSchema, ConditionLevel,
    CreationStep, DerivedField, DerivationType, EntityRefType, FieldDefinition, FieldLayout,
    FieldUpdate, FieldUpdateResponse, FieldValidation, LadderLabel, ProficiencyOption,
    ResourceColor, SchemaFieldType, SchemaSection, SchemaSelectOption, SectionType,
    ValidationError,
};

// Re-export game time types
pub use game_time::{
    GameTime, GameTimeConfig, TimeAdvanceReason, TimeCostConfig, TimeFormat, TimeMode, TimeOfDay,
};

// Re-export ID types
pub use ids::{
    ActId, ActionId, AssetId, BatchId, ChallengeId, CharacterId, ConnectionId, EventChainId,
    EventId, GoalId, GridMapId, InteractionId, ItemId, LocationId, LocationStateId, LoreChunkId,
    LoreId, NarrativeEventId, ParticipantId, PlayerCharacterId, QueueItemId, RegionId,
    RegionStateId, RelationshipId, SceneId, SkillId, StagingId, StoryEventId, UserId, WantId,
    WorkflowConfigId, WorkflowId, WorldId,
};

// Re-export value objects (explicit list in value_objects/mod.rs)
pub use value_objects::{
    count_tokens,
    exceeds_token_budget,
    get_prompt_default,
    key_to_env_var,
    // Dialogue marker parsing functions
    parse_dialogue,
    parse_dialogue_markers,
    prompt_defaults,
    prompt_keys,
    prompt_template_keys,
    prompt_template_metadata,
    settings_metadata,
    validate_markers,
    ActantialActor,
    ActantialActorEntry,
    ActantialContext,
    ActantialLLMContext,
    ActantialTarget,
    ActivationEvaluation,
    ActivationLogic,
    ActivationRule,
    ActiveChallengeContext,
    ActiveEventContext,
    ActiveNarrativeEventContext,
    ActorType,
    AdHocOutcomes,
    AppSettings,
    // Queue data value objects
    ApprovalDecisionType,
    ApprovalRequestData,
    ApprovalType,
    ApprovalUrgency,
    ArchetypeChange,
    AssetGenerationData,
    CampbellArchetype,
    ChallengeOutcomeData,
    ChallengeSignificance,
    ChallengeSuggestion,
    ChallengeSuggestionOutcomes,
    ChangeAmount,
    CharacterContext,
    ComfyUIConfig,
    ContextBudgetConfig,
    ContextCategory,
    ConversationEntry,
    ConversationTurn,
    // Dialogue marker parsing
    DialogueMarker,
    DiceRollInput,
    DiceSystem,
    DirectorialNotes,
    DispositionLevel,
    DmActionData,
    DmActionType,
    DmApprovalDecision,
    // Expression configuration
    ExpressionConfig,
    GamePromptRequest,
    LlmRequestData,
    LlmRequestType,
    MoodState,
    MotivationEntry,
    MotivationsContext,
    NarrativeEventSuggestion,
    NpcDialogueContext,
    NpcDispositionState,
    PacingGuidance,
    // Dialogue marker types
    ParsedDialogue,
    PendingApprovalItem,
    PlayerActionContext,
    PlayerActionData,
    PromptTemplateCategory,
    PromptTemplateMetadata,
    ProposedTool,
    RegionFrequency,
    RegionItemContext,
    RegionRelationship,
    RegionRelationshipType,
    RegionShift,
    Relationship,
    RelationshipChange,
    RelationshipEvent,
    RelationshipLevel,
    RelationshipType,
    RollResult,
    RuleBasedSuggestion,
    RuleSystemConfig,
    RuleSystemType,
    RuleSystemVariant,
    SceneContext,
    SecretMotivationContext,
    SecretMotivationEntry,
    SettingsFieldMetadata,
    SocialRelationEntry,
    SocialStanceContext,
    SocialViewSummary,
    Speaker,
    StagingContext,
    StatDefinition,
    SuccessComparison,
    SuggestionContext,
    TokenCountMethod,
    TokenCounter,
    ToneGuidance,
    WantContext,
    WantTarget,
};
