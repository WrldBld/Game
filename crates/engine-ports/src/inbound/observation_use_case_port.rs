//! Observation Use Case Port
//!
//! Defines the inbound port for observation operations. This trait abstracts
//! the `ObservationUseCase` from `engine-app`, allowing adapters to handle
//! NPC observation and event triggering without depending on the application layer.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        ADAPTER LAYER                                    │
//! │                                                                         │
//! │  observation_handler.rs                                                 │
//! │      │                                                                  │
//! │      └──> dyn ObservationUseCasePort                                    │
//! │                                                                         │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                        PORTS LAYER                                      │
//! │                                                                         │
//! │  ObservationUseCasePort trait (this file)                               │
//! │                                                                         │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                     APPLICATION LAYER                                   │
//! │                                                                         │
//! │  ObservationUseCase implements ObservationUseCasePort                   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Operations
//!
//! - **share_npc_location**: DM shares NPC location info with a PC (creates "HeardAbout" observation)
//! - **trigger_approach_event**: DM makes an NPC approach a PC, optionally revealing identity
//! - **trigger_location_event**: DM broadcasts an event to all players in a region
//!
//! # DM-Only Operations
//!
//! All operations in this port are DM-only. They affect player perception of the
//! game world through the observation system.

use async_trait::async_trait;

use super::UseCaseContext;
use crate::outbound::ObservationError;
use crate::outbound::{
    ShareNpcLocationInput, ShareNpcLocationResult, TriggerApproachInput, TriggerApproachResult,
    TriggerLocationEventInput, TriggerLocationEventResult,
};

/// Port for observation operations
///
/// This port abstracts observation use case operations, allowing adapters to
/// invoke NPC observation and event triggering without depending on the
/// application layer directly.
///
/// # Implementors
///
/// - `ObservationUseCase` in `engine-app`
///
/// # Example
///
/// ```ignore
/// async fn share_location(
///     observation_port: Arc<dyn ObservationUseCasePort>,
///     ctx: UseCaseContext,
///     pc_id: PlayerCharacterId,
///     npc_id: CharacterId,
/// ) -> Result<ShareNpcLocationResult, ObservationError> {
///     let input = ShareNpcLocationInput {
///         pc_id,
///         npc_id,
///         location_id: some_location_id,
///         region_id: some_region_id,
///         notes: Some("The bartender mentioned seeing him".to_string()),
///     };
///     observation_port.share_npc_location(ctx, input).await
/// }
/// ```
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ObservationUseCasePort: Send + Sync {
    /// Share an NPC's location with a player character
    ///
    /// DM-only operation that creates a "HeardAbout" observation for the PC.
    /// This represents the PC learning about an NPC's whereabouts through
    /// rumors, information gathering, or DM narrative.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case execution context with world and user info (must be DM)
    /// * `input` - Contains PC ID, NPC ID, location/region, and optional notes
    ///
    /// # Returns
    ///
    /// * `Ok(ShareNpcLocationResult)` - Observation created successfully
    /// * `Err(ObservationError::Database)` - Not authorized (not DM) or DB error
    async fn share_npc_location(
        &self,
        ctx: UseCaseContext,
        input: ShareNpcLocationInput,
    ) -> Result<ShareNpcLocationResult, ObservationError>;

    /// Trigger an NPC approach event
    ///
    /// DM-only operation that makes an NPC approach a PC. Optionally reveals
    /// the NPC's identity. Creates a direct observation (or unrevealed observation)
    /// and sends the approach event to the target player.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case execution context with world and user info (must be DM)
    /// * `input` - Contains NPC ID, target PC ID, description, and reveal flag
    ///
    /// # Returns
    ///
    /// * `Ok(TriggerApproachResult)` - Approach event triggered, contains NPC and PC names
    /// * `Err(ObservationError::NpcNotFound)` - The specified NPC does not exist
    /// * `Err(ObservationError::PcNotFound)` - The target PC does not exist
    /// * `Err(ObservationError::Database)` - Not authorized (not DM) or DB error
    async fn trigger_approach_event(
        &self,
        ctx: UseCaseContext,
        input: TriggerApproachInput,
    ) -> Result<TriggerApproachResult, ObservationError>;

    /// Trigger a location-wide event
    ///
    /// DM-only operation that broadcasts an event to all players in a world.
    /// Players' clients filter by their current region to determine visibility.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case execution context with world and user info (must be DM)
    /// * `input` - Contains region ID and event description
    ///
    /// # Returns
    ///
    /// * `Ok(TriggerLocationEventResult)` - Event broadcast successfully
    /// * `Err(ObservationError::Database)` - Not authorized (not DM) or broadcast failed
    async fn trigger_location_event(
        &self,
        ctx: UseCaseContext,
        input: TriggerLocationEventInput,
    ) -> Result<TriggerLocationEventResult, ObservationError>;
}
