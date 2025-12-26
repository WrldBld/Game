//! Value objects - Immutable objects defined by their attributes

mod actantial_context;
mod archetype;
mod comfyui_config;
mod context_budget;
mod dice;
mod directorial;
mod game_tools;
// IDs live in `wrldbldr-domain`
mod llm_context;
mod mood;
mod region;
mod relationship;
mod rule_system;
mod settings;
mod prompt_templates;
mod staging_context;


// Actantial model context for LLM consumption
pub use actantial_context::{
    ActantialActor, ActantialContext, ActantialLLMContext, ActantialTarget, ActorType,
    SecretMotivationContext, SocialViewSummary, WantContext, WantTarget,
};

// Engine-specific archetype with methods (protocol version is simpler wire format)
pub use archetype::{ArchetypeChange, CampbellArchetype};

pub use comfyui_config::ComfyUIConfig;
pub use context_budget::{
    ContextBudgetConfig, ContextCategory,
    TokenCountMethod, TokenCounter, count_tokens, exceeds_token_budget,
};
pub use dice::DiceRollInput;
pub use directorial::{DirectorialNotes};
pub use game_tools::{ChangeAmount, GameTool, InfoImportance, RelationshipChange};
pub use llm_context::{
    ActantialActorEntry, ActiveChallengeContext, ActiveNarrativeEventContext,
    CharacterContext, ConversationTurn, GamePromptRequest, MotivationEntry, MotivationsContext,
    PlayerActionContext, RegionItemContext, SceneContext, SecretMotivationEntry,
    SocialRelationEntry, SocialStanceContext,
};
pub use mood::{
    ChallengeSignificance, InteractionOutcome, MoodLevel, NpcMoodState, NpcMoodStateDto, RelationshipLevel,
};
pub use region::{RegionFrequency, RegionRelationship, RegionRelationshipType, RegionShift};
pub use relationship::{FamilyRelation, Relationship, RelationshipEvent, RelationshipType};
pub use rule_system::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition, SuccessComparison,
};
pub use settings::{AppSettings, SettingsFieldMetadata, settings_metadata};
pub use prompt_templates::{
    PromptTemplateCategory, PromptTemplateMetadata, all_keys as prompt_template_keys,
    defaults as prompt_defaults, get_default as get_prompt_default, key_to_env_var,
    keys as prompt_keys, prompt_template_metadata,
};
pub use staging_context::{
    ActiveEventContext, NpcDialogueContext, RollResult, RuleBasedSuggestion, StagingContext,
};

// NOTE: Want has been promoted to an entity (domain/entities/want.rs)
// ActantTarget is no longer used - targets are now Neo4j edges
