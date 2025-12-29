//! Trigger Evaluation Service - Evaluates narrative event triggers
//!
//! This service is responsible for checking if any active narrative events
//! have satisfied triggers. It queries the graph for entity-based triggers
//! and checks JSON-stored triggers (flags, stats, custom, time).
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
//! Events can be triggered from two sources:
//! 1. **Engine-detected**: This service evaluates game state against trigger conditions
//! 2. **LLM-suggested**: The LLM can suggest triggers via `<narrative_event_suggestion>` tags
//!
//! Both sources feed into the DM approval queue before execution.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::{debug, info, instrument, warn};

use async_trait::async_trait;
use wrldbldr_domain::entities::{NarrativeEvent, TriggerContext, TriggerEvaluation};
use wrldbldr_domain::{ChallengeId, CharacterId, LocationId, NarrativeEventId, WorldId};
use wrldbldr_engine_ports::outbound::{
    CompletedChallenge as PortCompletedChallenge,
    CompletedNarrativeEvent as PortCompletedNarrativeEvent,
    GameStateSnapshot as PortGameStateSnapshot, ImmediateContext as PortImmediateContext,
    NarrativeEventCrudPort, PlayerCharacterRepositoryPort, StoryEventEdgePort, StoryEventQueryPort,
    TriggerEvaluationResult as PortTriggerEvaluationResult, TriggerEvaluationServicePort,
    TriggerSource as PortTriggerSource, TriggeredEventCandidate as PortTriggeredEventCandidate,
};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during trigger evaluation
#[derive(Debug, thiserror::Error)]
pub enum TriggerEvaluationError {
    #[error("Failed to fetch narrative events: {0}")]
    EventFetch(String),

    #[error("Failed to fetch game state: {0}")]
    StateFetch(String),

    #[error("Failed to build trigger context: {0}")]
    ContextBuild(String),

    #[error("Repository error: {0}")]
    Repository(#[from] anyhow::Error),
}

// =============================================================================
// Result Types
// =============================================================================

/// Source of a trigger suggestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

impl TriggerEvaluationResult {
    pub fn empty() -> Self {
        Self {
            ready_to_trigger: Vec::new(),
            partially_satisfied: Vec::new(),
            total_evaluated: 0,
        }
    }
}

// =============================================================================
// Game State for Trigger Context
// =============================================================================

/// Game state snapshot used to build trigger context
///
/// This struct holds the current state of the game session that's needed
/// to evaluate trigger conditions. It can be built from various sources
/// (repositories, session state, etc.)
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
    pub challenge_id: ChallengeId,
    pub was_successful: bool,
}

/// Information about a completed narrative event
#[derive(Debug, Clone)]
pub struct CompletedNarrativeEvent {
    pub event_id: NarrativeEventId,
    pub outcome_name: String,
}

impl GameStateSnapshot {
    /// Convert to the domain TriggerContext used by NarrativeEvent::evaluate_triggers
    pub fn to_trigger_context(&self) -> TriggerContext {
        TriggerContext {
            current_location: self.current_location_id,
            current_scene: None, // Scene context could be added if needed
            time_context: None,
            flags: self.flags.clone(),
            inventory: self.inventory.clone(),
            completed_events: self.completed_event_ids.clone(),
            event_outcomes: self.event_outcomes.clone(),
            turns_since_event: self.turns_since_event.clone(),
            completed_challenges: self.completed_challenge_ids.clone(),
            challenge_successes: self.challenge_successes.clone(),
            turn_count: self.turn_count,
            recent_dialogue_topics: self.recent_dialogue_topics.clone(),
            recent_player_action: None,
        }
    }
}

// =============================================================================
// Service Implementation
// =============================================================================

