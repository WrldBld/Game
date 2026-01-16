//! Value objects - Immutable objects defined by their attributes

mod actantial_context;
mod activation_rules;
mod ad_hoc_outcomes;
mod archetype;
mod character_state;
mod comfyui_config;
mod dice;
mod directorial;
mod names;
// IDs live in `wrldbldr-domain`
mod dialogue_markers;
mod disposition;
mod expression_config;
mod quantity;
mod region;
mod relationship;
mod rule_system;
mod staging_context;
pub mod stat_block;
mod world_state;

// Activation rules for visual states
pub use activation_rules::{ActivationEvaluation, ActivationLogic, ActivationRule};

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
pub use dialogue_markers::{
    parse_dialogue, parse_dialogue_markers, validate_markers, DialogueMarker, ParsedDialogue,
};
pub use dice::{DiceFormula, DiceParseError, DiceRollInput, DiceRollResult};
pub use directorial::{
    DirectorialNotes, NpcMotivation as DomainNpcMotivation, PacingGuidance, ToneGuidance,
};
pub use disposition::{
    ChallengeSignificance, DispositionLevel, InteractionOutcome, MoodState, NpcDispositionState,
    RelationshipLevel,
};
pub use expression_config::ExpressionConfig;

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
pub use staging_context::{
    ActiveEventContext, NpcDialogueContext, RollResult, RuleBasedSuggestion, StagingContext,
};
pub use world_state::{ApprovalType, ConversationEntry, PendingApprovalItem, Speaker};

// Validated name newtypes
pub use names::{
    CharacterName, Description, LocationName, NarrativeEventName, SceneName, WorldName,
};

// Character lifecycle state enum
pub use character_state::CharacterState;

// Stat block value objects
pub use stat_block::{StatBlock, StatModifier, StatValue};

// NOTE: Want has been promoted to an entity (domain/entities/want.rs)
// ActantTarget is no longer used - targets are now Neo4j edges
