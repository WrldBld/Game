//! Challenge Use Case Port
//!
//! Inbound port trait defining the contract for challenge operations.
//!
//! # Overview
//!
//! This port abstracts the ChallengeUseCase for adapter consumption, enabling:
//! - Dice roll submission for active challenges
//! - Challenge triggering by DM against target characters
//! - Ad-hoc challenge creation by DM
//! - Challenge outcome decisions and approval workflow
//! - Outcome suggestion and branch selection
//!
//! # Authorization
//!
//! Methods are split into player-facing and DM-only operations:
//! - Player operations: `submit_roll`, `submit_dice_input`
//! - DM operations: All other methods require `ctx.is_dm == true`
//!
//! # Architecture
//!
//! This trait is implemented by `ChallengeUseCase` in the application layer.
//! Adapters (e.g., WebSocket handlers) depend on this trait to invoke
//! challenge operations without coupling to the concrete implementation.

use async_trait::async_trait;

#[cfg(any(test, feature = "testing"))]
use mockall::automock;

use super::use_case_errors::ChallengeError;
use super::UseCaseContext;
use crate::outbound::{
    AdHocResult, ChallengeSuggestionDecisionInput, CreateAdHocInput, DiscardChallengeInput,
    DiscardResult, OutcomeDecisionInput, OutcomeDecisionResult, RegenerateOutcomeInput,
    RegenerateResult, RequestBranchesInput, RequestSuggestionInput, RollResultData,
    SelectBranchInput, SubmitDiceInputInput, SubmitRollInput, TriggerChallengeInput, TriggerResult,
};

// =============================================================================
// Challenge Use Case Port
// =============================================================================

/// Inbound port for challenge use case operations
///
/// This trait defines the contract for all challenge-related operations
/// that adapters can invoke. It abstracts the application layer's
/// ChallengeUseCase implementation.
///
/// # Example
///
/// ```ignore
/// use wrldbldr_engine_ports::inbound::{ChallengeUseCasePort, UseCaseContext};
/// use wrldbldr_engine_ports::outbound::SubmitRollInput;
///
/// async fn handle_roll(
///     challenge_port: &dyn ChallengeUseCasePort,
///     ctx: UseCaseContext,
///     challenge_id: String,
///     roll: i32,
/// ) {
///     let input = SubmitRollInput { challenge_id, roll };
///     match challenge_port.submit_roll(ctx, input).await {
///         Ok(result) => println!("Roll result: {:?}", result),
///         Err(e) => eprintln!("Roll failed: {}", e),
///     }
/// }
/// ```
#[cfg_attr(any(test, feature = "testing"), automock)]
#[async_trait]
pub trait ChallengeUseCasePort: Send + Sync {
    /// Submit a dice roll for an active challenge
    ///
    /// Player-facing operation: submits a roll result for challenge resolution.
    /// The roll is evaluated against the challenge's difficulty and the outcome
    /// is determined based on the rule system.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context containing world/user/PC info
    /// * `input` - Roll submission containing challenge_id and roll value
    ///
    /// # Returns
    ///
    /// * `Ok(RollResultData)` - The result of the roll including outcome type and description
    /// * `Err(ChallengeError::PcNotFound)` - If the PC is not found
    /// * `Err(ChallengeError::ChallengeNotFound)` - If the challenge doesn't exist
    /// * `Err(ChallengeError::ResolutionFailed)` - If resolution service fails
    async fn submit_roll(
        &self,
        ctx: UseCaseContext,
        input: SubmitRollInput,
    ) -> Result<RollResultData, ChallengeError>;

    /// Submit dice input (formula or manual value) for a challenge
    ///
    /// Player-facing operation: submits dice input which can be either a
    /// dice formula (e.g., "1d20+5") or a manual value.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context containing world/user/PC info
    /// * `input` - Dice input containing challenge_id and input_type (Formula or Manual)
    ///
    /// # Returns
    ///
    /// * `Ok(RollResultData)` - The result of the roll including outcome
    /// * `Err(ChallengeError)` - If the operation fails
    async fn submit_dice_input(
        &self,
        ctx: UseCaseContext,
        input: SubmitDiceInputInput,
    ) -> Result<RollResultData, ChallengeError>;

    /// Trigger a challenge against a target character
    ///
    /// DM-only operation: Initiates a challenge against a target character,
    /// sending them a challenge prompt with difficulty and skill requirements.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm == true`)
    /// * `input` - Trigger input containing challenge_id and target_character_id
    ///
    /// # Returns
    ///
    /// * `Ok(TriggerResult)` - Challenge prompt details for the target
    /// * `Err(ChallengeError::NotAuthorized)` - If caller is not DM
    /// * `Err(ChallengeError::ChallengeNotFound)` - If challenge doesn't exist
    /// * `Err(ChallengeError::TargetNotFound)` - If target character doesn't exist
    async fn trigger_challenge(
        &self,
        ctx: UseCaseContext,
        input: TriggerChallengeInput,
    ) -> Result<TriggerResult, ChallengeError>;

