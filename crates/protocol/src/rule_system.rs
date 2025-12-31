//! Rule system types for wire serialization
//!
//! Re-exports shared vocabulary types from domain-types crate.

// Re-export all rule system types from the shared domain-types crate
pub use wrldbldr_domain_types::{
    // Core rule system types
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition,
    SuccessComparison,
    // Narrative resolution types
    BladesPoolThresholds, DifficultyDescriptor, DifficultyLadder, EffectLevel, EffectTickConfig,
    LadderEntry, NarrativeDiceConfig, NarrativeDiceType, NarrativeResolutionConfig,
    NarrativeResolutionStyle, NarrativeThresholds, Position, PositionEffectConfig,
};
