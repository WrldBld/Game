//! Enter region use case.
//!
//! Handles player character movement to a region within the same location.
//! Coordinates with staging, observation, and narrative systems.

use std::sync::Arc;
use wrldbldr_domain::{NarrativeEvent, PlayerCharacter as DomainPlayerCharacter, PlayerCharacterId, Region, RegionId, StagedNpc, Staging as DomainStaging};

use crate::entities::{Location, Narrative, Observation, PlayerCharacter, Staging};
use crate::infrastructure::ports::{ClockPort, RepoError};

/// Result of entering a region.
#[derive(Debug)]
pub struct EnterRegionResult {
    /// The region entered
    pub region: Region,
    /// NPCs present in the region (empty if staging pending)
    pub npcs: Vec<StagedNpc>,
    /// Narrative events triggered by entry
    pub triggered_events: Vec<NarrativeEvent>,
    /// Staging status for this region
    pub staging_status: StagingStatus,
    /// The player character who moved (for context in pending staging)
    pub pc: DomainPlayerCharacter,
}

/// Status of staging for a region.
#[derive(Debug)]
pub enum StagingStatus {
    /// Valid staging exists, NPCs are resolved
    Ready,
    /// No valid staging, DM approval required
    Pending {
        /// Previous staging if it exists (may be expired)
        previous_staging: Option<DomainStaging>,
    },
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
    clock: Arc<dyn ClockPort>,
}

impl EnterRegion {
    pub fn new(
        player_character: Arc<PlayerCharacter>,
        location: Arc<Location>,
        staging: Arc<Staging>,
        observation: Arc<Observation>,
        narrative: Arc<Narrative>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            player_character,
            location,
            staging,
            observation,
            narrative,
            clock,
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

        // 5. Check for valid staging (with TTL check)
        let current_game_time = self.clock.now();
        let active_staging = self.staging.get_active_staging(region_id, current_game_time).await?;
        
        let (npcs, staging_status) = match active_staging {
            Some(staging) => {
                // Valid staging exists - resolve NPCs
                let visible_npcs: Vec<StagedNpc> = staging.npcs
                    .into_iter()
                    .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
                    .collect();
                (visible_npcs, StagingStatus::Ready)
            }
            None => {
                // No valid staging - DM approval required
                // Try to get any existing staging for reference (may be expired)
                let previous = self.staging.get_staged_npcs(region_id).await.ok()
                    .map(|npcs| {
                        // Create a minimal staging for reference
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

        // 6. Update player's observation state (even if staging pending, record the visit)
        if !npcs.is_empty() {
            self.observation
                .record_visit(pc_id, region_id, &npcs)
                .await?;
        }

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
            staging_status,
            pc,
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
