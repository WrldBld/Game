use std::sync::Arc;

use serde_json::Value;

use wrldbldr_domain::{self as domain, ChallengeId, Difficulty, WorldId};

use crate::infrastructure::ports::RepoError;
use crate::repositories::Challenge;
use crate::use_cases::validation::{require_non_empty, ValidationError};

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
    challenge: Arc<Challenge>,
}

impl ChallengeOps {
    pub fn new(challenge: Arc<Challenge>) -> Self {
        Self { challenge }
    }

    pub async fn list(&self, world_id: WorldId) -> Result<Vec<Value>, ChallengeError> {
        let challenges = self.challenge.list_for_world(world_id).await?;
        Ok(challenges.iter().map(challenge_to_json).collect())
    }

    pub async fn get(&self, challenge_id: ChallengeId) -> Result<Option<Value>, ChallengeError> {
        let challenge = self.challenge.get(challenge_id).await?;
        Ok(challenge.as_ref().map(challenge_to_json))
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        input: CreateChallengeInput,
    ) -> Result<Value, ChallengeError> {
        require_non_empty(&input.name, "Challenge name")?;

        // Note: Difficulty::parse never fails - invalid formats become Difficulty::Custom(string)
        // This is intentional to support freeform difficulty descriptions
        let challenge =
            domain::Challenge::new(world_id, &input.name, Difficulty::parse(&input.difficulty))
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
        Ok(challenge_to_json(&challenge))
    }

