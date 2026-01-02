//! Prompt context service port - Interface for building LLM prompt context
//!
//! This port abstracts the orchestration of gathering context for LLM prompts.
//! It serves as the interface for infrastructure adapters that need to build
//! game prompt requests from player actions.
//!
//! # Design Notes
//!
//! This port replaces the direct dependency on websocket_helpers by providing
//! a clean interface for prompt context building. The implementation lives in
//! the application layer (PromptContextService).

use async_trait::async_trait;

use wrldbldr_domain::value_objects::{GamePromptRequest, PlayerActionData};
use wrldbldr_domain::{PlayerCharacterId, SceneId, WorldId};

/// Error type for prompt context operations
#[derive(Debug, thiserror::Error)]
pub enum PromptContextError {
    /// World not found in the database
    #[error("World not found: {0}")]
    WorldNotFound(WorldId),

    /// Player character not found in the database
    #[error("Player character not found: {0}")]
    PlayerCharacterNotFound(PlayerCharacterId),

    /// Scene not found in the database
    #[error("Scene not found: {0}")]
    SceneNotFound(SceneId),

    /// Internal error during context building
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Port for building prompt context for LLM requests.
///
/// This trait provides methods for constructing the context needed to generate
/// LLM prompts from player actions. It abstracts away the complexity of gathering
/// scene information, character data, and other contextual elements.
///
/// # Usage
///
/// Infrastructure adapters (like WebSocket handlers) should depend on this trait
/// rather than importing prompt building logic directly, maintaining proper
/// hexagonal architecture boundaries.
#[async_trait]
pub trait PromptContextServicePort: Send + Sync {
    /// Build a complete prompt request for a player action.
    ///
    /// This method orchestrates gathering all the context needed to generate
    /// an LLM response, including:
    /// - Scene and location information
    /// - Character context (responding NPC)
    /// - Active challenges and narrative events
    /// - Conversation history
    /// - Directorial notes
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world containing the action
    /// * `action` - The player action data containing action type, target, dialogue, etc.
    ///
    /// # Returns
    ///
    /// A `GamePromptRequest` ready to be sent to the LLM service.
    async fn build_prompt_from_action(
        &self,
        world_id: WorldId,
        action: &PlayerActionData,
    ) -> Result<GamePromptRequest, PromptContextError>;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    /// Mock implementation of PromptContextServicePort for testing.
    pub PromptContextServicePort {}

    #[async_trait]
    impl PromptContextServicePort for PromptContextServicePort {
        async fn build_prompt_from_action(
            &self,
            world_id: WorldId,
            action: &PlayerActionData,
        ) -> Result<GamePromptRequest, PromptContextError>;
    }
}
