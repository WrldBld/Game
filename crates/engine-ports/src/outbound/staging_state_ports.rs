use async_trait::async_trait;
use uuid::Uuid;

use wrldbldr_domain::{GameTime, RegionId, WorldId};

use super::{PendingStagingData, PendingStagingInfo, RegeneratedNpc};

/// Outbound port for managing pending staging state.
///
/// Used by the application (movement + staging use cases); implemented by adapters.
#[async_trait]
pub trait StagingStatePort: Send + Sync {
    /// Get current game time for the world
    fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime>;

    /// Check if there's a pending staging for a region
    fn has_pending_staging(&self, world_id: &WorldId, region_id: &RegionId) -> bool;

    /// Add a PC to the waiting list for a pending staging
    fn add_waiting_pc(
        &self,
        world_id: &WorldId,
        region_id: &RegionId,
        pc_id: Uuid,
        pc_name: String,
        user_id: String,
        client_id: String,
    );

    /// Store a new pending staging approval
    fn store_pending_staging(&self, pending: PendingStagingData);
}

/// Extended port for staging state management.
#[async_trait]
pub trait StagingStateExtPort: StagingStatePort {
    /// Get a pending staging by request ID
    fn get_pending_staging(
        &self,
        world_id: &WorldId,
        request_id: &str,
    ) -> Option<PendingStagingInfo>;

    /// Remove a pending staging
    fn remove_pending_staging(&self, world_id: &WorldId, request_id: &str);

    /// Update the LLM suggestions for a pending staging
    fn update_llm_suggestions(
        &self,
        world_id: &WorldId,
        request_id: &str,
        npcs: Vec<RegeneratedNpc>,
    );
}
