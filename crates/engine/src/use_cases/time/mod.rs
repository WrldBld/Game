// Time use cases - fields for future time advancement
#![allow(dead_code)]

//! Time use cases.
//!
//! Handles game time operations including:
//! - Suggesting time passage based on player actions
//! - Advancing time (with DM approval in suggested mode)
//! - Setting exact time
//! - Skipping to time periods

use std::sync::Arc;
use uuid::Uuid;

use wrldbldr_domain::{
    GameTime, PlayerCharacterId, TimeAdvanceReason, TimeMode, TimeOfDay, TimeSuggestionDecision,
    WorldId,
};

use crate::infrastructure::ports::{ClockPort, QueueError, RepoError, WorldRepo};
use crate::repositories::WorldError;
use crate::stores::TimeSuggestionStore;

/// Container for time use cases.
pub struct TimeUseCases {
    pub suggest_time: Arc<SuggestTime>,
    pub control: Arc<TimeControl>,
    pub suggestions: Arc<TimeSuggestions>,
}

impl TimeUseCases {
    pub fn new(
        suggest_time: Arc<SuggestTime>,
        control: Arc<TimeControl>,
        suggestions: Arc<TimeSuggestions>,
    ) -> Self {
        Self {
            suggest_time,
            control,
            suggestions,
        }
    }
}

// =============================================================================
// Time Suggestion
// =============================================================================

// Re-export TimeSuggestion from ports for backwards compatibility
pub use crate::infrastructure::ports::TimeSuggestion;

/// Result of suggesting time passage.
#[derive(Debug)]
pub enum SuggestTimeResult {
    /// Time suggestion created for DM approval (mode = Suggested)
    SuggestionCreated(TimeSuggestion),
    /// No time cost for this action (cost = 0)
    NoCost,
    /// Time mode is manual, no suggestion generated
    ManualMode,
}

/// Use case for suggesting time passage.
///
/// When a player performs an action that should cost time, this use case:
/// 1. Looks up the configured cost for the action type
/// 2. Based on time mode:
///    - Suggested: Creates a suggestion for DM approval
///    - Manual: Does nothing (DM advances manually)
#[allow(dead_code)]
pub struct SuggestTime {
    world: Arc<dyn WorldRepo>,
    clock: Arc<dyn ClockPort>,
}

impl SuggestTime {
    pub fn new(world: Arc<dyn WorldRepo>, clock: Arc<dyn ClockPort>) -> Self {
        Self { world, clock }
    }

    /// Suggest time passage for an action.
    ///
    /// # Arguments
    /// * `world_id` - The world this is happening in
    /// * `pc_id` - The player character performing the action
    /// * `pc_name` - Name of the PC (for display)
    /// * `action_type` - Type of action (e.g., "travel_location", "challenge")
    /// * `action_description` - Human-readable description
    ///
    /// # Returns
    /// - `SuggestionCreated` if mode is Suggested
    /// - `NoCost` if the action has zero time cost
    /// - `ManualMode` if mode is Manual
    pub async fn execute(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
        pc_name: String,
        action_type: &str,
        action_description: String,
    ) -> Result<SuggestTimeResult, SuggestTimeError> {
        // Get the world to check config
        let world = self
            .world
            .get(world_id)
            .await?
            .ok_or(SuggestTimeError::WorldNotFound)?;

        let config = world.time_config();
        let cost_minutes = config.time_costs.cost_for_action(action_type);

        // If no cost, nothing to do
        if cost_minutes == 0 {
            return Ok(SuggestTimeResult::NoCost);
        }

        // Check time mode
        match config.mode {
            TimeMode::Manual => {
                // DM controls time manually, no suggestions
                Ok(SuggestTimeResult::ManualMode)
            }
            TimeMode::Auto | TimeMode::Suggested => {
                // Create suggestion for DM approval
                let mut resulting_time = world.game_time().clone();
                let previous_period = resulting_time.time_of_day();
                resulting_time.advance_minutes(cost_minutes);
                let new_period = resulting_time.time_of_day();

                let period_change = if previous_period != new_period {
                    Some((previous_period, new_period))
                } else {
                    None
                };

                let suggestion = TimeSuggestion {
                    id: Uuid::new_v4(),
                    world_id,
                    pc_id,
                    pc_name,
                    action_type: action_type.to_string(),
                    action_description,
                    suggested_minutes: cost_minutes,
                    current_time: world.game_time().clone(),
                    resulting_time,
                    period_change,
                };

                Ok(SuggestTimeResult::SuggestionCreated(suggestion))
            }
        }
    }
}

