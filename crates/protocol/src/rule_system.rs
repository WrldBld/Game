//! Rule system configuration types
//!
//! Re-exports domain types for wire serialization.
//! Domain types have serde derives, so they work directly in protocol.

// ARCHITECTURE EXCEPTION: [APPROVED 2025-12-28]
// Re-exports stable domain types for wire serialization. Domain remains
// the canonical source. These types have serde derives and are used
// unchanged in protocol messages.
// See: docs/architecture/hexagonal-architecture.md
pub use wrldbldr_domain::value_objects::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition,
    SuccessComparison,
};
