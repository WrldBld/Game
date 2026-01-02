//! Staging Use Case Port
//!
//! Defines the inbound port trait for staging approval operations.
//! This trait abstracts the `StagingApprovalUseCase` implementation,
//! allowing adapters (e.g., WebSocket handlers) to interact with
//! staging functionality without direct coupling to the application layer.
//!
//! # Operations
//!
//! - **approve**: DM approves a staging proposal with selected NPCs
//! - **regenerate**: Request new LLM suggestions with DM guidance
//! - **pre_stage**: Proactively stage a region before player arrival
//!
//! # Error Handling
//!
//! All methods return `Result<T, StagingError>` where `StagingError`
//! provides meaningful error codes via the `ErrorCode` trait.

use async_trait::async_trait;
#[cfg(any(test, feature = "testing"))]
use mockall::automock;

use super::UseCaseContext;
use crate::outbound::StagingError;
use crate::outbound::{
    ApproveInput, ApproveResult, PreStageInput, PreStageResult, RegenerateInput,
    StagingRegenerateResult as RegenerateResult,
};

// =============================================================================
// Staging Use Case Port
// =============================================================================

/// Inbound port for staging approval use case operations
///
/// This trait defines the contract for staging approval operations that
/// can be invoked by adapters (WebSocket handlers, REST endpoints, etc.).
///
/// # Implementations
///
/// The primary implementation is `StagingApprovalUseCase` in the engine-app crate,
/// which coordinates staging state, NPC staging services, and broadcast notifications.
///
/// # Example
///
/// ```ignore
/// use wrldbldr_engine_ports::inbound::{StagingUseCasePort, UseCaseContext};
///
/// async fn handle_approve(
///     use_case: &dyn StagingUseCasePort,
///     ctx: UseCaseContext,
///     input: ApproveInput,
/// ) -> Result<ApproveResult, StagingError> {
///     use_case.approve(ctx, input).await
/// }
/// ```
#[cfg_attr(any(test, feature = "testing"), automock)]
#[async_trait]
pub trait StagingUseCasePort: Send + Sync {
    /// Approve a staging proposal
    ///
    /// DM approves a staging proposal with their chosen NPCs.
    /// This will:
    /// 1. Validate the pending staging exists
    /// 2. Build approved NPC data with character info
    /// 3. Persist the staging to the database
    /// 4. Broadcast `StagingReady` event to DMs
    /// 5. Send `SceneChanged` events to all waiting PCs
    /// 6. Remove the pending staging from state
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context containing world_id and user_id
    /// * `input` - Approval input containing request_id, approved NPCs, TTL, and source
    ///
    /// # Returns
    ///
    /// * `Ok(ApproveResult)` - Contains NPCs now present and count of notified PCs
    /// * `Err(StagingError::PendingNotFound)` - If the request_id doesn't match a pending staging
    /// * `Err(StagingError::ApprovalFailed)` - If the staging service fails to persist
    async fn approve(
        &self,
        ctx: UseCaseContext,
        input: ApproveInput,
    ) -> Result<ApproveResult, StagingError>;

    /// Regenerate LLM suggestions with DM guidance
    ///
    /// Requests new NPC staging suggestions from the LLM with optional
    /// DM guidance to steer the generation.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context containing world_id and user_id
    /// * `input` - Regenerate input containing request_id and guidance text
    ///
    /// # Returns
    ///
    /// * `Ok(RegenerateResult)` - Contains new LLM-based NPC suggestions
    /// * `Err(StagingError::PendingNotFound)` - If the request_id doesn't match a pending staging
    /// * `Err(StagingError::RegenerationFailed)` - If the LLM service fails
    async fn regenerate(
        &self,
        ctx: UseCaseContext,
        input: RegenerateInput,
    ) -> Result<RegenerateResult, StagingError>;

    /// Pre-stage a region before player arrival
    ///
    /// Proactively stages NPCs in a region before any player arrives.
    /// This is useful for DMs who want to set up locations in advance.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context containing world_id and user_id
    /// * `input` - Pre-stage input containing region_id, NPCs to stage, and TTL
    ///
    /// # Returns
    ///
    /// * `Ok(PreStageResult)` - Contains NPCs now present in the region
    /// * `Err(StagingError::RegionNotFound)` - If the region doesn't exist
    /// * `Err(StagingError::PreStagingFailed)` - If the staging service fails
    async fn pre_stage(
        &self,
        ctx: UseCaseContext,
        input: PreStageInput,
    ) -> Result<PreStageResult, StagingError>;
}
