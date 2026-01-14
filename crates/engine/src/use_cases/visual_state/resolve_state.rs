//! Visual state resolution use case.
//!
//! Evaluates activation rules to determine which LocationState and RegionState
//! should be active for a given context.

use std::sync::Arc;

use chrono::Datelike;
use wrldbldr_domain::{
    ActivationEvaluation, ActivationLogic, ActivationRule, CharacterId, GameTime, LocationId,
    LocationState, NarrativeEventId, RegionId, RegionState, WorldId,
};

use crate::infrastructure::ports::RepoError;
use crate::repositories::{Flag, LocationStateEntity, RegionStateEntity};

/// Context for evaluating activation rules
#[derive(Debug, Clone)]
pub struct StateResolutionContext {
    pub world_id: WorldId,
    /// Current game time
    pub game_time: GameTime,
    /// World flags that are currently set
    pub world_flags: Vec<String>,
    /// PC flags that are currently set (for the active PC)
    pub pc_flags: Vec<String>,
    /// Narrative events that have been triggered
    pub triggered_events: Vec<NarrativeEventId>,
    /// Characters currently present in the staging
    pub present_characters: Vec<CharacterId>,
}

impl StateResolutionContext {
    pub fn new(world_id: WorldId, game_time: GameTime) -> Self {
        Self {
            world_id,
            game_time,
            world_flags: Vec::new(),
            pc_flags: Vec::new(),
            triggered_events: Vec::new(),
            present_characters: Vec::new(),
        }
    }

    pub fn with_world_flags(mut self, flags: Vec<String>) -> Self {
        self.world_flags = flags;
        self
    }

    pub fn with_pc_flags(mut self, flags: Vec<String>) -> Self {
        self.pc_flags = flags;
        self
    }

    pub fn with_triggered_events(mut self, events: Vec<NarrativeEventId>) -> Self {
        self.triggered_events = events;
        self
    }

    pub fn with_present_characters(mut self, characters: Vec<CharacterId>) -> Self {
        self.present_characters = characters;
        self
    }
}

/// Information about soft rules that need LLM evaluation
#[derive(Debug, Clone)]
pub struct SoftRuleContext {
    /// State ID this rule belongs to
    pub state_id: String,
    /// State name for context
    pub state_name: String,
    /// The rule description
    pub description: String,
    /// Optional LLM prompt
    pub llm_prompt: Option<String>,
}

/// Information about a resolved state
#[derive(Debug, Clone)]
pub struct ResolvedStateInfo {
    pub id: String,
    pub name: String,
    pub backdrop_override: Option<String>,
    pub atmosphere_override: Option<String>,
    pub ambient_sound: Option<String>,
    pub priority: i32,
    pub is_default: bool,
    pub evaluation: ActivationEvaluation,
}

/// Result of state resolution
#[derive(Debug, Clone)]
pub struct StateResolutionResult {
    /// The resolved location state (if any)
    pub location_state: Option<ResolvedStateInfo>,
    /// The resolved region state (if any)
    pub region_state: Option<ResolvedStateInfo>,
    /// All available location states with their evaluations
    pub available_location_states: Vec<ResolvedStateInfo>,
    /// All available region states with their evaluations
    pub available_region_states: Vec<ResolvedStateInfo>,
    /// Soft rules that need LLM evaluation before final resolution
    pub pending_soft_rules: Vec<SoftRuleContext>,
    /// Whether resolution is complete (no pending soft rules)
    pub is_complete: bool,
}

impl StateResolutionResult {
    /// Create a fully resolved result
    pub fn complete(
        location_state: Option<ResolvedStateInfo>,
        region_state: Option<ResolvedStateInfo>,
        available_location_states: Vec<ResolvedStateInfo>,
        available_region_states: Vec<ResolvedStateInfo>,
    ) -> Self {
        Self {
            location_state,
            region_state,
            available_location_states,
            available_region_states,
            pending_soft_rules: Vec::new(),
            is_complete: true,
        }
    }

    /// Create a result that needs LLM evaluation
    pub fn needs_llm(
        available_location_states: Vec<ResolvedStateInfo>,
        available_region_states: Vec<ResolvedStateInfo>,
        pending_soft_rules: Vec<SoftRuleContext>,
    ) -> Self {
        Self {
            location_state: None,
            region_state: None,
            available_location_states,
            available_region_states,
            pending_soft_rules,
            is_complete: false,
        }
    }
}

