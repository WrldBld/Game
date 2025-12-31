use async_trait::async_trait;

use wrldbldr_domain::entities::{StagedNpc, StagingSource};
use wrldbldr_domain::{GameTime, LocationId, RegionId, WorldId};

use super::{ApprovedNpcData, RegeneratedNpc, StagingProposalData};

/// Outbound port for staging operations as used by movement + staging use cases.
///
/// This is intentionally distinct from the domain-facing `StagingServicePort`.
#[async_trait]
pub trait StagingUseCaseServicePort: Send + Sync {
    /// Get current valid staging for a region
    async fn get_current_staging(
        &self,
        region_id: RegionId,
        game_time: &GameTime,
    ) -> Result<Option<Vec<StagedNpc>>, String>;

    /// Generate a staging proposal for a region
    async fn generate_proposal(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_id: LocationId,
        location_name: &str,
        game_time: &GameTime,
        ttl_hours: i32,
        dm_guidance: Option<&str>,
    ) -> Result<StagingProposalData, String>;
}

/// Extended outbound port for staging operations.
#[async_trait]
pub trait StagingUseCaseServiceExtPort: StagingUseCaseServicePort {
    /// Approve staging and persist it
    async fn approve_staging(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: &GameTime,
        approved_npcs: Vec<ApprovedNpcData>,
        ttl_hours: i32,
        source: StagingSource,
        approved_by: &str,
    ) -> Result<Vec<StagedNpc>, String>;

    /// Regenerate LLM suggestions with guidance
    async fn regenerate_suggestions(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_name: &str,
        game_time: &GameTime,
        guidance: &str,
    ) -> Result<Vec<RegeneratedNpc>, String>;

    /// Pre-stage a region
    async fn pre_stage_region(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: &GameTime,
        npcs: Vec<ApprovedNpcData>,
        ttl_hours: i32,
        dm_user_id: &str,
    ) -> Result<Vec<StagedNpc>, String>;
}
