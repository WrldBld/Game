//! Observation entity operations.

use std::sync::Arc;
use chrono::{DateTime, Utc};
use wrldbldr_domain::{CharacterId, NpcObservation, PlayerCharacterId, StagedNpc};

use crate::infrastructure::ports::{ClockPort, LocationRepo, ObservationRepo, RepoError};

/// Observation entity operations.
///
/// Tracks what NPCs a player character has observed/met.
pub struct Observation {
    repo: Arc<dyn ObservationRepo>,
    location_repo: Arc<dyn LocationRepo>,
    clock: Arc<dyn ClockPort>,
}

impl Observation {
    pub fn new(
        repo: Arc<dyn ObservationRepo>,
        location_repo: Arc<dyn LocationRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            repo,
            location_repo,
            clock,
        }
    }

    /// Get all observations for a player character.
    pub async fn get_observations(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcObservation>, RepoError> {
        self.repo.get_observations(pc_id).await
    }

    /// Record a new observation.
    pub async fn save_observation(&self, observation: &NpcObservation) -> Result<(), RepoError> {
        self.repo.save_observation(observation).await
    }

    /// Delete an observation between a PC and NPC.
    pub async fn delete_observation(
        &self,
        pc_id: PlayerCharacterId,
        target_id: CharacterId,
    ) -> Result<(), RepoError> {
        self.repo.delete_observation(pc_id, target_id).await
    }

    /// Check if a PC has observed a specific character.
    pub async fn has_observed(
        &self,
        pc_id: PlayerCharacterId,
        target_id: CharacterId,
    ) -> Result<bool, RepoError> {
        self.repo.has_observed(pc_id, target_id).await
    }

    /// Record deduced information from a challenge outcome.
    ///
    /// Creates a "deduced" observation for the PC, storing the revealed information.
    /// This is used by the RevealInformation trigger with persist=true.
    pub async fn record_deduced_info(
        &self,
        pc_id: PlayerCharacterId,
        info: String,
    ) -> Result<(), RepoError> {
        // Store the deduced info as a journal entry
        // We use the observation repo's deduced observation functionality
        self.repo.save_deduced_info(pc_id, info).await
    }

    /// Record that a PC has visited a region and seen its NPCs.
    ///
    /// Creates direct observations for each present, visible NPC in the region.
    /// Skips NPCs the player has already observed to avoid duplicate records.
    ///
    /// # Arguments
    /// * `pc_id` - The player character who visited
    /// * `region_id` - The region that was visited
    /// * `npcs` - NPCs present in the region
    /// * `game_time` - Current game time (from World.game_time.current())
    pub async fn record_visit(
        &self,
        pc_id: PlayerCharacterId,
        region_id: wrldbldr_domain::RegionId,
        npcs: &[StagedNpc],
        game_time: DateTime<Utc>,
    ) -> Result<(), RepoError> {
        // Get the region to find its location_id
        let region = self.location_repo.get_region(region_id).await?;
        let location_id = match region {
            Some(r) => r.location_id,
            None => return Ok(()), // Region not found, nothing to record
        };

        let now = self.clock.now(); // Real time for created_at

        // Create observations for each present, visible NPC
        for npc in npcs.iter().filter(|n| n.is_present && !n.is_hidden_from_players) {
            // Check if already observed to avoid duplicates
            let already_observed = self
                .repo
                .has_observed(pc_id, npc.character_id)
                .await?;

            if !already_observed {
                let observation = NpcObservation::direct(
                    pc_id,
                    npc.character_id,
                    location_id,
                    region_id,
                    game_time, // Game time for when the observation occurred in-game
                    now,       // Real time for when record was created
                );

                self.repo.save_observation(&observation).await?;
            }
        }

        Ok(())
    }
}
