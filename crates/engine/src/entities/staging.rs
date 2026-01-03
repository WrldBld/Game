//! Staging entity operations.

use std::sync::Arc;
use chrono::{DateTime, Utc};
use wrldbldr_domain::{CharacterId, RegionId, StagedNpc, Staging as DomainStaging, StagingId, WorldId};

use crate::infrastructure::ports::{RepoError, StagingRepo};

/// Staging entity operations.
///
/// Manages NPC presence in regions (staging). The staging system determines
/// "who is on stage" for each region, using a DM approval workflow.
///
/// ## Staging Workflow
///
/// 1. **Active Staging**: A DM-approved staging with NPCs marked present/absent
/// 2. **Staging Resolution**: When a player enters a region:
///    - If active staging exists (not expired), use it
///    - Otherwise, generate suggestions and queue for DM approval
/// 3. **Rule-Based Suggestions**: Based on NPC relationships to region
///    (WORKS_AT, FREQUENTS, HOME_REGION) and frequency settings
/// 4. **LLM-Enhanced Suggestions**: Context-aware NPC presence reasoning
///
/// The full workflow with DM approval is handled at the WebSocket/API layer.
/// This entity module provides the building blocks.
pub struct Staging {
    repo: Arc<dyn StagingRepo>,
}

impl Staging {
    pub fn new(repo: Arc<dyn StagingRepo>) -> Self {
        Self { repo }
    }

    /// Get all NPCs in the staging configuration for a region.
    ///
    /// Returns the raw staging data including NPCs that may be marked
    /// as not present or hidden.
    pub async fn get_staged_npcs(&self, region_id: RegionId) -> Result<Vec<StagedNpc>, RepoError> {
        self.repo.get_staged_npcs(region_id).await
    }

    /// Stage an NPC in a region.
    pub async fn stage_npc(
        &self,
        region_id: RegionId,
        character_id: CharacterId,
    ) -> Result<(), RepoError> {
        self.repo.stage_npc(region_id, character_id).await
    }

    /// Remove an NPC from a region.
    pub async fn unstage_npc(
        &self,
        region_id: RegionId,
        character_id: CharacterId,
    ) -> Result<(), RepoError> {
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
    
    /// Get the active (non-expired) staging for a region.
    /// 
    /// Returns `None` if no staging exists or if the current staging has expired.
    /// This is used to determine if DM approval is needed before showing scene.
    pub async fn get_active_staging(
        &self,
        region_id: RegionId,
        current_game_time: DateTime<Utc>,
    ) -> Result<Option<DomainStaging>, RepoError> {
        self.repo.get_active_staging(region_id, current_game_time).await
    }
    
    /// Activate a staging after DM approval.
    /// 
    /// This replaces any existing current staging for the region.
    pub async fn activate_staging(
        &self,
        staging_id: StagingId,
        region_id: RegionId,
    ) -> Result<(), RepoError> {
        self.repo.activate_staging(staging_id, region_id).await
    }

    /// Resolve which NPCs are present in a region for player view.
    ///
    /// Returns NPCs that are:
    /// - Currently staged in the region (from an active, DM-approved staging)
    /// - Marked as present (`is_present = true`)
    /// - Not hidden from players (`is_hidden_from_players = false`)
    ///
    /// If no staging exists, returns an empty list. The WebSocket handler
    /// should trigger the DM approval workflow in this case.
    ///
    /// ## Future Improvements
    ///
    /// A more complete implementation would:
    /// 1. Check if active staging is expired (TTL-based)
    /// 2. Generate rule-based suggestions if no valid staging exists
    /// 3. Queue for DM approval if needed
    pub async fn resolve_for_region(&self, region_id: RegionId) -> Result<Vec<StagedNpc>, RepoError> {
        let all_staged = self.get_staged_npcs(region_id).await?;

        // Filter to only present, visible NPCs
        let visible_npcs: Vec<StagedNpc> = all_staged
            .into_iter()
            .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
            .collect();

        Ok(visible_npcs)
    }

    /// Get all staged NPCs including hidden ones (for DM view).
    ///
    /// Used by DM-facing UIs that need to see the full staging picture.
    pub async fn resolve_for_region_dm_view(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<StagedNpc>, RepoError> {
        let all_staged = self.get_staged_npcs(region_id).await?;

        // Filter to only present NPCs (including hidden ones)
        let present_npcs: Vec<StagedNpc> = all_staged
            .into_iter()
            .filter(|npc| npc.is_present)
            .collect();

        Ok(present_npcs)
    }
    
    /// Get staging history for a region (most recent first).
    ///
    /// Returns past stagings that are no longer active. Useful for:
    /// - Viewing previous NPC configurations
    /// - Restoring a past staging
    /// - Auditing staging decisions
    pub async fn get_history(
        &self,
        region_id: RegionId,
        limit: usize,
    ) -> Result<Vec<DomainStaging>, RepoError> {
        self.repo.get_staging_history(region_id, limit).await
    }
}