/// Use case for resolving visual states
pub struct ResolveVisualState {
    location_state: Arc<LocationStateEntity>,
    region_state: Arc<RegionStateEntity>,
    flag: Arc<Flag>,
}

impl ResolveVisualState {
    pub fn new(
        location_state: Arc<LocationStateEntity>,
        region_state: Arc<RegionStateEntity>,
        flag: Arc<Flag>,
    ) -> Self {
        Self {
            location_state,
            region_state,
            flag,
        }
    }

    /// Resolve visual states for a location and region
    pub async fn execute(
        &self,
        location_id: LocationId,
        region_id: RegionId,
        context: &StateResolutionContext,
    ) -> Result<StateResolutionResult, RepoError> {
        // Get all states for location and region
        let location_states = self.location_state.list_for_location(location_id).await?;
        let region_states = self.region_state.list_for_region(region_id).await?;

        // Evaluate all location states
        let mut location_evaluations: Vec<(LocationState, ActivationEvaluation)> = Vec::new();
        let mut pending_soft_rules: Vec<SoftRuleContext> = Vec::new();

        for state in &location_states {
            let (eval, soft_rules) = self.evaluate_rules(
                &state.activation_rules,
                state.activation_logic,
                context,
                &state.id.to_string(),
                &state.name,
            );

            for soft in soft_rules {
                pending_soft_rules.push(soft);
            }

            location_evaluations.push((state.clone(), eval));
        }

        // Evaluate all region states
        let mut region_evaluations: Vec<(RegionState, ActivationEvaluation)> = Vec::new();

        for state in &region_states {
            let (eval, soft_rules) = self.evaluate_rules(
                &state.activation_rules,
                state.activation_logic,
                context,
                &state.id.to_string(),
                &state.name,
            );

            for soft in soft_rules {
                pending_soft_rules.push(soft);
            }

            region_evaluations.push((state.clone(), eval));
        }

        // Build available states lists
        let available_location_states: Vec<ResolvedStateInfo> = location_evaluations
            .iter()
            .map(|(state, eval)| ResolvedStateInfo {
                id: state.id.to_string(),
                name: state.name.clone(),
                backdrop_override: state.backdrop_override.clone(),
                atmosphere_override: state.atmosphere_override.clone(),
                ambient_sound: state.ambient_sound.clone(),
                priority: state.priority,
                is_default: state.is_default,
                evaluation: eval.clone(),
            })
            .collect();

        let available_region_states: Vec<ResolvedStateInfo> = region_evaluations
            .iter()
            .map(|(state, eval)| ResolvedStateInfo {
                id: state.id.to_string(),
                name: state.name.clone(),
                backdrop_override: state.backdrop_override.clone(),
                atmosphere_override: state.atmosphere_override.clone(),
                ambient_sound: state.ambient_sound.clone(),
                priority: state.priority,
                is_default: state.is_default,
                evaluation: eval.clone(),
            })
            .collect();

        // If there are pending soft rules, return incomplete result
        if !pending_soft_rules.is_empty() {
            return Ok(StateResolutionResult::needs_llm(
                available_location_states,
                available_region_states,
                pending_soft_rules,
            ));
        }

        // Select best matching states (highest priority among active states)
        let best_location = self.select_best_state(&location_evaluations);
        let best_region = self.select_best_region_state(&region_evaluations);

        Ok(StateResolutionResult::complete(
            best_location,
            best_region,
            available_location_states,
            available_region_states,
        ))
    }

    /// Resolve only location state
    pub async fn resolve_location_state(
        &self,
        location_id: LocationId,
        context: &StateResolutionContext,
    ) -> Result<Option<ResolvedStateInfo>, RepoError> {
        let location_states = self.location_state.list_for_location(location_id).await?;

        let mut evaluations: Vec<(LocationState, ActivationEvaluation)> = Vec::new();

        for state in &location_states {
            let (eval, _) = self.evaluate_rules(
                &state.activation_rules,
                state.activation_logic,
                context,
                &state.id.to_string(),
                &state.name,
            );
            evaluations.push((state.clone(), eval));
        }

        Ok(self.select_best_state(&evaluations))
    }

