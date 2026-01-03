//! Exit to location use case.
//!
//! Handles player character movement to a different location entirely.
//! Determines the arrival region and coordinates with staging/narrative systems.

use std::sync::Arc;
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId};

use crate::entities::{Location, Narrative, Observation, PlayerCharacter, Staging};
use crate::infrastructure::ports::RepoError;

use super::enter_region::EnterRegionResult;

/// Exit to location use case.
///
/// Handles moving to a different location entirely.
pub struct ExitLocation {
    player_character: Arc<PlayerCharacter>,
    location: Arc<Location>,
    staging: Arc<Staging>,
    observation: Arc<Observation>,
    narrative: Arc<Narrative>,
}

impl ExitLocation {
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

    /// Execute the exit to location use case.
    ///
    /// # Arguments
    /// * `pc_id` - The player character moving
    /// * `target_location_id` - The destination location
    /// * `arrival_region_id` - Optional specific region to arrive in
    ///
    /// # Returns
    /// * `Ok(EnterRegionResult)` - Successfully arrived at new location
    /// * `Err(ExitLocationError)` - Failed to move
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        target_location_id: LocationId,
        arrival_region_id: Option<RegionId>,
    ) -> Result<EnterRegionResult, ExitLocationError> {
        // 1. Validate player character exists
        let _pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ExitLocationError::PlayerCharacterNotFound)?;

        // 2. Get the target location
        let location = self
            .location
            .get_location(target_location_id)
            .await?
            .ok_or(ExitLocationError::LocationNotFound)?;

        // 3. Determine arrival region
        let region_id = self
            .determine_arrival_region(target_location_id, arrival_region_id)
            .await?;

        // 4. Get the arrival region
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(ExitLocationError::RegionNotFound)?;

        // Verify region belongs to target location
        if region.location_id != location.id {
            return Err(ExitLocationError::RegionLocationMismatch);
        }

        // 5. Update player character position (both location and region)
        self.player_character
            .update_position(pc_id, target_location_id, region_id)
            .await?;

        // 6. Resolve staging for arrival region
        let npcs = self.staging.resolve_for_region(region_id).await?;

        // 7. Update observation
        self.observation
            .record_visit(pc_id, region_id, &npcs)
            .await?;

        // 8. Check triggers
        let triggered_events = self.narrative.check_triggers(region_id, pc_id).await?;

        Ok(EnterRegionResult {
            region,
            npcs,
            triggered_events,
        })
    }

    /// Determine the arrival region for a location.
    async fn determine_arrival_region(
        &self,
        location_id: LocationId,
        specified_region_id: Option<RegionId>,
    ) -> Result<RegionId, ExitLocationError> {
        // If a specific region was specified, use it
        if let Some(region_id) = specified_region_id {
            // Verify region exists and belongs to location
            let region = self
                .location
                .get_region(region_id)
                .await?
                .ok_or(ExitLocationError::RegionNotFound)?;

            if region.location_id != location_id {
                return Err(ExitLocationError::RegionLocationMismatch);
            }

            return Ok(region_id);
        }

        // Try location's default arrival region
        let location = self
            .location
            .get_location(location_id)
            .await?
            .ok_or(ExitLocationError::LocationNotFound)?;

        if let Some(default_region_id) = location.default_region_id {
            if self.location.get_region(default_region_id).await?.is_some() {
                return Ok(default_region_id);
            }
        }

        // Fall back to first spawn point in location
        let regions = self
            .location
            .list_regions_in_location(location_id)
            .await?;

        regions
            .into_iter()
            .find(|r| r.is_spawn_point)
            .map(|r| r.id)
            .ok_or(ExitLocationError::NoArrivalRegion)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExitLocationError {
    #[error("Player character not found")]
    PlayerCharacterNotFound,
    #[error("Location not found")]
    LocationNotFound,
    #[error("Region not found")]
    RegionNotFound,
    #[error("No arrival region specified and no default found")]
    NoArrivalRegion,
    #[error("Region does not belong to target location")]
    RegionLocationMismatch,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
