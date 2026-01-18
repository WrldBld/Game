// Scene resolution - methods for future scene features
#![allow(dead_code)]

//! Scene resolution use case.
//!
//! Evaluates scene entry conditions to determine which scene to display
//! for a player character at a given region.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use wrldbldr_domain::{
    self as domain, CharacterId, ItemId, PlayerCharacterId, RegionId, SceneCondition, SceneId,
    TimeContext, TimeOfDay,
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

/// Scene resolution use case.
///
/// Evaluates scene entry conditions to determine which scene to display
/// for a player character at a given region.
pub struct ResolveScene {
    scene_repo: Arc<dyn SceneRepo>,
}

impl ResolveScene {
    pub fn new(scene_repo: Arc<dyn SceneRepo>) -> Self {
        Self { scene_repo }
    }

    /// Get all unique custom condition descriptions from scenes in a region.
    ///
    /// This allows callers to pre-evaluate custom conditions via LLM before
    /// calling `execute`. Returns unique condition descriptions.
    pub async fn get_custom_conditions_for_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<String>, RepoError> {
        let scenes = self.scene_repo.list_for_region(region_id).await?;

        let mut conditions = HashSet::new();
        for scene in scenes {
            for condition in scene.entry_conditions() {
                if let SceneCondition::Custom(desc) = condition {
                    conditions.insert(desc.clone());
                }
            }
        }

        Ok(conditions.into_iter().collect())
    }

    /// Get all completed scene IDs for a PC.
    ///
    /// Convenience method that delegates to the SceneRepo.
    pub async fn get_completed_scenes(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<SceneId>, RepoError> {
        self.scene_repo.get_completed_scenes(pc_id).await
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
    pub async fn execute(
        &self,
        region_id: RegionId,
        context: &SceneResolutionContext,
    ) -> Result<SceneResolutionResult, RepoError> {
        // Get all scenes at this region
        let scenes = self.scene_repo.list_for_region(region_id).await?;

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
        matched_scenes.sort_by_key(|b| std::cmp::Reverse(b.order()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::MockSceneRepo;
    use wrldbldr_domain::{ActId, CharacterId, ItemId, SceneCondition, SceneName, TimeContext};

    /// Helper to create a test scene with specified properties
    fn create_test_scene(
        name: &str,
        order: u32,
        time_context: TimeContext,
        conditions: Vec<SceneCondition>,
    ) -> domain::Scene {
        let act_id = ActId::new();
        domain::Scene::new(act_id, SceneName::new(name).unwrap())
            .with_order(order)
            .with_time(time_context)
            .with_entry_conditions(conditions)
    }

    // =========================================================================
    // Basic Resolution Tests
    // =========================================================================

    #[tokio::test]
    async fn when_no_scenes_returns_none() {
        let region_id = RegionId::new();
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(|_| Ok(vec![]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_none());
        assert!(result.considered_scenes.is_empty());
    }

    #[tokio::test]
    async fn when_single_matching_scene_returns_it() {
        let region_id = RegionId::new();
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        let scene = create_test_scene("The Opening", 1, TimeContext::Unspecified, vec![]);
        let scene_id = scene.id();
        let scene_name = scene.name().to_string();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_some());
        let resolved_scene = result.scene.unwrap();
        assert_eq!(resolved_scene.id(), scene_id);
        assert_eq!(resolved_scene.name().to_string(), scene_name);

        assert_eq!(result.considered_scenes.len(), 1);
        assert!(result.considered_scenes[0].conditions_met);
    }

    #[tokio::test]
    async fn when_multiple_scenes_returns_highest_priority() {
        let region_id = RegionId::new();
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        let low_priority = create_test_scene("Low Priority", 1, TimeContext::Unspecified, vec![]);
        let high_priority =
            create_test_scene("High Priority", 10, TimeContext::Unspecified, vec![]);
        let mid_priority = create_test_scene("Mid Priority", 5, TimeContext::Unspecified, vec![]);

        let high_priority_id = high_priority.id();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| {
                Ok(vec![
                    low_priority.clone(),
                    high_priority.clone(),
                    mid_priority.clone(),
                ])
            });

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_some());
        let resolved_scene = result.scene.unwrap();
        assert_eq!(resolved_scene.id(), high_priority_id);
        assert_eq!(resolved_scene.order(), 10);

        // All 3 scenes should have been considered
        assert_eq!(result.considered_scenes.len(), 3);
    }

    // =========================================================================
    // Condition Evaluation Tests
    // =========================================================================

    #[tokio::test]
    async fn when_no_conditions_scene_matches() {
        // A scene with no entry conditions should always match (equivalent to "Always")
        let region_id = RegionId::new();
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        let scene = create_test_scene("No Conditions", 1, TimeContext::Unspecified, vec![]);
        let scene_id = scene.id();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_some());
        assert_eq!(result.scene.unwrap().id(), scene_id);
    }

    #[tokio::test]
    async fn when_completed_scene_condition_met_matches() {
        let region_id = RegionId::new();
        let prerequisite_scene_id = SceneId::new();

        // Context where PC has completed the prerequisite scene
        let context = SceneResolutionContext::new(TimeOfDay::Morning)
            .with_completed_scenes(vec![prerequisite_scene_id]);

        let scene = create_test_scene(
            "After Intro",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::CompletedScene(prerequisite_scene_id)],
        );
        let scene_id = scene.id();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_some());
        assert_eq!(result.scene.unwrap().id(), scene_id);
        assert!(result.considered_scenes[0].conditions_met);
    }

