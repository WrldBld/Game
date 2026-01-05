//! Movement use cases.

mod enter_region;
mod exit_location;

pub use enter_region::{EnterRegion, EnterRegionError, EnterRegionResult, StagingStatus};
pub use exit_location::{ExitLocation, ExitLocationError};

use std::sync::Arc;
use chrono::{DateTime, Utc};
use wrldbldr_domain::{LocationId, RegionId, StagedNpc, Staging, StagingSource, WorldId};
use crate::entities::Staging as StagingEntity;
use crate::infrastructure::ports::RepoError;

/// Container for movement use cases.
pub struct MovementUseCases {
    pub enter_region: Arc<EnterRegion>,
    pub exit_location: Arc<ExitLocation>,
}

impl MovementUseCases {
    pub fn new(enter_region: Arc<EnterRegion>, exit_location: Arc<ExitLocation>) -> Self {
        Self {
            enter_region,
            exit_location,
        }
    }
}

/// Resolve staging for a region, returning the visible NPCs and status.
///
/// This is shared logic between EnterRegion and ExitLocation use cases.
///
/// # Returns
/// A tuple of (visible NPCs, staging status)
pub async fn resolve_staging_for_region(
    staging: &StagingEntity,
    region_id: RegionId,
    location_id: LocationId,
    world_id: WorldId,
    current_game_time: DateTime<Utc>,
) -> Result<(Vec<StagedNpc>, StagingStatus), RepoError> {
    let active_staging = staging.get_active_staging(region_id, current_game_time).await?;

    match active_staging {
        Some(s) => {
            // Valid staging exists - resolve NPCs visible to players
            let visible_npcs: Vec<StagedNpc> = s.npcs
                .into_iter()
                .filter(|npc| npc.is_visible_to_players())
                .collect();
            Ok((visible_npcs, StagingStatus::Ready))
        }
        None => {
            // No valid staging - DM approval required
            // Try to get any existing staging for reference (may be expired)
            let previous = staging.get_staged_npcs(region_id).await.ok()
                .map(|npcs| {
                    Staging::new(
                        region_id,
                        location_id,
                        world_id,
                        current_game_time,
                        "expired",
                        StagingSource::RuleBased,
                        0,
                        current_game_time,
                    ).with_npcs(npcs)
                })
                .filter(|s| !s.npcs.is_empty());

            Ok((vec![], StagingStatus::Pending { previous_staging: previous }))
        }
    }
}
