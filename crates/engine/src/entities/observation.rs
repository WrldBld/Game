//! Observation entity operations.

use std::sync::Arc;
use wrldbldr_domain::{CharacterId, NpcObservation, PlayerCharacterId};

use crate::infrastructure::ports::{ObservationRepo, RepoError};

/// Observation entity operations.
///
/// Tracks what NPCs a player character has observed/met.
pub struct Observation {
    repo: Arc<dyn ObservationRepo>,
}

impl Observation {
    pub fn new(repo: Arc<dyn ObservationRepo>) -> Self {
        Self { repo }
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

    /// Check if a PC has observed a specific character.
    pub async fn has_observed(&self, pc_id: PlayerCharacterId, target_id: CharacterId) -> Result<bool, RepoError> {
        self.repo.has_observed(pc_id, target_id).await
    }

    /// Record that a PC has visited a region and seen its NPCs.
    pub async fn record_visit(
        &self,
        _pc_id: PlayerCharacterId,
        _region_id: wrldbldr_domain::RegionId,
        _npcs: &[wrldbldr_domain::StagedNpc],
    ) -> Result<(), RepoError> {
        // TODO: Create observations for each NPC in the region
        Ok(())
    }
}
