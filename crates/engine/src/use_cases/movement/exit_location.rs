//! Exit to location use case.
//!
//! Handles player character movement to a different location entirely.
//! Determines the arrival region and coordinates with staging/narrative/time systems.

use std::sync::Arc;
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId, StagedNpc};

use crate::entities::{Location, Narrative, Observation, PlayerCharacter, Staging, World};
use crate::infrastructure::ports::{ClockPort, RepoError};
use crate::use_cases::time::{SuggestTime, SuggestTimeResult, TimeSuggestion};

use super::enter_region::{EnterRegionResult, StagingStatus};

/// Exit to location use case.
///
/// Handles moving to a different location entirely.
pub struct ExitLocation {
    player_character: Arc<PlayerCharacter>,
    location: Arc<Location>,
    staging: Arc<Staging>,
    observation: Arc<Observation>,
    narrative: Arc<Narrative>,
    world: Arc<World>,
    suggest_time: Arc<SuggestTime>,
    clock: Arc<dyn ClockPort>,
}

impl ExitLocation {
    pub fn new(
        player_character: Arc<PlayerCharacter>,
        location: Arc<Location>,
        staging: Arc<Staging>,
        observation: Arc<Observation>,
        narrative: Arc<Narrative>,
        world: Arc<World>,
        suggest_time: Arc<SuggestTime>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            player_character,
            location,
            staging,
            observation,
            narrative,
            world,
            suggest_time,
            clock,
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
            .get(target_location_id)
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

        // 6. Get fresh PC data after position update
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ExitLocationError::PlayerCharacterNotFound)?;

        // 7. Get the world to access game time for TTL checks and observations
        let world_data = self
            .world
            .get(pc.world_id)
            .await?
            .ok_or(ExitLocationError::WorldNotFound)?;
        let current_game_time = world_data.game_time.current();

        // 8. Check for valid staging (with TTL check using game time)
        let active_staging = self.staging.get_active_staging(region_id, current_game_time).await?;
        
        let (npcs, staging_status) = match active_staging {
            Some(staging) => {
                // Valid staging exists - resolve NPCs visible to players
                let visible_npcs: Vec<StagedNpc> = staging.npcs
                    .into_iter()
                    .filter(|npc| npc.is_visible_to_players())
                    .collect();
                (visible_npcs, StagingStatus::Ready)
            }
            None => {
                // No valid staging - DM approval required
                let previous = self.staging.get_staged_npcs(region_id).await.ok()
                    .map(|npcs| {
                        wrldbldr_domain::Staging::new(
                            region_id,
                            region.location_id,
                            pc.world_id,
                            current_game_time,
                            "expired",
                            wrldbldr_domain::StagingSource::RuleBased,
                            0,
                            current_game_time,
                        ).with_npcs(npcs)
                    })
                    .filter(|s| !s.npcs.is_empty());
                
                (vec![], StagingStatus::Pending { previous_staging: previous })
            }
        };

        // 9. Update observation (only if staging ready)
        // Use game time for when the observation occurred in-game
        if !npcs.is_empty() {
            self.observation
                .record_visit(pc_id, region_id, &npcs, current_game_time)
                .await?;
        }

        // 10. Check triggers
        let triggered_events = self.narrative.check_triggers(region_id, pc_id).await?;

        // 11. Generate time suggestion for location travel
        let time_suggestion = self.suggest_time_for_travel(
            pc.world_id,
            pc_id,
            pc.name.clone(),
            &location.name,
        ).await;

        // Note: Scene resolution is not implemented for ExitLocation yet.
        // The EnterRegion use case handles full scene resolution when moving within a location.
        // For location-to-location travel, scene resolution would need to be added here.
        Ok(EnterRegionResult {
            region,
            npcs,
            triggered_events,
            staging_status,
            pc,
            resolved_scene: None,
            time_suggestion,
        })
    }

    /// Generate a time suggestion for location-to-location travel.
    ///
    /// Uses the "travel_location" action type to look up time cost.
    async fn suggest_time_for_travel(
        &self,
        world_id: wrldbldr_domain::WorldId,
        pc_id: PlayerCharacterId,
        pc_name: String,
        destination_name: &str,
    ) -> Option<TimeSuggestion> {
        match self.suggest_time.execute(
            world_id,
            pc_id,
            pc_name,
            "travel_location",
            format!("Travel to {}", destination_name),
        ).await {
            Ok(SuggestTimeResult::SuggestionCreated(suggestion)) => Some(suggestion),
            Ok(SuggestTimeResult::AutoAdvanced { .. }) => {
                // In auto mode, time was advanced - no suggestion needed
                None
            }
            Ok(SuggestTimeResult::NoCost) | Ok(SuggestTimeResult::ManualMode) => None,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to generate time suggestion for location travel");
                None
            }
        }
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
            .get(location_id)
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
    #[error("World not found")]
    WorldNotFound,
    #[error("Region not found")]
    RegionNotFound,
    #[error("No arrival region specified and no default found")]
    NoArrivalRegion,
    #[error("Region does not belong to target location")]
    RegionLocationMismatch,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
