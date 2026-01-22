//! Movement use cases.

mod can_move;
mod enter_region;
mod exit_location;
mod get_exits;
pub mod scene_change;

#[cfg(test)]
mod tests;

pub use enter_region::{EnterRegion, EnterRegionError, StagingStatus};
pub use exit_location::{ExitLocation, ExitLocationError};
pub use get_exits::GetRegionExits;
pub use scene_change::SceneChangeBuilder;

use crate::infrastructure::ports::{
    FlagRepo, LocationStateRepo, ObservationRepo, PlayerCharacterRepo, RegionStateRepo, RepoError,
    SceneRepo, StagingRepo,
};
use crate::use_cases::custom_condition::{CustomConditionEvaluator, EvaluationContext};
use crate::use_cases::scene::{ResolveScene, SceneResolutionContext};
use crate::use_cases::time::{SuggestTime, SuggestTimeResult, TimeSuggestion};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use wrldbldr_domain::{
    GameTime, LocationId, LocationStateId, PlayerCharacterId, RegionId, RegionStateId,
    Scene as DomainScene, StagedNpc, Staging, StagingSource, WorldId,
};

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

/// Resolve staging for a region, returning the visible NPCs, status, and visual state.
///
/// This is shared logic between EnterRegion and ExitLocation use cases.
///
/// # Arguments
/// * `staging` - The staging repository
/// * `location_state_repo` - Location state repository for visual state
/// * `region_state_repo` - Region state repository for visual state
/// * `region_id` - The region to resolve staging for
/// * `location_id` - The location containing of region
/// * `world_id` - The world containing of location
/// * `current_game_time_seconds` - Current game time in total seconds since epoch
/// * `real_timestamp` - Real-world timestamp for audit purposes
///
/// # Returns
/// A tuple of (visible NPCs, staging status, optional visual state)
pub async fn resolve_staging_for_region(
    staging: &dyn StagingRepo,
    location_state_repo: &dyn LocationStateRepo,
    region_state_repo: &dyn RegionStateRepo,
    region_id: RegionId,
    location_id: LocationId,
    world_id: WorldId,
    current_game_time_seconds: i64,
    real_timestamp: DateTime<Utc>,
) -> Result<(Vec<StagedNpc>, StagingStatus, Option<crate::use_cases::staging::ResolvedVisualState>), RepoError> {
    let active_staging = staging
        .get_active_staging(region_id, current_game_time_seconds)
        .await?;

    match active_staging {
        Some(s) => {
            // Valid staging exists - resolve NPCs visible to players
            let visible_npcs: Vec<StagedNpc> = s
                .npcs()
                .iter()
                .filter(|npc| npc.is_visible_to_players())
                .cloned()
                .collect();

            // Data integrity check: active staging should have visual state IDs
            // If both are None, this is a data integrity error - staging was approved
            // without visual state IDs, which should never happen
            if s.location_state_id().is_none() && s.region_state_id().is_none() {
                return Err(RepoError::database(
                    "staging_integrity",
                    format!(
                        "Active staging {} has no visual state IDs. This indicates data integrity - staging was approved without resolving visual state IDs.",
                        s.id()
                    ),
                ));
            }

            // Resolve visual state from active staging - fail-fast on errors
            let visual_state = resolve_visual_state_from_staging(
                s.location_state_id(),
                s.region_state_id(),
                location_state_repo,
                region_state_repo,
            )
            .await?;

            Ok((visible_npcs, StagingStatus::Ready, visual_state))
        }
        None => {
            // No valid staging - DM approval required
            // Try to get any existing staging for reference (may be expired)
            let previous = match staging.get_staged_npcs(region_id).await {
                Ok(npcs) => Some(npcs),
                Err(e) => {
                    tracing::warn!(
                        region_id = %region_id,
                        error = %e,
                        "Failed to get existing staging for reference - continuing without previous staging"
                    );
                    None
                }
            }
            .map(|npcs| {
                Staging::new(
                    region_id,
                    location_id,
                    world_id,
                    current_game_time_seconds,
                    "expired",
                    StagingSource::RuleBased,
                    0,
                    real_timestamp,
                )
                .with_npcs(npcs)
            })
            .filter(|s| !s.npcs().is_empty());

            Ok((
                vec![],
                StagingStatus::Pending {
                    previous_staging: Box::new(previous),
                },
                None,
            ))
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
    match suggest_time
        .execute(
            world_id,
            pc_id,
            pc_name,
            action_type,
            format!("Travel to {}", destination_name),
        )
        .await
    {
        Ok(SuggestTimeResult::SuggestionCreated(suggestion)) => Some(suggestion),
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

/// Resolve which scene to display for a PC entering a region.
///
/// This is shared logic between EnterRegion and ExitLocation use cases.
///
/// Builds the evaluation context from the PC's state (inventory, observations, completed scenes, flags)
/// and calls the scene resolution use case.
///
/// # Arguments
/// * `resolve_scene` - Scene resolution use case
/// * `scene` - Scene repository for completion tracking
/// * `inventory` - Inventory repository for PC items
/// * `observation` - Observation repository for known characters
/// * `flag` - Flag repository for flag state
/// * `pc_id` - Player character ID
/// * `world_id` - World ID for flags
/// * `region_id` - Region to resolve scene for
/// * `game_time` - Current game time for time-of-day checks
///
/// # Returns
/// The resolved scene, if any matches the conditions
pub async fn resolve_scene_for_region(
    resolve_scene: &ResolveScene,
    scene: &dyn SceneRepo,
    pc_repo: &dyn PlayerCharacterRepo,
    observation: &dyn ObservationRepo,
    flag: &dyn FlagRepo,
    pc_id: PlayerCharacterId,
    world_id: WorldId,
    region_id: RegionId,
    game_time: &GameTime,
) -> Result<Option<DomainScene>, RepoError> {
    resolve_scene_for_region_with_evaluator(
        resolve_scene,
        scene,
        pc_repo,
        observation,
        flag,
        pc_id,
        world_id,
        region_id,
        game_time,
        None,
        None,
    )
    .await
}

/// Resolve scene for a region with optional LLM-based custom condition evaluation.
///
/// This is shared logic between EnterRegion and ExitLocation use cases.
/// When a custom condition evaluator is provided, custom scene conditions will
/// be evaluated via LLM instead of being treated as unmet.
///
/// # Arguments
/// * `resolve_scene` - Scene resolution use case
/// * `scene` - Scene repository for completion tracking
/// * `pc_repo` - Player character repository for PC inventory
/// * `observation` - Observation repository for known characters
/// * `flag` - Flag repository for flag state
/// * `pc_id` - Player character ID
/// * `world_id` - World ID for flags
/// * `region_id` - Region to resolve scene for
/// * `game_time` - Current game time for time-of-day checks
/// * `custom_evaluator` - Optional LLM-based custom condition evaluator
/// * `location_description` - Optional location description for LLM context
///
/// # Returns
/// The resolved scene, if any matches the conditions
pub async fn resolve_scene_for_region_with_evaluator(
    resolve_scene: &ResolveScene,
    scene: &dyn SceneRepo,
    pc_repo: &dyn PlayerCharacterRepo,
    observation: &dyn ObservationRepo,
    flag: &dyn FlagRepo,
    pc_id: PlayerCharacterId,
    world_id: WorldId,
    region_id: RegionId,
    game_time: &GameTime,
    custom_evaluator: Option<&CustomConditionEvaluator>,
    location_description: Option<&str>,
) -> Result<Option<DomainScene>, RepoError> {
    // Get current time of day from the world's game time (not wall clock)
    let time_of_day = game_time.time_of_day();

    // Build the scene resolution context
    let completed_scenes = scene.get_completed_scenes(pc_id).await?;
    let inventory_items = pc_repo.get_inventory(pc_id).await?;
    let observations = observation.get_observations(pc_id).await?;
    // Combine world and PC flags
    let world_flags = flag.get_world_flags(world_id).await?;
    let pc_flags = flag.get_pc_flags(pc_id).await?;
    let mut flags: Vec<String> = world_flags;
    for f in pc_flags {
        if !flags.contains(&f) {
            flags.push(f);
        }
    }

    // Extract item names and flag names for LLM context
    let inventory_names: Vec<String> = inventory_items
        .iter()
        .map(|i| i.name.as_str().to_string())
        .collect();
    // LIMITATION: We use character IDs (UUIDs) instead of names because NpcObservation
    // only stores IDs. Fetching names would require an additional repository call per
    // character. This is acceptable for now as the LLM can still match conditions like
    // "has met the blacksmith" if the ID is consistent. Future improvement: batch fetch
    // character names via a dedicated method on the Character entity.
    let known_character_ids: Vec<String> = observations
        .iter()
        .map(|o| o.npc_id().to_string())
        .collect();
    let flag_names: Vec<String> = flags.clone();

    let mut context = SceneResolutionContext::new(time_of_day)
        .with_completed_scenes(completed_scenes)
        .with_inventory(inventory_items.into_iter().map(|item| item.id))
        .with_known_characters(observations.into_iter().map(|obs| obs.npc_id()))
        .with_flags(flags);

    // If custom evaluator is provided, pre-evaluate custom conditions via LLM
    if let Some(evaluator) = custom_evaluator {
        let custom_conditions = resolve_scene
            .get_custom_conditions_for_region(region_id)
            .await?;

        if !custom_conditions.is_empty() {
            tracing::debug!(
                region_id = %region_id,
                conditions = ?custom_conditions,
                "Pre-evaluating custom scene conditions via LLM"
            );

            // Build evaluation context for LLM
            let eval_context = EvaluationContext::new()
                .with_time_of_day(format!("{:?}", time_of_day))
                .with_inventory(inventory_names)
                .with_known_characters(known_character_ids)
                .with_flags(flag_names)
                .with_location(location_description.unwrap_or("Unknown location"));

            // Evaluate each custom condition
            for condition_desc in custom_conditions {
                match evaluator.evaluate(&condition_desc, &eval_context).await {
                    Ok(result) => {
                        let is_met = evaluator.is_condition_met(&result);
                        tracing::debug!(
                            condition = %condition_desc,
                            is_met = %is_met,
                            confidence = %result.confidence,
                            reasoning = %result.reasoning,
                            "Custom condition evaluated"
                        );
                        context.add_custom_condition_result(condition_desc, is_met);
                    }
                    Err(e) => {
                        // Log error and explicitly mark as unmet to prevent duplicate warnings
                        // in scene resolution (which would also log if condition is missing)
                        tracing::warn!(
                            error = %e,
                            condition = %condition_desc,
                            "Failed to evaluate custom condition via LLM - treating as unmet"
                        );
                        context.add_custom_condition_result(condition_desc, false);
                    }
                }
            }
        }
    }

    // Resolve the scene
    let result = resolve_scene.execute(region_id, &context).await?;

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

/// Resolve visual state from active staging by fetching state details.
///
/// Returns Result - fail-fast if state IDs exist but fetching fails.
async fn resolve_visual_state_from_staging(
    location_state_id: Option<LocationStateId>,
    region_state_id: Option<RegionStateId>,
    location_state_repo: &dyn LocationStateRepo,
    region_state_repo: &dyn RegionStateRepo,
) -> Result<Option<crate::use_cases::staging::ResolvedVisualState>, RepoError> {
    use crate::use_cases::staging::{ResolvedStateInfo, ResolvedVisualState};

    // If no visual state IDs are set, return None
    if location_state_id.is_none() && region_state_id.is_none() {
        return Ok(None);
    }

    let location_state = if let Some(loc_id) = location_state_id {
        match location_state_repo.get(loc_id).await? {
            Some(state) => Some(ResolvedStateInfo {
                id: state.id().to_string(),
                name: state.name().to_string(),
                backdrop_override: state.backdrop_override().map(|s| s.to_string()),
                atmosphere_override: state.atmosphere_override().map(|s| s.to_string()),
                ambient_sound: state.ambient_sound().map(|s| s.to_string()),
            }),
            None => {
                tracing::warn!(
                    location_state_id = %loc_id,
                    "Location state ID not found for visual state"
                );
                return Err(RepoError::not_found(
                    "LocationState",
                    loc_id.to_string(),
                ));
            }
        }
    } else {
        None
    };

    let region_state = if let Some(reg_id) = region_state_id {
        match region_state_repo.get(reg_id).await? {
            Some(state) => Some(ResolvedStateInfo {
                id: state.id().to_string(),
                name: state.name().to_string(),
                backdrop_override: state.backdrop_override().map(|s| s.to_string()),
                atmosphere_override: state.atmosphere_override().map(|s| s.to_string()),
                ambient_sound: state.ambient_sound().map(|s| s.to_string()),
            }),
            None => {
                tracing::warn!(
                    region_state_id = %reg_id,
                    "Region state ID not found for visual state"
                );
                return Err(RepoError::not_found(
                    "RegionState",
                    reg_id.to_string(),
                ));
            }
        }
    } else {
        None
    };

    // Return None if both are None
    if location_state.is_none() && region_state.is_none() {
        return Ok(None);
    }

    Ok(Some(ResolvedVisualState {
        location_state,
        region_state,
    }))
}
