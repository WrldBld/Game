//! Challenge use cases.
//!
//! Handles challenge (skill check) resolution. The flow is:
//! 1. DM or LLM suggests a challenge
//! 2. Player rolls dice (client-side)
//! 3. Roll is submitted for resolution (RollChallenge)
//! 4. Outcome goes to DM for approval
//! 5. Approved outcome triggers effects (ResolveOutcome)

use std::sync::Arc;

use wrldbldr_domain::{
    ApprovalDecisionType, ApprovalRequestData, ApprovalUrgency, ChallengeId, ChallengeOutcomeData,
    OutcomeTrigger, OutcomeType, PlayerCharacterId, ProposedTool, WorldId,
};

use crate::entities::{Challenge, PlayerCharacter};
use crate::infrastructure::ports::{ClockPort, QueuePort, RandomPort, RepoError};

/// Container for challenge use cases.
pub struct ChallengeUseCases {
    pub roll: Arc<RollChallenge>,
    pub resolve: Arc<ResolveOutcome>,
}

impl ChallengeUseCases {
    pub fn new(roll: Arc<RollChallenge>, resolve: Arc<ResolveOutcome>) -> Self {
        Self { roll, resolve }
    }
}

/// Result of a challenge roll.
#[derive(Debug)]
pub struct RollResult {
    /// The raw dice roll value
    pub roll: i32,
    /// The modifier applied
    pub modifier: i32,
    /// The total (roll + modifier)
    pub total: i32,
    /// The outcome type determined
    pub outcome_type: OutcomeType,
    /// Narrative description of the outcome
    pub outcome_description: String,
    /// Whether this goes to DM for approval
    pub requires_approval: bool,
    /// ID of the approval queue item (if approval required)
    pub approval_queue_id: Option<uuid::Uuid>,
}

/// Roll a challenge use case.
///
/// Handles dice rolling and outcome determination. The outcome is then
/// queued for DM approval before effects are applied.
pub struct RollChallenge {
    challenge: Arc<Challenge>,
    player_character: Arc<PlayerCharacter>,
    queue: Arc<dyn QueuePort>,
    random: Arc<dyn RandomPort>,
    clock: Arc<dyn ClockPort>,
}

impl RollChallenge {
    pub fn new(
        challenge: Arc<Challenge>,
        player_character: Arc<PlayerCharacter>,
        queue: Arc<dyn QueuePort>,
        random: Arc<dyn RandomPort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            challenge,
            player_character,
            queue,
            random,
            clock,
        }
    }

    /// Execute a challenge roll.
    ///
    /// # Arguments
    /// * `world_id` - The world context
    /// * `challenge_id` - The challenge being attempted
    /// * `pc_id` - The player character attempting the challenge
    /// * `client_roll` - Optional client-provided roll (if None, server rolls)
    /// * `modifier` - The modifier to apply (skill bonus, etc.)
    ///
    /// # Returns
    /// * `Ok(RollResult)` - The roll result with outcome
    /// * `Err(ChallengeError)` - Failed to process challenge
    pub async fn execute(
        &self,
        world_id: WorldId,
        challenge_id: ChallengeId,
        pc_id: PlayerCharacterId,
        client_roll: Option<i32>,
        modifier: i32,
    ) -> Result<RollResult, ChallengeError> {
        // 1. Get the challenge
        let challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;

        // 2. Get the player character for name
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ChallengeError::PlayerCharacterNotFound)?;

        // 3. Determine the roll value
        let roll = if let Some(r) = client_roll {
            r
        } else {
            // Server-side roll based on difficulty type
            match &challenge.difficulty {
                wrldbldr_domain::Difficulty::DC(_) => self.random.gen_range(1, 20),
                wrldbldr_domain::Difficulty::Percentage(_) => self.random.gen_range(1, 100),
                _ => self.random.gen_range(1, 20), // Default to d20
            }
        };

        // 4. Evaluate the roll
        let (outcome_type, outcome) = challenge.evaluate_roll(roll, modifier);
        let total = roll + modifier;

        // 5. Build outcome data for DM approval
        let outcome_data = ChallengeOutcomeData {
            resolution_id: uuid::Uuid::new_v4().to_string(),
            world_id,
            challenge_id: challenge_id.to_string(),
            challenge_name: challenge.name.clone(),
            challenge_description: challenge.description.clone(),
            skill_name: None, // Would need to fetch from edge
            character_id: wrldbldr_domain::CharacterId::from_uuid(*pc_id.as_uuid()), // Use same UUID
            character_name: pc.name.clone(),
            roll,
            modifier,
            total,
            outcome_type: format!("{:?}", outcome_type),
            outcome_description: outcome.description.clone(),
            outcome_triggers: outcome
                .triggers
                .iter()
                .map(|t| ProposedTool {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("{:?}", t),
                    description: format!("{:?}", t),
                    arguments: serde_json::Value::Null,
                })
                .collect(),
            roll_breakdown: Some(format!("d20({}) + modifier({}) = {}", roll, modifier, total)),
            timestamp: self.clock.now(),
            suggestions: None,
            is_generating_suggestions: false,
        };

        // 6. Queue for DM approval
        let approval_data = ApprovalRequestData {
            world_id,
            source_action_id: uuid::Uuid::new_v4(), // Generate a new action ID
            decision_type: ApprovalDecisionType::ChallengeOutcome,
            urgency: ApprovalUrgency::AwaitingPlayer,
            pc_id: Some(pc_id),
            npc_id: None,
            npc_name: String::new(),
            proposed_dialogue: outcome.description.clone(),
            internal_reasoning: format!(
                "Challenge '{}' - Roll: {} + {} = {} -> {:?}",
                challenge.name, roll, modifier, total, outcome_type
            ),
            proposed_tools: outcome_data.outcome_triggers.clone(),
            retry_count: 0,
            challenge_suggestion: None,
            narrative_event_suggestion: None,
            player_dialogue: None,
            scene_id: None,
            location_id: None,
            game_time: None,
            topics: vec![],
        };

        let approval_queue_id = self
            .queue
            .enqueue_dm_approval(&approval_data)
            .await
            .map_err(|e| ChallengeError::QueueError(e.to_string()))?;

        Ok(RollResult {
            roll,
            modifier,
            total,
            outcome_type,
            outcome_description: outcome.description.clone(),
            requires_approval: true,
            approval_queue_id: Some(approval_queue_id),
        })
    }
}

