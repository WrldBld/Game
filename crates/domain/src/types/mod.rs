//! # WrldBldr Domain Types
//!
//! Shared vocabulary types that form the innermost layer of the hexagonal architecture.
//! These types are used by both the domain layer and the protocol layer, serving as
//! the stable contract between them.
//!
//! ## Design Principles
//!
//! 1. **Pure data types** - No I/O, no async, no side effects
//! 2. **Stable API** - Changes here affect both domain and protocol
//! 3. **Serializable** - All types derive Serialize/Deserialize
//!
//! Note: This module was previously the separate `wrldbldr-domain-types` crate.

// Narrative types
mod monomyth;
pub use monomyth::MonomythStage;

mod archetype;
pub use archetype::CampbellArchetype;

// Disposition types
mod disposition;
pub use disposition::{DispositionLevel, RelationshipLevel};

// Mood types (three-tier emotional model)
mod mood;
pub use mood::MoodState;

// Generation types
mod batch_status;
pub use batch_status::BatchStatus;

mod asset_types;
pub use asset_types::{AssetType, ChangeType, EntityType};

// Rule system types
mod rule_system;
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

// Session types
mod session;
pub use session::WorldRole;

// Decision types (for DM approval workflows)
mod time_decisions;
pub use time_decisions::TimeSuggestionDecision;

mod challenge_decisions;
pub use challenge_decisions::ChallengeOutcomeDecision;

// Character sheet types
pub mod character_sheet;
pub use character_sheet::{CharacterSheetValues, SheetValue};
