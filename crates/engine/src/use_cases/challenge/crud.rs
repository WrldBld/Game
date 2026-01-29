use std::sync::Arc;

use wrldbldr_domain::{
    self as domain, ChallengeId, ChallengeName, Description, Difficulty, DomainError, WorldId,
    LocationId, RegionId, SkillId,
};

use crate::infrastructure::ports::{ChallengeRepo, ContentRepo, RepoError};
use crate::use_cases::validation::ValidationError;

use super::types::{
    ChallengeSummary, DifficultySummary, OutcomeSummary, OutcomeTriggerData, OutcomeTriggerSummary,
    OutcomesSummary, TriggerConditionSummary, TriggerTypeData, TriggerTypeSummary,
};

/// Input for creating a challenge (domain representation).
pub struct CreateChallengeInput {
    pub name: String,
    pub skill_id: Option<SkillId>,
    pub difficulty: String,
    pub description: Option<String>,
    pub success_outcome: Option<String>,
    pub failure_outcome: Option<String>,
    // P1 Challenge availability fields
    pub available_at_location_id: Option<LocationId>,
    pub available_at_region_id: Option<RegionId>,
    pub availability_time_restriction: Option<String>,
    pub always_available: bool,
}

/// Input for updating a challenge (domain representation).
pub struct UpdateChallengeInput {
    pub name: Option<String>,
    pub skill_id: Option<SkillId>,
    pub description: Option<String>,
    pub difficulty: Option<String>,
    pub success_outcome: Option<String>,
    pub failure_outcome: Option<String>,
    // P1 Challenge availability fields
    pub available_at_location_id: Option<LocationId>,
    pub available_at_region_id: Option<RegionId>,
    pub availability_time_restriction: Option<String>,
    pub always_available: Option<bool>,
}

pub struct ChallengeOps {
    challenge: Arc<dyn ChallengeRepo>,
    content: Arc<dyn ContentRepo>,
}

impl ChallengeOps {
    pub fn new(challenge: Arc<dyn ChallengeRepo>, content: Arc<dyn ContentRepo>) -> Self {
        Self { challenge, content }
    }

    /// Validate availability input constraints (P1 review remediation).
    ///
    /// Rules:
    /// 1. Cannot have both location_id and region_id set
    /// 2. If always_available=true, no ids or time restriction allowed
    /// 3. If region selected, clear location id and vice versa
    fn validate_availability_input(
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
        always_available: bool,
        time_restriction: Option<&str>,
    ) -> Result<(), ChallengeError> {
        // Rule 1: Cannot have both location_id and region_id set
        if location_id.is_some() && region_id.is_some() {
            return Err(ChallengeError::ValidationError(
                DomainError::validation(
                    "Cannot specify both location_id and region_id. Specify one or the other.",
                ),
            ));
        }

        // Rule 2: If always_available=true, no ids or time restriction allowed
        if always_available && (location_id.is_some() || region_id.is_some()) {
            return Err(ChallengeError::ValidationError(
                DomainError::validation(
                    "always_available=true cannot have location_id or region_id specified",
                ),
            ));
        }

        if always_available && time_restriction.is_some() {
            return Err(ChallengeError::ValidationError(
                DomainError::validation(
                    "always_available=true cannot have time_restriction specified",
                ),
            ));
        }

        Ok(())
    }

