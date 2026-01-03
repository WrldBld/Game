//! Rule system types for wire serialization
//!
//! Re-exports shared vocabulary types from domain::types module.

// Re-export all rule system types from the domain::types module
pub use wrldbldr_domain::types::{
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
