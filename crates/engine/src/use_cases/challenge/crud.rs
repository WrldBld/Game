use std::sync::Arc;

use wrldbldr_domain::{self as domain, ChallengeId, ChallengeName, Difficulty, WorldId};

use crate::infrastructure::ports::{ChallengeRepo, RepoError};
use crate::use_cases::validation::ValidationError;

// Type alias for old name to maintain compatibility
type ChallengeRepository = dyn ChallengeRepo;

use super::types::{
    ChallengeSummary, DifficultySummary, OutcomeSummary, OutcomeTriggerData, OutcomeTriggerSummary,
    OutcomesSummary, TriggerConditionSummary, TriggerTypeData, TriggerTypeSummary,
};

/// Input for creating a challenge (domain representation).
pub struct CreateChallengeInput {
    pub name: String,
    pub difficulty: String,
    pub description: Option<String>,
    pub success_outcome: Option<String>,
    pub failure_outcome: Option<String>,
}

/// Input for updating a challenge (domain representation).
pub struct UpdateChallengeInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub difficulty: Option<String>,
    pub success_outcome: Option<String>,
    pub failure_outcome: Option<String>,
}

pub struct ChallengeOps {
    challenge: Arc<ChallengeRepository>,
}

impl ChallengeOps {
    pub fn new(challenge: Arc<ChallengeRepository>) -> Self {
        Self { challenge }
    }

    pub async fn list(&self, world_id: WorldId) -> Result<Vec<ChallengeSummary>, ChallengeError> {
        let challenges = self.challenge.list_for_world(world_id).await?;
        Ok(challenges.iter().map(challenge_to_summary).collect())
    }

