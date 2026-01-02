//! Player Action Use Case Port
//!
//! Defines the inbound port for player action operations. This trait abstracts
//! the `PlayerActionUseCase` from `engine-app`, allowing adapters to handle
//! player actions without depending on the application layer implementation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        ADAPTER LAYER                                    │
//! │                                                                         │
//! │  player_action_handler.rs                                               │
//! │      │                                                                  │
//! │      └──> dyn PlayerActionUseCasePort                                   │
//! │                                                                         │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                        PORTS LAYER                                      │
//! │                                                                         │
//! │  PlayerActionUseCasePort trait (this file)                              │
//! │                                                                         │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                     APPLICATION LAYER                                   │
//! │                                                                         │
//! │  PlayerActionUseCase implements PlayerActionUseCasePort                 │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Operations
//!
//! - **handle_action**: Process a player action (travel, speak, interact, etc.)
//!   - Travel actions execute immediately
//!   - Non-travel actions are queued for DM/LLM processing

use async_trait::async_trait;

use super::UseCaseContext;
use crate::outbound::ActionError;
use crate::outbound::{ActionResult, PlayerActionInput};

/// Port for player action operations
///
/// This port abstracts player action use case operations, allowing adapters to
/// invoke action handling without depending on the application layer directly.
///
/// # Implementors
///
/// - `PlayerActionUseCase` in `engine-app`
///
/// # Example
///
/// ```ignore
/// async fn handle_player_action(
///     action_port: Arc<dyn PlayerActionUseCasePort>,
///     ctx: UseCaseContext,
///     action_type: String,
///     target: Option<String>,
/// ) -> Result<ActionResult, ActionError> {
///     let input = PlayerActionInput {
///         action_type,
///         target,
///         dialogue: None,
///     };
///     action_port.handle_action(ctx, input).await
/// }
/// ```
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait PlayerActionUseCasePort: Send + Sync {
    /// Handle a player action
    ///
    /// Processes player actions based on type:
    /// - Travel actions are executed immediately, updating PC location and resolving scenes
    /// - Non-travel actions (speak, interact, etc.) are queued for LLM/DM processing
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case execution context with world and user info
    /// * `input` - The player action input containing action type, target, and optional dialogue
    ///
    /// # Returns
    ///
    /// * `Ok(ActionResult::TravelCompleted)` - Travel completed, scene data returned
    /// * `Ok(ActionResult::TravelPending)` - Travel pending staging approval
    /// * `Ok(ActionResult::Queued)` - Action queued for processing, queue depth returned
    /// * `Err(ActionError::NoPcSelected)` - No player character selected for action
    /// * `Err(ActionError::MissingTarget)` - Travel action without target
    /// * `Err(ActionError::MovementFailed)` - Movement operation failed
    /// * `Err(ActionError::MovementBlocked)` - Movement blocked (locked door, etc.)
    /// * `Err(ActionError::QueueFailed)` - Failed to enqueue action
    async fn handle_action(
        &self,
        ctx: UseCaseContext,
        input: PlayerActionInput,
    ) -> Result<ActionResult, ActionError>;
}
