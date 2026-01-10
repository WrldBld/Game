//! Challenge use cases.
//!
//! Handles challenge (skill check) resolution. The flow is:
//! 1. DM or LLM suggests a challenge
//! 2. Player rolls dice (client-side)
//! 3. Roll is submitted for resolution (RollChallenge)
//! 4. Outcome goes to DM for approval
//! 5. Approved outcome triggers effects (ResolveOutcome)

use std::sync::Arc;

use uuid::Uuid;
use wrldbldr_domain::{
    ApprovalDecisionType, ApprovalRequestData, ApprovalUrgency, ChallengeId, ChallengeOutcomeData,
    DiceRollInput, OutcomeTrigger, OutcomeType, PlayerCharacterId, ProposedTool, WorldId,
};
use wrldbldr_domain::value_objects::DiceParseError;

mod crud;

pub use crud::{ChallengeError as ChallengeCrudError, ChallengeOps};

use crate::entities::{Challenge, Inventory, Observation, PlayerCharacter, Scene};
use crate::infrastructure::ports::{ClockPort, QueuePort, RandomPort, RepoError};

/// Container for challenge use cases.
pub struct ChallengeUseCases {
    pub roll: Arc<RollChallenge>,
    pub resolve: Arc<ResolveOutcome>,
    pub trigger_prompt: Arc<TriggerChallengePrompt>,
    pub outcome_decision: Arc<OutcomeDecision>,
    pub ops: Arc<ChallengeOps>,
}

impl ChallengeUseCases {
    pub fn new(
        roll: Arc<RollChallenge>,
        resolve: Arc<ResolveOutcome>,
        trigger_prompt: Arc<TriggerChallengePrompt>,
        outcome_decision: Arc<OutcomeDecision>,
        ops: Arc<ChallengeOps>,
    ) -> Self {
        Self {
            roll,
            resolve,
            trigger_prompt,
            outcome_decision,
            ops,
        }
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
    /// Challenge ID
    pub challenge_id: ChallengeId,
    /// Challenge name
    pub challenge_name: String,
    /// Character ID who attempted the challenge
    pub character_id: PlayerCharacterId,
    /// Character name who attempted the challenge
    pub character_name: String,
    /// Outcome triggers (tools) to execute on approval
    pub outcome_triggers: Vec<ProposedTool>,
    /// Roll breakdown string (e.g., "d20(18) + modifier(3) = 21")
    pub roll_breakdown: Option<String>,
}

/// Challenge prompt data for sending to players.
#[derive(Debug, Clone)]
pub struct ChallengePromptData {
    pub challenge_id: ChallengeId,
    pub challenge_name: String,
    pub difficulty_display: String,
    pub description: String,
    pub skill_name: String,
    pub character_modifier: i32,
    pub suggested_dice: Option<String>,
    pub rule_system_hint: Option<String>,
}

/// Build a challenge prompt for a player.
pub struct TriggerChallengePrompt {
    challenge: Arc<Challenge>,
}

impl TriggerChallengePrompt {
    pub fn new(challenge: Arc<Challenge>) -> Self {
        Self { challenge }
    }

