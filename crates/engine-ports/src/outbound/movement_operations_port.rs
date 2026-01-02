//! Movement Operations Port
//!
//! Defines the outbound port for movement operations that use cases can depend on.
//! This trait provides the movement functionality needed by other use cases (like
//! `PlayerActionUseCase`) without creating an architectural violation where use cases
//! depend on inbound ports.
//!
//! # Architecture
//!
//! Per hexagonal-architecture.md:543:
//! > "Use cases MUST depend only on outbound ports, never on inbound ports."
//!
//! The inbound `MovementUseCasePort` is called by adapters/handlers. This outbound
//! `MovementOperationsPort` is depended upon by other use cases. `MovementUseCase`
//! implements both traits.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     APPLICATION LAYER                                   │
//! │                                                                         │
//! │  PlayerActionUseCase ──depends on──> dyn MovementOperationsPort         │
//! │                                            ▲                            │
//! │                                            │ implements                 │
//! │                                            │                            │
//! │                                      MovementUseCase                    │
//! │                                            │ implements                 │
//! │                                            ▼                            │
//! │  movement_handler.rs <──calls──── dyn MovementUseCasePort (inbound)     │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Operations
//!
//! - **move_to_region**: Move within the same location to a different region
//! - **exit_to_location**: Move to a different location entirely

use async_trait::async_trait;

use crate::inbound::UseCaseContext;
use crate::outbound::{ExitToLocationInput, MoveToRegionInput, MovementError, MovementResult};

/// Outbound port for movement operations
///
/// This port allows use cases to invoke movement functionality without depending
/// on the inbound `MovementUseCasePort`. The `MovementUseCase` implements both
/// this outbound port and the inbound `MovementUseCasePort`.
///
/// # Implementors
///
/// - `MovementUseCase` in `engine-app`
///
/// # Example
///
/// ```ignore
/// // In PlayerActionUseCase
/// async fn execute_move_action(
///     &self,
///     ctx: UseCaseContext,
///     target_region_id: RegionId,
/// ) -> Result<MovementResult, MovementError> {
///     let input = MoveToRegionInput {
///         pc_id: ctx.require_pc()?,
///         target_region_id,
///     };
///     self.movement_ops.move_to_region(ctx, input).await
/// }
/// ```
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait MovementOperationsPort: Send + Sync {
    /// Move a player character to a different region within the same location
    ///
    /// Validates the movement, checks for locked connections, updates the PC's
    /// position, and coordinates with the staging system for NPC presence.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case execution context with world and user info
    /// * `input` - Contains PC ID and target region ID
    ///
    /// # Returns
    ///
    /// * `Ok(MovementResult::SceneChanged)` - Movement succeeded, scene data returned
    /// * `Ok(MovementResult::StagingPending)` - Movement pending staging approval
    /// * `Ok(MovementResult::Blocked)` - Movement blocked (locked connection)
    /// * `Err(MovementError::PcNotFound)` - The PC does not exist
    /// * `Err(MovementError::RegionNotFound)` - Target region does not exist
    /// * `Err(MovementError::Database)` - Database operation failed
    async fn move_to_region(
        &self,
        ctx: UseCaseContext,
        input: MoveToRegionInput,
    ) -> Result<MovementResult, MovementError>;

    /// Move a player character to a different location
    ///
    /// Validates the movement, determines the arrival region (specified, default,
    /// or spawn point), updates the PC's position, and coordinates with staging.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case execution context with world and user info
    /// * `input` - Contains PC ID, target location ID, and optional arrival region
    ///
    /// # Returns
    ///
    /// * `Ok(MovementResult::SceneChanged)` - Movement succeeded, scene data returned
    /// * `Ok(MovementResult::StagingPending)` - Movement pending staging approval
    /// * `Ok(MovementResult::Blocked)` - Movement blocked (locked connection)
    /// * `Err(MovementError::PcNotFound)` - The PC does not exist
    /// * `Err(MovementError::LocationNotFound)` - Target location does not exist
    /// * `Err(MovementError::NoArrivalRegion)` - No valid arrival region found
    /// * `Err(MovementError::RegionLocationMismatch)` - Specified region not in target location
    /// * `Err(MovementError::Database)` - Database operation failed
    async fn exit_to_location(
        &self,
        ctx: UseCaseContext,
        input: ExitToLocationInput,
    ) -> Result<MovementResult, MovementError>;
}