    pub async fn get(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Option<ChallengeSummary>, ChallengeError> {
        let challenge = self.challenge.get(challenge_id).await?;
        Ok(challenge.as_ref().map(challenge_to_summary))
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        input: CreateChallengeInput,
    ) -> Result<ChallengeSummary, ChallengeError> {
        // Convert input name to ChallengeName with validation
        let challenge_name = ChallengeName::new(&input.name)
            .map_err(|e| ChallengeError::ValidationError(e.to_string()))?;

        // Note: Difficulty::parse never fails - invalid formats become Difficulty::Custom(string)
        // This is intentional to support freeform difficulty descriptions
        let challenge = domain::Challenge::new(
            world_id,
            challenge_name,
            Difficulty::parse(&input.difficulty),
        )
        .with_description(input.description.unwrap_or_default())
        .with_outcomes(domain::ChallengeOutcomes::simple(
            input.success_outcome.unwrap_or_default(),
            input.failure_outcome.unwrap_or_default(),
        ))
        .with_order(0);

        // Validate triggers before saving
        let trigger_errors = challenge.validate_triggers();
        if !trigger_errors.is_empty() {
            return Err(ChallengeError::ValidationError(format!(
                "Invalid triggers: {}",
                trigger_errors.join("; ")
            )));
        }

        self.challenge.save(&challenge).await?;
        Ok(challenge_to_summary(&challenge))
    }

    pub async fn update(
        &self,
        challenge_id: ChallengeId,
        input: UpdateChallengeInput,
    ) -> Result<ChallengeSummary, ChallengeError> {
        let mut challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;

        // Since Challenge has private fields, we need to rebuild with updated values
        // Use accessors to get current values, then rebuild with builder pattern
        let name = if let Some(new_name) = input.name {
            ChallengeName::new(&new_name)
                .map_err(|e| ChallengeError::ValidationError(e.to_string()))?
        } else {
            challenge.name().clone()
        };

        let description = input
            .description
            .unwrap_or_else(|| challenge.description().to_string());

        let difficulty = if let Some(diff_str) = input.difficulty {
            // Note: Difficulty::parse never fails - invalid formats become Difficulty::Custom(string)
            // This is intentional to support freeform difficulty descriptions
            Difficulty::parse(&diff_str)
        } else {
            challenge.difficulty().clone()
        };

        // For outcomes, we need to preserve existing and update what's provided
        let success_desc = input
            .success_outcome
            .unwrap_or_else(|| challenge.outcomes().success.description.clone());
        let failure_desc = input
            .failure_outcome
            .unwrap_or_else(|| challenge.outcomes().failure.description.clone());
        let outcomes = domain::ChallengeOutcomes::simple(success_desc, failure_desc);

        // Rebuild the challenge with updated values
        challenge = domain::Challenge::new(challenge.world_id(), name, difficulty)
            .with_id(challenge.id())
            .with_description(description)
            .with_outcomes(outcomes)
            .with_active(challenge.active())
            .with_order(challenge.order())
            .with_is_favorite(challenge.is_favorite());

        // Validate triggers before saving
        let trigger_errors = challenge.validate_triggers();
        if !trigger_errors.is_empty() {
            return Err(ChallengeError::ValidationError(format!(
                "Invalid triggers: {}",
                trigger_errors.join("; ")
            )));
        }

        self.challenge.save(&challenge).await?;
        Ok(challenge_to_summary(&challenge))
    }

    pub async fn delete(&self, challenge_id: ChallengeId) -> Result<(), ChallengeError> {
        self.challenge.delete(challenge_id).await?;
        Ok(())
    }

    pub async fn set_active(
        &self,
        challenge_id: ChallengeId,
        active: bool,
    ) -> Result<(), ChallengeError> {
        let challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;
        // Rebuild challenge with updated active status
        let updated = domain::Challenge::new(
            challenge.world_id(),
            challenge.name().clone(),
            challenge.difficulty().clone(),
        )
        .with_id(challenge.id())
        .with_description(challenge.description())
        .with_outcomes(challenge.outcomes().clone())
        .with_active(active)
        .with_order(challenge.order())
        .with_is_favorite(challenge.is_favorite());
        self.challenge.save(&updated).await?;
        Ok(())
    }

    pub async fn set_favorite(
        &self,
        challenge_id: ChallengeId,
        favorite: bool,
    ) -> Result<(), ChallengeError> {
        let challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;
        // Rebuild challenge with updated favorite status
        let updated = domain::Challenge::new(
            challenge.world_id(),
            challenge.name().clone(),
            challenge.difficulty().clone(),
        )
        .with_id(challenge.id())
        .with_description(challenge.description())
        .with_outcomes(challenge.outcomes().clone())
        .with_active(challenge.active())
        .with_order(challenge.order())
        .with_is_favorite(favorite);
        self.challenge.save(&updated).await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChallengeError {
    #[error("Challenge not found")]
    NotFound,
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

impl From<ValidationError> for ChallengeError {
    fn from(err: ValidationError) -> Self {
        ChallengeError::ValidationError(err.to_string())
    }
}

fn challenge_to_summary(challenge: &domain::Challenge) -> ChallengeSummary {
    ChallengeSummary {
        id: challenge.id(),
        world_id: challenge.world_id(),
        scene_id: None,
        name: challenge.name().to_string(),
        description: challenge.description().to_string(),
        challenge_type: challenge_type_to_str(&challenge.challenge_type()).to_string(),
        skill_id: String::new(),
        difficulty: difficulty_to_summary(challenge.difficulty()),
        outcomes: OutcomesSummary {
            success: outcome_to_summary(&challenge.outcomes().success),
            failure: outcome_to_summary(&challenge.outcomes().failure),
            partial: challenge
                .outcomes()
                .partial
                .as_ref()
                .map(outcome_to_summary),
            critical_success: challenge
                .outcomes()
                .critical_success
                .as_ref()
                .map(outcome_to_summary),
            critical_failure: challenge
                .outcomes()
                .critical_failure
                .as_ref()
                .map(outcome_to_summary),
        },
        trigger_conditions: challenge
            .trigger_conditions()
            .iter()
            .map(trigger_condition_to_summary)
            .collect(),
        prerequisite_challenges: Vec::new(),
        active: challenge.active(),
        order: challenge.order(),
        is_favorite: challenge.is_favorite(),
        tags: challenge.tags().iter().map(|t| t.to_string()).collect(),
    }
}

fn outcome_to_summary(outcome: &domain::Outcome) -> OutcomeSummary {
    OutcomeSummary {
        description: outcome.description.clone(),
        triggers: outcome
            .triggers
            .iter()
            .map(outcome_trigger_to_summary)
            .collect(),
    }
}

fn outcome_trigger_to_summary(trigger: &domain::OutcomeTrigger) -> OutcomeTriggerSummary {
    match trigger {
        domain::OutcomeTrigger::RevealInformation { info, persist } => OutcomeTriggerSummary {
            trigger_type: "reveal_information".to_string(),
            data: OutcomeTriggerData::RevealInformation {
                info: info.clone(),
                persist: *persist,
            },
        },
        domain::OutcomeTrigger::EnableChallenge { challenge_id } => OutcomeTriggerSummary {
            trigger_type: "enable_challenge".to_string(),
            data: OutcomeTriggerData::EnableChallenge {
                challenge_id: challenge_id.to_string(),
            },
        },
        domain::OutcomeTrigger::DisableChallenge { challenge_id } => OutcomeTriggerSummary {
            trigger_type: "disable_challenge".to_string(),
            data: OutcomeTriggerData::DisableChallenge {
                challenge_id: challenge_id.to_string(),
            },
        },
        domain::OutcomeTrigger::ModifyCharacterStat { stat, modifier } => OutcomeTriggerSummary {
            trigger_type: "modify_character_stat".to_string(),
            data: OutcomeTriggerData::ModifyCharacterStat {
                stat: stat.clone(),
                modifier: *modifier,
            },
        },
        domain::OutcomeTrigger::TriggerScene { scene_id } => OutcomeTriggerSummary {
            trigger_type: "trigger_scene".to_string(),
            data: OutcomeTriggerData::TriggerScene {
                scene_id: scene_id.to_string(),
            },
        },
        domain::OutcomeTrigger::GiveItem {
            item_name,
            item_description,
        } => OutcomeTriggerSummary {
            trigger_type: "give_item".to_string(),
            data: OutcomeTriggerData::GiveItem {
                item_name: item_name.clone(),
                item_description: item_description.clone(),
            },
        },
        domain::OutcomeTrigger::Custom { description } => OutcomeTriggerSummary {
            trigger_type: "custom".to_string(),
            data: OutcomeTriggerData::Custom {
                description: description.clone(),
            },
        },
    }
}

fn trigger_condition_to_summary(condition: &domain::TriggerCondition) -> TriggerConditionSummary {
    TriggerConditionSummary {
        condition_type: trigger_type_to_summary(&condition.condition_type),
        description: condition.description.clone(),
        required: condition.required,
    }
}

fn trigger_type_to_summary(trigger_type: &domain::TriggerType) -> TriggerTypeSummary {
    match trigger_type {
        domain::TriggerType::ObjectInteraction { keywords } => TriggerTypeSummary {
            trigger_type: "object_interaction".to_string(),
            data: TriggerTypeData::ObjectInteraction {
                keywords: keywords.clone(),
            },
        },
        domain::TriggerType::EnterArea { area_keywords } => TriggerTypeSummary {
            trigger_type: "enter_area".to_string(),
            data: TriggerTypeData::EnterArea {
                area_keywords: area_keywords.clone(),
            },
        },
        domain::TriggerType::DialogueTopic { topic_keywords } => TriggerTypeSummary {
            trigger_type: "dialogue_topic".to_string(),
            data: TriggerTypeData::DialogueTopic {
                topic_keywords: topic_keywords.clone(),
            },
        },
        domain::TriggerType::ChallengeComplete {
            challenge_id,
            requires_success,
        } => TriggerTypeSummary {
            trigger_type: "challenge_complete".to_string(),
            data: TriggerTypeData::ChallengeComplete {
                challenge_id: challenge_id.to_string(),
                requires_success: *requires_success,
            },
        },
        domain::TriggerType::TimeBased { turns } => TriggerTypeSummary {
            trigger_type: "time_based".to_string(),
            data: TriggerTypeData::TimeBased { turns: *turns },
        },
        domain::TriggerType::NpcPresent { npc_keywords } => TriggerTypeSummary {
            trigger_type: "npc_present".to_string(),
            data: TriggerTypeData::NpcPresent {
                npc_keywords: npc_keywords.clone(),
            },
        },
        domain::TriggerType::Custom { description } => TriggerTypeSummary {
            trigger_type: "custom".to_string(),
            data: TriggerTypeData::Custom {
                description: description.clone(),
            },
        },
    }
}

fn challenge_type_to_str(challenge_type: &domain::ChallengeType) -> &'static str {
    match challenge_type {
        domain::ChallengeType::SkillCheck => "skill_check",
        domain::ChallengeType::AbilityCheck => "ability_check",
        domain::ChallengeType::SavingThrow => "saving_throw",
        domain::ChallengeType::OpposedCheck => "opposed_check",
        domain::ChallengeType::ComplexChallenge => "complex_challenge",
        domain::ChallengeType::Unknown => "unknown",
    }
}

fn difficulty_to_summary(difficulty: &domain::Difficulty) -> DifficultySummary {
    match difficulty {
        domain::Difficulty::DC(value) => DifficultySummary {
            difficulty_type: "dc".to_string(),
            value: Some(value.to_string()),
        },
        domain::Difficulty::Percentage(value) => DifficultySummary {
            difficulty_type: "percentage".to_string(),
            value: Some(value.to_string()),
        },
        domain::Difficulty::Descriptor(descriptor) => DifficultySummary {
            difficulty_type: "descriptor".to_string(),
            value: Some(descriptor.to_string()),
        },
        domain::Difficulty::Opposed => DifficultySummary {
            difficulty_type: "opposed".to_string(),
            value: None,
        },
        domain::Difficulty::Custom(custom) => DifficultySummary {
            difficulty_type: "custom".to_string(),
            value: Some(custom.clone()),
        },
    }
}
