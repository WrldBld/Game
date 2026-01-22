//! System-agnostic rule configuration
//!
//! Re-exports shared types from the types module.
//!
//! # Tier Classification
//!
//! - **Tier 5: Complex Value Object** - Rule system types (e.g., `BladesPoolThresholds`,
//!   `DiceSystem`) have significant business logic for game mechanics.
//!
//! See [docs/architecture/tier-levels.md](../../../../docs/architecture/tier-levels.md)
//! for complete tier classification system.

// Re-export all rule system types from the types module
pub use crate::types::{
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
