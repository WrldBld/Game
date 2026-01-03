//! Exit to location use case.

use std::sync::Arc;
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId};

use crate::entities::{Location, Narrative, Observation, Staging};
use crate::infrastructure::ports::RepoError;

use super::enter_region::EnterRegionResult;

/// Exit to location use case.
///
/// Handles moving to a different location entirely.
pub struct ExitLocation {
    location: Arc<Location>,
    staging: Arc<Staging>,
    observation: Arc<Observation>,
    narrative: Arc<Narrative>,
}

impl ExitLocation {
    pub fn new(
        location: Arc<Location>,
        staging: Arc<Staging>,
        observation: Arc<Observation>,
        narrative: Arc<Narrative>,
    ) -> Self {
        Self {
            location,
            staging,
            observation,
            narrative,
        }
    }

    /// Execute the exit to location use case.
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        target_location_id: LocationId,
        arrival_region_id: Option<RegionId>,
    ) -> Result<EnterRegionResult, ExitLocationError> {
        // 1. Get the target location
        let location = self
            .location
            .get_location(target_location_id)
            .await?
            .ok_or(ExitLocationError::LocationNotFound)?;

        // 2. Determine arrival region
        let region_id = match arrival_region_id {
            Some(id) => id,
            None => {
                // Use default region or spawn point
                // TODO: Look up default/spawn region for location
                return Err(ExitLocationError::NoArrivalRegion);
            }
        };

        // 3. Get the arrival region
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(ExitLocationError::RegionNotFound)?;

        // Verify region belongs to target location
        if region.location_id != location.id {
            return Err(ExitLocationError::RegionLocationMismatch);
        }

        // 4. Resolve staging for arrival region
        let npcs = self.staging.resolve_for_region(region_id).await?;

        // 5. Update observation
        self.observation.record_visit(pc_id, region_id, &npcs).await?;

        // 6. Check triggers
        let triggered_events = self.narrative.check_triggers(region_id, pc_id).await?;

        Ok(EnterRegionResult {
            region,
            npcs,
            triggered_events,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExitLocationError {
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
