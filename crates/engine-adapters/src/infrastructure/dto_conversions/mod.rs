//! DTO conversion functions for domain types.
//!
//! These functions translate between domain entities and protocol DTOs.
//! Located in engine-adapters to maintain hexagonal architecture boundaries.
//!
//! # Why functions instead of From trait impls?
//!
//! Rust's orphan rule prevents implementing `From<DomainType> for DtoType`
//! in this crate since neither type is defined here. Using functions:
//! - Avoids orphan rule issues
//! - Makes conversions explicit at call sites
//! - Follows the adapters layer's responsibility of translating between layers

mod asset_conversions;
mod disposition_conversions;
mod workflow_conversions;

pub use asset_conversions::*;
pub use disposition_conversions::*;
pub use workflow_conversions::*;
