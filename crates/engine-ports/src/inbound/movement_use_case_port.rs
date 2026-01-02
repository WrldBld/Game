//! Movement Use Case Port
//!
//! Defines the inbound port for movement operations. This trait abstracts
//! the `MovementUseCase` from `engine-app`, allowing adapters to call movement
//! operations without depending on the application layer implementation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        ADAPTER LAYER                                    │
//! │                                                                         │
//! │  movement_handler.rs                                                    │
//! │      │                                                                  │
//! │      └──> dyn MovementUseCasePort                                       │
//! │                                                                         │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                        PORTS LAYER                                      │
//! │                                                                         │
//! │  MovementUseCasePort trait (this file)                                  │
//! │                                                                         │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                     APPLICATION LAYER                                   │
//! │                                                                         │
//! │  MovementUseCase implements MovementUseCasePort                         │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Operations
//!
//! - **select_character**: Select a player character for play, returns position info
//! - **move_to_region**: Move within the same location to a different region
//! - **exit_to_location**: Move to a different location entirely

use async_trait::async_trait;

use super::UseCaseContext;
use crate::outbound::MovementError;
use crate::outbound::{
    ExitToLocationInput, MoveToRegionInput, MovementResult, SelectCharacterInput,
    SelectCharacterResult,
};

/// Port for player character movement operations
///
/// This port abstracts movement use case operations, allowing adapters to
/// invoke movement logic without depending on the application layer directly.
///
/// # Implementors
///
/// - `MovementUseCase` in `engine-app`
///
/// # Example
///
/// ```ignore
/// async fn handle_move(
///     movement_port: Arc<dyn MovementUseCasePort>,
///     ctx: UseCaseContext,
///     region_id: RegionId,
/// ) -> Result<MovementResult, MovementError> {
///     let input = MoveToRegionInput {
///         pc_id: ctx.require_pc()?,
///         target_region_id: region_id,
///     };
///     movement_port.move_to_region(ctx, input).await
/// }
/// ```
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait MovementUseCasePort: Send + Sync {
    /// Select a player character for play
    ///
    /// Returns the PC's current position information including location and region.
    /// This is typically called when a player joins a world and selects their character.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case execution context with world and user info
    /// * `input` - Contains the PC ID to select
    ///
    /// # Returns
    ///
    /// * `Ok(SelectCharacterResult)` - PC selected successfully with position info
    /// * `Err(MovementError::PcNotFound)` - The specified PC does not exist
    /// * `Err(MovementError::Database)` - Database operation failed
    async fn select_character(
        &self,
        ctx: UseCaseContext,
        input: SelectCharacterInput,
    ) -> Result<SelectCharacterResult, MovementError>;

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