    pub async fn list(&self, world_id: WorldId) -> Result<Vec<ChallengeSummary>, ChallengeError> {
        let challenges = self.challenge.list_for_world(world_id).await?;
        let mut summaries = Vec::new();
        for challenge in &challenges {
            // Fetch skill_id from REQUIRES_SKILL edge
            // TODO: Avoid N+1 query by creating a list_for_world_with_skills method that
            // fetches skill_id in a single query using OPTIONAL MATCH on REQUIRES_SKILL edge
            let skill_id = self
                .challenge
                .get_required_skill(challenge.id())
                .await?;

            // Fetch availability (P1)
            let availabilities = self
                .challenge
                .get_availability(challenge.id())
                .await?;
            let availability = availabilities.first();

            let location_id_summary = availability.and_then(|a| a.location_id);
            let region_id_summary = availability.and_then(|a| a.region_id);
            let time_restriction_summary = availability.and_then(|a| a.time_restriction.clone());
            let always_available_summary = availability.map(|a| a.always_available).unwrap_or(true);

            if let Some(sid) = skill_id {
                summaries.push(challenge_to_summary_with_skill(
                    challenge,
                    Some(sid),
                    location_id_summary,
                    region_id_summary,
                    time_restriction_summary,
                    always_available_summary,
                ));
            } else {
                summaries.push(challenge_to_summary(
                    challenge,
                    location_id_summary,
                    region_id_summary,
                    time_restriction_summary,
                    always_available_summary,
                ));
            }
        }
        Ok(summaries)
    }