    /// Resolve only region state
    pub async fn resolve_region_state(
        &self,
        region_id: RegionId,
        context: &StateResolutionContext,
    ) -> Result<Option<ResolvedStateInfo>, RepoError> {
        let region_states = self.region_state.list_for_region(region_id).await?;

        let mut evaluations: Vec<(RegionState, ActivationEvaluation)> = Vec::new();

        for state in &region_states {
            let (eval, _) = self.evaluate_rules(
                &state.activation_rules,
                state.activation_logic,
                context,
                &state.id.to_string(),
                &state.name,
            );
            evaluations.push((state.clone(), eval));
        }

        Ok(self.select_best_region_state(&evaluations))
    }

    /// Evaluate activation rules and return evaluation result plus any pending soft rules
    fn evaluate_rules(
        &self,
        rules: &[ActivationRule],
        logic: ActivationLogic,
        context: &StateResolutionContext,
        state_id: &str,
        state_name: &str,
    ) -> (ActivationEvaluation, Vec<SoftRuleContext>) {
        let mut matched: Vec<String> = Vec::new();
        let mut unmatched: Vec<String> = Vec::new();
        let mut pending_soft: Vec<SoftRuleContext> = Vec::new();

        // Empty rules = always active (implicit Always rule)
        if rules.is_empty() {
            return (
                ActivationEvaluation::resolved(
                    true,
                    vec!["No rules (always active)".to_string()],
                    vec![],
                ),
                vec![],
            );
        }

        for rule in rules {
            if rule.is_soft_rule() {
                // Soft rules need LLM evaluation
                if let ActivationRule::Custom {
                    description,
                    llm_prompt,
                } = rule
                {
                    pending_soft.push(SoftRuleContext {
                        state_id: state_id.to_string(),
                        state_name: state_name.to_string(),
                        description: description.clone(),
                        llm_prompt: llm_prompt.clone(),
                    });
                }
            } else {
                // Evaluate hard rule
                let rule_desc = rule.description();
                if self.evaluate_hard_rule(rule, context) {
                    matched.push(rule_desc);
                } else {
                    unmatched.push(rule_desc);
                }
            }
        }

        // Short-circuit for Any logic: if any hard rule matched, we don't need LLM.
        // With Any logic, once a single hard rule matches, the state is definitively
        // active. We return an empty soft rules vec because the resolution is complete
        // and we don't want to trigger unnecessary LLM evaluation.
        if logic == ActivationLogic::Any && !matched.is_empty() {
            return (
                ActivationEvaluation::resolved(true, matched, unmatched),
                vec![],
            );
        }

        // If there are pending soft rules, we can't determine final state yet
        if !pending_soft.is_empty() {
            return (
                ActivationEvaluation::needs_llm(
                    matched,
                    unmatched,
                    pending_soft.iter().map(|s| s.description.clone()).collect(),
                ),
                pending_soft,
            );
        }

        // Determine if active based on logic and hard rule results
        let is_active = self.check_logic(logic, matched.len(), unmatched.len(), rules.len());

        (
            ActivationEvaluation::resolved(is_active, matched, unmatched),
            vec![],
        )
    }

