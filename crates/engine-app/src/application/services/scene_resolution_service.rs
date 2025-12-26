//! Scene Resolution Service - Resolves scenes based on player character locations
//!
//! This service determines which scene to show based on where player characters
//! are located in the world. It handles single-location and split-party scenarios.
//!
//! ## Scene Entry Conditions
//!
//! Scenes can have entry conditions that must be met before they are shown:
//! - `CompletedScene(SceneId)` - PC must have completed another scene
//! - `HasItem(ItemId)` - PC must possess a specific item
//! - `KnowsCharacter(CharacterId)` - PC must have observed/met an NPC
//! - `FlagSet(String)` - A game flag must be set (world or PC scope)
//! - `Custom(String)` - Custom condition (evaluated by LLM)

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use wrldbldr_engine_ports::outbound::{
    CharacterRepositoryPort, FlagRepositoryPort, ObservationRepositoryPort,
    PlayerCharacterRepositoryPort, SceneRepositoryPort,
};
use wrldbldr_domain::entities::{Scene, SceneCondition};
use wrldbldr_domain::{LocationId, PlayerCharacterId, WorldId};

/// Result of scene resolution
#[derive(Debug, Clone)]
pub struct SceneResolutionResult {
    /// The resolved scene (if any)
    pub scene: Option<Scene>,
    /// Whether the party is split across multiple locations
    pub is_split_party: bool,
    /// Locations where PCs are located (for split party scenarios)
    pub pc_locations: Vec<LocationId>,
}

/// Scene resolution service trait
#[async_trait]
pub trait SceneResolutionService: Send + Sync {
    /// Resolve the scene for a world based on PC locations
    async fn resolve_scene_for_world(
        &self,
        world_id: &WorldId,
    ) -> Result<SceneResolutionResult>;

    /// Resolve the scene for a specific player character
    async fn resolve_scene_for_pc(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<Scene>>;
}

/// Default implementation of SceneResolutionService
#[derive(Clone)]
pub struct SceneResolutionServiceImpl {
    pc_repository: Arc<dyn PlayerCharacterRepositoryPort>,
    scene_repository: Arc<dyn SceneRepositoryPort>,
    character_repository: Arc<dyn CharacterRepositoryPort>,
    flag_repository: Arc<dyn FlagRepositoryPort>,
    observation_repository: Arc<dyn ObservationRepositoryPort>,
}

impl SceneResolutionServiceImpl {
    /// Create a new SceneResolutionServiceImpl with the given repositories
    pub fn new(
        pc_repository: Arc<dyn PlayerCharacterRepositoryPort>,
        scene_repository: Arc<dyn SceneRepositoryPort>,
        character_repository: Arc<dyn CharacterRepositoryPort>,
        flag_repository: Arc<dyn FlagRepositoryPort>,
        observation_repository: Arc<dyn ObservationRepositoryPort>,
    ) -> Self {
        Self {
            pc_repository,
            scene_repository,
            character_repository,
            flag_repository,
            observation_repository,
        }
    }

