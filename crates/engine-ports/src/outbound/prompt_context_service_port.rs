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

use wrldbldr_domain::value_objects::GamePromptRequest;
use wrldbldr_domain::{CharacterId, PlayerCharacterId, RegionId, SceneId, WorldId};

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
    /// * `pc_id` - The player character performing the action
    /// * `action_type` - Type of action (e.g., "speak", "examine", "use_item")
    /// * `target` - Optional target of the action (NPC name, object, etc.)
    /// * `dialogue` - Optional dialogue content if the action is speech
    /// * `region_id` - Optional region ID for location context
    ///
    /// # Returns
    ///
    /// A `GamePromptRequest` ready to be sent to the LLM service.
    async fn build_prompt_from_action(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
        region_id: Option<RegionId>,
    ) -> Result<GamePromptRequest, PromptContextError>;

    /// Find which NPC should respond to the player.
    ///
    /// Given a scene and an optional target, determines which character
    /// should respond to the player's action. If a target is specified,
    /// attempts to match it to a character in the scene. Otherwise,
    /// selects an appropriate responding character.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world containing the scene
    /// * `scene_id` - The current scene
    /// * `target` - Optional target name to match
    ///
    /// # Returns
    ///
    /// The ID of the responding character, or `None` if no suitable
    /// character is found.
    async fn find_responding_character(
        &self,
        world_id: WorldId,
        scene_id: SceneId,
        target: Option<String>,
    ) -> Result<Option<CharacterId>, PromptContextError>;
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
            pc_id: PlayerCharacterId,
            action_type: String,
            target: Option<String>,
            dialogue: Option<String>,
            region_id: Option<RegionId>,
        ) -> Result<GamePromptRequest, PromptContextError>;

        async fn find_responding_character(
            &self,
            world_id: WorldId,
            scene_id: SceneId,
            target: Option<String>,
        ) -> Result<Option<CharacterId>, PromptContextError>;
    }
}
