//! Enter region use case.

use std::sync::Arc;
use wrldbldr_domain::{NarrativeEvent, PlayerCharacterId, Region, RegionId, StagedNpc};

use crate::entities::{Character, Location, Narrative, Observation, Staging};
use crate::infrastructure::ports::RepoError;

/// Result of entering a region.
#[derive(Debug)]
pub struct EnterRegionResult {
    pub region: Region,
    pub npcs: Vec<StagedNpc>,
    pub triggered_events: Vec<NarrativeEvent>,
}

/// Enter region use case.
///
/// Orchestrates: Movement validation, staging resolution, observation updates, trigger checks.
pub struct EnterRegion {
    character: Arc<Character>,
    location: Arc<Location>,
    staging: Arc<Staging>,
    observation: Arc<Observation>,
    narrative: Arc<Narrative>,
}

impl EnterRegion {
    pub fn new(
        character: Arc<Character>,
        location: Arc<Location>,
        staging: Arc<Staging>,
        observation: Arc<Observation>,
        narrative: Arc<Narrative>,
    ) -> Self {
        Self {
            character,
            location,
            staging,
            observation,
            narrative,
        }
    }

    /// Execute the enter region use case.
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        region_id: RegionId,
    ) -> Result<EnterRegionResult, EnterRegionError> {
        // 1. Get the target region
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(EnterRegionError::RegionNotFound)?;

        // 2. Resolve NPC staging for this region
        let npcs = self.staging.resolve_for_region(region_id).await?;

        // 3. Update player's observation state
        self.observation.record_visit(pc_id, region_id, &npcs).await?;

        // 4. Check for triggered narrative events
        let triggered_events = self.narrative.check_triggers(region_id, pc_id).await?;

        // 5. Update character position (via PC update - would need PlayerCharacter entity)
        // TODO: Add PlayerCharacter entity and update position

        Ok(EnterRegionResult {
            region,
            npcs,
            triggered_events,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EnterRegionError {
    #[error("Region not found")]
    RegionNotFound,
    #[error("Movement blocked")]
    MovementBlocked,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
