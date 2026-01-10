use std::sync::Arc;

use serde_json::Value;

use wrldbldr_domain::{self as domain, ChallengeId, Difficulty, WorldId};
use wrldbldr_protocol::requests::{CreateChallengeData, UpdateChallengeData};

use crate::entities::Challenge;
use crate::infrastructure::ports::RepoError;

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
        data: CreateChallengeData,
    ) -> Result<Value, ChallengeError> {
        // Note: Difficulty::parse never fails - invalid formats become Difficulty::Custom(string)
        // This is intentional to support freeform difficulty descriptions
        let mut challenge = domain::Challenge::new(
            world_id,
            &data.name,
            Difficulty::parse(&data.difficulty),
        );
        challenge.description = data.description.unwrap_or_default();
        challenge.outcomes.success.description = data.success_outcome.unwrap_or_default();
        challenge.outcomes.failure.description = data.failure_outcome.unwrap_or_default();
        challenge.order = 0;

        self.challenge.save(&challenge).await?;
        Ok(challenge_to_json(&challenge))
    }

    pub async fn update(
        &self,
        challenge_id: ChallengeId,
        data: UpdateChallengeData,
    ) -> Result<Value, ChallengeError> {
        let mut challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;

        if let Some(name) = data.name {
            challenge.name = name;
        }
        if let Some(description) = data.description {
            challenge.description = description;
        }
        if let Some(difficulty) = data.difficulty {
            // Note: Difficulty::parse never fails - invalid formats become Difficulty::Custom(string)
            // This is intentional to support freeform difficulty descriptions
            challenge.difficulty = Difficulty::parse(&difficulty);
        }
        if let Some(success) = data.success_outcome {
            challenge.outcomes.success.description = success;
        }
        if let Some(failure) = data.failure_outcome {
            challenge.outcomes.failure.description = failure;
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
        let mut challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;
        challenge.active = active;
        self.challenge.save(&challenge).await?;
        Ok(())
    }

    pub async fn set_favorite(
        &self,
        challenge_id: ChallengeId,
        favorite: bool,
    ) -> Result<(), ChallengeError> {
        let mut challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;
        challenge.is_favorite = favorite;
        self.challenge.save(&challenge).await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChallengeError {
    #[error("Challenge not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

fn challenge_to_json(challenge: &domain::Challenge) -> Value {
    serde_json::json!({
        "id": challenge.id.to_string(),
        "world_id": challenge.world_id.to_string(),
        "scene_id": serde_json::Value::Null,
        "name": challenge.name,
        "description": challenge.description,
        "challenge_type": challenge_type_to_str(&challenge.challenge_type),
        "skill_id": "",
        "difficulty": difficulty_to_json(&challenge.difficulty),
        "outcomes": {
            "success": outcome_to_json(&challenge.outcomes.success),
            "failure": outcome_to_json(&challenge.outcomes.failure),
            "partial": challenge
                .outcomes
                .partial
                .as_ref()
                .map(outcome_to_json),
            "critical_success": challenge
                .outcomes
                .critical_success
                .as_ref()
                .map(outcome_to_json),
            "critical_failure": challenge
                .outcomes
                .critical_failure
                .as_ref()
                .map(outcome_to_json),
        },
        "trigger_conditions": challenge
            .trigger_conditions
            .iter()
            .map(trigger_condition_to_json)
            .collect::<Vec<_>>(),
        "prerequisite_challenges": Vec::<String>::new(),
        "active": challenge.active,
        "order": challenge.order,
        "is_favorite": challenge.is_favorite,
        "tags": challenge.tags,
    })
}

fn outcome_to_json(outcome: &domain::Outcome) -> Value {
    serde_json::json!({
        "description": outcome.description,
        "triggers": outcome
            .triggers
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
        "condition_type": trigger_type_to_json(&condition.condition_type),
        "description": condition.description,
        "required": condition.required,
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
            "value": format!("{descriptor:?}"),
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