    pub async fn execute(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<ChallengePromptData, ChallengeError> {
        let challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;

        let difficulty_display = match &challenge.difficulty {
            wrldbldr_domain::Difficulty::DC(dc) => format!("DC {}", dc),
            wrldbldr_domain::Difficulty::Percentage(pct) => format!("{}%", pct),
            wrldbldr_domain::Difficulty::Opposed => "Opposed".to_string(),
            wrldbldr_domain::Difficulty::Descriptor(desc) => format!("{:?}", desc),
            wrldbldr_domain::Difficulty::Custom(custom) => custom.clone(),
        };

        Ok(ChallengePromptData {
            challenge_id,
            challenge_name: challenge.name.clone(),
            difficulty_display,
            description: challenge.description.clone(),
            skill_name: String::new(),
            character_modifier: 0,
            suggested_dice: Some("1d20".to_string()),
            rule_system_hint: None,
        })
    }
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
                    name: t.trigger_type_name().to_string(),
                    description: t.to_string(),
                    arguments: serde_json::Value::Null,
                })
                .collect(),
            roll_breakdown: Some(format!(
                "d20({}) + modifier({}) = {}",
                roll, modifier, total
            )),
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
            challenge_outcome: Some(outcome_data.clone()),
            player_dialogue: None,
            scene_id: None,
            location_id: None,
            game_time: None,
            topics: vec![],
            conversation_id: None, // Challenges don't have conversation context
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
            challenge_id,
            challenge_name: challenge.name.clone(),
            character_id: pc_id,
            character_name: pc.name.clone(),
            outcome_triggers: outcome_data.outcome_triggers,
            roll_breakdown: outcome_data.roll_breakdown,
        })
    }

    pub async fn execute_with_input(
        &self,
        world_id: WorldId,
        challenge_id: ChallengeId,
        pc_id: PlayerCharacterId,
        input: DiceRollInput,
    ) -> Result<RollResult, ChallengeError> {
        let roll_result = input
            .resolve(|min, max| self.random.gen_range(min, max))
            .map_err(ChallengeError::DiceParse)?;

        self.execute(
            world_id,
            challenge_id,
            pc_id,
            Some(roll_result.dice_total),
            roll_result.modifier_applied,
        )
        .await
    }
}

/// Resolve challenge outcome use case.
///
/// Called after DM approves the outcome to execute triggers.
pub struct ResolveOutcome {
    challenge: Arc<Challenge>,
    inventory: Arc<Inventory>,
    observation: Arc<Observation>,
    scene: Arc<Scene>,
    player_character: Arc<PlayerCharacter>,
}

impl ResolveOutcome {
    pub fn new(
        challenge: Arc<Challenge>,
        inventory: Arc<Inventory>,
        observation: Arc<Observation>,
        scene: Arc<Scene>,
        player_character: Arc<PlayerCharacter>,
    ) -> Self {
        Self {
            challenge,
            inventory,
            observation,
            scene,
            player_character,
        }
    }

    /// Execute the approved outcome with a known target PC.
    ///
    /// This variant is used when we know which PC attempted the challenge.
    pub async fn execute_for_pc(
        &self,
        challenge_id: ChallengeId,
        outcome_type: OutcomeType,
        target_pc_id: PlayerCharacterId,
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
            if let Err(e) = self
                .execute_trigger(trigger, &challenge.name, challenge.world_id, target_pc_id)
                .await
            {
                tracing::warn!(
                    challenge = %challenge.name,
                    error = %e,
                    "Failed to execute trigger, continuing with remaining triggers"
                );
            }
        }

        // Mark the challenge as resolved
        self.challenge.mark_resolved(challenge_id).await?;