    pub async fn update(
        &self,
        challenge_id: ChallengeId,
        input: UpdateChallengeInput,
    ) -> Result<Value, ChallengeError> {
        let mut challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;

        // Since Challenge has private fields, we need to rebuild with updated values
        // Use accessors to get current values, then rebuild with builder pattern
        let name = if let Some(new_name) = input.name {
            require_non_empty(&new_name, "Challenge name")?;
            new_name
        } else {
            challenge.name().to_string()
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
            .unwrap_or_else(|| challenge.outcomes().success().description().to_string());
        let failure_desc = input
            .failure_outcome
            .unwrap_or_else(|| challenge.outcomes().failure().description().to_string());
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
        Ok(challenge_to_json(&challenge))
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
        let challenge = domain::Challenge::new(
            challenge.world_id(),
            challenge.name(),
            challenge.difficulty().clone(),
        )
        .with_id(challenge.id())
        .with_description(challenge.description())
        .with_outcomes(challenge.outcomes().clone())
        .with_active(active)
        .with_order(challenge.order())
        .with_is_favorite(challenge.is_favorite());
        self.challenge.save(&challenge).await?;
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
        let challenge = domain::Challenge::new(
            challenge.world_id(),
            challenge.name(),
            challenge.difficulty().clone(),
        )
        .with_id(challenge.id())
        .with_description(challenge.description())
        .with_outcomes(challenge.outcomes().clone())
        .with_active(challenge.active())
        .with_order(challenge.order())
        .with_is_favorite(favorite);
        self.challenge.save(&challenge).await?;
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

fn challenge_to_json(challenge: &domain::Challenge) -> Value {
    serde_json::json!({
        "id": challenge.id().to_string(),
        "world_id": challenge.world_id().to_string(),
        "scene_id": serde_json::Value::Null,
        "name": challenge.name(),
        "description": challenge.description(),
        "challenge_type": challenge_type_to_str(&challenge.challenge_type()),
        "skill_id": "",
        "difficulty": difficulty_to_json(challenge.difficulty()),
        "outcomes": {
            "success": outcome_to_json(challenge.outcomes().success()),
            "failure": outcome_to_json(challenge.outcomes().failure()),
            "partial": challenge
                .outcomes()
                .partial()
                .map(outcome_to_json),
            "critical_success": challenge
                .outcomes()
                .critical_success()
                .map(outcome_to_json),
            "critical_failure": challenge
                .outcomes()
                .critical_failure()
                .map(outcome_to_json),
        },
        "trigger_conditions": challenge
            .trigger_conditions()
            .iter()
            .map(trigger_condition_to_json)
            .collect::<Vec<_>>(),
        "prerequisite_challenges": Vec::<String>::new(),
        "active": challenge.active(),
        "order": challenge.order(),
        "is_favorite": challenge.is_favorite(),
        "tags": challenge.tags(),
    })
}

fn outcome_to_json(outcome: &domain::Outcome) -> Value {
    serde_json::json!({
        "description": outcome.description(),
        "triggers": outcome
            .triggers()
            .iter()
            .map(outcome_trigger_to_json)
            .collect::<Vec<_>>(),
    })
}

fn outcome_trigger_to_json(trigger: &domain::OutcomeTrigger) -> Value {
    match trigger {
        domain::OutcomeTrigger::RevealInformation { info, persist } => serde_json::json!({
            "type": "reveal_information",
            "info": info,
            "persist": persist,
        }),
        domain::OutcomeTrigger::EnableChallenge { challenge_id } => serde_json::json!({
            "type": "enable_challenge",
            "challenge_id": challenge_id.to_string(),
        }),
        domain::OutcomeTrigger::DisableChallenge { challenge_id } => serde_json::json!({
            "type": "disable_challenge",
            "challenge_id": challenge_id.to_string(),
        }),
        domain::OutcomeTrigger::ModifyCharacterStat { stat, modifier } => serde_json::json!({
            "type": "modify_character_stat",
            "stat": stat,
            "modifier": modifier,
        }),
        domain::OutcomeTrigger::TriggerScene { scene_id } => serde_json::json!({
            "type": "trigger_scene",
            "scene_id": scene_id.to_string(),
        }),
        domain::OutcomeTrigger::GiveItem {
            item_name,
            item_description,
        } => serde_json::json!({
            "type": "give_item",
            "item_name": item_name,
            "item_description": item_description,
        }),
        domain::OutcomeTrigger::Custom { description } => serde_json::json!({
            "type": "custom",
            "description": description,
        }),
    }
}

fn trigger_condition_to_json(condition: &domain::TriggerCondition) -> Value {
    serde_json::json!({
        "condition_type": trigger_type_to_json(condition.condition_type()),
        "description": condition.description(),
        "required": condition.required(),
    })
}

fn trigger_type_to_json(trigger_type: &domain::TriggerType) -> Value {
    match trigger_type {
        domain::TriggerType::ObjectInteraction { keywords } => serde_json::json!({
            "type": "object_interaction",
            "keywords": keywords,
        }),
        domain::TriggerType::EnterArea { area_keywords } => serde_json::json!({
            "type": "enter_area",
            "area_keywords": area_keywords,
        }),
        domain::TriggerType::DialogueTopic { topic_keywords } => serde_json::json!({
            "type": "dialogue_topic",
            "topic_keywords": topic_keywords,
        }),
        domain::TriggerType::ChallengeComplete {
            challenge_id,
            requires_success,
        } => serde_json::json!({
            "type": "challenge_complete",
            "challenge_id": challenge_id.to_string(),
            "requires_success": requires_success,
        }),
        domain::TriggerType::TimeBased { turns } => serde_json::json!({
            "type": "time_based",
            "turns": turns,
        }),
        domain::TriggerType::NpcPresent { npc_keywords } => serde_json::json!({
            "type": "npc_present",
            "npc_keywords": npc_keywords,
        }),
        domain::TriggerType::Custom { description } => serde_json::json!({
            "type": "custom",
            "description": description,
        }),
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

fn difficulty_to_json(difficulty: &domain::Difficulty) -> Value {
    match difficulty {
        domain::Difficulty::DC(value) => serde_json::json!({
            "type": "dc",
            "value": value,
        }),
        domain::Difficulty::Percentage(value) => serde_json::json!({
            "type": "percentage",
            "value": value,
        }),
        domain::Difficulty::Descriptor(descriptor) => serde_json::json!({
            "type": "descriptor",
            "value": descriptor.to_string(),
        }),
        domain::Difficulty::Opposed => serde_json::json!({
            "type": "opposed",
        }),
        domain::Difficulty::Custom(custom) => serde_json::json!({
            "type": "custom",
            "value": custom,
        }),
    }
}
