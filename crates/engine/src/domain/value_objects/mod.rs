//! Value objects - Immutable objects defined by their attributes

mod archetype;
mod comfyui_config;
mod context_budget;
mod dice;
mod directorial;
mod game_time;
mod game_tools;
mod ids;
mod llm_context;
mod region;
mod relationship;
mod rule_system;
mod settings;

// Re-export shared types from protocol crate
pub use wrldbldr_protocol::{
    // Approval types
    ApprovalDecision, ProposedToolInfo,
    // Challenge/navigation suggestion types
    ChallengeSuggestionInfo, NarrativeEventSuggestionInfo,
};

// Engine-specific game_time has richer implementation than protocol's simple wire format
pub use game_time::{GameTime, TimeOfDay};

// Engine-specific archetype with methods (protocol version is simpler wire format)
pub use archetype::{ArchetypeChange, CampbellArchetype};

pub use comfyui_config::ComfyUIConfig;
pub use context_budget::{
    AssembledContext, CategoryContext, ContextBudgetConfig, ContextCategory,
    TokenCountMethod, TokenCounter, count_tokens, exceeds_token_budget,
};
pub use dice::DiceRollInput;
pub use directorial::{DirectorialNotes};
pub use game_tools::{ChangeAmount, GameTool, InfoImportance, RelationshipChange};
pub use ids::*;
pub use llm_context::{
    ActiveChallengeContext, ActiveNarrativeEventContext, CharacterContext, ConversationTurn,
    GamePromptRequest, PlayerActionContext, SceneContext,
};
pub use region::{RegionFrequency, RegionRelationship, RegionRelationshipType, RegionShift};
pub use relationship::{FamilyRelation, Relationship, RelationshipEvent, RelationshipType};
pub use rule_system::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition, SuccessComparison,
};
pub use settings::{AppSettings, SettingsFieldMetadata, settings_metadata};

// NOTE: Want has been promoted to an entity (domain/entities/want.rs)
// ActantTarget is no longer used - targets are now Neo4j edges
