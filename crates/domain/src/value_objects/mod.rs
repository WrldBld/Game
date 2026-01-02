//! Value objects - Immutable objects defined by their attributes

mod actantial_context;
mod ad_hoc_outcomes;
mod archetype;
mod comfyui_config;
mod context_budget;
mod dice;
mod directorial;
mod game_tools;
// IDs live in `wrldbldr-domain`
mod context_budget_enforcement;
mod disposition;
mod llm_context;
mod prompt_templates;
mod quantity;
mod queue_data;
mod region;
mod relationship;
mod rule_system;
mod settings;
mod staging_context;
mod world_state;

// Ad-hoc challenge outcomes
pub use ad_hoc_outcomes::AdHocOutcomes;

// Actantial model context for LLM consumption
pub use actantial_context::{
    ActantialActor, ActantialContext, ActantialLLMContext, ActantialTarget, ActorType,
    SecretMotivationContext, SocialViewSummary, WantContext, WantTarget,
};

// Engine-specific archetype with methods (protocol version is simpler wire format)
pub use archetype::{ArchetypeChange, CampbellArchetype};

pub use comfyui_config::ComfyUIConfig;
pub use context_budget::{
    count_tokens, exceeds_token_budget, ContextBudgetConfig, ContextCategory, TokenCountMethod,
    TokenCounter,
};
pub use context_budget_enforcement::{
    ContextBudgetEnforcer, ContextBuilder, EnforcementResult, EnforcementStats,
};
pub use dice::{DiceFormula, DiceParseError, DiceRollInput, DiceRollResult};
pub use directorial::{
    DirectorialNotes, NpcMotivation as DomainNpcMotivation, PacingGuidance, ToneGuidance,
};
pub use disposition::{
    ChallengeSignificance, DispositionLevel, InteractionOutcome, NpcDispositionState,
    RelationshipLevel,
};
pub use game_tools::{ChangeAmount, GameTool, InfoImportance, RelationshipChange};
pub use llm_context::{
    ActantialActorEntry, ActiveChallengeContext, ActiveNarrativeEventContext, CharacterContext,
    ConversationTurn, GamePromptRequest, MotivationEntry, MotivationsContext, PlayerActionContext,
    RegionItemContext, SceneContext, SecretMotivationEntry, SocialRelationEntry,
    SocialStanceContext,
};
pub use prompt_templates::{
    all_keys as prompt_template_keys, defaults as prompt_defaults,
    get_default as get_prompt_default, key_to_env_var, keys as prompt_keys,
    prompt_template_metadata, PromptTemplateCategory, PromptTemplateMetadata,
};
pub use quantity::QuantityChangeResult;
pub use region::{RegionFrequency, RegionRelationship, RegionRelationshipType, RegionShift};
pub use relationship::{FamilyRelation, Relationship, RelationshipEvent, RelationshipType};
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
pub use settings::{
    settings_metadata, AppSettings, BatchQueueFailurePolicy, SettingsFieldMetadata,
};
pub use staging_context::{
    ActiveEventContext, NpcDialogueContext, RollResult, RuleBasedSuggestion, StagingContext,
};
pub use world_state::{ApprovalType, ConversationEntry, PendingApprovalItem, Speaker};

// Queue data value objects (pure domain representations)
pub use queue_data::{
    ApprovalDecisionType, ApprovalRequestData, ApprovalUrgency, AssetGenerationData,
    ChallengeOutcomeData, ChallengeSuggestion, ChallengeSuggestionOutcomes, DmActionData,
    DmActionType, DmApprovalDecision, LlmRequestData, LlmRequestType, NarrativeEventSuggestion,
    PlayerActionData, ProposedTool, SuggestionContext,
};

// NOTE: Want has been promoted to an entity (domain/entities/want.rs)
// ActantTarget is no longer used - targets are now Neo4j edges