    /// Handle DM's decision on a challenge suggestion
    ///
    /// DM-only operation: Processes approval or rejection of a challenge
    /// suggestion, optionally with modified difficulty.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm == true`)
    /// * `input` - Decision input with request_id, approved flag, and optional modified_difficulty
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Decision was processed successfully
    /// * `Err(ChallengeError::NotAuthorized)` - If caller is not DM
    /// * `Err(ChallengeError::ResolutionFailed)` - If processing fails
    async fn suggestion_decision(
        &self,
        ctx: UseCaseContext,
        input: ChallengeSuggestionDecisionInput,
    ) -> Result<(), ChallengeError>;

    /// Regenerate challenge outcome text
    ///
    /// DM-only operation: Requests regeneration of outcome flavor text,
    /// optionally for a specific outcome type and with guidance.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm == true`)
    /// * `input` - Regeneration request with request_id, optional outcome_type, and guidance
    ///
    /// # Returns
    ///
    /// * `Ok(RegenerateResult)` - New outcome text with flavor and scene direction
    /// * `Err(ChallengeError::NotAuthorized)` - If caller is not DM
    async fn regenerate_outcome(
        &self,
        ctx: UseCaseContext,
        input: RegenerateOutcomeInput,
    ) -> Result<RegenerateResult, ChallengeError>;

    /// Discard a challenge from the approval queue
    ///
    /// DM-only operation: Removes a pending challenge from the approval queue,
    /// optionally with feedback for logging/analytics.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm == true`)
    /// * `input` - Discard request with request_id and optional feedback
    ///
    /// # Returns
    ///
    /// * `Ok(DiscardResult)` - Confirmation with the discarded request_id
    /// * `Err(ChallengeError::NotAuthorized)` - If caller is not DM
    async fn discard_challenge(
        &self,
        ctx: UseCaseContext,
        input: DiscardChallengeInput,
    ) -> Result<DiscardResult, ChallengeError>;

    /// Create an ad-hoc challenge
    ///
    /// DM-only operation: Creates a custom challenge on-the-fly with
    /// specified name, skill, difficulty, target, and outcomes.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm == true`)
    /// * `input` - Ad-hoc challenge definition including outcomes for success/failure
    ///
    /// # Returns
    ///
    /// * `Ok(AdHocResult)` - Created challenge details
    /// * `Err(ChallengeError::NotAuthorized)` - If caller is not DM
    /// * `Err(ChallengeError::ResolutionFailed)` - If creation fails
    async fn create_adhoc(
        &self,
        ctx: UseCaseContext,
        input: CreateAdHocInput,
    ) -> Result<AdHocResult, ChallengeError>;

    /// Handle DM's decision on a challenge outcome
    ///
    /// DM-only operation: Processes the DM's decision on a resolved challenge
    /// outcome (accept, edit, or suggest alternatives).
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm == true`)
    /// * `input` - Decision with resolution_id and decision type (Accept/Edit/Suggest)
    ///
    /// # Returns
    ///
    /// * `Ok(OutcomeDecisionResult)` - Result indicating if suggestions are pending
    /// * `Err(ChallengeError::NotAuthorized)` - If caller is not DM
    /// * `Err(ChallengeError::ResolutionFailed)` - If processing fails
    async fn outcome_decision(
        &self,
        ctx: UseCaseContext,
        input: OutcomeDecisionInput,
    ) -> Result<OutcomeDecisionResult, ChallengeError>;

    /// Request AI-generated outcome suggestions
    ///
    /// DM-only operation: Requests the AI to generate alternative outcome
    /// suggestions for a resolved challenge, optionally with guidance.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm == true`)
    /// * `input` - Request with resolution_id and optional guidance for the AI
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Request was submitted (suggestions delivered asynchronously)
    /// * `Err(ChallengeError::NotAuthorized)` - If caller is not DM
    /// * `Err(ChallengeError::ResolutionFailed)` - If request fails
    async fn request_suggestion(
        &self,
        ctx: UseCaseContext,
        input: RequestSuggestionInput,
    ) -> Result<(), ChallengeError>;

    /// Request branching outcome options
    ///
    /// DM-only operation: Requests multiple branching outcome options for
    /// a resolved challenge, allowing the DM to choose a narrative path.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm == true`)
    /// * `input` - Request with resolution_id and optional guidance
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Request was submitted (branches delivered asynchronously)
    /// * `Err(ChallengeError::NotAuthorized)` - If caller is not DM
    /// * `Err(ChallengeError::ResolutionFailed)` - If request fails
    async fn request_branches(
        &self,
        ctx: UseCaseContext,
        input: RequestBranchesInput,
    ) -> Result<(), ChallengeError>;

    /// Select a specific outcome branch
    ///
    /// DM-only operation: Selects one of the previously requested branches
    /// as the final outcome, optionally with modifications to the description.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Use case context (must have `is_dm == true`)
    /// * `input` - Selection with resolution_id, branch_id, and optional modified_description
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Branch was selected and outcome finalized
    /// * `Err(ChallengeError::NotAuthorized)` - If caller is not DM
    /// * `Err(ChallengeError::ResolutionFailed)` - If selection fails
    async fn select_branch(
        &self,
        ctx: UseCaseContext,
        input: SelectBranchInput,
    ) -> Result<(), ChallengeError>;
}
