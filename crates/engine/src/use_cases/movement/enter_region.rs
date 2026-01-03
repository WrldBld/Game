//! Enter region use case.
//!
//! Handles player character movement to a region within the same location.
//! Coordinates with staging, observation, scene resolution, and narrative systems.

use std::sync::Arc;
use wrldbldr_domain::{
    GameTime, NarrativeEvent, PlayerCharacter as DomainPlayerCharacter, PlayerCharacterId,
    Region, RegionId, Scene as DomainScene, StagedNpc, Staging as DomainStaging,
};

use crate::entities::{
    Inventory, Location, Narrative, Observation, PlayerCharacter, Scene, SceneResolutionContext,
    Staging,
};
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
    /// Resolved scene for this region (if any)
    pub resolved_scene: Option<DomainScene>,
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
/// Orchestrates: Movement validation, staging resolution, scene resolution, observation updates, trigger checks.
pub struct EnterRegion {
    player_character: Arc<PlayerCharacter>,
    location: Arc<Location>,
    staging: Arc<Staging>,
    observation: Arc<Observation>,
    narrative: Arc<Narrative>,
    scene: Arc<Scene>,
    inventory: Arc<Inventory>,
    clock: Arc<dyn ClockPort>,
}

impl EnterRegion {
    pub fn new(
        player_character: Arc<PlayerCharacter>,
        location: Arc<Location>,
        staging: Arc<Staging>,
        observation: Arc<Observation>,
        narrative: Arc<Narrative>,
        scene: Arc<Scene>,
        inventory: Arc<Inventory>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            player_character,
            location,
            staging,
            observation,
            narrative,
            scene,
            inventory,
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

        // 7. Resolve scene for this region
        let resolved_scene = self.resolve_scene_for_region(pc_id, region_id).await?;
        if let Some(ref scene) = resolved_scene {
            tracing::info!(
                pc_id = %pc_id,
                region_id = %region_id,
                scene_id = %scene.id,
                scene_name = %scene.name,
                "Scene resolved for region entry"
            );
        }

        // 8. Check for triggered narrative events
        let triggered_events = self.narrative.check_triggers(region_id, pc_id).await?;

        // 9. Update player character position
        self.player_character
            .update_position(pc_id, pc.current_location_id, region_id)
            .await?;

        Ok(EnterRegionResult {
            region,
            npcs,
            triggered_events,
            staging_status,
            pc,
            resolved_scene,
        })
    }

    /// Resolve which scene to display for a PC entering a region.
    ///
    /// Builds the evaluation context from the PC's state (inventory, observations, completed scenes)
    /// and calls the scene resolution service.
    async fn resolve_scene_for_region(
        &self,
        pc_id: PlayerCharacterId,
        region_id: RegionId,
    ) -> Result<Option<DomainScene>, RepoError> {
        // Get current time of day from game time
        let now = self.clock.now();
        let game_time = GameTime::new(now);
        let time_of_day = game_time.time_of_day();

        // Build the scene resolution context
        let completed_scenes = self.scene.get_completed_scenes(pc_id).await?;
        let inventory = self.inventory.get_pc_inventory(pc_id).await?;
        let observations = self.observation.get_observations(pc_id).await?;

        let context = SceneResolutionContext::new(time_of_day)
            .with_completed_scenes(completed_scenes)
            .with_inventory(inventory.into_iter().map(|item| item.id))
            .with_known_characters(observations.into_iter().map(|obs| obs.npc_id));
        // Note: Flags not implemented yet - would need a flag storage system

        // Resolve the scene
        let result = self.scene.resolve_scene(region_id, &context).await?;

        // Log considered scenes for debugging
        for consideration in &result.considered_scenes {
            if !consideration.conditions_met {
                tracing::debug!(
                    scene_id = %consideration.scene_id,
                    scene_name = %consideration.scene_name,
                    unmet_conditions = ?consideration.unmet_conditions,
                    "Scene not matched due to unmet conditions"
                );
            }
        }

        Ok(result.scene)
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