    #[tokio::test]
    async fn when_completed_scene_condition_not_met_skipped() {
        let region_id = RegionId::new();
        let prerequisite_scene_id = SceneId::new();

        // Context where PC has NOT completed the prerequisite scene
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        let scene = create_test_scene(
            "After Intro",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::CompletedScene(prerequisite_scene_id)],
        );

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_none());
        assert!(!result.considered_scenes[0].conditions_met);
        assert!(result.considered_scenes[0].unmet_conditions[0].contains("Scene not completed"));
    }

    #[tokio::test]
    async fn when_has_item_condition_met_matches() {
        let region_id = RegionId::new();
        let required_item_id = ItemId::new();

        // Context where PC has the required item
        let context =
            SceneResolutionContext::new(TimeOfDay::Morning).with_inventory(vec![required_item_id]);

        let scene = create_test_scene(
            "Has Key",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::HasItem(required_item_id)],
        );
        let scene_id = scene.id();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_some());
        assert_eq!(result.scene.unwrap().id(), scene_id);
    }

    #[tokio::test]
    async fn when_has_item_condition_not_met_skipped() {
        let region_id = RegionId::new();
        let required_item_id = ItemId::new();

        // Context where PC does NOT have the required item
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        let scene = create_test_scene(
            "Has Key",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::HasItem(required_item_id)],
        );

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_none());
        assert!(result.considered_scenes[0].unmet_conditions[0].contains("Missing item"));
    }

    #[tokio::test]
    async fn when_knows_character_condition_met_matches() {
        let region_id = RegionId::new();
        let known_char_id = CharacterId::new();

        // Context where PC knows the required character
        let context = SceneResolutionContext::new(TimeOfDay::Morning)
            .with_known_characters(vec![known_char_id]);

        let scene = create_test_scene(
            "Met the Baron",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::KnowsCharacter(known_char_id)],
        );
        let scene_id = scene.id();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_some());
        assert_eq!(result.scene.unwrap().id(), scene_id);
    }

    #[tokio::test]
    async fn when_knows_character_condition_not_met_skipped() {
        let region_id = RegionId::new();
        let known_char_id = CharacterId::new();

        // Context where PC does NOT know the required character
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        let scene = create_test_scene(
            "Met the Baron",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::KnowsCharacter(known_char_id)],
        );

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_none());
        assert!(result.considered_scenes[0].unmet_conditions[0].contains("Character not known"));
    }

    #[tokio::test]
    async fn when_flag_set_condition_met_matches() {
        let region_id = RegionId::new();

        // Context where flag is set
        let context = SceneResolutionContext::new(TimeOfDay::Morning)
            .with_flags(vec!["quest_started".to_string()]);

        let scene = create_test_scene(
            "Quest Active",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::FlagSet("quest_started".to_string())],
        );
        let scene_id = scene.id();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_some());
        assert_eq!(result.scene.unwrap().id(), scene_id);
    }

    #[tokio::test]
    async fn when_flag_set_condition_not_met_skipped() {
        let region_id = RegionId::new();

        // Context where flag is NOT set
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        let scene = create_test_scene(
            "Quest Active",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::FlagSet("quest_started".to_string())],
        );

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_none());
        assert!(result.considered_scenes[0].unmet_conditions[0].contains("Flag not set"));
    }

    // =========================================================================
    // Time Context Tests
    // =========================================================================

    #[tokio::test]
    async fn when_time_of_day_matches_scene_selected() {
        let region_id = RegionId::new();

        // Morning context matches morning scene
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        let scene = create_test_scene(
            "Morning Market",
            1,
            TimeContext::TimeOfDay(TimeOfDay::Morning),
            vec![],
        );
        let scene_id = scene.id();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_some());
        assert_eq!(result.scene.unwrap().id(), scene_id);
    }

    #[tokio::test]
    async fn when_time_of_day_mismatches_scene_skipped() {
        let region_id = RegionId::new();

        // Night context does NOT match morning scene
        let context = SceneResolutionContext::new(TimeOfDay::Night);

        let scene = create_test_scene(
            "Morning Market",
            1,
            TimeContext::TimeOfDay(TimeOfDay::Morning),
            vec![],
        );

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_none());
        assert!(!result.considered_scenes[0].conditions_met);
        assert!(result.considered_scenes[0].unmet_conditions[0].contains("Time mismatch"));
    }

    #[tokio::test]
    async fn when_time_unspecified_always_matches() {
        let region_id = RegionId::new();

        // Any time context should match Unspecified
        for time in [
            TimeOfDay::Morning,
            TimeOfDay::Afternoon,
            TimeOfDay::Evening,
            TimeOfDay::Night,
        ] {
            let context = SceneResolutionContext::new(time);

            let scene = create_test_scene("Any Time Scene", 1, TimeContext::Unspecified, vec![]);
            let scene_id = scene.id();

            let mut scene_repo = MockSceneRepo::new();
            scene_repo
                .expect_list_for_region()
                .withf(move |id| *id == region_id)
                .returning(move |_| Ok(vec![scene.clone()]));

            let use_case = ResolveScene::new(Arc::new(scene_repo));
            let result = use_case.execute(region_id, &context).await.unwrap();

            assert!(
                result.scene.is_some(),
                "TimeContext::Unspecified should match {:?}",
                time
            );
            assert_eq!(result.scene.unwrap().id(), scene_id);
        }
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[tokio::test]
    async fn when_condition_not_met_scene_skipped_fallback_selected() {
        let region_id = RegionId::new();
        let required_item_id = ItemId::new();

        // Context without the required item
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        // High priority scene requires item (should be skipped)
        let guarded_scene = create_test_scene(
            "Guarded Room",
            10,
            TimeContext::Unspecified,
            vec![SceneCondition::HasItem(required_item_id)],
        );

        // Low priority fallback scene with no conditions
        let fallback_scene = create_test_scene("Main Hall", 1, TimeContext::Unspecified, vec![]);
        let fallback_id = fallback_scene.id();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![guarded_scene.clone(), fallback_scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        // Fallback should be selected since guarded scene's condition wasn't met
        assert!(result.scene.is_some());
        assert_eq!(result.scene.unwrap().id(), fallback_id);

        // Both scenes should be in considered list
        assert_eq!(result.considered_scenes.len(), 2);
    }

    #[tokio::test]
    async fn when_multiple_conditions_all_must_be_met() {
        let region_id = RegionId::new();
        let required_item_id = ItemId::new();
        let known_char_id = CharacterId::new();

        // Context with only one of two conditions met
        let context =
            SceneResolutionContext::new(TimeOfDay::Morning).with_inventory(vec![required_item_id]);
        // Note: known_characters is NOT set

        let scene = create_test_scene(
            "Complex Scene",
            1,
            TimeContext::Unspecified,
            vec![
                SceneCondition::HasItem(required_item_id),
                SceneCondition::KnowsCharacter(known_char_id),
            ],
        );

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        // Should NOT match because KnowsCharacter condition is not met
        assert!(result.scene.is_none());
        assert!(!result.considered_scenes[0].conditions_met);
        // Should have exactly one unmet condition (KnowsCharacter)
        assert_eq!(result.considered_scenes[0].unmet_conditions.len(), 1);
        assert!(result.considered_scenes[0].unmet_conditions[0].contains("Character not known"));
    }

    #[tokio::test]
    async fn when_all_multiple_conditions_met_scene_matches() {
        let region_id = RegionId::new();
        let required_item_id = ItemId::new();
        let known_char_id = CharacterId::new();
        let prerequisite_scene_id = SceneId::new();

        // Context with all conditions met
        let context = SceneResolutionContext::new(TimeOfDay::Morning)
            .with_inventory(vec![required_item_id])
            .with_known_characters(vec![known_char_id])
            .with_completed_scenes(vec![prerequisite_scene_id])
            .with_flags(vec!["quest_started".to_string()]);

        let scene = create_test_scene(
            "Complex Scene",
            1,
            TimeContext::Unspecified,
            vec![
                SceneCondition::HasItem(required_item_id),
                SceneCondition::KnowsCharacter(known_char_id),
                SceneCondition::CompletedScene(prerequisite_scene_id),
                SceneCondition::FlagSet("quest_started".to_string()),
            ],
        );
        let scene_id = scene.id();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_some());
        assert_eq!(result.scene.unwrap().id(), scene_id);
        assert!(result.considered_scenes[0].conditions_met);
        assert!(result.considered_scenes[0].unmet_conditions.is_empty());
    }

    // =========================================================================
    // Custom Condition Tests
    // =========================================================================

    #[tokio::test]
    async fn when_custom_condition_pre_evaluated_true_matches() {
        let region_id = RegionId::new();

        let context = SceneResolutionContext::new(TimeOfDay::Morning)
            .with_custom_condition_results(vec![(
                "Player has proven their worth".to_string(),
                true,
            )]);

        let scene = create_test_scene(
            "Worthy Scene",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::Custom(
                "Player has proven their worth".to_string(),
            )],
        );
        let scene_id = scene.id();

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_some());
        assert_eq!(result.scene.unwrap().id(), scene_id);
    }

    #[tokio::test]
    async fn when_custom_condition_pre_evaluated_false_skipped() {
        let region_id = RegionId::new();

        let context = SceneResolutionContext::new(TimeOfDay::Morning)
            .with_custom_condition_results(vec![(
                "Player has proven their worth".to_string(),
                false,
            )]);

        let scene = create_test_scene(
            "Worthy Scene",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::Custom(
                "Player has proven their worth".to_string(),
            )],
        );

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_none());
        assert!(
            result.considered_scenes[0].unmet_conditions[0].contains("Custom condition not met")
        );
    }

    #[tokio::test]
    async fn when_custom_condition_not_pre_evaluated_treated_as_unmet() {
        let region_id = RegionId::new();

        // No pre-evaluated custom conditions
        let context = SceneResolutionContext::new(TimeOfDay::Morning);

        let scene = create_test_scene(
            "Worthy Scene",
            1,
            TimeContext::Unspecified,
            vec![SceneCondition::Custom(
                "Player has proven their worth".to_string(),
            )],
        );

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let result = use_case.execute(region_id, &context).await.unwrap();

        assert!(result.scene.is_none());
        assert!(result.considered_scenes[0].unmet_conditions[0]
            .contains("Custom condition not evaluated"));
    }

    // =========================================================================
    // get_custom_conditions_for_region Tests
    // =========================================================================

    #[tokio::test]
    async fn get_custom_conditions_returns_unique_conditions() {
        let region_id = RegionId::new();

        let scene1 = create_test_scene(
            "Scene 1",
            1,
            TimeContext::Unspecified,
            vec![
                SceneCondition::Custom("Condition A".to_string()),
                SceneCondition::Custom("Condition B".to_string()),
            ],
        );

        let scene2 = create_test_scene(
            "Scene 2",
            2,
            TimeContext::Unspecified,
            vec![
                SceneCondition::Custom("Condition A".to_string()), // Duplicate
                SceneCondition::Custom("Condition C".to_string()),
            ],
        );

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene1.clone(), scene2.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let conditions = use_case
            .get_custom_conditions_for_region(region_id)
            .await
            .unwrap();

        // Should have 3 unique conditions (A, B, C)
        assert_eq!(conditions.len(), 3);
        assert!(conditions.contains(&"Condition A".to_string()));
        assert!(conditions.contains(&"Condition B".to_string()));
        assert!(conditions.contains(&"Condition C".to_string()));
    }

    #[tokio::test]
    async fn get_custom_conditions_ignores_non_custom_conditions() {
        let region_id = RegionId::new();
        let item_id = ItemId::new();
        let scene_id = SceneId::new();

        let scene = create_test_scene(
            "Mixed Conditions",
            1,
            TimeContext::Unspecified,
            vec![
                SceneCondition::HasItem(item_id),
                SceneCondition::CompletedScene(scene_id),
                SceneCondition::Custom("Only Custom".to_string()),
                SceneCondition::FlagSet("flag".to_string()),
            ],
        );

        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_list_for_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(vec![scene.clone()]));

        let use_case = ResolveScene::new(Arc::new(scene_repo));
        let conditions = use_case
            .get_custom_conditions_for_region(region_id)
            .await
            .unwrap();

        // Should only have the Custom condition
        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0], "Only Custom");
    }

    // =========================================================================
    // SceneResolutionContext Builder Tests
    // =========================================================================

    #[test]
    fn context_builder_works_correctly() {
        let scene_id = SceneId::new();
        let item_id = ItemId::new();
        let char_id = CharacterId::new();

        let context = SceneResolutionContext::new(TimeOfDay::Evening)
            .with_completed_scenes(vec![scene_id])
            .with_inventory(vec![item_id])
            .with_known_characters(vec![char_id])
            .with_flags(vec!["flag1".to_string(), "flag2".to_string()])
            .with_custom_condition_results(vec![("custom".to_string(), true)]);

        assert_eq!(context.time_of_day, TimeOfDay::Evening);
        assert!(context.completed_scenes.contains(&scene_id));
        assert!(context.inventory_items.contains(&item_id));
        assert!(context.known_characters.contains(&char_id));
        assert!(context.flags.contains("flag1"));
        assert!(context.flags.contains("flag2"));
        assert_eq!(context.custom_condition_results.get("custom"), Some(&true));
    }

    #[test]
    fn add_custom_condition_result_works() {
        let mut context = SceneResolutionContext::new(TimeOfDay::Morning);
        context.add_custom_condition_result("test condition".to_string(), true);

        assert_eq!(
            context.custom_condition_results.get("test condition"),
            Some(&true)
        );
    }
}
