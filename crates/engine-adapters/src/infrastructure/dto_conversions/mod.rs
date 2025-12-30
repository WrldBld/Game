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

// Asset conversions
pub use asset_conversions::{
    gallery_asset_ref_to_dto, gallery_asset_to_dto, generation_batch_ref_to_dto,
    generation_batch_to_dto,
};

// Disposition conversions
pub use disposition_conversions::npc_disposition_to_dto;

// Workflow conversions
pub use workflow_conversions::{
    workflow_analysis_to_dto, workflow_config_to_export_dto, workflow_config_to_full_response_dto,
    workflow_config_to_response_dto,
};