/// Build a TimeAdvanceReason from an action type and human-readable description.
///
/// Used when a DM approves a time suggestion.
pub fn time_advance_reason_for_action(action_type: &str, description: &str) -> TimeAdvanceReason {
    match action_type {
        "travel_location" => TimeAdvanceReason::TravelLocation {
            from: "previous location".to_string(),
            to: description.to_string(),
        },
        "travel_region" => TimeAdvanceReason::TravelRegion {
            from: "previous region".to_string(),
            to: description.to_string(),
        },
        "rest_short" => TimeAdvanceReason::RestShort,
        "rest_long" => TimeAdvanceReason::RestLong,
        "challenge" => TimeAdvanceReason::Challenge {
            name: description.to_string(),
        },
        "scene_transition" => TimeAdvanceReason::SceneTransition {
            scene_name: description.to_string(),
        },
        _ => TimeAdvanceReason::DmManual { hours: 0 },
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SuggestTimeError {
    #[error("World not found")]
    WorldNotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("World error: {0}")]
    World(#[from] WorldError),
}

// =============================================================================
// Time Control
// =============================================================================

/// Consolidated use case for time control operations (get/advance/set/config).
pub struct TimeControl {
    world: Arc<dyn WorldRepo>,
    clock: Arc<dyn ClockPort>,
}

impl TimeControl {
    pub fn new(world: Arc<dyn WorldRepo>, clock: Arc<dyn ClockPort>) -> Self {
        Self { world, clock }
    }

    pub async fn get_game_time(&self, world_id: WorldId) -> Result<GameTime, TimeControlError> {
        let world = self
            .world
            .get(world_id)
            .await?
            .ok_or(TimeControlError::WorldNotFound)?;

        Ok(world.game_time().clone())
    }

    pub async fn advance_hours(
        &self,
        world_id: WorldId,
        hours: u32,
    ) -> Result<TimeAdvanceOutcome, TimeControlError> {
        let mut world = self
            .world
            .get(world_id)
            .await?
            .ok_or(TimeControlError::WorldNotFound)?;

        let previous_time = world.game_time().clone();
        // Use the aggregate's advance_hours which auto-updates updated_at
        let _ = world.advance_hours(hours, self.clock.now());

        self.world.save(&world).await?;

        Ok(TimeAdvanceOutcome {
            previous_time,
            new_time: world.game_time().clone(),
            minutes_advanced: hours * 60,
        })
    }

    pub async fn advance_minutes(
        &self,
        world_id: WorldId,
        minutes: u32,
        reason: TimeAdvanceReason,
    ) -> Result<TimeAdvanceOutcome, TimeControlError> {
        let mut world = self
            .world
            .get(world_id)
            .await?
            .ok_or(TimeControlError::WorldNotFound)?;

        let previous_time = world.game_time().clone();
        let result = world.advance_time(minutes, reason, self.clock.now());

        self.world.save(&world).await?;

        Ok(TimeAdvanceOutcome {
            previous_time,
            new_time: result.new_time.clone(),
            minutes_advanced: result.minutes_advanced,
        })
    }

    pub async fn set_game_time(
        &self,
        world_id: WorldId,
        day: u32,
        hour: u8,
    ) -> Result<TimeAdvanceOutcome, TimeControlError> {
        let mut world = self
            .world
            .get(world_id)
            .await?
            .ok_or(TimeControlError::WorldNotFound)?;

        let previous_time = world.game_time().clone();
        // Mutate game time directly - aggregate setters auto-update updated_at
        world.game_time_mut().set_day_and_hour(day, hour as u32);

        self.world.save(&world).await?;

        Ok(TimeAdvanceOutcome {
            previous_time,
            new_time: world.game_time().clone(),
            minutes_advanced: 0,
        })
    }

    pub async fn skip_to_period(
        &self,
        world_id: WorldId,
        period: TimeOfDay,
    ) -> Result<TimeAdvanceOutcome, TimeControlError> {
        let mut world = self
            .world
            .get(world_id)
            .await?
            .ok_or(TimeControlError::WorldNotFound)?;

        let previous_time = world.game_time().clone();
        let minutes_until = world.game_time().minutes_until_period(period);
        world.game_time_mut().skip_to_period(period);

        self.world.save(&world).await?;

        Ok(TimeAdvanceOutcome {
            previous_time,
            new_time: world.game_time().clone(),
            minutes_advanced: minutes_until,
        })
    }

    pub async fn set_paused(
        &self,
        world_id: WorldId,
        paused: bool,
    ) -> Result<GameTime, TimeControlError> {
        let mut world = self
            .world
            .get(world_id)
            .await?
            .ok_or(TimeControlError::WorldNotFound)?;

        world.game_time_mut().set_paused(paused);

        self.world.save(&world).await?;

        Ok(world.game_time().clone())
    }

    pub async fn get_time_config(
        &self,
        world_id: WorldId,
    ) -> Result<wrldbldr_domain::GameTimeConfig, TimeControlError> {
        let world = self
            .world
            .get(world_id)
            .await?
            .ok_or(TimeControlError::WorldNotFound)?;

        Ok(world.time_config().clone())
    }

    pub async fn update_time_config(
        &self,
        world_id: WorldId,
        config: wrldbldr_domain::GameTimeConfig,
    ) -> Result<TimeConfigUpdate, TimeControlError> {
        let mut world = self
            .world
            .get(world_id)
            .await?
            .ok_or(TimeControlError::WorldNotFound)?;

        let normalized_config = normalize_domain_time_config(config);

        // Update via the individual setters (which auto-update updated_at)
        let now = self.clock.now();
        world.set_time_mode(normalized_config.mode, now);
        world.set_time_costs(normalized_config.time_costs.clone(), now);

        self.world.save(&world).await?;

        Ok(TimeConfigUpdate {
            world_id,
            normalized_config,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TimeAdvanceOutcome {
    pub previous_time: GameTime,
    pub new_time: GameTime,
    pub minutes_advanced: u32,
}

/// Result of updating time configuration.
///
/// Contains the world ID and the normalized domain config after update.
#[derive(Debug, Clone)]
pub struct TimeConfigUpdate {
    pub world_id: WorldId,
    pub normalized_config: wrldbldr_domain::GameTimeConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum TimeControlError {
    #[error("World not found")]
    WorldNotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("World error: {0}")]
    World(#[from] WorldError),
}

// =============================================================================
// Time Suggestions
// =============================================================================

/// In-memory store + approval flow for time suggestions.
pub struct TimeSuggestions {
    control: Arc<TimeControl>,
}

impl TimeSuggestions {
    pub fn new(control: Arc<TimeControl>) -> Self {
        Self { control }
    }

    pub async fn resolve(
        &self,
        store: &TimeSuggestionStore,
        world_id: WorldId,
        suggestion_id: Uuid,
        decision: TimeSuggestionDecision,
    ) -> Result<Option<TimeSuggestionResolution>, TimeSuggestionError> {
        let suggestion = match store.remove(suggestion_id).await {
            Some(s) => s,
            None => return Err(TimeSuggestionError::NotFound),
        };

        if suggestion.world_id != world_id {
            return Err(TimeSuggestionError::WorldMismatch);
        }

        let minutes_to_advance = decision.resolved_minutes(suggestion.suggested_minutes);

        if minutes_to_advance == 0 {
            return Ok(None);
        }

        let reason = crate::use_cases::time::time_advance_reason_for_action(
            &suggestion.action_type,
            &suggestion.action_description,
        );

        let result = self
            .control
            .advance_minutes(world_id, minutes_to_advance, reason.clone())
            .await?;

        let advance_data = crate::use_cases::time::build_time_advance_data(
            &result.previous_time,
            &result.new_time,
            result.minutes_advanced,
            &reason,
        );

        Ok(Some(TimeSuggestionResolution {
            world_id,
            suggestion_id,
            minutes_advanced: minutes_to_advance,
            advance_data,
        }))
    }
}

/// Result of resolving a time suggestion.
///
/// Contains all the data needed to broadcast the time advance event.
#[derive(Debug, Clone)]
pub struct TimeSuggestionResolution {
    pub world_id: WorldId,
    pub suggestion_id: Uuid,
    pub minutes_advanced: u32,
    /// Domain-level time advance data (convert to protocol at API boundary)
    pub advance_data: TimeAdvanceResultData,
}

/// Domain-level time advance result data.
///
/// Contains all information about a time advancement for use within the engine.
/// Converted to `wrldbldr_shared::types::TimeAdvanceData` at the API boundary.
#[derive(Debug, Clone)]
pub struct TimeAdvanceResultData {
    pub previous_time: GameTime,
    pub new_time: GameTime,
    pub minutes_advanced: u32,
    pub reason: String,
    pub period_changed: bool,
    pub new_period: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum TimeSuggestionError {
    #[error("Time suggestion not found")]
    NotFound,
    #[error("Time suggestion world mismatch")]
    WorldMismatch,
    #[error("Time control error: {0}")]
    Control(#[from] TimeControlError),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("World error: {0}")]
    World(#[from] WorldError),
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),
}

/// Normalize domain time config, handling the Auto -> Suggested transition.
///
/// # TimeMode::Auto Behavior
///
/// **Important**: `TimeMode::Auto` is currently normalized to `TimeMode::Suggested`.
///
/// This is an intentional design decision with the following rationale:
/// - The original vision for "Auto" mode was to automatically advance time without DM approval
/// - However, this behavior was never fully implemented due to concerns about:
///   - Game pacing control (auto-advancing could disrupt narrative flow)
///   - Edge cases around period transitions (dawn/dusk/etc.)
///   - Lack of undo capability if time advances incorrectly
///
/// As a result, "Auto" currently behaves identically to "Suggested" mode, where:
/// - Time suggestions are generated for player actions
/// - The DM must approve/modify/skip each suggestion
///
/// **API Contract Note**: If a client sets `TimeMode::Auto`, the UI may display "Automatic"
/// but the actual behavior will be "Suggested" (DM approval required).
///
/// # TODO
///
/// Either:
/// 1. Implement true auto-advancement behavior (advance time immediately without DM approval)
/// 2. Remove the `Auto` variant from the protocol and mark it as deprecated in domain types
///
/// See: https://github.com/WrldBldr/Game/issues/XXX (replace with actual tracking issue)
fn normalize_domain_time_config(
    mut config: wrldbldr_domain::GameTimeConfig,
) -> wrldbldr_domain::GameTimeConfig {
    if matches!(config.mode, wrldbldr_domain::TimeMode::Auto) {
        tracing::warn!(
            "TimeMode::Auto is not fully implemented - normalizing to TimeMode::Suggested. \
             Time suggestions will still require DM approval."
        );
        config.mode = wrldbldr_domain::TimeMode::Suggested;
    }
    config
}

/// Build domain-level time advance result data.
///
/// This returns a domain type that can be converted to protocol format at the API boundary.
pub fn build_time_advance_data(
    previous: &GameTime,
    new: &GameTime,
    minutes: u32,
    reason: &TimeAdvanceReason,
) -> TimeAdvanceResultData {
    let previous_period = previous.time_of_day();
    let new_period = new.time_of_day();
    let period_changed = previous_period != new_period;

    TimeAdvanceResultData {
        previous_time: previous.clone(),
        new_time: new.clone(),
        minutes_advanced: minutes,
        reason: reason.description(),
        period_changed,
        new_period: if period_changed {
            Some(new_period.display_name().to_string())
        } else {
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use wrldbldr_domain::value_objects::WorldName;
    use wrldbldr_domain::{GameTimeConfig, TimeMode, WorldId};

    use crate::infrastructure::ports::{ClockPort, MockWorldRepo};
    use crate::repositories;
    use crate::repositories::ClockService;

    struct FixedClock(chrono::DateTime<chrono::Utc>);

    impl ClockPort for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
    }

    fn build_clock(now: chrono::DateTime<chrono::Utc>) -> (Arc<dyn ClockPort>, Arc<ClockService>) {
        let clock_port: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let clock = Arc::new(ClockService::new(clock_port.clone()));
        (clock_port, clock)
    }

    #[tokio::test]
    async fn when_time_mode_auto_then_suggests_time_and_does_not_persist() {
        let now = Utc::now();
        let world_id = WorldId::new();

        let mut time_config = GameTimeConfig::default();
        time_config.mode = TimeMode::Auto;

        let world_name = WorldName::new("World").unwrap();
        let domain_world = wrldbldr_domain::World::new(world_name, now)
            .with_id(world_id)
            .with_time_config(time_config);

        let mut world_repo = MockWorldRepo::new();
        let domain_world_for_get = domain_world.clone();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(domain_world_for_get.clone())));
        world_repo.expect_save().times(0);

        let (clock_port, clock) = build_clock(now);
        let world_entity = Arc::new(repositories::WorldRepository::new(
            Arc::new(world_repo),
            clock_port.clone(),
        ));
        let suggest_time = super::SuggestTime::new(world_entity, clock);

        let result = suggest_time
            .execute(
                world_id,
                wrldbldr_domain::PlayerCharacterId::new(),
                "PC".to_string(),
                "challenge",
                "Try to pick the lock".to_string(),
            )
            .await
            .expect("SuggestTime should succeed");

        let super::SuggestTimeResult::SuggestionCreated(suggestion) = result else {
            panic!("expected SuggestionCreated");
        };

        assert_eq!(suggestion.world_id, world_id);
        assert_eq!(suggestion.action_type, "challenge");
        assert_eq!(suggestion.suggested_minutes, 10);
        assert_eq!(suggestion.current_time, *domain_world.game_time());
        assert_eq!(
            suggestion.resulting_time.day(),
            domain_world.game_time().day(),
            "time suggestion should not change day for small increments"
        );
        assert_ne!(
            suggestion.resulting_time.minute(),
            domain_world.game_time().minute()
        );
    }

    #[tokio::test]
    async fn when_time_mode_manual_then_returns_manual_mode_and_does_not_persist() {
        let now = Utc::now();
        let world_id = WorldId::new();

        let mut time_config = GameTimeConfig::default();
        time_config.mode = TimeMode::Manual;

        let world_name = WorldName::new("World").unwrap();
        let domain_world = wrldbldr_domain::World::new(world_name, now)
            .with_id(world_id)
            .with_time_config(time_config);

        let mut world_repo = MockWorldRepo::new();
        let domain_world_for_get = domain_world.clone();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(domain_world_for_get.clone())));
        world_repo.expect_save().times(0);

        let (clock_port, clock) = build_clock(now);
        let world_entity = Arc::new(repositories::WorldRepository::new(
            Arc::new(world_repo),
            clock_port.clone(),
        ));
        let suggest_time = super::SuggestTime::new(world_entity, clock);

        let result = suggest_time
            .execute(
                world_id,
                wrldbldr_domain::PlayerCharacterId::new(),
                "PC".to_string(),
                "challenge",
                "Try to pick the lock".to_string(),
            )
            .await
            .expect("SuggestTime should succeed");

        assert!(matches!(result, super::SuggestTimeResult::ManualMode));
    }

    #[tokio::test]
    async fn when_action_has_no_cost_then_returns_no_cost_and_does_not_persist() {
        let now = Utc::now();
        let world_id = WorldId::new();

        // Default config cost for unknown action types is 0.
        let world_name = WorldName::new("World").unwrap();
        let domain_world = wrldbldr_domain::World::new(world_name, now).with_id(world_id);

        let mut world_repo = MockWorldRepo::new();
        let domain_world_for_get = domain_world.clone();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(domain_world_for_get.clone())));
        world_repo.expect_save().times(0);

        let (clock_port, clock) = build_clock(now);
        let world_entity = Arc::new(repositories::WorldRepository::new(
            Arc::new(world_repo),
            clock_port.clone(),
        ));
        let suggest_time = super::SuggestTime::new(world_entity, clock);

        let result = suggest_time
            .execute(
                world_id,
                wrldbldr_domain::PlayerCharacterId::new(),
                "PC".to_string(),
                "unknown_action_type",
                "Whatever".to_string(),
            )
            .await
            .expect("SuggestTime should succeed");

        assert!(matches!(result, super::SuggestTimeResult::NoCost));
    }
}
