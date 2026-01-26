// Port traits define the full contract - many methods are for future use
#![allow(dead_code)]

//! Port traits for infrastructure boundaries.
//!
//! These are the ONLY abstractions in the engine. Everything else is concrete types.
//! Ports exist for:
//! - Database access (could swap Neo4j -> Postgres)
//! - LLM calls (could swap Ollama -> Claude/OpenAI)
//! - Image generation (could swap ComfyUI -> other)
//! - Queues (could swap SQLite -> Redis)
//! - Clock/Random (for testing)

mod error;
mod external;
mod repos;
mod testing;
pub mod types; // Make types module public

// =============================================================================
// Repository Ports
// =============================================================================
pub use repos::*;

// =============================================================================
// Types from types module (re-export for visibility)
// =============================================================================
pub use types::{
    // Session/Connection Types
    ConnectionInfo, DirectorialContext, NpcMotivation,
    // Session Result Types
    ConnectedUserInfo, UserJoinedInfo,
    // NPC disposition
    NpcDispositionInfo,
    // Infrastructure Types
    NpcRegionRelationship, NpcRegionRelationType, NpcWithRegionInfo,
    // Actantial Model Types
    WantDetails, GoalDetails, WantTargetRef, ActantialViewRecord,
    // Dialogue/Conversation Types
    ConversationTurnRecord,
    // Staging Storage Data Types
    PendingStagingRequest,
    // Time Suggestion Data Types
    TimeSuggestion,
    // Conversation Management Types (for DM monitoring)
    ActiveConversationRecord, ConversationLocationContext, ConversationSceneContext,
    ConversationDetails, ConversationParticipantDetail, DialogueTurnDetail, ParticipantType,
    // WorldRole is re-exported from wrldbldr_domain via types module
    WorldRole,
};

// =============================================================================
// External Service Ports
// =============================================================================
pub use external::{
    ChatMessage, FinishReason, ImageGenPort, ImageRequest, ImageResult,
    LlmPort, LlmRequest, LlmResponse, MessageRole, QueueItem, QueueItemData,
    QueueItemStatus, QueuePort, TokenUsage,
};

// =============================================================================
// Test-Only Mock Repositories (only available during test builds)
// =============================================================================
#[cfg(test)]
pub use repos::{
    MockActRepo, MockAssetRepo, MockChallengeRepo, MockCharacterRepo,
    MockContentRepo, MockFlagRepo, MockGoalRepo, MockInteractionRepo,
    MockItemRepo, MockLocationRepo, MockLocationStateRepo, MockLoreRepo,
    MockNarrativeRepo, MockObservationRepo, MockPlayerCharacterRepo,
    MockPromptTemplateRepo, MockRegionStateRepo, MockSceneRepo,
    MockSettingsRepo, MockStagingRepo, MockWorldRepo,
};

#[cfg(test)]
pub use testing::MockClockPort;

// =============================================================================
// Testing Ports
// =============================================================================
pub use testing::{ClockPort, RandomPort};

// =============================================================================
// Error Types
// =============================================================================
pub use error::{
    ImageGenError, JoinWorldError, LlmError, QueueError, RepoError, SessionError,
};