    pub async fn get(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Option<ChallengeSummary>, ChallengeError> {
        let challenge = self.challenge.get(challenge_id).await?;
        if let Some(ref ch) = challenge {
            // Fetch skill_id from REQUIRES_SKILL edge
            let skill_id = self
                .challenge
                .get_required_skill(challenge_id)
                .await?;

            // Fetch availability (P1)
            let availabilities = self.challenge.get_availability(challenge_id).await?;
            let availability = availabilities.first();

            let location_id_summary = availability.and_then(|a| a.location_id);
            let region_id_summary = availability.and_then(|a| a.region_id);
            let time_restriction_summary = availability.and_then(|a| a.time_restriction.clone());
            let always_available_summary = availability.map(|a| a.always_available).unwrap_or(true);

            Ok(Some(if let Some(sid) = skill_id {
                challenge_to_summary_with_skill(
                    ch,
                    Some(sid),
                    location_id_summary,
                    region_id_summary,
                    time_restriction_summary,
                    always_available_summary,
                )
            } else {
                challenge_to_summary(
                    ch,
                    location_id_summary,
                    region_id_summary,
                    time_restriction_summary,
                    always_available_summary,
                )
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        input: CreateChallengeInput,
    ) -> Result<ChallengeSummary, ChallengeError> {
        // Validate input constraints (P1)
        Self::validate_availability_input(
            input.available_at_location_id,
            input.available_at_region_id,
            input.always_available,
            input.availability_time_restriction.as_deref(),
        )?;

        // Convert input name to ChallengeName with validation
        let challenge_name = ChallengeName::new(&input.name)?;

        // Get skill_id from input
        let skill_id = input.skill_id.ok_or_else(|| {
            ChallengeError::ValidationError(DomainError::validation("skill_id is required"))
        })?;

        // Fetch skill to get base_attribute
        let skill = self
            .content
            .get_skill(skill_id)
            .await?
            .ok_or(ChallengeError::ValidationError(DomainError::validation(
                "Skill not found",
            )))?;

        // Set check_stat from skill's base_attribute
        let check_stat = skill.base_attribute;

        // Note: Difficulty::parse never fails - invalid formats become Difficulty::Custom(string)
        // This is intentional to support freeform difficulty descriptions
        let challenge = domain::Challenge::new(
            world_id,
            challenge_name,
            Difficulty::parse(&input.difficulty),
        )
        .with_description(
            Description::new(input.description.unwrap_or_default()).unwrap_or_default(),
        )
        .with_outcomes(domain::ChallengeOutcomes {
            success: domain::Outcome::new(input.success_outcome.unwrap_or_default()),
            failure: domain::Outcome::new(input.failure_outcome.unwrap_or_default()),
            partial: None,
            critical_success: None,
            critical_failure: None,
        })
        .with_order(0);

        // Apply check_stat if skill has a base_attribute
        let challenge = if let Some(stat) = check_stat {
            challenge.with_check_stat(stat)
        } else {
            challenge
        };

        // Validate triggers before saving
        let trigger_errors = challenge.validate_triggers();
        if !trigger_errors.is_empty() {
            return Err(ChallengeError::ValidationError(
                DomainError::validation(format!(
                    "Invalid triggers: {}",
                    trigger_errors.join("; ")
                ))
            ));
        }

        self.challenge.save(&challenge).await?;

        // Persist REQUIRES_SKILL edge
        self.challenge.set_required_skill(challenge.id(), skill_id).await?;

        // Persist availability edge (P1)
        if let Some(location_id) = input.available_at_location_id {
            self.challenge
                .set_availability_at_location(
                    challenge.id(),
                    location_id,
                    input.always_available,
                    input.availability_time_restriction.clone(),
                )
                .await?;
        } else if let Some(region_id) = input.available_at_region_id {
            self.challenge
                .set_availability_at_region(
                    challenge.id(),
                    region_id,
                    input.always_available,
                    input.availability_time_restriction.clone(),
                )
                .await?;
        }

        Ok(challenge_to_summary_with_skill(
            &challenge,
            Some(skill_id),
            input.available_at_location_id,
            input.available_at_region_id,
            input.availability_time_restriction,
            input.always_available,
        ))
    }

    pub async fn update(
        &self,
        challenge_id: ChallengeId,
        input: UpdateChallengeInput,
    ) -> Result<ChallengeSummary, ChallengeError> {
        // Validate availability input constraints (P1 review remediation)
        Self::validate_availability_input(
            input.available_at_location_id,
            input.available_at_region_id,
            input.always_available.unwrap_or(true),
            input.availability_time_restriction.as_deref(),
        )?;

        let mut challenge = self
            .challenge
            .get(challenge_id)
            .await?
            .ok_or(ChallengeError::NotFound)?;
        // Since Challenge has private fields, we need to rebuild with updated values
        // Use accessors to get current values, then rebuild with builder pattern
        let name = if let Some(new_name) = input.name {
            ChallengeName::new(&new_name)?
        } else {
            challenge.name().clone()
        };

        let description = if let Some(desc_str) = input.description {
            Description::new(desc_str).unwrap_or_default()
        } else {
            challenge.description().clone()
        };

        let difficulty = if let Some(diff_str) = input.difficulty {
            // Note: Difficulty::parse never fails - invalid formats become Difficulty::Custom(string)
            // This is intentional to support freeform difficulty descriptions
            Difficulty::parse(&diff_str)
        } else {
            challenge.difficulty().clone()
        };

        // Handle skill_id update
        let check_stat = if let Some(skill_id) = input.skill_id {
            // Fetch skill to get base_attribute
            let skill = self
                .content
                .get_skill(skill_id)
                .await?
                .ok_or(ChallengeError::ValidationError(DomainError::validation(
                    "Skill not found",
                )))?;

            // Update REQUIRES_SKILL edge if skill_id is provided
            self.challenge
                .set_required_skill(challenge_id, skill_id)
                .await?;

            skill.base_attribute
        } else {
            // If no skill_id provided, keep existing check_stat
            challenge.check_stat()
        };

        // For outcomes, we need to preserve existing and update what's provided
        let success_desc = input
            .success_outcome
            .unwrap_or_else(|| challenge.outcomes().success.description.clone());
        let failure_desc = input
            .failure_outcome
            .unwrap_or_else(|| challenge.outcomes().failure.description.clone());
        let outcomes = domain::ChallengeOutcomes {
            success: domain::Outcome::new(success_desc),
            failure: domain::Outcome::new(failure_desc),
            partial: challenge.outcomes().partial.clone(),
            critical_success: challenge.outcomes().critical_success.clone(),
            critical_failure: challenge.outcomes().critical_failure.clone(),
        };

        // Rebuild the challenge with updated values
        challenge = domain::Challenge::new(challenge.world_id(), name, difficulty)
            .with_id(challenge.id())
            .with_description(description)
            .with_outcomes(outcomes)
            .with_active(challenge.active())
            .with_order(challenge.order())
            .with_is_favorite(challenge.is_favorite());

        // Apply check_stat from skill or keep existing
        if let Some(stat) = check_stat {
            challenge = challenge.with_check_stat(stat);
        }

        // Validate triggers before saving
        let trigger_errors = challenge.validate_triggers();
        if !trigger_errors.is_empty() {
            return Err(ChallengeError::ValidationError(
                DomainError::validation(format!(
                    "Invalid triggers: {}",
                    trigger_errors.join("; ")
                ))
            ));
        }

        self.challenge.save(&challenge).await?;

        // Get skill_id for summary
        let skill_id_for_summary = match input.skill_id {
            Some(sid) => Some(sid),
            None => {
                // Try to get from existing edge
                self.challenge.get_required_skill(challenge_id).await.ok().flatten()
            }
        };

        // Availability edge cleanup (P1 review remediation):
        // If switching types or setting always_available, remove existing edges
        if input.available_at_location_id.is_some() || input.available_at_region_id.is_some() {
            // Remove old availability edges before setting new ones
            // This prevents duplicate edges when switching from location to region
            self.challenge.remove_availability(challenge_id).await?;
        }

        // Persist availability edge (P1)
        if let Some(location_id) = input.available_at_location_id {
            self.challenge
                .set_availability_at_location(
                    challenge_id,
                    location_id,
                    input.always_available.unwrap_or(true),
                    input.availability_time_restriction.clone(),
                )
                .await?;
        } else if let Some(region_id) = input.available_at_region_id {
            self.challenge
                .set_availability_at_region(
                    challenge_id,
                    region_id,
                    input.always_available.unwrap_or(true),
                    input.availability_time_restriction.clone(),
                )
                .await?;
        }

        // Get final availability values for summary
        let location_id_summary = input.available_at_location_id;
        let region_id_summary = input.available_at_region_id;
        let time_restriction_summary = input.availability_time_restriction;
        let always_available_summary = input.always_available.unwrap_or(true);

        Ok(if let Some(sid) = skill_id_for_summary {
            challenge_to_summary_with_skill(
                &challenge,
                Some(sid),
                location_id_summary,
                region_id_summary,
                time_restriction_summary,
                always_available_summary,
            )
        } else {
            challenge_to_summary(
                &challenge,
                location_id_summary,
                region_id_summary,
                time_restriction_summary,
                always_available_summary,
            )
        })
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
        .with_description(challenge.description().clone())
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
        .with_description(challenge.description().clone())
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
    ValidationError(#[from] DomainError),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

impl From<ValidationError> for ChallengeError {
    fn from(err: ValidationError) -> Self {
        ChallengeError::ValidationError(DomainError::validation(err.to_string()))
    }
}

fn challenge_to_summary_with_skill(
    challenge: &domain::Challenge,
    skill_id: Option<SkillId>,
    available_at_location_id: Option<LocationId>,
    available_at_region_id: Option<RegionId>,
    availability_time_restriction: Option<String>,
    always_available: bool,
) -> ChallengeSummary {
    ChallengeSummary {
        id: challenge.id(),
        world_id: challenge.world_id(),
        scene_id: None,
        name: challenge.name().to_string(),
        description: challenge.description().to_string(),
        challenge_type: challenge_type_to_str(&challenge.challenge_type()).to_string(),
        skill_id,
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
        available_at_location_id,
        available_at_region_id,
        availability_time_restriction,
        always_available,
    }
}

fn challenge_to_summary(
    challenge: &domain::Challenge,
    available_at_location_id: Option<LocationId>,
    available_at_region_id: Option<RegionId>,
    availability_time_restriction: Option<String>,
    always_available: bool,
) -> ChallengeSummary {
    ChallengeSummary {
        id: challenge.id(),
        world_id: challenge.world_id(),
        scene_id: None,
        name: challenge.name().to_string(),
        description: challenge.description().to_string(),
        challenge_type: challenge_type_to_str(&challenge.challenge_type()).to_string(),
        skill_id: None,
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
        available_at_location_id,
        available_at_region_id,
        availability_time_restriction,
        always_available,
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
                stat: stat.to_string(),
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