/// Resolve challenge outcome use case.
///
/// Called after DM approves the outcome to execute triggers.
pub struct ResolveOutcome {
    challenge: Arc<Challenge>,
}

impl ResolveOutcome {
    pub fn new(challenge: Arc<Challenge>) -> Self {
        Self { challenge }
    }

    /// Execute the approved outcome.
    ///
    /// This marks the challenge as resolved and executes any outcome triggers.
    pub async fn execute(
        &self,
        challenge_id: ChallengeId,
        outcome_type: OutcomeType,
    ) -> Result<(), ChallengeError> {
        // Get the challenge to access its outcomes
        let challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;

        // Find the matching outcome based on outcome_type
        let outcome = match outcome_type {
            OutcomeType::CriticalSuccess => challenge
                .outcomes
                .critical_success
                .as_ref()
                .unwrap_or(&challenge.outcomes.success),
            OutcomeType::Success => &challenge.outcomes.success,
            OutcomeType::Partial => challenge
                .outcomes
                .partial
                .as_ref()
                .unwrap_or(&challenge.outcomes.success),
            OutcomeType::Failure => &challenge.outcomes.failure,
            OutcomeType::CriticalFailure => challenge
                .outcomes
                .critical_failure
                .as_ref()
                .unwrap_or(&challenge.outcomes.failure),
        };

        // Execute each trigger in the outcome
        for trigger in &outcome.triggers {
            self.execute_trigger(trigger, &challenge.name).await;
        }

        // Mark the challenge as resolved
        self.challenge.mark_resolved(challenge_id).await?;

        Ok(())
    }

    /// Execute a single outcome trigger.
    ///
    /// Currently logs what would happen; actual implementations will be added
    /// as the corresponding entity modules are wired in.
    async fn execute_trigger(&self, trigger: &OutcomeTrigger, challenge_name: &str) {
        match trigger {
            OutcomeTrigger::RevealInformation { info, persist } => {
                tracing::info!(
                    challenge = %challenge_name,
                    info = %info,
                    persist = %persist,
                    "Would reveal information to player"
                );
            }
            OutcomeTrigger::GiveItem {
                item_name,
                item_description,
            } => {
                tracing::info!(
                    challenge = %challenge_name,
                    item_name = %item_name,
                    item_description = ?item_description,
                    "Would give item to player"
                );
            }
            OutcomeTrigger::TriggerScene { scene_id } => {
                tracing::info!(
                    challenge = %challenge_name,
                    scene_id = %scene_id,
                    "Would trigger scene transition"
                );
            }
            OutcomeTrigger::EnableChallenge { challenge_id } => {
                tracing::debug!(
                    challenge = %challenge_name,
                    target_challenge_id = %challenge_id,
                    "Trigger type EnableChallenge not yet implemented"
                );
            }
            OutcomeTrigger::DisableChallenge { challenge_id } => {
                tracing::debug!(
                    challenge = %challenge_name,
                    target_challenge_id = %challenge_id,
                    "Trigger type DisableChallenge not yet implemented"
                );
            }
            OutcomeTrigger::ModifyCharacterStat { stat, modifier } => {
                tracing::debug!(
                    challenge = %challenge_name,
                    stat = %stat,
                    modifier = %modifier,
                    "Trigger type ModifyCharacterStat not yet implemented"
                );
            }
            OutcomeTrigger::Custom { description } => {
                tracing::debug!(
                    challenge = %challenge_name,
                    description = %description,
                    "Trigger type Custom not yet implemented"
                );
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChallengeError {
    #[error("Challenge not found")]
    NotFound,
    #[error("Player character not found")]
    PlayerCharacterNotFound,
    #[error("Queue error: {0}")]
    QueueError(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
