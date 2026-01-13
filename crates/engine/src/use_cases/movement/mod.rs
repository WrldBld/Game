//! Movement use cases.

mod enter_region;
mod exit_location;
pub mod scene_change;

pub use enter_region::{EnterRegion, EnterRegionError, EnterRegionResult, StagingStatus};
pub use exit_location::{ExitLocation, ExitLocationError};
pub use scene_change::{
    NavigationExitInfo, NavigationInfo, NavigationTargetInfo, NpcPresenceInfo, RegionInfo,
    RegionItemInfo, SceneChangeBuilder, SceneChangeData, SceneChangeError,
};

use crate::repositories::{
    Flag, Inventory, Observation,
};
use crate::repositories::scene::{Scene, SceneResolutionContext};
use crate::repositories::staging::Staging as StagingEntity;
use crate::infrastructure::ports::RepoError;
use crate::use_cases::custom_condition::{CustomConditionEvaluator, EvaluationContext};
use crate::use_cases::time::{SuggestTime, SuggestTimeResult, TimeSuggestion};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use wrldbldr_domain::{
    GameTime, LocationId, PlayerCharacterId, RegionId, Scene as DomainScene, StagedNpc, Staging,
    StagingSource, WorldId,
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
    let active_staging = staging
        .get_active_staging(region_id, current_game_time)
        .await?;

    match active_staging {
        Some(s) => {
            // Valid staging exists - resolve NPCs visible to players
            let visible_npcs: Vec<StagedNpc> = s
                .npcs
                .into_iter()
                .filter(|npc| npc.is_visible_to_players())
                .collect();
            Ok((visible_npcs, StagingStatus::Ready))
        }
        None => {
            // No valid staging - DM approval required
            // Try to get any existing staging for reference (may be expired)
            let previous = staging
                .get_staged_npcs(region_id)
                .await
                .ok()
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
                    )
                    .with_npcs(npcs)
                })
                .filter(|s| !s.npcs.is_empty());

            Ok((
                vec![],
                StagingStatus::Pending {
                    previous_staging: previous,
                },
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
/// and calls the scene resolution service.
///
/// # Arguments
/// * `scene` - Scene entity for resolution
/// * `inventory` - Inventory entity for PC items
/// * `observation` - Observation entity for known characters
/// * `flag` - Flag entity for flag state
/// * `pc_id` - Player character ID
/// * `world_id` - World ID for flags
/// * `region_id` - Region to resolve scene for
/// * `game_time` - Current game time for time-of-day checks
///
/// # Returns
/// The resolved scene, if any matches the conditions
pub async fn resolve_scene_for_region(
    scene: &Scene,
    inventory: &Inventory,
    observation: &Observation,
    flag: &Flag,
    pc_id: PlayerCharacterId,
    world_id: WorldId,
    region_id: RegionId,
    game_time: &GameTime,
) -> Result<Option<DomainScene>, RepoError> {
    resolve_scene_for_region_with_evaluator(
        scene,
        inventory,
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
/// * `scene` - Scene entity for resolution
/// * `inventory` - Inventory entity for PC items
/// * `observation` - Observation entity for known characters
/// * `flag` - Flag entity for flag state
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
    scene: &Scene,
    inventory: &Inventory,
    observation: &Observation,
    flag: &Flag,
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
    let inventory_items = inventory.get_pc_inventory(pc_id).await?;
    let observations = observation.get_observations(pc_id).await?;
    let flags = flag.get_all_flags_for_pc(world_id, pc_id).await?;

    // Extract item names and flag names for LLM context
    let inventory_names: Vec<String> = inventory_items.iter().map(|i| i.name.clone()).collect();
    // LIMITATION: We use character IDs (UUIDs) instead of names because NpcObservation
    // only stores IDs. Fetching names would require an additional repository call per
    // character. This is acceptable for now as the LLM can still match conditions like
    // "has met the blacksmith" if the ID is consistent. Future improvement: batch fetch
    // character names via a dedicated method on the Character entity.
    let known_character_ids: Vec<String> = observations.iter().map(|o| o.npc_id.to_string()).collect();
    let flag_names: Vec<String> = flags.clone();

    let mut context = SceneResolutionContext::new(time_of_day)
        .with_completed_scenes(completed_scenes)
        .with_inventory(inventory_items.into_iter().map(|item| item.id))
        .with_known_characters(observations.into_iter().map(|obs| obs.npc_id))
        .with_flags(flags);

    // If custom evaluator is provided, pre-evaluate custom conditions via LLM
    if let Some(evaluator) = custom_evaluator {
        let custom_conditions = scene.get_custom_conditions_for_region(region_id).await?;

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
    let result = scene.resolve_scene(region_id, &context).await?;

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
