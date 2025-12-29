//! Trigger evaluation service port - Interface for evaluating narrative event triggers
//!
//! This port abstracts the trigger evaluation logic that checks if narrative events
//! should fire based on current game state.
//!
//! # Architecture
//!
//! The service follows hexagonal architecture:
//! - Depends on repository ports for data access
//! - Returns domain-level results
//! - Can be called by the DM approval queue or game loop
//!
//! # Trigger Sources
//!
//! Events can be triggered from multiple sources:
//! 1. **Engine-detected**: This service evaluates game state against trigger conditions
//! 2. **LLM-suggested**: The LLM can suggest triggers via narrative_event_suggestion tags
//! 3. **DM-manual**: The DM can manually trigger events
//!
//! All sources feed into the DM approval queue before execution.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use wrldbldr_domain::entities::{NarrativeEvent, TriggerEvaluation};
use wrldbldr_domain::{
    ChallengeId, CharacterId, LocationId, NarrativeEventId, PlayerCharacterId, WorldId,
};

/// Source of a trigger suggestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    /// Engine detected that trigger conditions are satisfied
    Engine,
    /// LLM suggested this event should trigger
    Llm,
    /// DM manually triggered the event
    DmManual,
}

/// A narrative event that has been evaluated and is ready for triggering
#[derive(Debug, Clone)]
pub struct TriggeredEventCandidate {
    /// The event that may trigger
    pub event: NarrativeEvent,
    /// Evaluation result showing which triggers matched
    pub evaluation: TriggerEvaluation,
    /// Source of this trigger suggestion
    pub source: TriggerSource,
    /// Optional reason (for LLM suggestions)
    pub reason: Option<String>,
}

/// Result of evaluating all active triggers
#[derive(Debug, Clone)]
pub struct TriggerEvaluationResult {
    /// Events that are ready to trigger (all conditions met)
    pub ready_to_trigger: Vec<TriggeredEventCandidate>,
    /// Events that are partially satisfied (for DM visibility)
    pub partially_satisfied: Vec<TriggeredEventCandidate>,
    /// Total events evaluated
    pub total_evaluated: usize,
}

/// Game state snapshot used to build trigger context
///
/// This struct holds the current state of the game session that's needed
/// to evaluate trigger conditions.
#[derive(Debug, Clone, Default)]
pub struct GameStateSnapshot {
    /// Current player location
    pub current_location_id: Option<LocationId>,
    /// Character the player is currently talking to (if any)
    pub talking_to_character_id: Option<CharacterId>,
    /// Challenge that was just completed (if any)
    pub just_completed_challenge: Option<CompletedChallenge>,
    /// Narrative event that was just completed (if any)
    pub just_completed_event: Option<CompletedNarrativeEvent>,
    /// Game flags (boolean flags set during gameplay)
    pub flags: HashMap<String, bool>,
    /// Player inventory (item names)
    pub inventory: Vec<String>,
    /// IDs of completed narrative events
    pub completed_event_ids: Vec<NarrativeEventId>,
    /// Outcomes of completed events (event_id -> outcome_name)
    pub event_outcomes: HashMap<NarrativeEventId, String>,
    /// IDs of completed challenges
    pub completed_challenge_ids: Vec<ChallengeId>,
    /// Success status of completed challenges
    pub challenge_successes: HashMap<ChallengeId, bool>,
    /// Turns elapsed since event (for TurnCount triggers)
    pub turns_since_event: HashMap<NarrativeEventId, u32>,
    /// Total turn count for the session
    pub turn_count: u32,
    /// Recent dialogue topics (keywords from conversation)
    pub recent_dialogue_topics: Vec<String>,
}

/// Information about a completed challenge
#[derive(Debug, Clone)]
pub struct CompletedChallenge {
    /// The challenge ID
    pub challenge_id: ChallengeId,
    /// Whether the challenge was successful
    pub was_successful: bool,
}

/// Information about a completed narrative event
#[derive(Debug, Clone)]
pub struct CompletedNarrativeEvent {
    /// The event ID
    pub event_id: NarrativeEventId,
    /// The outcome that was selected
    pub outcome_name: String,
}

/// Immediate context for trigger evaluation
///
/// This struct holds context about things that just happened in the current
/// game turn, which may not yet be persisted to repositories.
#[derive(Debug, Clone, Default)]
pub struct ImmediateContext {
    /// Challenge that was just completed this turn
    pub just_completed_challenge: Option<CompletedChallenge>,
    /// Narrative event that was just completed this turn
    pub just_completed_event: Option<CompletedNarrativeEvent>,
    /// Character being talked to
    pub talking_to_character_id: Option<CharacterId>,
    /// Recent dialogue topics from this conversation
    pub recent_dialogue_topics: Vec<String>,
    /// Current game flags
    pub game_flags: HashMap<String, bool>,
    /// Current turn count
    pub turn_count: u32,
}

/// Port for trigger evaluation service operations
///
/// This trait defines the application use cases for evaluating narrative event triggers.
/// It checks active events against the current game state to determine which events
/// should be suggested to the DM for triggering.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait TriggerEvaluationServicePort: Send + Sync {
    /// Evaluate all active narrative events for a world
    ///
    /// This method fetches all active (non-triggered) events and evaluates
    /// their trigger conditions against the provided game state.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to evaluate events for
    /// * `game_state` - Current game state snapshot
    ///
    /// # Returns
    ///
    /// A result containing events ready to trigger and partially satisfied events.
    async fn evaluate_triggers(
        &self,
        world_id: WorldId,
        game_state: &GameStateSnapshot,
    ) -> Result<TriggerEvaluationResult>;

    /// Check if a specific event's triggers are satisfied
    ///
    /// This is useful for checking a single event without evaluating all events.
    async fn check_event_triggers(
        &self,
        event_id: NarrativeEventId,
        game_state: &GameStateSnapshot,
    ) -> Result<Option<TriggeredEventCandidate>>;

    /// Build a game state snapshot from repositories
    ///
    /// This helper method builds a GameStateSnapshot by querying the repositories
    /// for the current state. It's useful when you don't have a pre-built snapshot.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to build state for
    /// * `player_character_id` - Optional player character to get location/inventory from
    /// * `immediate_context` - Optional immediate context (just completed challenge, etc.)
    async fn build_game_state_snapshot(
        &self,
        world_id: WorldId,
        player_character_id: Option<PlayerCharacterId>,
        immediate_context: Option<ImmediateContext>,
    ) -> Result<GameStateSnapshot>;

    /// Create an LLM-suggested trigger candidate
    ///
    /// This method creates a TriggeredEventCandidate from an LLM suggestion.
    /// The event is validated to ensure it exists and is active.
    async fn create_llm_suggestion(
        &self,
        event_id: NarrativeEventId,
        reason: String,
    ) -> Result<Option<TriggeredEventCandidate>>;
}