    /// Evaluate a single hard rule against the context
    fn evaluate_hard_rule(&self, rule: &ActivationRule, context: &StateResolutionContext) -> bool {
        match rule {
            ActivationRule::Always => true,

            ActivationRule::DateExact { month, day } => {
                // Validate the date before comparing
                if !Self::is_valid_date(*month, *day) {
                    tracing::warn!(
                        month = *month,
                        day = *day,
                        "Invalid DateExact rule - date does not exist"
                    );
                    return false;
                }
                let current = context.game_time.current();
                current.month() as u32 == *month && current.day() as u32 == *day
            }

            ActivationRule::DateRange {
                start_month,
                start_day,
                end_month,
                end_day,
            } => {
                // Validate both dates before comparing
                if !Self::is_valid_date(*start_month, *start_day) {
                    tracing::warn!(
                        month = *start_month,
                        day = *start_day,
                        "Invalid DateRange start - date does not exist"
                    );
                    return false;
                }
                if !Self::is_valid_date(*end_month, *end_day) {
                    tracing::warn!(
                        month = *end_month,
                        day = *end_day,
                        "Invalid DateRange end - date does not exist"
                    );
                    return false;
                }

                let current = context.game_time.current();
                let current_month = current.month() as u32;
                let current_day = current.day() as u32;

                // Simple range check (doesn't handle year wrap-around)
                let current_ordinal = current_month * 100 + current_day;
                let start_ordinal = start_month * 100 + start_day;
                let end_ordinal = end_month * 100 + end_day;

                if start_ordinal <= end_ordinal {
                    // Normal range (e.g., Jun 20 to Jul 25)
                    current_ordinal >= start_ordinal && current_ordinal <= end_ordinal
                } else {
                    // Wrapping range (e.g., Dec 20 to Jan 5)
                    current_ordinal >= start_ordinal || current_ordinal <= end_ordinal
                }
            }

            ActivationRule::TimeOfDay { period } => context.game_time.time_of_day() == *period,

            ActivationRule::EventTriggered { event_id, .. } => {
                context.triggered_events.contains(event_id)
            }

            ActivationRule::FlagSet { flag_name } => {
                context.world_flags.contains(flag_name) || context.pc_flags.contains(flag_name)
            }

            ActivationRule::CharacterPresent { character_id, .. } => {
                context.present_characters.contains(character_id)
            }

            ActivationRule::Custom { .. } => {
                // Soft rules should not reach here
                false
            }
        }
    }

    /// Validate that a month/day combination represents a valid date.
    ///
    /// Returns false for invalid dates like Feb 30, month 13, etc.
    /// Note: For February, we allow up to 29 days to account for leap years.
    fn is_valid_date(month: u32, day: u32) -> bool {
        // Validate month is 1-12
        if month < 1 || month > 12 {
            return false;
        }

        // Validate day is at least 1
        if day < 1 {
            return false;
        }

        // Get max days for the month
        // Note: For February, we allow 29 to account for leap years
        let max_days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => 29,           // Allow 29 for leap years
            _ => return false, // Should never reach here due to check above
        };

