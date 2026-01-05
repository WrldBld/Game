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
    GameTime, PlayerCharacterId, TimeAdvanceReason, TimeMode, TimeOfDay, WorldId,
};

use crate::entities::World;
use crate::infrastructure::ports::{ClockPort, RepoError};

/// Container for time use cases.
pub struct TimeUseCases {
    pub suggest_time: Arc<SuggestTime>,
}

impl TimeUseCases {
    pub fn new(suggest_time: Arc<SuggestTime>) -> Self {
        Self { suggest_time }
    }
}

// =============================================================================
// Time Suggestion
// =============================================================================

/// Data for a pending time suggestion.
#[derive(Debug, Clone)]
pub struct TimeSuggestion {
    pub id: Uuid,
    pub world_id: WorldId,
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub action_type: String,
    pub action_description: String,
    pub suggested_minutes: u32,
    pub current_time: GameTime,
    pub resulting_time: GameTime,
    pub period_change: Option<(TimeOfDay, TimeOfDay)>,
}

/// Result of suggesting time passage.
#[derive(Debug)]
pub enum SuggestTimeResult {
    /// Time was auto-advanced (mode = Auto)
    AutoAdvanced {
        previous_time: GameTime,
        new_time: GameTime,
        minutes: u32,
        reason: TimeAdvanceReason,
    },
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
///    - Auto: Advances time immediately
///    - Suggested: Creates a suggestion for DM approval
///    - Manual: Does nothing (DM advances manually)
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
    /// - `AutoAdvanced` if mode is Auto and time was advanced
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

        let config = &world.time_config;
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
            TimeMode::Auto => {
                // Auto-advance time immediately
                // Note: The actual world update should be done by the caller
                // after receiving this result, to maintain transaction boundaries.
                // Period change detection is handled by build_time_advance_data().
                let mut new_time = world.game_time.clone();
                new_time.advance_minutes(cost_minutes);

                let reason = self.build_reason(action_type, &action_description);

                Ok(SuggestTimeResult::AutoAdvanced {
                    previous_time: world.game_time.clone(),
                    new_time,
                    minutes: cost_minutes,
                    reason,
                })
            }
            TimeMode::Suggested => {
                // Create suggestion for DM approval
                let mut resulting_time = world.game_time.clone();
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
                    current_time: world.game_time.clone(),
                    resulting_time,
                    period_change,
                };

                Ok(SuggestTimeResult::SuggestionCreated(suggestion))
            }
        }
    }

    /// Build a TimeAdvanceReason from action type and description.
    fn build_reason(&self, action_type: &str, description: &str) -> TimeAdvanceReason {
        match action_type {
            "travel_location" => {
                // Try to parse "from X to Y" from description
                TimeAdvanceReason::TravelLocation {
                    from: "previous location".to_string(),
                    to: description.to_string(),
                }
            }
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
}

#[derive(Debug, thiserror::Error)]
pub enum SuggestTimeError {
    #[error("World not found")]
    WorldNotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
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
                (from.display_name().to_string(), to.display_name().to_string())
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
