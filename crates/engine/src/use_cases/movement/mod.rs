//! Movement use cases.

mod enter_region;
mod exit_location;

pub use enter_region::{EnterRegion, EnterRegionError, EnterRegionResult, StagingStatus};
pub use exit_location::{ExitLocation, ExitLocationError};

use std::sync::Arc;
use chrono::{DateTime, Utc};
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId, StagedNpc, Staging, StagingSource, WorldId};
use crate::entities::Staging as StagingEntity;
use crate::infrastructure::ports::RepoError;
use crate::use_cases::time::{SuggestTime, SuggestTimeResult, TimeSuggestion};

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

/// Generate a time suggestion for movement.
///
/// This is shared logic between EnterRegion and ExitLocation use cases.
///
/// # Arguments
/// * `suggest_time` - The SuggestTime use case
/// * `world_id` - World the movement is in
/// * `pc_id` - Player character making the movement
/// * `pc_name` - Character name for suggestion display
/// * `action_type` - "travel_region" or "travel_location"
/// * `destination_name` - Name of destination for display
///
/// # Returns
/// Some(TimeSuggestion) if a suggestion was created, None otherwise
pub async fn suggest_time_for_movement(
    suggest_time: &SuggestTime,
    world_id: WorldId,
    pc_id: PlayerCharacterId,
    pc_name: String,
    action_type: &str,
    destination_name: &str,
) -> Option<TimeSuggestion> {
    match suggest_time.execute(
        world_id,
        pc_id,
        pc_name,
        action_type,
        format!("Travel to {}", destination_name),
    ).await {
        Ok(SuggestTimeResult::SuggestionCreated(suggestion)) => Some(suggestion),
        Ok(SuggestTimeResult::AutoAdvanced { .. }) => {
            // In auto mode, time was advanced - no suggestion needed
            None
        }
        Ok(SuggestTimeResult::NoCost) | Ok(SuggestTimeResult::ManualMode) => None,
        Err(e) => {
            tracing::warn!(
                error = %e,
                action_type = %action_type,
                "Failed to generate time suggestion for movement"
            );
            None
        }
    }
}
