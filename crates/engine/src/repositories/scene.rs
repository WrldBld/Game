//! Scene entity CRUD and resolution operations.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use wrldbldr_domain::{
    self as domain, CharacterId, ItemId, PlayerCharacterId, RegionId, SceneCondition, SceneId,
    TimeContext, TimeOfDay, WorldId,
};

use crate::infrastructure::ports::{RepoError, SceneRepo};

/// Context for evaluating scene entry conditions.
///
/// Provides all the data needed to check if a PC can enter a scene.
#[derive(Debug)]
pub struct SceneResolutionContext {
    /// IDs of scenes the PC has completed
    pub completed_scenes: HashSet<SceneId>,
    /// IDs of items the PC possesses
    pub inventory_items: HashSet<ItemId>,
    /// IDs of characters the PC has observed/met
    pub known_characters: HashSet<CharacterId>,
    /// Flags that are currently set (for FlagSet condition)
    pub flags: HashSet<String>,
    /// Current time of day
    pub time_of_day: TimeOfDay,
    /// Pre-evaluated custom condition results.
    /// Key is the condition description, value is whether the condition is met.
    /// If a custom condition is not in this map, it will be treated as unmet.
    pub custom_condition_results: HashMap<String, bool>,
}

impl SceneResolutionContext {
    pub fn new(time_of_day: TimeOfDay) -> Self {
        Self {
            completed_scenes: HashSet::new(),
            inventory_items: HashSet::new(),
            known_characters: HashSet::new(),
            flags: HashSet::new(),
            time_of_day,
            custom_condition_results: HashMap::new(),
        }
    }

    pub fn with_completed_scenes(mut self, scenes: impl IntoIterator<Item = SceneId>) -> Self {
        self.completed_scenes = scenes.into_iter().collect();
        self
    }

    pub fn with_inventory(mut self, items: impl IntoIterator<Item = ItemId>) -> Self {
        self.inventory_items = items.into_iter().collect();
        self
    }

    pub fn with_known_characters(
        mut self,
        characters: impl IntoIterator<Item = CharacterId>,
    ) -> Self {
        self.known_characters = characters.into_iter().collect();
        self
    }

    pub fn with_flags(mut self, flags: impl IntoIterator<Item = String>) -> Self {
        self.flags = flags.into_iter().collect();
        self
    }

    /// Add pre-evaluated custom condition results.
    ///
    /// These will be used when evaluating `SceneCondition::Custom` variants
    /// instead of treating them as unmet.
    pub fn with_custom_condition_results(
        mut self,
        results: impl IntoIterator<Item = (String, bool)>,
    ) -> Self {
        self.custom_condition_results = results.into_iter().collect();
        self
    }

    /// Add a single custom condition result.
    pub fn add_custom_condition_result(&mut self, description: String, met: bool) {
        self.custom_condition_results.insert(description, met);
    }
}

/// Result of scene resolution.
#[derive(Debug)]
pub struct SceneResolutionResult {
    /// The resolved scene, if any
    pub scene: Option<domain::Scene>,
    /// Scenes that were considered but didn't match
    pub considered_scenes: Vec<SceneConsideration>,
}

/// Record of a scene that was considered during resolution.
#[derive(Debug)]
pub struct SceneConsideration {
    pub scene_id: SceneId,
    pub scene_name: String,
    /// Why this scene wasn't selected (empty if matched)
    pub unmet_conditions: Vec<String>,
    /// Whether all conditions were met
    pub conditions_met: bool,
}

/// Scene entity operations.
pub struct SceneRepository {
    repo: Arc<dyn SceneRepo>,
}

impl SceneRepository {
    pub fn new(repo: Arc<dyn SceneRepo>) -> Self {
        Self { repo }
    }

    pub async fn get(&self, id: SceneId) -> Result<Option<domain::Scene>, RepoError> {
        self.repo.get(id).await
    }