        Ok(())
    }

    /// Execute a single outcome trigger.
    ///
    /// # Arguments
    /// * `trigger` - The trigger to execute
    /// * `challenge_name` - For logging context
    /// * `world_id` - The world context for the trigger
    /// * `target_pc_id` - The player character affected by this trigger (if applicable)
    async fn execute_trigger(
        &self,
        trigger: &OutcomeTrigger,
        challenge_name: &str,
        world_id: WorldId,
        target_pc_id: PlayerCharacterId,
    ) -> Result<(), ChallengeError> {
        match trigger {
            OutcomeTrigger::RevealInformation { info, persist } => {
                tracing::info!(
                    challenge = %challenge_name,
                    info = %info,
                    persist = %persist,
                    target_pc = %target_pc_id,
                    "Revealing information to player"
                );

                // If persistent, create an observation for the PC
                if *persist {
                    // Create a "deduced" observation from the challenge
                    // This records that the PC learned this information
                    if let Err(e) = self
                        .observation
                        .record_deduced_info(target_pc_id, info.clone())
                        .await
                    {
                        tracing::warn!(error = %e, "Failed to persist revealed information");
                    }
                }
                Ok(())
            }
            OutcomeTrigger::GiveItem {
                item_name,
                item_description,
            } => {
                tracing::info!(
                    challenge = %challenge_name,
                    item_name = %item_name,
                    item_description = ?item_description,
                    target_pc = %target_pc_id,
                    "Giving item to player"
                );

                // Create a new item and add it to the PC's inventory
                if let Err(e) = self
                    .inventory
                    .give_item_to_pc(target_pc_id, item_name.clone(), item_description.clone())
                    .await
                {
                    tracing::warn!(error = %e, "Failed to give item to player");
                }
                Ok(())
            }
            OutcomeTrigger::TriggerScene { scene_id } => {
                tracing::info!(
                    challenge = %challenge_name,
                    scene_id = %scene_id,
                    "Triggering scene transition"
                );

                // Set the scene as current for this world
                if let Err(e) = self.scene.set_current(world_id, *scene_id).await {
                    tracing::warn!(error = %e, "Failed to set current scene");
                }
                Ok(())
            }
            OutcomeTrigger::EnableChallenge { challenge_id } => {
                tracing::info!(
                    challenge = %challenge_name,
                    target_challenge_id = %challenge_id,
                    "Enabling challenge"
                );

                // Enable the target challenge (make it available)
                if let Err(e) = self.challenge.set_enabled(*challenge_id, true).await {
                    tracing::warn!(error = %e, "Failed to enable challenge");
                }
                Ok(())
            }
            OutcomeTrigger::DisableChallenge { challenge_id } => {
                tracing::info!(
                    challenge = %challenge_name,
                    target_challenge_id = %challenge_id,
                    "Disabling challenge"
                );

                // Disable the target challenge (remove from available)
                if let Err(e) = self.challenge.set_enabled(*challenge_id, false).await {
                    tracing::warn!(error = %e, "Failed to disable challenge");
                }
                Ok(())
            }
            OutcomeTrigger::ModifyCharacterStat { stat, modifier } => {
                tracing::info!(
                    challenge = %challenge_name,
                    stat = %stat,
                    modifier = %modifier,
                    target_pc = %target_pc_id,
                    "Modifying character stat"
                );

                if let Err(e) = self
                    .player_character
                    .modify_stat(target_pc_id, stat, *modifier)
                    .await
                {
                    tracing::warn!(error = %e, "Failed to modify character stat");
                }
                Ok(())
            }
            OutcomeTrigger::Custom { description } => {
                // Custom triggers are logged for DM reference but not automatically executed
                tracing::info!(
                    challenge = %challenge_name,
                    description = %description,
                    "Custom trigger (requires DM action)"
                );
                Ok(())
            }
        }
    }
}

/// Decision flow for challenge outcome approvals.
pub struct OutcomeDecision {
    queue: Arc<dyn QueuePort>,
    resolve: Arc<ResolveOutcome>,
}

impl OutcomeDecision {
    pub fn new(queue: Arc<dyn QueuePort>, resolve: Arc<ResolveOutcome>) -> Self {
        Self { queue, resolve }
    }

