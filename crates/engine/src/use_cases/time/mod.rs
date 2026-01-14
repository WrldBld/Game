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

use crate::infrastructure::ports::QueueError;
use crate::infrastructure::ports::{ClockPort, RepoError, TimeSuggestionStore};
use crate::repositories::{World, WorldError};

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
    world: Arc<World>,
    clock: Arc<dyn ClockPort>,
}

impl SuggestTime {
    pub fn new(world: Arc<World>, clock: Arc<dyn ClockPort>) -> Self {
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
    world: Arc<World>,
    clock: Arc<dyn ClockPort>,
}

impl TimeControl {
    pub fn new(world: Arc<World>, clock: Arc<dyn ClockPort>) -> Self {
        Self { world, clock }
    }

    pub async fn get_game_time(&self, world_id: WorldId) -> Result<GameTime, TimeControlError> {
        self.world
            .get_current_time(world_id)
            .await
            .map_err(TimeControlError::from)
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
        let result = self.world.advance_time(world_id, minutes, reason).await?;

        Ok(TimeAdvanceOutcome {
            previous_time: result.previous_time,
            new_time: result.new_time,
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
    ) -> Result<wrldbldr_protocol::types::GameTimeConfig, TimeControlError> {
        let world = self
            .world
            .get(world_id)
            .await?
            .ok_or(TimeControlError::WorldNotFound)?;

        Ok(domain_time_config_to_protocol(world.time_config()))
    }

    pub async fn update_time_config(
        &self,
        world_id: WorldId,
        config: wrldbldr_protocol::types::GameTimeConfig,
    ) -> Result<TimeConfigUpdate, TimeControlError> {
        let mut world = self
            .world
            .get(world_id)
            .await?
            .ok_or(TimeControlError::WorldNotFound)?;

        let normalized_config = normalize_protocol_time_config(config);
        let domain_config = protocol_time_config_to_domain(&normalized_config);

        // Update via the individual setters (which auto-update updated_at)
        let now = self.clock.now();
        world.set_time_mode(domain_config.mode, now);
        world.set_time_costs(domain_config.time_costs, now);

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

#[derive(Debug, Clone)]
pub struct TimeConfigUpdate {
    pub world_id: WorldId,
    pub normalized_config: wrldbldr_protocol::types::GameTimeConfig,
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
        store: &dyn TimeSuggestionStore,
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

#[derive(Debug, Clone)]
pub struct TimeSuggestionResolution {
    pub world_id: WorldId,
    pub suggestion_id: Uuid,
    pub minutes_advanced: u32,
    pub advance_data: wrldbldr_protocol::types::TimeAdvanceData,
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

/// Normalize protocol time config, handling the Auto -> Suggested transition.
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
fn normalize_protocol_time_config(
    mut config: wrldbldr_protocol::types::GameTimeConfig,
) -> wrldbldr_protocol::types::GameTimeConfig {
    if matches!(config.mode, wrldbldr_protocol::types::TimeMode::Auto) {
        tracing::warn!(
            "TimeMode::Auto is not fully implemented - normalizing to TimeMode::Suggested. \
             Time suggestions will still require DM approval."
        );
        config.mode = wrldbldr_protocol::types::TimeMode::Suggested;
    }
    config
}

fn protocol_time_config_to_domain(
    config: &wrldbldr_protocol::types::GameTimeConfig,
) -> wrldbldr_domain::GameTimeConfig {
    let mode = match config.mode {
        wrldbldr_protocol::types::TimeMode::Manual => wrldbldr_domain::TimeMode::Manual,
        wrldbldr_protocol::types::TimeMode::Suggested => wrldbldr_domain::TimeMode::Suggested,
        wrldbldr_protocol::types::TimeMode::Auto => wrldbldr_domain::TimeMode::Suggested,
    };

    let time_costs = wrldbldr_domain::TimeCostConfig {
        travel_location: config.time_costs.travel_location,
        travel_region: config.time_costs.travel_region,
        rest_short: config.time_costs.rest_short,
        rest_long: config.time_costs.rest_long,
        conversation: config.time_costs.conversation,
        challenge: config.time_costs.challenge,
        scene_transition: config.time_costs.scene_transition,
    };

    wrldbldr_domain::GameTimeConfig {
        mode,
        time_costs,
        show_time_to_players: config.show_time_to_players,
        time_format: wrldbldr_domain::TimeFormat::TwelveHour,
    }
}

fn domain_time_config_to_protocol(
    config: &wrldbldr_domain::GameTimeConfig,
) -> wrldbldr_protocol::types::GameTimeConfig {
    wrldbldr_protocol::types::GameTimeConfig {
        mode: match config.mode {
            wrldbldr_domain::TimeMode::Manual => wrldbldr_protocol::types::TimeMode::Manual,
            wrldbldr_domain::TimeMode::Suggested => wrldbldr_protocol::types::TimeMode::Suggested,
            wrldbldr_domain::TimeMode::Auto => wrldbldr_protocol::types::TimeMode::Suggested,
        },
        time_costs: wrldbldr_protocol::types::TimeCostConfig {
            travel_location: config.time_costs.travel_location,
            travel_region: config.time_costs.travel_region,
            rest_short: config.time_costs.rest_short,
            rest_long: config.time_costs.rest_long,
            conversation: config.time_costs.conversation,
            challenge: config.time_costs.challenge,
            scene_transition: config.time_costs.scene_transition,
        },
        show_time_to_players: config.show_time_to_players,
        time_format: wrldbldr_protocol::types::TimeFormat::TwelveHour,
    }
}

// =============================================================================
// Conversion to Protocol Types
// =============================================================================

impl TimeSuggestion {
    /// Convert to protocol type for sending to client.
    pub fn to_protocol(&self) -> wrldbldr_protocol::types::TimeSuggestionData {
        wrldbldr_protocol::types::TimeSuggestionData {
            suggestion_id: self.id.to_string(),
            pc_id: self.pc_id.to_string(),
            pc_name: self.pc_name.clone(),
            action_type: self.action_type.clone(),
            action_description: self.action_description.clone(),
            suggested_minutes: self.suggested_minutes,
            current_time: game_time_to_protocol(&self.current_time),
            resulting_time: game_time_to_protocol(&self.resulting_time),
            period_change: self.period_change.as_ref().map(|(from, to)| {
                (
                    from.display_name().to_string(),
                    to.display_name().to_string(),
                )
            }),
        }
    }
}

/// Convert domain GameTime to protocol GameTime.
pub fn game_time_to_protocol(gt: &GameTime) -> wrldbldr_protocol::types::GameTime {
    wrldbldr_protocol::types::GameTime {
        day: gt.day(),
        hour: gt.hour(),
        minute: gt.minute(),
        is_paused: gt.is_paused(),
    }
}

/// Build TimeAdvanceData for broadcasting.
pub fn build_time_advance_data(
    previous: &GameTime,
    new: &GameTime,
    minutes: u32,
    reason: &TimeAdvanceReason,
) -> wrldbldr_protocol::types::TimeAdvanceData {
    let previous_period = previous.time_of_day();
    let new_period = new.time_of_day();
    let period_changed = previous_period != new_period;

    wrldbldr_protocol::types::TimeAdvanceData {
        previous_time: game_time_to_protocol(previous),
        new_time: game_time_to_protocol(new),
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

    struct FixedClock(chrono::DateTime<chrono::Utc>);

    impl ClockPort for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
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

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let world_entity = Arc::new(repositories::World::new(
            Arc::new(world_repo),
            clock.clone(),
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

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let world_entity = Arc::new(repositories::World::new(
            Arc::new(world_repo),
            clock.clone(),
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

        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));
        let world_entity = Arc::new(repositories::World::new(
            Arc::new(world_repo),
            clock.clone(),
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
