//! Rule system configuration types
//!
//! Re-exports domain types for wire serialization.
//! Domain types have serde derives, so they work directly in protocol.

// Re-export domain types (they have serde derives)
pub use wrldbldr_domain::value_objects::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition,
    SuccessComparison,
};