    pub async fn execute(
        &self,
        world_id: WorldId,
        resolution_id: String,
        decision: wrldbldr_protocol::ChallengeOutcomeDecisionData,
    ) -> Result<OutcomeDecisionResult, OutcomeDecisionError> {
        let approval_id = Uuid::parse_str(&resolution_id)
            .map_err(|_| OutcomeDecisionError::InvalidResolutionId)?;

        let approval_data = self
            .queue
            .get_approval_request(approval_id)
            .await
            .map_err(|e| OutcomeDecisionError::QueueError(e.to_string()))?
            .ok_or(OutcomeDecisionError::ApprovalNotFound)?;

        let outcome_data = approval_data
            .challenge_outcome
            .ok_or(OutcomeDecisionError::MissingOutcomeData)?;

        let challenge_id = parse_challenge_id_str(&outcome_data.challenge_id)
            .ok_or(OutcomeDecisionError::InvalidChallengeId)?;
        let outcome_type = parse_outcome_type(&outcome_data.outcome_type);

        match decision {
            wrldbldr_protocol::ChallengeOutcomeDecisionData::Accept => {
                let pc_id = approval_data.pc_id.ok_or(OutcomeDecisionError::MissingPcId)?;
                self.resolve
                    .execute_for_pc(challenge_id, outcome_type.clone(), pc_id)
                    .await
                    .map_err(OutcomeDecisionError::Resolve)?;

                if let Err(e) = self.queue.mark_complete(approval_id).await {
                    tracing::warn!(error = %e, "Failed to mark approval request as complete");
                }

                Ok(OutcomeDecisionResult::Resolved(ChallengeResolvedPayload {
                    challenge_id: outcome_data.challenge_id.clone(),
                    challenge_name: outcome_data.challenge_name.clone(),
                    character_name: outcome_data.character_name.clone(),
                    roll: outcome_data.roll,
                    modifier: outcome_data.modifier,
                    total: outcome_data.total,
                    outcome: outcome_type_to_str(&outcome_type).to_string(),
                    outcome_description: outcome_data.outcome_description.clone(),
                    roll_breakdown: outcome_data.roll_breakdown.clone(),
                }))
            }
            wrldbldr_protocol::ChallengeOutcomeDecisionData::Edit { modified_description } => {
                let pc_id = approval_data.pc_id.ok_or(OutcomeDecisionError::MissingPcId)?;
                self.resolve
                    .execute_for_pc(challenge_id, outcome_type.clone(), pc_id)
                    .await
                    .map_err(OutcomeDecisionError::Resolve)?;

                if let Err(e) = self.queue.mark_complete(approval_id).await {
                    tracing::warn!(error = %e, "Failed to mark approval request as complete");
                }

                Ok(OutcomeDecisionResult::Resolved(ChallengeResolvedPayload {
                    challenge_id: outcome_data.challenge_id.clone(),
                    challenge_name: outcome_data.challenge_name.clone(),
                    character_name: outcome_data.character_name.clone(),
                    roll: outcome_data.roll,
                    modifier: outcome_data.modifier,
                    total: outcome_data.total,
                    outcome: outcome_type_to_str(&outcome_type).to_string(),
                    outcome_description: modified_description,
                    roll_breakdown: outcome_data.roll_breakdown.clone(),
                }))
            }
            wrldbldr_protocol::ChallengeOutcomeDecisionData::Suggest { guidance } => {
                let llm_request = wrldbldr_domain::LlmRequestData {
                    request_type: wrldbldr_domain::LlmRequestType::OutcomeSuggestion {
                        resolution_id: approval_id,
                        world_id,
                        challenge_name: outcome_data.challenge_name.clone(),
                        current_description: outcome_data.outcome_description.clone(),
                        guidance: guidance.clone(),
                    },
                    world_id,
                    pc_id: approval_data.pc_id,
                    prompt: None,
                    suggestion_context: Some(wrldbldr_domain::SuggestionContext {
                        entity_type: Some("challenge_outcome".to_string()),
                        entity_name: Some(outcome_data.challenge_name.clone()),
                        world_setting: None,
                        hints: guidance.clone(),
                        additional_context: Some(format!(
                            "Current outcome: {} ({})\nRoll: {} + {} = {}",
                            outcome_data.outcome_description,
                            outcome_data.outcome_type,
                            outcome_data.roll,
                            outcome_data.modifier,
                            outcome_data.total
                        )),
                        world_id: Some(world_id),
                    }),
                    callback_id: format!("outcome_suggestion:{}", approval_id),
                    conversation_id: None,
                };

                self.queue
                    .enqueue_llm_request(&llm_request)
                    .await
                    .map_err(|e| OutcomeDecisionError::QueueError(e.to_string()))?;

                Ok(OutcomeDecisionResult::Queued)
            }
            wrldbldr_protocol::ChallengeOutcomeDecisionData::Unknown => {
                Err(OutcomeDecisionError::InvalidDecision)
            }
        }
    }
}

