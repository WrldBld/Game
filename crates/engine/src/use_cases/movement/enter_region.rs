//! Enter region use case.
//!
//! Handles player character movement to a region within the same location.
//! Coordinates with staging, observation, and narrative systems.

use std::sync::Arc;
use wrldbldr_domain::{NarrativeEvent, PlayerCharacterId, Region, RegionId, StagedNpc};

use crate::entities::{Location, Narrative, Observation, PlayerCharacter, Staging};
use crate::infrastructure::ports::RepoError;

/// Result of entering a region.
#[derive(Debug)]
pub struct EnterRegionResult {
    /// The region entered
    pub region: Region,
    /// NPCs present in the region
    pub npcs: Vec<StagedNpc>,
    /// Narrative events triggered by entry
    pub triggered_events: Vec<NarrativeEvent>,
}

/// Enter region use case.
///
/// Orchestrates: Movement validation, staging resolution, observation updates, trigger checks.
pub struct EnterRegion {
    player_character: Arc<PlayerCharacter>,
    location: Arc<Location>,
    staging: Arc<Staging>,
    observation: Arc<Observation>,
    narrative: Arc<Narrative>,
}

impl EnterRegion {
    pub fn new(
        player_character: Arc<PlayerCharacter>,
        location: Arc<Location>,
        staging: Arc<Staging>,
        observation: Arc<Observation>,
        narrative: Arc<Narrative>,
    ) -> Self {
        Self {
            player_character,
            location,
            staging,
            observation,
            narrative,
        }
    }

    /// Execute the enter region use case.
    ///
    /// # Arguments
    /// * `pc_id` - The player character moving
    /// * `region_id` - The target region to enter
    ///
    /// # Returns
    /// * `Ok(EnterRegionResult)` - Successfully entered region with scene data
    /// * `Err(EnterRegionError)` - Failed to enter region
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        region_id: RegionId,
    ) -> Result<EnterRegionResult, EnterRegionError> {
        // 1. Get the player character to validate and get current location
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(EnterRegionError::PlayerCharacterNotFound)?;

        // 2. Get the target region
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(EnterRegionError::RegionNotFound)?;

        // 3. Verify region is in the same location (for move_to_region)
        if region.location_id != pc.current_location_id {
            return Err(EnterRegionError::RegionNotInCurrentLocation);
        }

        // 4. Check for locked connections if PC has a current region
        if let Some(current_region_id) = pc.current_region_id {
            if let Some(reason) = self.check_locked_connection(current_region_id, region_id).await {
                return Err(EnterRegionError::MovementBlocked(reason));
            }
        }

        // 5. Resolve NPC staging for this region
        let npcs = self.staging.resolve_for_region(region_id).await?;

        // 6. Update player's observation state
        self.observation
            .record_visit(pc_id, region_id, &npcs)
            .await?;

        // 7. Check for triggered narrative events
        let triggered_events = self.narrative.check_triggers(region_id, pc_id).await?;

        // 8. Update player character position
        self.player_character
            .update_position(pc_id, pc.current_location_id, region_id)
            .await?;

        Ok(EnterRegionResult {
            region,
            npcs,
            triggered_events,
        })
    }

    /// Check if a connection between regions is locked.
    async fn check_locked_connection(
        &self,
        from_region_id: RegionId,
        to_region_id: RegionId,
    ) -> Option<String> {
        let connections = self.location.get_connections(from_region_id).await.ok()?;

        connections
            .iter()
            .find(|c| c.to_region == to_region_id && c.is_locked)
            .map(|c| {
                c.lock_description
                    .clone()
                    .unwrap_or_else(|| "The way is blocked".to_string())
            })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EnterRegionError {
    #[error("Player character not found")]
    PlayerCharacterNotFound,
    #[error("Region not found")]
    RegionNotFound,
    #[error("Region is not in the current location")]
    RegionNotInCurrentLocation,
    #[error("Movement blocked: {0}")]
    MovementBlocked(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
