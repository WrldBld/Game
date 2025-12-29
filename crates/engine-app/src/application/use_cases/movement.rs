//! Movement Use Case
//!
//! Handles player character movement between regions and locations.
//! Integrates with the staging system for NPC presence approval.
//!
//! # Responsibilities
//!
//! - Validate PC exists and is in a valid state
//! - Check for locked region connections
//! - Update PC position in database
//! - Coordinate with staging system for NPC presence
//! - Build scene data for successful moves
//! - Broadcast events to DM and waiting players
//!
//! # Movement Flow
//!
//! ```text
//! MoveToRegion/ExitToLocation
//!         │
//!         ▼
//!   ┌─────────────┐
//!   │ Validate PC │
//!   └──────┬──────┘
//!          │
//!          ▼
//!   ┌─────────────────┐
//!   │ Check for locks │ ──blocked──> MovementBlocked
//!   └────────┬────────┘
//!            │
//!            ▼
//!   ┌────────────────┐
//!   │ Update position│
//!   └────────┬───────┘
//!            │
//!            ▼
//!   ┌────────────────────┐
//!   │ Check valid staging│──yes──> SceneChanged
//!   └────────┬───────────┘
//!            │ no
//!            ▼
//!   ┌──────────────────────┐
//!   │ Check pending staging│──yes──> Add to waiting, StagingPending
//!   └────────┬─────────────┘
//!            │ no
//!            ▼
//!   ┌───────────────────┐
//!   │ Generate proposal │
//!   │ Store pending     │
//!   │ Notify DM         │
//!   └────────┬──────────┘
//!            ▼
//!      StagingPending
//! ```

use std::sync::Arc;
use tracing::{debug, info, warn};

use async_trait::async_trait;
use wrldbldr_domain::entities::{Location, Region};
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId};
use wrldbldr_engine_ports::inbound::{MovementUseCasePort, UseCaseContext};
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, GameEvent, LocationRepositoryPort, PlayerCharacterRepositoryPort,
    RegionRepositoryPort, StagingPendingEvent, StagingRequiredEvent, WaitingPcData,
};

use super::builders::SceneBuilder;
use super::errors::MovementError;

// Import port traits from engine-ports
pub use wrldbldr_engine_ports::inbound::{StagingServicePort, StagingStatePort};

// Import types from engine-ports
pub use wrldbldr_engine_ports::outbound::{
    ExitToLocationInput, MoveToRegionInput, MovementResult, PendingStagingData,
    SelectCharacterInput, SelectCharacterResult, StagingProposalData,
};

// =============================================================================
// Movement Use Case
// =============================================================================

/// Use case for player character movement
///
/// Coordinates movement between regions and locations, integrating with
/// the staging system for NPC presence management.
pub struct MovementUseCase {
    pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
    region_repo: Arc<dyn RegionRepositoryPort>,
    location_repo: Arc<dyn LocationRepositoryPort>,
    staging_service: Arc<dyn StagingServicePort>,
    staging_state: Arc<dyn StagingStatePort>,
    broadcast: Arc<dyn BroadcastPort>,
    scene_builder: Arc<SceneBuilder>,
}

impl MovementUseCase {
    /// Create a new MovementUseCase with all dependencies
    pub fn new(
        pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
        region_repo: Arc<dyn RegionRepositoryPort>,
        location_repo: Arc<dyn LocationRepositoryPort>,
        staging_service: Arc<dyn StagingServicePort>,
        staging_state: Arc<dyn StagingStatePort>,
        broadcast: Arc<dyn BroadcastPort>,
        scene_builder: Arc<SceneBuilder>,
    ) -> Self {
        Self {
            pc_repo,
            region_repo,
            location_repo,
            staging_service,
            staging_state,
            broadcast,
            scene_builder,
        }
    }

