use async_trait::async_trait;

use wrldbldr_domain::value_objects::NarrativeResolutionConfig;
use wrldbldr_domain::{CharacterId, PlayerCharacterId, WorldId};

use super::{
    AdHocOutcomes, AdHocResult, DiceInputType, NarrativeRollContext, RollResultData, TriggerResult,
};

/// Outbound port for challenge resolution operations.
///
/// Implemented by adapters; used by the application.
#[async_trait]
pub trait ChallengeResolutionPort: Send + Sync {
    /// Handle a dice roll submission
    async fn handle_roll(
        &self,
        world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        roll: i32,
        narrative_config: &NarrativeResolutionConfig,
        narrative_context: Option<&NarrativeRollContext>,
    ) -> Result<RollResultData, String>;

    /// Handle dice input (formula or manual)
    async fn handle_roll_input(
        &self,
        world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        input_type: DiceInputType,
        narrative_config: &NarrativeResolutionConfig,
        narrative_context: Option<&NarrativeRollContext>,
    ) -> Result<RollResultData, String>;

    /// Trigger a challenge against a target
    async fn trigger_challenge(
        &self,
        world_id: &WorldId,
        challenge_id: String,
        target_character_id: CharacterId,
    ) -> Result<TriggerResult, String>;

    /// Handle DM's decision on a suggestion
    async fn handle_suggestion_decision(
        &self,
        world_id: &WorldId,
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    ) -> Result<(), String>;

    /// Create an ad-hoc challenge
    async fn create_adhoc_challenge(
        &self,
        world_id: &WorldId,
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: PlayerCharacterId,
        outcomes: AdHocOutcomes,
    ) -> Result<AdHocResult, String>;
}
