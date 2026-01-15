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
    ChallengeRegionAvailability, ChallengeType, ChallengeUnlock, CharacterFeats,
    CharacterFeatures, CharacterIdentity, CharacterSpells, CharacterWant, ClassFeature, ClassLevel,
    CombatEventType, CombatOutcome, Difficulty, DifficultyDescriptor, DmMarkerType, DurationUnit,
    EntityType, EventChain, EventChainMembership, EventEffect, EventOutcome, Feat, FeatBenefit,
    FeaturedNpc, FeatureUses, FlagScope, FrequencyLevel, GalleryAsset, GameFlag, GenerationBatch,
    GenerationMetadata, GenerationRequest, Goal, GridMap, InfoType, InputDefault, InputType,
    InteractionCondition, InteractionRequirement, InteractionTarget, InteractionTargetType,
    InteractionTemplate, InteractionType, InventoryItem, InvolvedCharacter, Item, ItemSource,
    KnownSpell, LocationConnection, ConnectionType, LocationState, LocationStateSummary, LocationType,
    Lore, LoreCategory, LoreChunk, LoreDiscoverySource, LoreKnowledge, MapBounds, MarkerImportance,
    MaterialComponent, MonomythStage, NarrativeTrigger, NarrativeTriggerType,
    NpcObservation, ObservationSummary, ObservationType, Outcome, OutcomeCondition, OutcomeTrigger,
    OutcomeType, Prerequisite, PromptMapping, PromptMappingType, RacialTrait,
    RechargeType, Region, RegionConnection, RegionExit, RegionState, RegionStateSummary,
    ResolvedStateInfo, ResolvedVisualState, SceneCharacter, SceneCharacterRole,
    SceneCondition, Skill, SkillCategory, Spell, SpellComponents, SpellDuration, SpellLevel,
    SpellRange, SpellSlotPool, StagedNpc, Staging, StagingSource, StatBlock, StatModifier, StatValue, StoryEvent,
    StoryEventInfoImportance, StoryEventType, TimeAdvanceResult, TimeContext, TriggerCondition,
    TriggerContext, TriggerEvaluation, TriggerLogic, TriggerType, UsesFormula, VisualStateSource,
    Want, WantTargetType, WantVisibility, WorkflowAnalysis, WorkflowConfiguration, WorkflowInput,
    WorkflowSlot,
    // Content types for compendium system
    ContentFilter, ContentItem, ContentSource, ContentType, SourceType,
};

// Re-export Character from aggregates (the Rustic DDD version with private fields)
// This replaces the legacy entities::Character with public fields
pub use aggregates::Character;

// Re-export Location from aggregates (the Rustic DDD version with private fields)
// This replaces the legacy entities::Location with public fields
pub use aggregates::Location;

// Re-export World from aggregates (the Rustic DDD version with private fields)
// This replaces the legacy entities::World with public fields
pub use aggregates::World;

// Re-export Scene from aggregates (the Rustic DDD version with private fields)
// This replaces the legacy entities::Scene with public fields
pub use aggregates::Scene;

// Re-export PlayerCharacter from aggregates (the Rustic DDD version with private fields)
// This replaces the legacy entities::PlayerCharacter with public fields
pub use aggregates::{PlayerCharacter, PlayerCharacterStateChange};

// Re-export NarrativeEvent from aggregates (the Rustic DDD version with private fields)
// This replaces the legacy entities::NarrativeEvent with public fields
pub use aggregates::NarrativeEvent;

pub use error::DomainError;
pub use events::{
    ArchetypeShift, ChallengeOutcome, CharacterStateChange, CharacterUpdate, DamageOutcome,
    DomainEvent, HealOutcome, NarrativeEventUpdate, ResurrectOutcome, SceneUpdate,
};

// Re-export aggregate types with "Aggregate" suffix for migration compatibility
// These aliases allow existing code using `CharacterAggregate` to continue working
pub use aggregates::Character as CharacterAggregate;
// NOTE: aggregates::Location is the new Rustic DDD version with private fields.
// entities::Location is the legacy version with public fields.
// During migration (Phase 4-6), both exist. After Phase 7, only aggregates::Location remains.
pub use aggregates::Location as LocationAggregate;
// NOTE: aggregates::NarrativeEvent is the new Rustic DDD version with private fields.
// entities::NarrativeEvent is the legacy version with public fields.
// During migration (Phase 4-6), both exist. After Phase 7, only aggregates::NarrativeEvent remains.
pub use aggregates::NarrativeEvent as NarrativeEventAggregate;
// NOTE: aggregates::PlayerCharacter is the new Rustic DDD version with private fields.
// entities::PlayerCharacter is the legacy version with public fields.
// During migration (Phase 4-6), both exist. After Phase 7, only aggregates::PlayerCharacter remains.
pub use aggregates::PlayerCharacter as PlayerCharacterAggregate;
// NOTE: aggregates::Scene is the new Rustic DDD version with private fields.
// entities::Scene is the legacy version with public fields.
// During migration (Phase 4-6), both exist. After Phase 7, only aggregates::Scene remains.
pub use aggregates::Scene as SceneAggregate;
// NOTE: aggregates::World is the new Rustic DDD version with private fields.
// entities::World is the legacy version with public fields.
// During migration (Phase 4-6), both exist. After Phase 7, only aggregates::World remains.
pub use aggregates::World as WorldAggregate;

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
    ActantialContext,
    ActantialLLMContext,
    ActantialTarget,
    ActivationEvaluation,
    ActivationLogic,
    ActivationRule,
    ActiveEventContext,
    ActorType,
    AdHocOutcomes,
    AppSettings,
    ApprovalType,
    ArchetypeChange,
    CampbellArchetype,
    ChallengeSignificance,
    ChangeAmount,
    // Validated name newtypes
    CharacterName,
    // Character lifecycle state enum
    CharacterState,
    GameTool,
    InfoImportance,
    ComfyUIConfig,
    ContextBudgetConfig,
    ContextCategory,
    ConversationEntry,
    Description,
    // Dialogue marker parsing
    DialogueMarker,
    DiceRollInput,
    DiceSystem,
    DirectorialNotes,
    DispositionLevel,
    // Expression configuration
    ExpressionConfig,
    LocationName,
    NarrativeEventName,
    MoodState,
    NpcDialogueContext,
    NpcDispositionState,
    PacingGuidance,
    // Dialogue marker types
    ParsedDialogue,
    PendingApprovalItem,
    PromptTemplateCategory,
    PromptTemplateMetadata,
    RegionFrequency,
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
    SceneName,
    SecretMotivationContext,
    SettingsFieldMetadata,
    SocialViewSummary,
    Speaker,
    StagingContext,
    StatDefinition,
    SuccessComparison,
    TokenCountMethod,
    TokenCounter,
    ToneGuidance,
    WantContext,
    WantTarget,
    WorldName,
};

// Re-export session types from types module
pub use types::WorldRole;

// Re-export decision types from types module (for DM approval workflows)
pub use types::{ChallengeOutcomeDecision, TimeSuggestionDecision};
