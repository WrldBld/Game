//! Observation Use Case
//!
//! Handles NPC observation and event triggering operations.
//! Extracts from misc.rs handlers: share_npc_location, trigger_approach_event,
//! trigger_location_event.
//!
//! # Responsibilities
//!
//! - Share NPC location with a PC (DM creates "HeardAbout" observation)
//! - Trigger NPC approach events (DM makes NPC approach a PC)
//! - Trigger location-wide events (DM broadcasts event to region)
//!
//! # Architecture Note
//!
//! These are DM-only operations that affect player perception of the game world.
//! The observation system tracks what PCs know about NPCs.

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn};

use wrldbldr_domain::entities::NpcObservation;
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, CharacterRepositoryPort, ClockPort, ObservationRepositoryPort,
    PlayerCharacterRepositoryPort,
};

use super::errors::ObservationError;

// Import port traits from engine-ports
pub use wrldbldr_engine_ports::inbound::{ObservationUseCasePort, WorldMessagePort};

// Re-export types from engine-ports for backwards compatibility
pub use wrldbldr_engine_ports::outbound::{
    ApproachEventData, LocationEventData, ShareNpcLocationInput, ShareNpcLocationResult,
    TriggerApproachInput, TriggerApproachResult, TriggerLocationEventInput,
    TriggerLocationEventResult,
};

// =============================================================================
// Observation Use Case
// =============================================================================

/// Use case for observation operations
///
/// Handles NPC observation tracking and event triggering.
pub struct ObservationUseCase {
    pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
    character_repo: Arc<dyn CharacterRepositoryPort>,
    observation_repo: Arc<dyn ObservationRepositoryPort>,
    message_port: Arc<dyn WorldMessagePort>,
    broadcast: Arc<dyn BroadcastPort>,
    /// Clock for time operations (required for testability)
    clock: Arc<dyn ClockPort>,
}