        day <= max_days
    }

    /// Check if activation logic is satisfied
    fn check_logic(
        &self,
        logic: ActivationLogic,
        matched: usize,
        _unmatched: usize,
        total: usize,
    ) -> bool {
        match logic {
            ActivationLogic::All => matched == total,
            ActivationLogic::Any => matched > 0,
            ActivationLogic::AtLeast(n) => matched >= n as usize,
        }
    }

    /// Select the best matching location state (highest priority among active)
    /// Falls back to the default state if no active states are found
    fn select_best_state(
        &self,
        evaluations: &[(LocationState, ActivationEvaluation)],
    ) -> Option<ResolvedStateInfo> {
        // First try to find highest priority active state
        let active_state = evaluations
            .iter()
            .filter(|(_, eval)| eval.is_active)
            .max_by_key(|(state, _)| state.priority);

        if let Some((state, eval)) = active_state {
            return Some(ResolvedStateInfo {
                id: state.id.to_string(),
                name: state.name.clone(),
                backdrop_override: state.backdrop_override.clone(),
                atmosphere_override: state.atmosphere_override.clone(),
                ambient_sound: state.ambient_sound.clone(),
                priority: state.priority,
                is_default: state.is_default,
                evaluation: eval.clone(),
            });
        }

        // Fall back to the default state if no active states found
        evaluations
            .iter()
            .find(|(state, _)| state.is_default)
            .map(|(state, eval)| ResolvedStateInfo {
                id: state.id.to_string(),
                name: state.name.clone(),
                backdrop_override: state.backdrop_override.clone(),
                atmosphere_override: state.atmosphere_override.clone(),
                ambient_sound: state.ambient_sound.clone(),
                priority: state.priority,
                is_default: state.is_default,
                evaluation: eval.clone(),
            })
    }

    /// Select the best matching region state (highest priority among active)
    /// Falls back to the default state if no active states are found
    fn select_best_region_state(
        &self,
        evaluations: &[(RegionState, ActivationEvaluation)],
    ) -> Option<ResolvedStateInfo> {
        // First try to find highest priority active state
        let active_state = evaluations
            .iter()
            .filter(|(_, eval)| eval.is_active)
            .max_by_key(|(state, _)| state.priority);

        if let Some((state, eval)) = active_state {
            return Some(ResolvedStateInfo {
                id: state.id.to_string(),
                name: state.name.clone(),
                backdrop_override: state.backdrop_override.clone(),
                atmosphere_override: state.atmosphere_override.clone(),
                ambient_sound: state.ambient_sound.clone(),
                priority: state.priority,
                is_default: state.is_default,
                evaluation: eval.clone(),
            });
        }

        // Fall back to the default state if no active states found
        evaluations
            .iter()
            .find(|(state, _)| state.is_default)
            .map(|(state, eval)| ResolvedStateInfo {
                id: state.id.to_string(),
                name: state.name.clone(),
                backdrop_override: state.backdrop_override.clone(),
                atmosphere_override: state.atmosphere_override.clone(),
                ambient_sound: state.ambient_sound.clone(),
                priority: state.priority,
                is_default: state.is_default,
                evaluation: eval.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_context() -> StateResolutionContext {
        let game_time = GameTime::new(Utc::now());
        StateResolutionContext::new(WorldId::new(), game_time)
    }

    #[test]
    fn test_always_rule_matches() {
        let resolve = create_test_resolve();
        let context = create_test_context();

        assert!(resolve.evaluate_hard_rule(&ActivationRule::Always, &context));
    }

    #[test]
    fn test_flag_rule_matches_world_flag() {
        let resolve = create_test_resolve();
        let context = create_test_context().with_world_flags(vec!["festival_active".to_string()]);

        let rule = ActivationRule::FlagSet {
            flag_name: "festival_active".to_string(),
        };

        assert!(resolve.evaluate_hard_rule(&rule, &context));
    }

    #[test]
    fn test_flag_rule_no_match() {
        let resolve = create_test_resolve();
        let context = create_test_context();

        let rule = ActivationRule::FlagSet {
            flag_name: "festival_active".to_string(),
        };

        assert!(!resolve.evaluate_hard_rule(&rule, &context));
    }

    #[test]
    fn test_character_present_matches() {
        let resolve = create_test_resolve();
        let char_id = CharacterId::new();
        let context = create_test_context().with_present_characters(vec![char_id]);

        let rule = ActivationRule::CharacterPresent {
            character_id: char_id,
            character_name: "Test NPC".to_string(),
        };

        assert!(resolve.evaluate_hard_rule(&rule, &context));
    }

    #[test]
    fn test_logic_all_requires_all_matched() {
        let resolve = create_test_resolve();

        assert!(resolve.check_logic(ActivationLogic::All, 3, 0, 3));
        assert!(!resolve.check_logic(ActivationLogic::All, 2, 1, 3));
    }

    #[test]
    fn test_logic_any_requires_one_matched() {
        let resolve = create_test_resolve();

        assert!(resolve.check_logic(ActivationLogic::Any, 1, 2, 3));
        assert!(!resolve.check_logic(ActivationLogic::Any, 0, 3, 3));
    }

    #[test]
    fn test_logic_at_least() {
        let resolve = create_test_resolve();

        assert!(resolve.check_logic(ActivationLogic::AtLeast(2), 2, 1, 3));
        assert!(resolve.check_logic(ActivationLogic::AtLeast(2), 3, 0, 3));
        assert!(!resolve.check_logic(ActivationLogic::AtLeast(2), 1, 2, 3));
    }

    // Helper to create a test resolver (would need mock repos in real tests)
    fn create_test_resolve() -> ResolveVisualState {
        use crate::infrastructure::ports::{FlagRepo, LocationStateRepo, RegionStateRepo};
        use async_trait::async_trait;
        use std::sync::Arc;
        use wrldbldr_domain::{LocationStateId, RegionStateId};

        // Mock implementations for testing
        struct MockLocationStateRepo;
        struct MockRegionStateRepo;
        struct MockFlagRepo;

        #[async_trait]
        impl LocationStateRepo for MockLocationStateRepo {
            async fn get(&self, _id: LocationStateId) -> Result<Option<LocationState>, RepoError> {
                Ok(None)
            }
            async fn save(&self, _state: &LocationState) -> Result<(), RepoError> {
                Ok(())
            }
            async fn delete(&self, _id: LocationStateId) -> Result<(), RepoError> {
                Ok(())
            }
            async fn list_for_location(
                &self,
                _location_id: LocationId,
            ) -> Result<Vec<LocationState>, RepoError> {
                Ok(vec![])
            }
            async fn get_default(
                &self,
                _location_id: LocationId,
            ) -> Result<Option<LocationState>, RepoError> {
                Ok(None)
            }
            async fn set_active(
                &self,
                _location_id: LocationId,
                _state_id: LocationStateId,
            ) -> Result<(), RepoError> {
                Ok(())
            }
            async fn get_active(
                &self,
                _location_id: LocationId,
            ) -> Result<Option<LocationState>, RepoError> {
                Ok(None)
            }
            async fn clear_active(&self, _location_id: LocationId) -> Result<(), RepoError> {
                Ok(())
            }
        }

        #[async_trait]
        impl RegionStateRepo for MockRegionStateRepo {
            async fn get(&self, _id: RegionStateId) -> Result<Option<RegionState>, RepoError> {
                Ok(None)
            }
            async fn save(&self, _state: &RegionState) -> Result<(), RepoError> {
                Ok(())
            }
            async fn delete(&self, _id: RegionStateId) -> Result<(), RepoError> {
                Ok(())
            }
            async fn list_for_region(
                &self,
                _region_id: RegionId,
            ) -> Result<Vec<RegionState>, RepoError> {
                Ok(vec![])
            }
            async fn get_default(
                &self,
                _region_id: RegionId,
            ) -> Result<Option<RegionState>, RepoError> {
                Ok(None)
            }
            async fn set_active(
                &self,
                _region_id: RegionId,
                _state_id: RegionStateId,
            ) -> Result<(), RepoError> {
                Ok(())
            }
            async fn get_active(
                &self,
                _region_id: RegionId,
            ) -> Result<Option<RegionState>, RepoError> {
                Ok(None)
            }
            async fn clear_active(&self, _region_id: RegionId) -> Result<(), RepoError> {
                Ok(())
            }
        }

        #[async_trait]
        impl FlagRepo for MockFlagRepo {
            async fn get_world_flags(&self, _world_id: WorldId) -> Result<Vec<String>, RepoError> {
                Ok(vec![])
            }
            async fn get_pc_flags(
                &self,
                _pc_id: wrldbldr_domain::PlayerCharacterId,
            ) -> Result<Vec<String>, RepoError> {
                Ok(vec![])
            }
            async fn set_world_flag(
                &self,
                _world_id: WorldId,
                _flag_name: &str,
            ) -> Result<(), RepoError> {
                Ok(())
            }
            async fn unset_world_flag(
                &self,
                _world_id: WorldId,
                _flag_name: &str,
            ) -> Result<(), RepoError> {
                Ok(())
            }
            async fn set_pc_flag(
                &self,
                _pc_id: wrldbldr_domain::PlayerCharacterId,
                _flag_name: &str,
            ) -> Result<(), RepoError> {
                Ok(())
            }
            async fn unset_pc_flag(
                &self,
                _pc_id: wrldbldr_domain::PlayerCharacterId,
                _flag_name: &str,
            ) -> Result<(), RepoError> {
                Ok(())
            }
            async fn is_world_flag_set(
                &self,
                _world_id: WorldId,
                _flag_name: &str,
            ) -> Result<bool, RepoError> {
                Ok(false)
            }
            async fn is_pc_flag_set(
                &self,
                _pc_id: wrldbldr_domain::PlayerCharacterId,
                _flag_name: &str,
            ) -> Result<bool, RepoError> {
                Ok(false)
            }
        }

        let location_state = Arc::new(crate::repositories::LocationStateEntity::new(Arc::new(
            MockLocationStateRepo,
        )
            as Arc<dyn LocationStateRepo>));
        let region_state = Arc::new(crate::repositories::RegionStateEntity::new(Arc::new(
            MockRegionStateRepo,
        )
            as Arc<dyn RegionStateRepo>));
        let flag = Arc::new(crate::repositories::Flag::new(
            Arc::new(MockFlagRepo) as Arc<dyn FlagRepo>
        ));

        ResolveVisualState::new(location_state, region_state, flag)
    }
}