    /// Evaluate whether a PC meets all entry conditions for a scene
    async fn evaluate_conditions(
        &self,
        pc_id: PlayerCharacterId,
        world_id: WorldId,
        conditions: &[SceneCondition],
    ) -> Result<bool> {
        for condition in conditions {
            let met = match condition {
                SceneCondition::CompletedScene(scene_id) => {
                    self.scene_repository
                        .is_scene_completed(pc_id, *scene_id)
                        .await?
                }
                SceneCondition::HasItem(item_id) => {
                    // Check if PC has this item in their inventory
                    self.pc_repository
                        .get_inventory_item(pc_id, *item_id)
                        .await?
                        .is_some()
                }
                SceneCondition::KnowsCharacter(character_id) => {
                    // Check if PC has observed this NPC via ObservationRepositoryPort
                    match self.observation_repository.has_observed(pc_id, *character_id).await {
                        Ok(has_observed) => {
                            if !has_observed {
                                debug!(
                                    pc_id = %pc_id,
                                    character_id = %character_id,
                                    "KnowsCharacter condition: PC has not observed this NPC"
                                );
                            }
                            has_observed
                        }
                        Err(e) => {
                            warn!(
                                pc_id = %pc_id,
                                character_id = %character_id,
                                error = %e,
                                "Failed to check observation, treating as not met"
                            );
                            false
                        }
                    }
                }
                SceneCondition::FlagSet(flag_name) => {
                    // Check PC-scoped flag first, then world-scoped
                    let pc_flag = self.flag_repository.get_pc_flag(pc_id, flag_name).await?;
                    if pc_flag {
                        true
                    } else {
                        self.flag_repository.get_world_flag(world_id, flag_name).await?
                    }
                }
                SceneCondition::Custom(description) => {
                    // Custom conditions would need LLM evaluation
                    // For now, log and skip (treat as met)
                    warn!(
                        pc_id = %pc_id,
                        condition = %description,
                        "Custom scene condition not yet evaluated, treating as met"
                    );
                    true
                }
            };

            if !met {
                debug!(
                    pc_id = %pc_id,
                    condition = ?condition,
                    "Scene entry condition not met"
                );
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[async_trait]
impl SceneResolutionService for SceneResolutionServiceImpl {
    #[instrument(skip(self), fields(world_id = %world_id))]
    async fn resolve_scene_for_world(
        &self,
        world_id: &WorldId,
    ) -> Result<SceneResolutionResult> {
        // Get all PCs in the world
        let pcs = self
            .pc_repository
            .get_all_by_world(*world_id)
            .await
            .context("Failed to get player characters for world")?;

        if pcs.is_empty() {
            debug!(world_id = %world_id, "No player characters in world, no scene to resolve");
            return Ok(SceneResolutionResult {
                scene: None,
                is_split_party: false,
                pc_locations: Vec::new(),
            });
        }

        // Group PCs by location
        let mut location_pcs: std::collections::HashMap<LocationId, Vec<_>> = std::collections::HashMap::new();
        for pc in &pcs {
            location_pcs
                .entry(pc.current_location_id)
                .or_insert_with(Vec::new)
                .push(pc);
        }

        let unique_locations: Vec<LocationId> = location_pcs.keys().cloned().collect();

        // If all PCs are at the same location
        if unique_locations.len() == 1 {
            let location_id = unique_locations[0];
            debug!(
                world_id = %world_id,
                location_id = %location_id,
                "All PCs at same location, resolving scene"
            );

            // Find active scene at that location
            let scenes = self
                .scene_repository
                .list_by_location(location_id)
                .await
                .context("Failed to list scenes by location")?;

            // Pick the first scene (could be enhanced to check entry conditions, order, etc.)
            let scene = scenes.into_iter().next();

            Ok(SceneResolutionResult {
                scene,
                is_split_party: false,
                pc_locations: unique_locations,
            })
        } else {
            // Party is split across multiple locations
            info!(
                world_id = %world_id,
                location_count = unique_locations.len(),
                "Party is split across {} locations",
                unique_locations.len()
            );

            // For split party, we could:
            // 1. Return None and let DM choose
            // 2. Return the first PC's location scene
            // 3. Return a special "split party" scene

            // For now, return the scene at the first location (first PC's location)
            let first_location = unique_locations[0];
            let scenes = self
                .scene_repository
                .list_by_location(first_location)
                .await
                .context("Failed to list scenes by location")?;

            let scene = scenes.into_iter().next();

            Ok(SceneResolutionResult {
                scene,
                is_split_party: true,
                pc_locations: unique_locations,
            })
        }
    }

    #[instrument(skip(self), fields(pc_id = %pc_id))]
    async fn resolve_scene_for_pc(&self, pc_id: PlayerCharacterId) -> Result<Option<Scene>> {
        // Get the PC
        let pc = self
            .pc_repository
            .get(pc_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Player character not found: {}", pc_id))?;

        // Find scenes at PC's location
        let scenes = self
            .scene_repository
            .list_by_location(pc.current_location_id)
            .await
            .context("Failed to list scenes by location")?;

        // Find the first scene where PC meets all entry conditions
        for scene in scenes {
            // Skip scenes with no entry conditions (always accessible)
            if scene.entry_conditions.is_empty() {
                debug!(
                    pc_id = %pc_id,
                    scene_id = %scene.id,
                    "Scene has no entry conditions, selecting"
                );
                return Ok(Some(scene));
            }

            // Evaluate entry conditions
            let conditions_met = self
                .evaluate_conditions(pc_id, pc.world_id, &scene.entry_conditions)
                .await?;

            if conditions_met {
                debug!(
                    pc_id = %pc_id,
                    scene_id = %scene.id,
                    "All entry conditions met, selecting scene"
                );
                return Ok(Some(scene));
            } else {
                debug!(
                    pc_id = %pc_id,
                    scene_id = %scene.id,
                    "Entry conditions not met, skipping scene"
                );
            }
        }

        // No scene with met conditions found
        debug!(
            pc_id = %pc_id,
            location_id = %pc.current_location_id,
            "No scene with met conditions at location"
        );
        Ok(None)
    }
}

