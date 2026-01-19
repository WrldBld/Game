extern crate self as wrldbldr_domain;

// Merged from wrldbldr-domain-types crate
pub mod types;

// Merged from wrldbldr-common crate
pub mod common;

pub mod aggregates;
pub mod entities;
pub mod error;
pub mod events;
pub mod game_time;
pub mod ids;
pub mod value_objects;

// Re-export all entities (explicit list in entities/mod.rs)
pub use entities::{
    default_skills_for_variant,
    AbilityUses,
    AcquiredFeat,
    AcquisitionMethod,
    Act,
    ActantialRole,
    ActantialView,
    ActiveFeature,
    AssetType,
    BackgroundFeature,
    BatchStatus,
    CastingTime,
    CastingTimeUnit,
    ChainStatus,
    ChainedEvent,
    Challenge,
    ChallengeEventOutcome,
    ChallengeLocationAvailability,
    ChallengeOutcomes,
    ChallengePrerequisite,
    ChallengeRegionAvailability,
    ChallengeType,
    ChallengeUnlock,
    CharacterFeats,
    CharacterFeatures,
    CharacterIdentity,
    CharacterSpells,
    CharacterWant,
    ClassFeature,
    ClassLevel,
    CombatEventType,
    CombatOutcome,
    ConnectionType,
    // Content types for compendium system
    ContentSource,
    ContentType,
    Difficulty,
    DifficultyDescriptor,
    DmMarkerType,
    DurationUnit,
    EntityType,
    EventChain,
    EventChainMembership,
    EventEffect,
    EventOutcome,
    Feat,
    FeatBenefit,
    FeatureUses,
    FeaturedNpc,
    FlagScope,
    FrequencyLevel,
    GalleryAsset,
    GameFlag,
    GenerationBatch,
    GenerationMetadata,
    GenerationRequest,
    Goal,
    GridMap,
    InfoType,
    InteractionCondition,
    InteractionRequirement,
    InteractionTarget,
    InteractionTargetType,
    InteractionTemplate,
    InteractionType,
    InventoryItem,
    InvolvedCharacter,
    Item,
    ItemSource,
    KnownSpell,
    LocationConnection,
    LocationState,
    LocationStateSummary,
    LocationType,
    Lore,
    LoreCategory,
    LoreChunk,
    LoreDiscoverySource,
    LoreKnowledge,
    MapBounds,
    MarkerImportance,
    MaterialComponent,
    MonomythStage,
    NarrativeTrigger,
    NarrativeTriggerType,
    NpcObservation,
    NpcPresence,
    ObservationSummary,
    ObservationType,
    Outcome,
    OutcomeCondition,
    OutcomeTrigger,
    OutcomeType,
    Prerequisite,
    RacialTrait,
    RechargeType,
    Region,
    RegionConnection,
    RegionExit,
    RegionState,
    RegionStateSummary,
    ResolvedStateInfo,
    ResolvedVisualState,
    SceneCharacter,
    SceneCharacterRole,
    SceneCondition,
    Skill,
    SkillCategory,
    SourceType,
    Spell,
    SpellComponents,
    SpellDuration,
    SpellLevel,
    SpellRange,
    SpellSlotPool,
    StagedNpc,
    Staging,
    StagingSource,
    StoryEvent,
    StoryEventInfoImportance,
    StoryEventType,
    TimeAdvanceResult,
    TimeContext,
    TriggerCondition,
    TriggerContext,
    TriggerEvaluation,
    TriggerLogic,
    TriggerType,
    UsesFormula,
    VisualStateSource,
    Want,
    WantTargetType,
    WantVisibility,
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

// Re-export game time types
pub use game_time::{
    GameTime, GameTimeConfig, TimeAdvanceReason, TimeCostConfig, TimeFormat, TimeMode, TimeOfDay,
};

// Re-export ID types
pub use ids::{
    ActId, ActionId, ApprovalRequestId, AssetId, BatchId, ChallengeId, CharacterId, ConnectionId,
    ConversationId, EventChainId, EventId, GoalId, GridMapId, InteractionId, ItemId, LocationId,
    LocationStateId, LoreChunkId, LoreId, NarrativeEventId, ParticipantId, PlayerCharacterId,
    QueueItemId, RegionId, RegionStateId, RelationshipId, SceneId, SkillId, StagingId,
    StatModifierId, StoryEventId, TimeSuggestionId, UserId, WantId, WorkflowId, WorldId,
};

// Re-export value objects (explicit list in value_objects/mod.rs)
pub use value_objects::{
    // Functions
    calculate_calendar_date,
    parse_dialogue,
    parse_dialogue_markers,
    validate_markers,
    // Actantial model types
    ActantialActor,
    ActantialContext,
    ActantialLLMContext,
    ActantialTarget,
    // Activation rules
    ActivationEvaluation,
    ActivationLogic,
    ActivationRule,
    ActiveEventContext,
    ActorType,
    // Ad-hoc outcomes
    AdHocOutcomes,
    ApprovalType,
    // Archetype types
    ArchetypeChange,
    // Asset paths
    AssetPath,
    // Atmosphere
    Atmosphere,
    // Rule system types
    BladesPoolThresholds,
    // Calendar types
    CalendarDate,
    CalendarDefinition,
    CalendarId,
    // Archetype
    CampbellArchetype,
    // Names
    ChallengeName,
    // Disposition types
    ChallengeSignificance,
    CharacterName,
    // Character state
    CharacterState,
    ConversationEntry,
    Description,
    // Dice types
    DialogueMarker,
    DiceFormula,
    DiceParseError,
    DiceRollInput,
    DiceRollResult,
    DiceSystem,
    DifficultyLadder,
    // Directorial types
    DirectorialNotes,
    DispositionLevel,
    DomainNpcMotivation,
    EffectLevel,
    EffectTickConfig,
    // Calendar epoch config
    EpochConfig,
    // Calendar era definition
    EraDefinition,
    ExpressionConfig,
    // Relationship types
    FamilyRelation,
    GoalName,
    InteractionOutcome,
    // Calendar intercalary day
    IntercalaryDay,
    ItemName,
    LadderEntry,
    LocationName,
    // Calendar month definition
    MonthDefinition,
    MoodState,
    NarrativeDiceConfig,
    NarrativeDiceType,
    NarrativeEventName,
    NarrativeResolutionConfig,
    NarrativeResolutionStyle,
    NarrativeThresholds,
    NpcDialogueContext,
    NpcDispositionState,
    PacingGuidance,
    ParsedDialogue,
    PendingApprovalItem,
    Position,
    PositionEffectConfig,
    RegionFrequency,
    RegionName,
    RegionRelationship,
    RegionRelationshipType,
    RegionShift,
    Relationship,
    RelationshipEvent,
    RelationshipLevel,
    RelationshipType,
    RollResult,
    RuleBasedSuggestion,
    RuleSystemConfig,
    RuleSystemType,
    RuleSystemVariant,
    SceneName,
    // Calendar season
    Season,
    SecretMotivationContext,
    SocialViewSummary,
    Speaker,
    StagingContext,
    // Stat enum for skill checks
    Stat,
    // Stat block
    StatBlock,
    StatDefinition,
    StatModifier,
    StatValue,
    StateName,
    SuccessComparison,
    Tag,
    ToneGuidance,
    WantContext,
    WantTarget,
    WorldName,
};

// Re-export session types from types module
pub use types::WorldRole;

// Re-export decision types from types module (for DM approval workflows)
pub use types::{ChallengeOutcomeDecision, TimeSuggestionDecision};
