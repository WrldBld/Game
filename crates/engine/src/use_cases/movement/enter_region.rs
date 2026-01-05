//! Enter region use case.
//!
//! Handles player character movement to a region within the same location.
//! Coordinates with staging, observation, scene resolution, narrative, and time systems.

use std::sync::Arc;
use wrldbldr_domain::{
    NarrativeEvent, PlayerCharacter as DomainPlayerCharacter, PlayerCharacterId,
    Region, RegionId, Scene as DomainScene, StagedNpc, Staging as DomainStaging,
};

use crate::entities::{
    Flag, Inventory, Location, Narrative, Observation, PlayerCharacter, Scene,
    Staging, World,
};
use crate::infrastructure::ports::RepoError;
use crate::use_cases::time::{SuggestTime, TimeSuggestion};

use super::{resolve_scene_for_region, resolve_staging_for_region, suggest_time_for_movement};

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
    /// Time suggestion for this movement (if time mode is Suggested)
    pub time_suggestion: Option<TimeSuggestion>,
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
/// Orchestrates: Movement validation, staging resolution, scene resolution, observation updates, trigger checks, time suggestions.
pub struct EnterRegion {
    player_character: Arc<PlayerCharacter>,
    location: Arc<Location>,
    staging: Arc<Staging>,
    observation: Arc<Observation>,
    narrative: Arc<Narrative>,
    scene: Arc<Scene>,
    inventory: Arc<Inventory>,
    flag: Arc<Flag>,
    world: Arc<World>,
    suggest_time: Arc<SuggestTime>,
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
        flag: Arc<Flag>,
        world: Arc<World>,
        suggest_time: Arc<SuggestTime>,
    ) -> Self {
        Self {
            player_character,
            location,
            staging,
            observation,
            narrative,
            scene,
            inventory,
            flag,
            world,
            suggest_time,
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

        // 4. Validate connection exists and is not locked (if PC has a current region)
        // Skip validation for initial spawn when PC has no current region
        if let Some(current_region_id) = pc.current_region_id {
            // Don't require path if already in target region
            if current_region_id != region_id {
                match self.check_connection(current_region_id, region_id).await {
                    ConnectionCheckResult::NoConnection => {
                        return Err(EnterRegionError::NoPathToRegion);
                    }
                    ConnectionCheckResult::Locked(reason) => {
                        return Err(EnterRegionError::MovementBlocked(reason));
                    }
                    ConnectionCheckResult::Open => {
                        // Connection exists and is unlocked - proceed
                    }
                }
            }
        }

        // 5. Get the world to access game time for TTL checks and observations
        let world_data = self
            .world
            .get(pc.world_id)
            .await?
            .ok_or(EnterRegionError::WorldNotFound)?;
        let current_game_time = world_data.game_time.current();

        // 6. Check for valid staging (with TTL check using game time)
        let (npcs, staging_status) = resolve_staging_for_region(
            &self.staging,
            region_id,
            region.location_id,
            pc.world_id,
            current_game_time,
        ).await?;

        // 7. Update player's observation state (even if staging pending, record the visit)
        // Use game time for when the observation occurred in-game
        if !npcs.is_empty() {
            self.observation
                .record_visit(pc_id, region_id, &npcs, current_game_time)
                .await?;
        }

        // 8. Resolve scene for this region (use world's game time for time-of-day checks)
        let resolved_scene = resolve_scene_for_region(
            &self.scene,
            &self.inventory,
            &self.observation,
            &self.flag,
            pc_id,
            pc.world_id,
            region_id,
            &world_data.game_time,
        ).await?;
        if let Some(ref scene) = resolved_scene {
            tracing::info!(
                pc_id = %pc_id,
                region_id = %region_id,
                scene_id = %scene.id,
                scene_name = %scene.name,
                "Scene resolved for region entry"
            );
        }

        // 9. Check for triggered narrative events
        let triggered_events = self.narrative.check_triggers(region_id, pc_id).await?;

        // 10. Update player character position
        self.player_character
            .update_position(pc_id, pc.current_location_id, region_id)
            .await?;

        // 11. Generate time suggestion for movement
        // This is a region-to-region move within the same location (travel_region)
        let time_suggestion = suggest_time_for_movement(
            &self.suggest_time,
            pc.world_id,
            pc_id,
            pc.name.clone(),
            "travel_region",
            &region.name,
        ).await;

        Ok(EnterRegionResult {
            region,
            npcs,
            triggered_events,
            staging_status,
            pc,
            resolved_scene,
            time_suggestion,
        })
    }

    /// Check if a valid connection exists between regions.
    ///
    /// Returns:
    /// - `Open` if connection exists and is unlocked
    /// - `Locked(reason)` if connection exists but is locked
    /// - `NoConnection` if no path exists between regions
    async fn check_connection(
        &self,
        from_region_id: RegionId,
        to_region_id: RegionId,
    ) -> ConnectionCheckResult {
        let connections = match self.location.get_connections(from_region_id).await {
            Ok(c) => c,
            Err(_) => return ConnectionCheckResult::NoConnection,
        };

        // Find connection to target region
        match connections.iter().find(|c| c.to_region == to_region_id) {
            Some(connection) if connection.is_locked => {
                let reason = connection
                    .lock_description
                    .clone()
                    .unwrap_or_else(|| "The way is blocked".to_string());
                ConnectionCheckResult::Locked(reason)
            }
            Some(_) => ConnectionCheckResult::Open,
            None => ConnectionCheckResult::NoConnection,
        }
    }
}

/// Result of checking a connection between regions.
enum ConnectionCheckResult {
    /// Connection exists and is open
    Open,
    /// Connection exists but is locked
    Locked(String),
    /// No connection exists between regions
    NoConnection,
}

#[derive(Debug, thiserror::Error)]
pub enum EnterRegionError {
    #[error("Player character not found")]
    PlayerCharacterNotFound,
    #[error("Region not found")]
    RegionNotFound,
    #[error("World not found")]
    WorldNotFound,
    #[error("Region is not in the current location")]
    RegionNotInCurrentLocation,
    #[error("No path exists to that region")]
    NoPathToRegion,
    #[error("Movement blocked: {0}")]
    MovementBlocked(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