/// Service for evaluating narrative event triggers
///
/// This service checks all active narrative events to see if their trigger
/// conditions are satisfied. Events that pass evaluation are candidates
/// for DM approval before being executed.
///
/// # Dependencies (ISP)
///
/// This service uses minimal trait dependencies following Interface Segregation:
/// - `NarrativeEventCrudPort`: For get, list_pending, list_by_world operations
/// - `StoryEventQueryPort`: For list_by_world to find completed challenges
/// - `StoryEventEdgePort`: For get_recorded_challenge to check challenge completions
pub struct TriggerEvaluationService {
    narrative_event_crud: Arc<dyn NarrativeEventCrudPort>,
    player_character_repo: Arc<dyn PlayerCharacterRepositoryPort>,
    story_event_query: Arc<dyn StoryEventQueryPort>,
    story_event_edge: Arc<dyn StoryEventEdgePort>,
}

impl TriggerEvaluationService {
    /// Create a new TriggerEvaluationService
    ///
    /// # Arguments
    ///
    /// * `narrative_event_crud` - For CRUD operations on narrative events
    /// * `player_character_repo` - For getting player character data
    /// * `story_event_query` - For querying story events by world
    /// * `story_event_edge` - For getting recorded challenge relationships
    pub fn new(
        narrative_event_crud: Arc<dyn NarrativeEventCrudPort>,
        player_character_repo: Arc<dyn PlayerCharacterRepositoryPort>,
        story_event_query: Arc<dyn StoryEventQueryPort>,
        story_event_edge: Arc<dyn StoryEventEdgePort>,
    ) -> Self {
        Self {
            narrative_event_crud,
            player_character_repo,
            story_event_query,
            story_event_edge,
        }
    }

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
    #[instrument(skip(self, game_state), fields(world_id = %world_id))]
    pub async fn evaluate_triggers(
        &self,
        world_id: WorldId,
        game_state: &GameStateSnapshot,
    ) -> Result<TriggerEvaluationResult, TriggerEvaluationError> {
        // Fetch all active (pending) narrative events
        let active_events = self
            .narrative_event_crud
            .list_pending(world_id)
            .await
            .map_err(|e| TriggerEvaluationError::EventFetch(e.to_string()))?;

        if active_events.is_empty() {
            debug!("No active narrative events to evaluate");
            return Ok(TriggerEvaluationResult::empty());
        }

        info!(
            event_count = active_events.len(),
            "Evaluating narrative event triggers"
        );

        // Convert game state to trigger context
        let trigger_context = game_state.to_trigger_context();

        let mut ready_to_trigger = Vec::new();
        let mut partially_satisfied = Vec::new();

        for event in active_events {
            let evaluation = event.evaluate_triggers(&trigger_context);

            debug!(
                event_id = %event.id,
                event_name = %event.name,
                is_triggered = evaluation.is_triggered,
                confidence = evaluation.confidence,
                matched = evaluation.matched_triggers.len(),
                total = evaluation.total_triggers,
                "Evaluated event triggers"
            );

            let candidate = TriggeredEventCandidate {
                event: event.clone(),
                evaluation: evaluation.clone(),
                source: TriggerSource::Engine,
                reason: None,
            };

            if evaluation.is_triggered {
                ready_to_trigger.push(candidate);
            } else if evaluation.confidence > 0.0 {
                // At least some triggers matched
                partially_satisfied.push(candidate);
            }
        }

        // Sort by priority (higher priority first)
        ready_to_trigger.sort_by(|a, b| b.event.priority.cmp(&a.event.priority));
        partially_satisfied.sort_by(|a, b| {
            b.evaluation
                .confidence
                .partial_cmp(&a.evaluation.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total = ready_to_trigger.len() + partially_satisfied.len();

        info!(
            ready = ready_to_trigger.len(),
            partial = partially_satisfied.len(),
            "Trigger evaluation complete"
        );

        Ok(TriggerEvaluationResult {
            ready_to_trigger,
            partially_satisfied,
            total_evaluated: total,
        })
    }

    /// Check if a specific event's triggers are satisfied
    ///
    /// This is useful for checking a single event without evaluating all events.
    #[instrument(skip(self, game_state), fields(event_id = %event_id))]
    pub async fn check_event_triggers(
        &self,
        event_id: NarrativeEventId,
        game_state: &GameStateSnapshot,
    ) -> Result<Option<TriggeredEventCandidate>, TriggerEvaluationError> {
        let event = self
            .narrative_event_crud
            .get(event_id)
            .await
            .map_err(|e| TriggerEvaluationError::EventFetch(e.to_string()))?;

        let Some(event) = event else {
            warn!(event_id = %event_id, "Event not found for trigger check");
            return Ok(None);
        };

        // Skip if already triggered (and not repeatable)
        if event.is_triggered && !event.is_repeatable {
            debug!(event_id = %event_id, "Event already triggered and not repeatable");
            return Ok(None);
        }

        // Skip if not active
        if !event.is_active {
            debug!(event_id = %event_id, "Event is not active");
            return Ok(None);
        }

        let trigger_context = game_state.to_trigger_context();
        let evaluation = event.evaluate_triggers(&trigger_context);

        if evaluation.is_triggered {
            Ok(Some(TriggeredEventCandidate {
                event,
                evaluation,
                source: TriggerSource::Engine,
                reason: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Build a game state snapshot from repositories
    ///
    /// This helper method builds a GameStateSnapshot by querying the repositories
    /// for the current state. It's useful when you don't have a pre-built snapshot.
    ///
    /// # Arguments
    ///
    /// * `world_id` - The world to build state for
    /// * `session_id` - The current session
    /// * `player_character_id` - Optional player character to get location/inventory from
    /// * `immediate_context` - Optional immediate context (just completed challenge, etc.)
    #[instrument(skip(self, immediate_context))]
    pub async fn build_game_state_snapshot(
        &self,
        world_id: WorldId,
        player_character_id: Option<wrldbldr_domain::PlayerCharacterId>,
        immediate_context: Option<ImmediateContext>,
    ) -> Result<GameStateSnapshot, TriggerEvaluationError> {
        let mut snapshot = GameStateSnapshot::default();

        // Get player character location if provided
        if let Some(pc_id) = player_character_id {
            if let Ok(Some(pc)) = self.player_character_repo.get(pc_id).await {
                snapshot.current_location_id = Some(pc.current_location_id);
                // TODO: Get inventory from player character when implemented
            }
        }

        // Get completed narrative events
        let triggered_events = self
            .narrative_event_crud
            .list_by_world(world_id)
            .await
            .map_err(|e| TriggerEvaluationError::StateFetch(e.to_string()))?;

        for event in triggered_events {
            if event.is_triggered {
                snapshot.completed_event_ids.push(event.id);
                if let Some(outcome) = event.selected_outcome {
                    snapshot.event_outcomes.insert(event.id, outcome);
                }
            }
        }

        // Get completed challenges from story events
        // Note: This is a simplified approach - in production, you might want
        // a dedicated challenge completion tracking system
        let story_events = self
            .story_event_query
            .list_by_world(world_id)
            .await
            .map_err(|e| TriggerEvaluationError::StateFetch(e.to_string()))?;

        for story_event in story_events {
            // Check if this story event records a challenge
            if let Ok(Some(challenge_id)) = self
                .story_event_edge
                .get_recorded_challenge(story_event.id)
                .await
            {
                snapshot.completed_challenge_ids.push(challenge_id);
                // Determine success from story event tags or data
                let was_success = story_event
                    .tags
                    .iter()
                    .any(|t| t == "success" || t == "challenge_success");
                snapshot
                    .challenge_successes
                    .insert(challenge_id, was_success);
            }
        }

        // Apply immediate context if provided
        if let Some(ctx) = immediate_context {
            if let Some(challenge) = ctx.just_completed_challenge {
                snapshot.just_completed_challenge = Some(challenge.clone());
                // Also add to the completed lists
                snapshot
                    .completed_challenge_ids
                    .push(challenge.challenge_id);
                snapshot
                    .challenge_successes
                    .insert(challenge.challenge_id, challenge.was_successful);
            }
            if let Some(event) = ctx.just_completed_event {
                snapshot.just_completed_event = Some(event.clone());
                snapshot.completed_event_ids.push(event.event_id);
                snapshot
                    .event_outcomes
                    .insert(event.event_id, event.outcome_name.clone());
            }
            if let Some(char_id) = ctx.talking_to_character_id {
                snapshot.talking_to_character_id = Some(char_id);
            }
            if !ctx.recent_dialogue_topics.is_empty() {
                snapshot.recent_dialogue_topics = ctx.recent_dialogue_topics;
            }
            snapshot.flags = ctx.game_flags;
            snapshot.turn_count = ctx.turn_count;
        }

        Ok(snapshot)
    }

    /// Create an LLM-suggested trigger candidate
    ///
    /// This method creates a TriggeredEventCandidate from an LLM suggestion.
    /// The event is validated to ensure it exists and is active.
    #[instrument(skip(self))]
    pub async fn create_llm_suggestion(
        &self,
        event_id: NarrativeEventId,
        reason: String,
    ) -> Result<Option<TriggeredEventCandidate>, TriggerEvaluationError> {
        let event = self
            .narrative_event_crud
            .get(event_id)
            .await
            .map_err(|e| TriggerEvaluationError::EventFetch(e.to_string()))?;

        let Some(event) = event else {
            warn!(event_id = %event_id, "LLM suggested event not found");
            return Ok(None);
        };

        // Validate the event is triggerable
        if !event.is_active {
            warn!(event_id = %event_id, "LLM suggested event is not active");
            return Ok(None);
        }

        if event.is_triggered && !event.is_repeatable {
            warn!(event_id = %event_id, "LLM suggested event already triggered");
            return Ok(None);
        }

        // Create a candidate with LLM source
        // Note: We create a "synthetic" evaluation since the LLM bypassed normal trigger checks
        let evaluation = TriggerEvaluation {
            is_triggered: true, // LLM says it should trigger
            matched_triggers: vec!["llm_suggested".to_string()],
            unmatched_triggers: vec![],
            total_triggers: 1,
            confidence: 1.0,
        };

        Ok(Some(TriggeredEventCandidate {
            event,
            evaluation,
            source: TriggerSource::Llm,
            reason: Some(reason),
        }))
    }
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

// =============================================================================
// Port Implementation
// =============================================================================

/// Implementation of the `TriggerEvaluationServicePort` for `TriggerEvaluationService`.
///
/// This exposes trigger evaluation methods to infrastructure adapters.
#[async_trait]
impl TriggerEvaluationServicePort for TriggerEvaluationService {
    async fn evaluate_triggers(
        &self,
        world_id: WorldId,
        game_state: &PortGameStateSnapshot,
    ) -> anyhow::Result<PortTriggerEvaluationResult> {
        // Convert port type to internal type
        let internal_state = GameStateSnapshot {
            current_location_id: game_state.current_location_id,
            talking_to_character_id: game_state.talking_to_character_id,
            just_completed_challenge: game_state.just_completed_challenge.as_ref().map(|c| {
                CompletedChallenge {
                    challenge_id: c.challenge_id,
                    was_successful: c.was_successful,
                }
            }),
            just_completed_event: game_state.just_completed_event.as_ref().map(|e| {
                CompletedNarrativeEvent {
                    event_id: e.event_id,
                    outcome_name: e.outcome_name.clone(),
                }
            }),
            flags: game_state.flags.clone(),
            inventory: game_state.inventory.clone(),
            completed_event_ids: game_state.completed_event_ids.clone(),
            event_outcomes: game_state.event_outcomes.clone(),
            completed_challenge_ids: game_state.completed_challenge_ids.clone(),
            challenge_successes: game_state.challenge_successes.clone(),
            turns_since_event: game_state.turns_since_event.clone(),
            turn_count: game_state.turn_count,
            recent_dialogue_topics: game_state.recent_dialogue_topics.clone(),
        };

        let result = TriggerEvaluationService::evaluate_triggers(self, world_id, &internal_state)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Convert internal result to port result
        Ok(PortTriggerEvaluationResult {
            ready_to_trigger: result
                .ready_to_trigger
                .into_iter()
                .map(|c| PortTriggeredEventCandidate {
                    event: c.event,
                    evaluation: c.evaluation,
                    source: match c.source {
                        TriggerSource::Engine => PortTriggerSource::Engine,
                        TriggerSource::Llm => PortTriggerSource::Llm,
                        TriggerSource::DmManual => PortTriggerSource::DmManual,
                    },
                    reason: c.reason,
                })
                .collect(),
            partially_satisfied: result
                .partially_satisfied
                .into_iter()
                .map(|c| PortTriggeredEventCandidate {
                    event: c.event,
                    evaluation: c.evaluation,
                    source: match c.source {
                        TriggerSource::Engine => PortTriggerSource::Engine,
                        TriggerSource::Llm => PortTriggerSource::Llm,
                        TriggerSource::DmManual => PortTriggerSource::DmManual,
                    },
                    reason: c.reason,
                })
                .collect(),
            total_evaluated: result.total_evaluated,
        })
    }

    async fn check_event_triggers(
        &self,
        event_id: NarrativeEventId,
        game_state: &PortGameStateSnapshot,
    ) -> anyhow::Result<Option<PortTriggeredEventCandidate>> {
        // Convert port type to internal type
        let internal_state = GameStateSnapshot {
            current_location_id: game_state.current_location_id,
            talking_to_character_id: game_state.talking_to_character_id,
            just_completed_challenge: game_state.just_completed_challenge.as_ref().map(|c| {
                CompletedChallenge {
                    challenge_id: c.challenge_id,
                    was_successful: c.was_successful,
                }
            }),
            just_completed_event: game_state.just_completed_event.as_ref().map(|e| {
                CompletedNarrativeEvent {
                    event_id: e.event_id,
                    outcome_name: e.outcome_name.clone(),
                }
            }),
            flags: game_state.flags.clone(),
            inventory: game_state.inventory.clone(),
            completed_event_ids: game_state.completed_event_ids.clone(),
            event_outcomes: game_state.event_outcomes.clone(),
            completed_challenge_ids: game_state.completed_challenge_ids.clone(),
            challenge_successes: game_state.challenge_successes.clone(),
            turns_since_event: game_state.turns_since_event.clone(),
            turn_count: game_state.turn_count,
            recent_dialogue_topics: game_state.recent_dialogue_topics.clone(),
        };

        let result =
            TriggerEvaluationService::check_event_triggers(self, event_id, &internal_state)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(result.map(|c| PortTriggeredEventCandidate {
            event: c.event,
            evaluation: c.evaluation,
            source: match c.source {
                TriggerSource::Engine => PortTriggerSource::Engine,
                TriggerSource::Llm => PortTriggerSource::Llm,
                TriggerSource::DmManual => PortTriggerSource::DmManual,
            },
            reason: c.reason,
        }))
    }

    async fn build_game_state_snapshot(
        &self,
        world_id: WorldId,
        player_character_id: Option<wrldbldr_domain::PlayerCharacterId>,
        immediate_context: Option<PortImmediateContext>,
    ) -> anyhow::Result<PortGameStateSnapshot> {
        // Convert port type to internal type
        let internal_context = immediate_context.map(|ctx| ImmediateContext {
            just_completed_challenge: ctx.just_completed_challenge.map(|c| CompletedChallenge {
                challenge_id: c.challenge_id,
                was_successful: c.was_successful,
            }),
            just_completed_event: ctx.just_completed_event.map(|e| CompletedNarrativeEvent {
                event_id: e.event_id,
                outcome_name: e.outcome_name,
            }),
            talking_to_character_id: ctx.talking_to_character_id,
            recent_dialogue_topics: ctx.recent_dialogue_topics,
            game_flags: ctx.game_flags,
            turn_count: ctx.turn_count,
        });

        let result = TriggerEvaluationService::build_game_state_snapshot(
            self,
            world_id,
            player_character_id,
            internal_context,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Convert internal result to port result
        Ok(PortGameStateSnapshot {
            current_location_id: result.current_location_id,
            talking_to_character_id: result.talking_to_character_id,
            just_completed_challenge: result.just_completed_challenge.map(|c| {
                PortCompletedChallenge {
                    challenge_id: c.challenge_id,
                    was_successful: c.was_successful,
                }
            }),
            just_completed_event: result.just_completed_event.map(|e| {
                PortCompletedNarrativeEvent {
                    event_id: e.event_id,
                    outcome_name: e.outcome_name,
                }
            }),
            flags: result.flags,
            inventory: result.inventory,
            completed_event_ids: result.completed_event_ids,
            event_outcomes: result.event_outcomes,
            completed_challenge_ids: result.completed_challenge_ids,
            challenge_successes: result.challenge_successes,
            turns_since_event: result.turns_since_event,
            turn_count: result.turn_count,
            recent_dialogue_topics: result.recent_dialogue_topics,
        })
    }

    async fn create_llm_suggestion(
        &self,
        event_id: NarrativeEventId,
        reason: String,
    ) -> anyhow::Result<Option<PortTriggeredEventCandidate>> {
        let result = TriggerEvaluationService::create_llm_suggestion(self, event_id, reason)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(result.map(|c| PortTriggeredEventCandidate {
            event: c.event,
            evaluation: c.evaluation,
            source: match c.source {
                TriggerSource::Engine => PortTriggerSource::Engine,
                TriggerSource::Llm => PortTriggerSource::Llm,
                TriggerSource::DmManual => PortTriggerSource::DmManual,
            },
            reason: c.reason,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain::entities::{NarrativeTrigger, NarrativeTriggerType, TriggerLogic};

    #[test]
    fn test_game_state_to_trigger_context() {
        let mut state = GameStateSnapshot::default();
        state.current_location_id = Some(LocationId::new());
        state.flags.insert("found_key".to_string(), true);
        state.inventory.push("magic_sword".to_string());
        state.turn_count = 5;

        let context = state.to_trigger_context();

        assert!(context.current_location.is_some());
        assert_eq!(context.flags.get("found_key"), Some(&true));
        assert!(context.inventory.contains(&"magic_sword".to_string()));
        assert_eq!(context.turn_count, 5);
    }

    #[test]
    fn test_trigger_evaluation_result_empty() {
        let result = TriggerEvaluationResult::empty();
        assert!(result.ready_to_trigger.is_empty());
        assert!(result.partially_satisfied.is_empty());
        assert_eq!(result.total_evaluated, 0);
    }

    #[test]
    fn test_triggered_event_candidate_creation() {
        let mut event = NarrativeEvent::new(WorldId::new(), "Test Event", chrono::Utc::now());
        event.trigger_conditions.push(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::FlagSet {
                flag_name: "test_flag".to_string(),
            },
            description: "Test flag is set".to_string(),
            is_required: false,
            trigger_id: "t1".to_string(),
        });
        event.trigger_logic = TriggerLogic::All;

        let evaluation = TriggerEvaluation {
            is_triggered: true,
            matched_triggers: vec!["t1".to_string()],
            unmatched_triggers: vec![],
            total_triggers: 1,
            confidence: 1.0,
        };

        let candidate = TriggeredEventCandidate {
            event,
            evaluation,
            source: TriggerSource::Engine,
            reason: None,
        };

        assert_eq!(candidate.source, TriggerSource::Engine);
        assert!(candidate.evaluation.is_triggered);
    }
}