    /// Select a player character for play
    ///
    /// Returns the PC's current position information.
    pub async fn select_character(
        &self,
        _ctx: UseCaseContext,
        input: SelectCharacterInput,
    ) -> Result<SelectCharacterResult, MovementError> {
        let pc = self
            .pc_repo
            .get(input.pc_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?
            .ok_or(MovementError::PcNotFound(input.pc_id))?;

        info!(
            pc_id = %input.pc_id,
            pc_name = %pc.name,
            "Player selected character"
        );

        Ok(SelectCharacterResult {
            pc_id: pc.id,
            pc_name: pc.name,
            location_id: pc.current_location_id,
            region_id: pc.current_region_id,
        })
    }

    /// Move a player character to a different region within the same location
    pub async fn move_to_region(
        &self,
        ctx: UseCaseContext,
        input: MoveToRegionInput,
    ) -> Result<MovementResult, MovementError> {
        // Get PC
        let pc = self
            .pc_repo
            .get(input.pc_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?
            .ok_or(MovementError::PcNotFound(input.pc_id))?;

        // Get target region
        let region = self
            .region_repo
            .get(input.target_region_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?
            .ok_or(MovementError::RegionNotFound(input.target_region_id))?;

        // Get location
        let location = self
            .location_repo
            .get(region.location_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?
            .ok_or(MovementError::LocationNotFound(region.location_id))?;

        // Check for locked connections if PC has a current region
        if let Some(current_region_id) = pc.current_region_id {
            if let Some(reason) = self
                .check_locked_connection(current_region_id, input.target_region_id)
                .await
            {
                return Ok(MovementResult::Blocked { reason });
            }
        }

        // Update PC position
        self.pc_repo
            .update_region(input.pc_id, input.target_region_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?;

        // Handle staging system
        self.handle_staging(
            ctx,
            input.pc_id,
            &pc.name,
            input.target_region_id,
            &region,
            &location,
        )
        .await
    }

    /// Move a player character to a different location
    pub async fn exit_to_location(
        &self,
        ctx: UseCaseContext,
        input: ExitToLocationInput,
    ) -> Result<MovementResult, MovementError> {
        // Get PC
        let pc = self
            .pc_repo
            .get(input.pc_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?
            .ok_or(MovementError::PcNotFound(input.pc_id))?;

        // Get target location
        let location = self
            .location_repo
            .get(input.target_location_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?
            .ok_or(MovementError::LocationNotFound(input.target_location_id))?;

        // Determine arrival region
        let arrival_region = self
            .determine_arrival_region(input.target_location_id, input.arrival_region_id)
            .await?;

        let arrival_region_id = arrival_region.id;

        // Update PC position (both location and region)
        self.pc_repo
            .update_position(
                input.pc_id,
                input.target_location_id,
                Some(arrival_region_id),
            )
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?;

        // Handle staging system
        self.handle_staging(
            ctx,
            input.pc_id,
            &pc.name,
            arrival_region_id,
            &arrival_region,
            &location,
        )
        .await
    }

    /// Check if a connection between regions is locked
    async fn check_locked_connection(
        &self,
        from_region_id: RegionId,
        to_region_id: RegionId,
    ) -> Option<String> {
        let connections = self
            .region_repo
            .get_connections(from_region_id)
            .await
            .ok()?;

        connections
            .iter()
            .find(|c| c.to_region == to_region_id && c.is_locked)
            .map(|c| {
                c.lock_description
                    .clone()
                    .unwrap_or_else(|| "The way is blocked".to_string())
            })
    }

    /// Determine the arrival region for a location exit
    async fn determine_arrival_region(
        &self,
        location_id: LocationId,
        specified_region_id: Option<RegionId>,
    ) -> Result<Region, MovementError> {
        // If a specific region was specified, verify it
        if let Some(region_id) = specified_region_id {
            let region = self
                .region_repo
                .get(region_id)
                .await
                .map_err(|e| MovementError::Database(e.to_string()))?
                .ok_or(MovementError::RegionNotFound(region_id))?;

            if region.location_id != location_id {
                return Err(MovementError::RegionLocationMismatch);
            }

            return Ok(region);
        }

        // Try location's default arrival region
        let location = self
            .location_repo
            .get(location_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?
            .ok_or(MovementError::LocationNotFound(location_id))?;

        if let Some(default_region_id) = location.default_region_id {
            if let Ok(Some(region)) = self.region_repo.get(default_region_id).await {
                return Ok(region);
            }
        }

        // Fall back to first spawn point
        let regions = self
            .location_repo
            .get_regions(location_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?;

        regions
            .into_iter()
            .find(|r| r.is_spawn_point)
            .ok_or(MovementError::NoArrivalRegion)
    }

    /// Handle the staging system for a region arrival
    async fn handle_staging(
        &self,
        ctx: UseCaseContext,
        pc_id: PlayerCharacterId,
        pc_name: &str,
        region_id: RegionId,
        region: &Region,
        location: &Location,
    ) -> Result<MovementResult, MovementError> {
        // Get game time
        let game_time = self
            .staging_state
            .get_game_time(&ctx.world_id)
            .unwrap_or_default();

        // Check for existing valid staging
        match self
            .staging_service
            .get_current_staging(region_id, &game_time)
            .await
        {
            Ok(Some(staged_npcs)) => {
                // Valid staging exists - build scene
                debug!(
                    region_id = %region_id,
                    "Using existing valid staging"
                );

                let scene_event = self
                    .scene_builder
                    .build_with_entities(pc_id, region, location, &staged_npcs)
                    .await;

                // Broadcast scene change to the player
                self.broadcast
                    .broadcast(
                        ctx.world_id,
                        GameEvent::SceneChanged {
                            user_id: ctx.user_id.clone(),
                            event: scene_event.clone(),
                        },
                    )
                    .await;

                return Ok(MovementResult::SceneChanged(scene_event));
            }
            Ok(None) => {
                debug!(
                    region_id = %region_id,
                    "No valid staging, checking for pending"
                );
            }
            Err(e) => {
                warn!(error = %e, "Failed to check staging, continuing without staging");
            }
        }

        // Check for pending staging approval
        if self
            .staging_state
            .has_pending_staging(&ctx.world_id, &region_id)
        {
            // Add PC to waiting list
            self.staging_state.add_waiting_pc(
                &ctx.world_id,
                &region_id,
                *pc_id.as_uuid(),
                pc_name.to_string(),
                ctx.user_id.clone(),
                String::new(), // client_id would come from handler layer
            );

            info!(
                pc_id = %pc_id,
                region_id = %region_id,
                "PC added to staging wait list"
            );

            // Notify the player they're waiting
            self.broadcast
                .broadcast(
                    ctx.world_id,
                    GameEvent::StagingPending {
                        user_id: ctx.user_id,
                        event: StagingPendingEvent {
                            region_id,
                            region_name: region.name.clone(),
                        },
                    },
                )
                .await;

            return Ok(MovementResult::StagingPending {
                region_id,
                region_name: region.name.clone(),
            });
        }

        // No staging exists - generate a proposal
        let ttl_hours = location.presence_cache_ttl_hours;

        let proposal = self
            .staging_service
            .generate_proposal(
                ctx.world_id,
                region_id,
                location.id,
                &location.name,
                &game_time,
                ttl_hours,
                None,
            )
            .await
            .map_err(|e| MovementError::Staging(e))?;

        // Create pending staging data
        let waiting_pc = WaitingPcData {
            pc_id,
            pc_name: pc_name.to_string(),
            user_id: ctx.user_id.clone(),
        };

        let pending = PendingStagingData {
            request_id: proposal.request_id.clone(),
            world_id: ctx.world_id,
            region_id,
            location_id: location.id,
            region_name: region.name.clone(),
            location_name: location.name.clone(),
            game_time: game_time.clone(),
            rule_based_npcs: proposal.rule_based_npcs.clone(),
            llm_based_npcs: proposal.llm_based_npcs.clone(),
            waiting_pcs: vec![waiting_pc.clone()],
            default_ttl_hours: ttl_hours,
        };

        // Store pending approval
        self.staging_state.store_pending_staging(pending);

        // Build staging required event for DM
        let staging_event = StagingRequiredEvent {
            request_id: proposal.request_id,
            region_id,
            region_name: region.name.clone(),
            location_id: location.id,
            location_name: location.name.clone(),
            game_time,
            rule_based_npcs: proposal.rule_based_npcs,
            llm_based_npcs: proposal.llm_based_npcs,
            waiting_pcs: vec![waiting_pc],
            previous_staging: None, // TODO: Add previous staging lookup
            default_ttl_hours: ttl_hours,
        };

        // Notify DM
        self.broadcast
            .broadcast(ctx.world_id, GameEvent::StagingRequired(staging_event))
            .await;

        // Notify player they're waiting
        self.broadcast
            .broadcast(
                ctx.world_id,
                GameEvent::StagingPending {
                    user_id: ctx.user_id,
                    event: StagingPendingEvent {
                        region_id,
                        region_name: region.name.clone(),
                    },
                },
            )
            .await;

        info!(
            pc_id = %pc_id,
            region_id = %region_id,
            "Staging approval requested from DM"
        );

        Ok(MovementResult::StagingPending {
            region_id,
            region_name: region.name.clone(),
        })
    }
}

// =============================================================================
// MovementUseCasePort Implementation
// =============================================================================

#[async_trait]
impl MovementUseCasePort for MovementUseCase {
    async fn select_character(
        &self,
        ctx: UseCaseContext,
        input: SelectCharacterInput,
    ) -> Result<SelectCharacterResult, MovementError> {
        self.select_character(ctx, input).await
    }

    async fn move_to_region(
        &self,
        ctx: UseCaseContext,
        input: MoveToRegionInput,
    ) -> Result<MovementResult, MovementError> {
        self.move_to_region(ctx, input).await
    }

    async fn exit_to_location(
        &self,
        ctx: UseCaseContext,
        input: ExitToLocationInput,
    ) -> Result<MovementResult, MovementError> {
        self.exit_to_location(ctx, input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests would use mocks for all the ports
    // Example structure:

    #[test]
    fn test_movement_result_variants() {
        // Test that result types can be constructed
        let scene_event = SceneChangedEvent {
            pc_id: PlayerCharacterId::from_uuid(Uuid::new_v4()),
            region: wrldbldr_engine_ports::outbound::RegionInfo {
                id: RegionId::from_uuid(Uuid::new_v4()),
                name: "Test Region".to_string(),
                location_id: LocationId::from_uuid(Uuid::new_v4()),
                location_name: "Test Location".to_string(),
                backdrop_asset: None,
                atmosphere: None,
                map_asset: None,
            },
            npcs_present: vec![],
            navigation: wrldbldr_engine_ports::outbound::NavigationInfo {
                connected_regions: vec![],
                exits: vec![],
            },
            region_items: vec![],
        };

        let _result = MovementResult::SceneChanged(scene_event);

        let _pending = MovementResult::StagingPending {
            region_id: RegionId::from_uuid(Uuid::new_v4()),
            region_name: "Test".to_string(),
        };

        let _blocked = MovementResult::Blocked {
            reason: "Door is locked".to_string(),
        };
    }
}
