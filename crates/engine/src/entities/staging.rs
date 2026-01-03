//! Staging entity operations.

use std::sync::Arc;
use wrldbldr_domain::{CharacterId, RegionId, StagedNpc, StagingId, WorldId};

use crate::infrastructure::ports::{RepoError, StagingRepo};

/// Staging entity operations.
///
/// Manages NPC presence in regions (staging).
pub struct Staging {
    repo: Arc<dyn StagingRepo>,
}

impl Staging {
    pub fn new(repo: Arc<dyn StagingRepo>) -> Self {
        Self { repo }
    }

    /// Get NPCs currently staged in a region.
    pub async fn get_staged_npcs(&self, region_id: RegionId) -> Result<Vec<StagedNpc>, RepoError> {
        self.repo.get_staged_npcs(region_id).await
    }

    /// Stage an NPC in a region.
    pub async fn stage_npc(&self, region_id: RegionId, character_id: CharacterId) -> Result<(), RepoError> {
        self.repo.stage_npc(region_id, character_id).await
    }

    /// Remove an NPC from a region.
    pub async fn unstage_npc(&self, region_id: RegionId, character_id: CharacterId) -> Result<(), RepoError> {
        self.repo.unstage_npc(region_id, character_id).await
    }

    /// Get pending staging proposals for DM approval.
    pub async fn get_pending(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<wrldbldr_domain::Staging>, RepoError> {
        self.repo.get_pending_staging(world_id).await
    }

    /// Save a pending staging proposal.
    pub async fn save_pending(
        &self,
        staging: &wrldbldr_domain::Staging,
    ) -> Result<(), RepoError> {
        self.repo.save_pending_staging(staging).await
    }

    /// Delete a pending staging proposal (after approval/rejection).
    pub async fn delete_pending(&self, id: StagingId) -> Result<(), RepoError> {
        self.repo.delete_pending_staging(id).await
    }

    /// Resolve staging for a region - get or create staged NPCs.
    pub async fn resolve_for_region(&self, region_id: RegionId) -> Result<Vec<StagedNpc>, RepoError> {
        // TODO: Implement staging resolution logic
        // For now, just return currently staged NPCs
        self.get_staged_npcs(region_id).await
    }
}