impl ObservationUseCase {
    /// Create a new ObservationUseCase with all dependencies
    ///
    /// # Arguments
    /// * `clock` - Clock for time operations. Use `SystemClock` in production,
    ///             `MockClockPort` in tests for deterministic behavior.
    pub fn new(
        pc_repo: Arc<dyn PlayerCharacterRepositoryPort>,
        character_repo: Arc<dyn CharacterRepositoryPort>,
        observation_repo: Arc<dyn ObservationRepositoryPort>,
        message_port: Arc<dyn WorldMessagePort>,
        broadcast: Arc<dyn BroadcastPort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            pc_repo,
            character_repo,
            observation_repo,
            message_port,
            broadcast,
            clock,
        }
    }

    /// Share an NPC's location with a player character
    ///
    /// DM-only operation that creates a "HeardAbout" observation for the PC.
    pub async fn share_npc_location(
        &self,
        ctx: UseCaseContext,
        input: ShareNpcLocationInput,
    ) -> Result<ShareNpcLocationResult, ObservationError> {
        if !ctx.is_dm {
            return Err(ObservationError::Database(
                "Only the DM can share NPC locations".to_string(),
            ));
        }

        info!(
            pc_id = %input.pc_id,
            npc_id = %input.npc_id,
            region_id = %input.region_id,
            "DM sharing NPC location with PC"
        );

        // Get game time - use current time for now
        // TODO: Use world-based game time when migrated
        let game_time = self.clock.now();

        // Create HeardAbout observation
        let observation = NpcObservation::heard_about(
            input.pc_id,
            input.npc_id,
            input.location_id,
            input.region_id,
            game_time,
            input.notes,
            self.clock.now(),
        );

        // Store the observation
        self.observation_repo
            .upsert(&observation)
            .await
            .map_err(|e| ObservationError::Database(e.to_string()))?;

        info!(
            pc_id = %input.pc_id,
            npc_id = %input.npc_id,
            "HeardAbout observation created"
        );

        Ok(ShareNpcLocationResult {
            observation_created: true,
        })
    }

    /// Trigger an NPC approach event
    ///
    /// DM-only operation that makes an NPC approach a PC and optionally
    /// reveals their identity.
    pub async fn trigger_approach_event(
        &self,
        ctx: UseCaseContext,
        input: TriggerApproachInput,
    ) -> Result<TriggerApproachResult, ObservationError> {
        if !ctx.is_dm {
            return Err(ObservationError::Database(
                "Only the DM can trigger approach events".to_string(),
            ));
        }

        info!(
            npc_id = %input.npc_id,
            target_pc = %input.target_pc_id,
            reveal = input.reveal,
            "DM triggering approach event"
        );

        // Get NPC details
        let npc = self
            .character_repo
            .get(input.npc_id)
            .await
            .map_err(|e| ObservationError::Database(e.to_string()))?
            .ok_or(ObservationError::NpcNotFound(input.npc_id))?;

        // Get PC details (for region and user_id)
        let pc = self
            .pc_repo
            .get(input.target_pc_id)
            .await
            .map_err(|e| ObservationError::Database(e.to_string()))?
            .ok_or(ObservationError::PcNotFound(input.target_pc_id))?;

        // Create observation if PC has a current region
        if let Some(region_id) = pc.current_region_id {
            let game_time = self.clock.now();

            let observation = if input.reveal {
                NpcObservation::direct(
                    input.target_pc_id,
                    input.npc_id,
                    pc.current_location_id,
                    region_id,
                    game_time,
                    self.clock.now(),
                )
            } else {
                NpcObservation::direct_unrevealed(
                    input.target_pc_id,
                    input.npc_id,
                    pc.current_location_id,
                    region_id,
                    game_time,
                    self.clock.now(),
                )
            };

            if let Err(e) = self.observation_repo.upsert(&observation).await {
                warn!(error = %e, "Failed to create observation for approach event");
            }
        }

        // Build the approach event data
        let (npc_name, npc_sprite) = if input.reveal {
            (npc.name.clone(), npc.sprite_asset.clone())
        } else {
            ("Unknown Figure".to_string(), None)
        };

        let approach_event = ApproachEventData {
            npc_id: input.npc_id.to_string(),
            npc_name: npc_name.clone(),
            npc_sprite,
            description: input.description,
            reveal: input.reveal,
        };

        // Send to the target PC's user
        let world_id_uuid = *ctx.world_id.as_uuid();
        self.message_port
            .send_to_user(&pc.user_id, world_id_uuid, approach_event)
            .await;

        info!(
            target_pc = %input.target_pc_id,
            npc = %npc.name,
            "Approach event triggered"
        );

        Ok(TriggerApproachResult {
            npc_name: npc.name,
            target_pc_name: pc.name,
        })
    }

    /// Trigger a location-wide event
    ///
    /// DM-only operation that broadcasts an event to all players in a world.
    /// Clients filter by their current region.
    pub async fn trigger_location_event(
        &self,
        ctx: UseCaseContext,
        input: TriggerLocationEventInput,
    ) -> Result<TriggerLocationEventResult, ObservationError> {
        if !ctx.is_dm {
            return Err(ObservationError::Database(
                "Only the DM can trigger location events".to_string(),
            ));
        }

        info!(
            region_id = %input.region_id,
            "DM triggering location event"
        );

        // Build the location event
        let location_event = LocationEventData {
            region_id: input.region_id.to_string(),
            description: input.description,
        };

        // Broadcast to all in world
        let world_id_uuid = *ctx.world_id.as_uuid();
        self.message_port
            .broadcast_to_world(world_id_uuid, location_event)
            .await;

        info!(
            region_id = %input.region_id,
            "Location event triggered"
        );

        Ok(TriggerLocationEventResult {
            event_broadcast: true,
        })
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

#[async_trait]
impl ObservationUseCasePort for ObservationUseCase {
    async fn share_npc_location(
        &self,
        ctx: UseCaseContext,
        input: ShareNpcLocationInput,
    ) -> Result<ShareNpcLocationResult, ObservationError> {
        self.share_npc_location(ctx, input).await
    }

    async fn trigger_approach_event(
        &self,
        ctx: UseCaseContext,
        input: TriggerApproachInput,
    ) -> Result<TriggerApproachResult, ObservationError> {
        self.trigger_approach_event(ctx, input).await
    }

    async fn trigger_location_event(
        &self,
        ctx: UseCaseContext,
        input: TriggerLocationEventInput,
    ) -> Result<TriggerLocationEventResult, ObservationError> {
        self.trigger_location_event(ctx, input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId};

    #[test]
    fn test_share_npc_location_input() {
        let input = ShareNpcLocationInput {
            pc_id: PlayerCharacterId::from_uuid(uuid::Uuid::new_v4()),
            npc_id: CharacterId::from_uuid(uuid::Uuid::new_v4()),
            location_id: LocationId::from_uuid(uuid::Uuid::new_v4()),
            region_id: RegionId::from_uuid(uuid::Uuid::new_v4()),
            notes: Some("The bartender mentioned seeing him".to_string()),
        };

        assert!(input.notes.is_some());
    }

    #[test]
    fn test_approach_event_data() {
        let event = ApproachEventData {
            npc_id: "test-npc".to_string(),
            npc_name: "Unknown Figure".to_string(),
            npc_sprite: None,
            description: "A shadowy figure approaches...".to_string(),
            reveal: false,
        };

        assert!(!event.reveal);
        assert_eq!(event.npc_name, "Unknown Figure");
    }
}
