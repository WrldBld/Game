//! Scene Use Case Port
//!
//! Inbound port trait for scene operations.
//!
//! # Responsibilities
//!
//! - Request scene changes
//! - Update directorial context (DM-only)
//! - Handle approval decisions (DM-only)
//!
//! # Architecture Note
//!
//! This port defines the interface for scene management operations.
//! Scene operations affect the narrative flow of the game - the directorial
//! context influences NPC behavior and narrative generation.
//!
//! Implementations should handle:
//! - Scene loading and relation resolution
//! - World state updates
//! - DM authorization checks
//! - Action queue management

use async_trait::async_trait;

#[cfg(feature = "testing")]
use mockall::automock;

use super::UseCaseContext;
use crate::outbound::{
    DirectorialUpdateResult, RequestSceneChangeInput, SceneApprovalDecisionInput,
    SceneApprovalDecisionResult, SceneChangeResult, UpdateDirectorialInput,
};

/// Error type for scene operations
///
/// Note: The actual error type is defined in engine-app to avoid
/// circular dependencies. Implementations should use their own
/// error type that can be converted to this trait's error.
pub type SceneUseCaseError = String;

// =============================================================================
// Scene Use Case Port
// =============================================================================

/// Port trait for scene use case operations
///
/// This trait defines the contract for scene management functionality.
/// It is implemented by the application layer's `SceneUseCase` and can be
/// mocked for testing.
///
/// # Authorization
///
/// - `request_scene_change`: Any connected player
/// - `update_directorial_context`: DM-only
/// - `handle_approval_decision`: DM-only
///
/// # Example
///
/// ```ignore
/// use wrldbldr_engine_ports::inbound::{SceneUseCasePort, UseCaseContext};
///
/// async fn change_scene(
///     use_case: &dyn SceneUseCasePort,
///     ctx: UseCaseContext,
///     scene_id: SceneId,
/// ) -> Result<(), SceneUseCaseError> {
///     let input = RequestSceneChangeInput { scene_id };
///     let result = use_case.request_scene_change(ctx, input).await?;
///     if result.scene_changed {
///         // Handle scene change
///     }
///     Ok(())
/// }
/// ```
#[cfg_attr(feature = "testing", automock)]
#[async_trait]
pub trait SceneUseCasePort: Send + Sync {
    /// Request a scene change
    ///
    /// Loads the scene with all relations (location, characters, interactions)
    /// and updates the world state.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context with world and user information
    /// * `input` - Scene change request containing the target scene ID
    ///
    /// # Returns
    ///
    /// * `Ok(SceneChangeResult)` - Scene data with characters and interactions
    /// * `Err(SceneUseCaseError)` - If scene not found or database error
    ///
    /// # Authorization
    ///
    /// Any connected player can request a scene change.
    async fn request_scene_change(
        &self,
        ctx: UseCaseContext,
        input: RequestSceneChangeInput,
    ) -> Result<SceneChangeResult, SceneUseCaseError>;

    /// Update directorial context
    ///
    /// Sets NPC motivations, scene mood, pacing, and DM notes for the
    /// current scene. This context influences AI-generated narrative
    /// and NPC behavior.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm = true`)
    /// * `input` - Directorial context update containing motivations and notes
    ///
    /// # Returns
    ///
    /// * `Ok(DirectorialUpdateResult)` - Confirmation of update
    /// * `Err(SceneUseCaseError)` - If not authorized or persistence fails
    ///
    /// # Authorization
    ///
    /// DM-only operation. Returns error if `ctx.is_dm` is false.
    async fn update_directorial_context(
        &self,
        ctx: UseCaseContext,
        input: UpdateDirectorialInput,
    ) -> Result<DirectorialUpdateResult, SceneUseCaseError>;

    /// Handle approval decision
    ///
    /// Processes a DM's decision on a pending scene-related approval request.
    /// The decision is enqueued to the DM action queue for processing.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm = true`)
    /// * `input` - Approval decision containing request ID and decision
    ///
    /// # Returns
    ///
    /// * `Ok(SceneApprovalDecisionResult)` - Confirmation that decision was processed
    /// * `Err(SceneUseCaseError)` - If not authorized or queue operation fails
    ///
    /// # Authorization
    ///
    /// DM-only operation. Returns error if `ctx.is_dm` is false.
    async fn handle_approval_decision(
        &self,
        ctx: UseCaseContext,
        input: SceneApprovalDecisionInput,
    ) -> Result<SceneApprovalDecisionResult, SceneUseCaseError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the trait is object-safe
    fn _assert_object_safe(_: &dyn SceneUseCasePort) {}
}
