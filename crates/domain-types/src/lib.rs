//! # WrldBldr Domain Types
//!
//! Shared vocabulary types that form the innermost layer of the hexagonal architecture.
//! These types are used by both the domain layer and the protocol layer, serving as
//! the stable contract between them.
//!
//! ## Architecture Role
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ domain-types (THIS CRATE)               │  ← Innermost layer, zero internal deps
//! │   Pure vocabulary types                 │
//! └─────────────────────────────────────────┘
//!                     │
//!         ┌───────────┴───────────┐
//!         ▼                       ▼
//!    ┌─────────┐            ┌──────────┐
//!    │ domain  │            │ protocol │
//!    │ (uses)  │            │ (uses)   │
//!    └─────────┘            └──────────┘
//! ```
//!
//! ## Design Principles
//!
//! 1. **Zero internal crate dependencies** - Only serde and serde_json
//! 2. **Pure data types** - No I/O, no async, no side effects
//! 3. **Stable API** - Changes here affect both domain and protocol
//! 4. **Serializable** - All types derive Serialize/Deserialize

// Narrative types
mod monomyth;
pub use monomyth::MonomythStage;

mod archetype;
pub use archetype::CampbellArchetype;

// Disposition types
mod disposition;
pub use disposition::{DispositionLevel, RelationshipLevel};

// Generation types
mod batch_status;
pub use batch_status::BatchStatus;

mod asset_types;
pub use asset_types::{AssetType, ChangeType, EntityType};

// Rule system types
mod rule_system;
pub use rule_system::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition,
    SuccessComparison,
};

// Workflow types
mod workflow;
pub use workflow::{
    // Types
    InputDefault, InputType, PromptMapping, PromptMappingType, WorkflowAnalysis, WorkflowInput,
    WorkflowSlot,
    // Pure analysis functions
    analyze_workflow, auto_detect_prompt_mappings, find_nodes_by_type, validate_workflow,
};
