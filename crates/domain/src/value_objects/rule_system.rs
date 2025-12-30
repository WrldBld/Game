//! System-agnostic rule configuration
//!
//! Re-exports shared types from domain-types crate.

// Re-export all rule system types from the shared domain-types crate
pub use wrldbldr_domain_types::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition,
    SuccessComparison,
};