    pub async fn save(&self, scene: &domain::Scene) -> Result<(), RepoError> {
        self.repo.save(scene).await
    }

    /// Delete a scene by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete(&self, id: SceneId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }

    pub async fn get_current(&self, world_id: WorldId) -> Result<Option<domain::Scene>, RepoError> {
        self.repo.get_current(world_id).await
    }

    pub async fn set_current(&self, world_id: WorldId, scene_id: SceneId) -> Result<(), RepoError> {
        self.repo.set_current(world_id, scene_id).await
    }

    pub async fn list_for_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<domain::Scene>, RepoError> {
        self.repo.list_for_region(region_id).await
    }

    pub async fn list_for_act(
        &self,
        act_id: domain::ActId,
    ) -> Result<Vec<domain::Scene>, RepoError> {
        self.repo.list_for_act(act_id).await
    }

    pub async fn get_featured_characters(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<domain::SceneCharacter>, RepoError> {
        self.repo.get_featured_characters(scene_id).await
    }

    pub async fn set_featured_characters(
        &self,
        scene_id: SceneId,
        characters: &[domain::SceneCharacter],
    ) -> Result<(), RepoError> {
        self.repo
            .set_featured_characters(scene_id, characters)
            .await
    }

    // =========================================================================
    // Scene Completion Tracking
    // =========================================================================

    /// Check if a PC has completed a specific scene.
    pub async fn has_completed_scene(
        &self,
        pc_id: PlayerCharacterId,
        scene_id: SceneId,
    ) -> Result<bool, RepoError> {
        self.repo.has_completed_scene(pc_id, scene_id).await
    }

    /// Mark a scene as completed for a PC.
    pub async fn mark_scene_completed(
        &self,
        pc_id: PlayerCharacterId,
        scene_id: SceneId,
    ) -> Result<(), RepoError> {
        self.repo.mark_scene_completed(pc_id, scene_id).await
    }

    /// Get all completed scene IDs for a PC.
    pub async fn get_completed_scenes(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<SceneId>, RepoError> {
        self.repo.get_completed_scenes(pc_id).await
    }

    // =========================================================================
    // Scene Resolution
    // =========================================================================

    /// Get all unique custom condition descriptions from scenes in a region.
    ///
    /// This allows callers to pre-evaluate custom conditions via LLM before
    /// calling `resolve_scene`. Returns unique condition descriptions.
    pub async fn get_custom_conditions_for_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<String>, RepoError> {
        let scenes = self.repo.list_for_region(region_id).await?;

        let mut conditions = std::collections::HashSet::new();
        for scene in scenes {
            for condition in scene.entry_conditions() {
                if let SceneCondition::Custom(desc) = condition {
                    conditions.insert(desc.clone());
                }
            }
        }

        Ok(conditions.into_iter().collect())
    }

    /// Resolve which scene to display for a PC at a given region.
    ///
    /// Evaluates all scenes at the region, filtering by time context and entry conditions.
    /// Returns the highest-order scene whose conditions are all met.
    ///
    /// # Arguments
    /// * `region_id` - The region to find scenes for
    /// * `context` - The evaluation context with PC state
    ///
    /// # Returns
    /// * `SceneResolutionResult` with the matched scene (if any) and considered scenes
    pub async fn resolve_scene(
        &self,
        region_id: RegionId,
        context: &SceneResolutionContext,
    ) -> Result<SceneResolutionResult, RepoError> {
        // Get all scenes at this region
        let scenes = self.repo.list_for_region(region_id).await?;

        if scenes.is_empty() {
            return Ok(SceneResolutionResult {
                scene: None,
                considered_scenes: vec![],
            });
        }

        let mut considered = Vec::new();
        let mut matched_scenes: Vec<domain::Scene> = Vec::new();

        for scene in scenes {
            // Check time context match
            let time_matches = self.check_time_context(scene.time_context(), context.time_of_day);

            // Check all entry conditions
            let (conditions_met, unmet) =
                self.evaluate_conditions(scene.entry_conditions(), context);

            let mut unmet_conditions = unmet;
            if !time_matches {
                unmet_conditions.push(format!(
                    "Time mismatch: scene requires {:?}, current is {:?}",
                    scene.time_context(),
                    context.time_of_day
                ));
            }

            let all_conditions_met = conditions_met && time_matches;

            considered.push(SceneConsideration {
                scene_id: scene.id(),
                scene_name: scene.name().to_string(),
                unmet_conditions: unmet_conditions.clone(),
                conditions_met: all_conditions_met,
            });

            if all_conditions_met {
                matched_scenes.push(scene);
            }
        }

        // Sort by order (highest first) and take the first match
        matched_scenes.sort_by(|a, b| b.order().cmp(&a.order()));
        let scene = matched_scenes.into_iter().next();

        Ok(SceneResolutionResult {
            scene,
            considered_scenes: considered,
        })
    }

    /// Check if a scene's time context matches the current time.
    fn check_time_context(&self, time_context: &TimeContext, current_time: TimeOfDay) -> bool {
        match time_context {
            TimeContext::Unspecified => true, // Always matches
            TimeContext::TimeOfDay(required) => *required == current_time,
            TimeContext::During(event_name) => {
                // KNOWN LIMITATION: Event-based time contexts require event tracking
                // which is not yet integrated. For now, During() always matches.
                // TODO: Add current_event field to scene resolution context
                tracing::debug!(event = %event_name, "Event-based TimeContext not evaluated - assuming match");
                true
            }
            TimeContext::Custom(desc) => {
                // KNOWN LIMITATION: Custom time contexts require LLM evaluation.
                // For now, Custom() always matches.
                // TODO: Implement custom time context evaluation via LLM
                tracing::debug!(description = %desc, "Custom TimeContext not evaluated - assuming match");
                true
            }
        }
    }

    /// Evaluate all entry conditions for a scene.
    ///
    /// Returns (all_met, list_of_unmet_conditions).
    fn evaluate_conditions(
        &self,
        conditions: &[SceneCondition],
        context: &SceneResolutionContext,
    ) -> (bool, Vec<String>) {
        if conditions.is_empty() {
            return (true, vec![]);
        }

        let mut unmet = Vec::new();

        for condition in conditions {
            match condition {
                SceneCondition::CompletedScene(scene_id) => {
                    if !context.completed_scenes.contains(scene_id) {
                        unmet.push(format!("Scene not completed: {}", scene_id));
                    }
                }
                SceneCondition::HasItem(item_id) => {
                    if !context.inventory_items.contains(item_id) {
                        unmet.push(format!("Missing item: {}", item_id));
                    }
                }
                SceneCondition::KnowsCharacter(char_id) => {
                    if !context.known_characters.contains(char_id) {
                        unmet.push(format!("Character not known: {}", char_id));
                    }
                }
                SceneCondition::FlagSet(flag) => {
                    if !context.flags.contains(flag) {
                        unmet.push(format!("Flag not set: {}", flag));
                    }
                }
                SceneCondition::Custom(expr) => {
                    // Check if this custom condition has been pre-evaluated via LLM
                    if let Some(&is_met) = context.custom_condition_results.get(expr) {
                        if !is_met {
                            unmet.push(format!("Custom condition not met: {}", expr));
                        }
                        tracing::debug!(
                            expression = %expr,
                            is_met = %is_met,
                            "Custom condition evaluated via LLM"
                        );
                    } else {
                        // No pre-evaluated result available - treat as unmet
                        // This happens when LLM evaluation is not available or failed
                        tracing::warn!(
                            expression = %expr,
                            "Custom scene condition not pre-evaluated - treating as unmet"
                        );
                        unmet.push(format!("Custom condition not evaluated: {}", expr));
                    }
                }
            }
        }

        let all_met = unmet.is_empty();
        (all_met, unmet)
    }
}
