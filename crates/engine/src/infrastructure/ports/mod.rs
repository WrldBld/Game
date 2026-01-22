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
mod types;

// =============================================================================
// Error Types
// =============================================================================
pub use error::{
    ImageGenError, JoinWorldError, LlmError, QueueError, RepoError, SessionError,
};

// =============================================================================
// External Service Ports
// =============================================================================
pub use external::{
    ChatMessage, FinishReason, ImageData, ImageGenPort, ImageRequest, ImageResult,
    LlmPort, LlmRequest, LlmResponse, MessageRole, QueueItem, QueueItemData,
    QueueItemId, QueueItemStatus, QueuePort, TokenUsage,
};

// =============================================================================
// Repository Ports
// =============================================================================
pub use repos::{
    ActRepo, AssetRepo, ChallengeRepo, CharacterRepo, ContentRepo,
    FlagRepo, GoalRepo, InteractionRepo, ItemRepo, LocationRepo, LocationStateRepo,
    LoreRepo, NarrativeRepo, ObservationRepo, PlayerCharacterRepo, RegionStateRepo,
    SceneRepo, SettingsRepo, StagingRepo, WorldRepo,
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
    MockRegionStateRepo, MockSceneRepo, MockSettingsRepo, MockStagingRepo,
    MockWorldRepo,
};

#[cfg(test)]
pub use testing::MockClockPort;

// =============================================================================
// Testing Ports
// =============================================================================
pub use testing::{ClockPort, RandomPort};

// =============================================================================
// Port Types
// =============================================================================
pub use types::{
    ActantialViewRecord, ConnectedUserInfo, ConnectionInfo, ConversationTurnRecord,
    DirectorialContext, GoalDetails, NpcDispositionInfo, NpcMotivation,
    NpcRegionRelationType, NpcRegionRelationship, NpcWithRegionInfo,
    PendingStagingRequest, TimeSuggestion, UserJoinedInfo, WantDetails, WantTargetRef,
    WorldRole,
};
