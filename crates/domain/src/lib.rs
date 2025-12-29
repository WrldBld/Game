extern crate self as wrldbldr_domain;

pub mod aggregates;
pub mod entities;
pub mod error;
pub mod events;
pub mod game_time;
pub mod ids;
pub mod value_objects;

// Re-export all entities (explicit list in entities/mod.rs)
pub use entities::{
    Challenge, ChallengeLocationAvailability, ChallengeOutcomes, ChallengePrerequisite,
    ChallengeRegionAvailability, ChallengeType, ChallengeUnlock, Difficulty, DifficultyDescriptor,
    Outcome, OutcomeType, OutcomeTrigger, TriggerCondition, TriggerType,
    Character, StatBlock,
    ChainStatus, EventChain,
    AssetType, EntityType, GalleryAsset, GenerationMetadata,
    FlagScope, GameFlag,
    BatchStatus, GenerationBatch, GenerationRequest,
    Goal,
    GridMap,
    InteractionCondition, InteractionRequirement, InteractionTarget, InteractionTargetType,
    InteractionTemplate, InteractionType,
    AcquisitionMethod, FrequencyLevel, InventoryItem, Item,
    Location, LocationConnection, LocationType,
    MapBounds, Region, RegionConnection, RegionExit,
    ChainedEvent, EventChainMembership, EventEffect, EventOutcome, FeaturedNpc, NarrativeEvent,
    NarrativeTrigger, NarrativeTriggerType, OutcomeCondition, TriggerContext, TriggerEvaluation,
    TriggerLogic,
    NpcObservation, ObservationSummary, ObservationType,
    PlayerCharacter,
    Scene, SceneCharacter, SceneCharacterRole, SceneCondition, TimeContext,
    CharacterSheetData, CharacterSheetTemplate, FieldType, FieldValue, ItemListType,
    SectionLayout, SelectOption, SheetField, SheetSection, SheetTemplateId,
    default_skills_for_variant, Skill, SkillCategory,
    StagedNpc, Staging, StagingSource,
    ChallengeEventOutcome, CombatEventType, CombatOutcome, DmMarkerType, InfoType, InvolvedCharacter,
    ItemSource, MarkerImportance, StoryEvent, StoryEventType, StoryEventInfoImportance,
    ActantialRole, ActantialView, CharacterWant, Want, WantTargetType, WantVisibility,
    InputDefault, InputType, PromptMapping, PromptMappingType, WorkflowAnalysis,
    WorkflowConfiguration, WorkflowInput, WorkflowSlot,
    Act, MonomythStage, World,
};

pub use error::DomainError;
pub use events::DomainEvent;

// Re-export game time types
pub use game_time::{GameTime, TimeOfDay};

// Re-export ID types
pub use ids::{
    WorldId, ActId, SceneId, LocationId, RegionId, CharacterId, PlayerCharacterId,
    ItemId, WantId, GoalId, RelationshipId, SkillId, ChallengeId,
    EventId, StoryEventId, NarrativeEventId, EventChainId,
    ParticipantId, UserId, ActionId, AssetId, BatchId, WorkflowConfigId,
    InteractionId, QueueItemId, GridMapId, StagingId, WorkflowId,
};

// Re-export value objects (explicit list in value_objects/mod.rs)
pub use value_objects::{
    ActantialActor, ActantialContext, ActantialLLMContext, ActantialTarget, ActorType,
    SecretMotivationContext, SocialViewSummary, WantContext, WantTarget,
    ArchetypeChange, CampbellArchetype,
    ComfyUIConfig,
    ContextBudgetConfig, ContextCategory, TokenCountMethod, TokenCounter, count_tokens, exceeds_token_budget,
    DiceRollInput,
    DirectorialNotes, DomainNpcMotivation, PacingGuidance, ToneGuidance,
    ChangeAmount, GameTool, InfoImportance, RelationshipChange,
    ActantialActorEntry, ActiveChallengeContext, ActiveNarrativeEventContext,
    CharacterContext, ConversationTurn, GamePromptRequest, MotivationEntry, MotivationsContext,
    PlayerActionContext, RegionItemContext, SceneContext, SecretMotivationEntry,
    SocialRelationEntry, SocialStanceContext,
    ChallengeSignificance, DispositionLevel, InteractionOutcome, NpcDispositionState, RelationshipLevel,
    RegionFrequency, RegionRelationship, RegionRelationshipType, RegionShift,
    FamilyRelation, Relationship, RelationshipEvent, RelationshipType,
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition, SuccessComparison,
    AppSettings, SettingsFieldMetadata, settings_metadata,
    PromptTemplateCategory, PromptTemplateMetadata, prompt_template_keys,
    prompt_defaults, get_prompt_default, key_to_env_var, prompt_keys, prompt_template_metadata,
    ActiveEventContext, NpcDialogueContext, RollResult, RuleBasedSuggestion, StagingContext,
    ApprovalType, ConversationEntry, PendingApprovalItem, Speaker,
};
