//! Data transfer objects
//!
//! DTOs are used to transfer data between layers. The application layer
//! provides these types so that presentation doesn't need to import
//! directly from infrastructure.
//!
//! TODO (Phase 16.3): replace infra re-exports with real application DTOs + conversions.

pub mod session_dto;
pub mod websocket_messages;
pub mod world_snapshot;
pub mod settings;

// Re-export session DTOs
pub use session_dto::AppConnectionStatus;

// Re-export WebSocket protocol DTOs (application-owned).
pub use websocket_messages::*;

// Re-export Engine snapshot contracts (application-owned).
pub use world_snapshot::{
    // Rule system types
    RuleSystemConfig, RuleSystemPresetDetails, RuleSystemType, RuleSystemVariant,
    StatDefinition, DiceSystem, SuccessComparison,
    // Skill types
    SkillData, SkillCategory,
    // Character sheet types
    SheetTemplate, SheetSection, SheetField, SectionLayout,
    FieldType, FieldValue,
    // Challenge types
    ChallengeData, ChallengeType, ChallengeDifficulty,
    ChallengeOutcomes, Outcome,
    // Story arc types
    StoryEventData, StoryEventTypeData,
    NarrativeEventData, CreateNarrativeEventRequest,
    // Session snapshot types (simplified format from Engine)
    SessionWorldSnapshot,
    // Inventory types (Phase 23B)
    ItemData, InventoryItemData,
};

// Re-export settings DTOs
pub use settings::{AppSettings, ContextBudgetConfig, SettingsFieldMetadata, SettingsMetadataResponse};

// NOTE: Infrastructure asset loader now depends inward on these DTOs.