pub enum OutcomeDecisionResult {
    Resolved(ChallengeResolvedPayload),
    Queued,
}

pub struct ChallengeResolvedPayload {
    pub challenge_id: String,
    pub challenge_name: String,
    pub character_name: String,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
    pub outcome: String,
    pub outcome_description: String,
    pub roll_breakdown: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OutcomeDecisionError {
    #[error("Approval request not found")]
    ApprovalNotFound,
    #[error("Invalid resolution ID")]
    InvalidResolutionId,
    #[error("Invalid challenge ID")]
    InvalidChallengeId,
    #[error("No challenge outcome data in approval request")]
    MissingOutcomeData,
    #[error("Missing target PC")]
    MissingPcId,
    #[error("Invalid decision")]
    InvalidDecision,
    #[error("Queue error: {0}")]
    QueueError(String),
    #[error("Resolve error: {0}")]
    Resolve(#[from] ChallengeError),
}

fn parse_challenge_id_str(id_str: &str) -> Option<ChallengeId> {
    Uuid::parse_str(id_str)
        .ok()
        .map(ChallengeId::from_uuid)
}

fn parse_outcome_type(outcome_type: &str) -> OutcomeType {
    match outcome_type {
        "CriticalSuccess" => OutcomeType::CriticalSuccess,
        "Success" => OutcomeType::Success,
        "Partial" => OutcomeType::Partial,
        "Failure" => OutcomeType::Failure,
        "CriticalFailure" => OutcomeType::CriticalFailure,
        _ => OutcomeType::Success,
    }
}

fn outcome_type_to_str(outcome_type: &OutcomeType) -> &'static str {
    match outcome_type {
        OutcomeType::CriticalSuccess => "critical_success",
        OutcomeType::Success => "success",
        OutcomeType::Partial => "partial",
        OutcomeType::Failure => "failure",
        OutcomeType::CriticalFailure => "critical_failure",
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChallengeError {
    #[error("Challenge not found")]
    NotFound,
    #[error("Player character not found")]
    PlayerCharacterNotFound,
    #[error("Missing target player character for challenge outcome")]
    MissingTargetPc,
    #[error("Dice parse error: {0}")]
    DiceParse(#[from] DiceParseError),
    #[error("Queue error: {0}")]
    QueueError(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use chrono::Utc;
    use wrldbldr_domain::{
        Challenge as DomainChallenge, ChallengeId, ChallengeOutcomes, Difficulty, ItemId,
        LocationId, Outcome, OutcomeTrigger, OutcomeType, PlayerCharacter as DomainPc,
        PlayerCharacterId, SceneId, WorldId,
    };

    use crate::entities;
    use crate::infrastructure::ports::{
        ClockPort, MockChallengeRepo, MockCharacterRepo, MockItemRepo, MockLocationRepo,
        MockObservationRepo, MockPlayerCharacterRepo, MockSceneRepo,
    };

    struct FixedClock(chrono::DateTime<chrono::Utc>);

    impl ClockPort for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
    }

    #[tokio::test]
    async fn resolve_outcome_executes_pc_dependent_triggers() {
        let world_id = WorldId::new();
        let pc_id = PlayerCharacterId::new();
        let challenge_id = ChallengeId::new();
        let scene_id = SceneId::new();
        let now = Utc::now();

        let pc = DomainPc::new("user-1", world_id, "PC", LocationId::new(), now);

        let success_outcome = Outcome::new("success")
            .with_trigger(OutcomeTrigger::reveal_persistent("secret"))
            .with_trigger(OutcomeTrigger::GiveItem {
                item_name: "Key".to_string(),
                item_description: Some("Rusty".to_string()),
            })
            .with_trigger(OutcomeTrigger::modify_stat("hp", -1))
            .with_trigger(OutcomeTrigger::scene(scene_id));

        let outcomes = ChallengeOutcomes {
            success: success_outcome,
            failure: Outcome::new("failure"),
            partial: None,
            critical_success: None,
            critical_failure: None,
        };

        let challenge = DomainChallenge::new(world_id, "Test Challenge", Difficulty::DC(10))
            .with_outcomes(outcomes);

        // ---------------------------------------------------------------------
        // Challenge repo expectations
        // ---------------------------------------------------------------------
        let mut challenge_repo = MockChallengeRepo::new();
        let challenge_for_get = challenge.clone();
        challenge_repo
            .expect_get()
            .withf(move |id| *id == challenge_id)
            .returning(move |_| Ok(Some(challenge_for_get.clone())));
        challenge_repo
            .expect_mark_resolved()
            .withf(move |id| *id == challenge_id)
            .returning(|_| Ok(()));

        // ---------------------------------------------------------------------
        // Observation expectations
        // ---------------------------------------------------------------------
        let mut observation_repo = MockObservationRepo::new();
        observation_repo
            .expect_save_deduced_info()
            .withf(move |id, info| *id == pc_id && info == "secret")
            .returning(|_, _| Ok(()));

        // ---------------------------------------------------------------------
        // Inventory expectations (give_item_to_pc)
        // ---------------------------------------------------------------------
        let expected_item_id: Arc<Mutex<Option<ItemId>>> = Arc::new(Mutex::new(None));
        let expected_item_id_for_save = expected_item_id.clone();
        let mut item_repo = MockItemRepo::new();
        item_repo
            .expect_save()
            .withf(|item| item.name == "Key" && item.description.as_deref() == Some("Rusty"))
            .returning(move |item| {
                let expected_item_id_for_save = expected_item_id_for_save.clone();
                let item_id = item.id;
                *expected_item_id_for_save.lock().unwrap() = Some(item_id);
                Ok(())
            });

        let expected_item_id_for_add = expected_item_id.clone();
        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));
        pc_repo
            .expect_add_to_inventory()
            .withf(move |id, item_id| {
                *id == pc_id && Some(*item_id) == *expected_item_id_for_add.lock().unwrap()
            })
            .returning(|_, _| Ok(()));
        pc_repo
            .expect_modify_stat()
            .withf(move |id, stat, delta| *id == pc_id && stat == "hp" && *delta == -1)
            .returning(|_, _, _| Ok(()));

        let character_repo = MockCharacterRepo::new();

        // ---------------------------------------------------------------------
        // Scene expectations
        // ---------------------------------------------------------------------
        let mut scene_repo = MockSceneRepo::new();
        scene_repo
            .expect_set_current()
            .withf(move |w, s| *w == world_id && *s == scene_id)
            .returning(|_, _| Ok(()));

        // Observation entity needs LocationRepo + ClockPort, but this test only
        // exercises record_deduced_info, so provide dummies.
        let location_repo = MockLocationRepo::new();
        let clock: Arc<dyn ClockPort> = Arc::new(FixedClock(now));

        // ---------------------------------------------------------------------
        // Wire entities + use case
        // ---------------------------------------------------------------------
        let challenge_entity = Arc::new(entities::Challenge::new(Arc::new(challenge_repo)));

        let pc_repo: Arc<dyn crate::infrastructure::ports::PlayerCharacterRepo> = Arc::new(pc_repo);
        let inventory_entity = Arc::new(entities::Inventory::new(
            Arc::new(item_repo),
            Arc::new(character_repo),
            pc_repo.clone(),
        ));
        let observation_entity = Arc::new(entities::Observation::new(
            Arc::new(observation_repo),
            Arc::new(location_repo),
            clock,
        ));
        let scene_entity = Arc::new(entities::Scene::new(Arc::new(scene_repo)));
        let player_character_entity = Arc::new(entities::PlayerCharacter::new(pc_repo));

        let resolve = super::ResolveOutcome::new(
            challenge_entity,
            inventory_entity,
            observation_entity,
            scene_entity,
            player_character_entity,
        );

        resolve
            .execute_for_pc(challenge_id, OutcomeType::Success, pc_id)
            .await
            .expect("resolve outcome should succeed");
    }
}
