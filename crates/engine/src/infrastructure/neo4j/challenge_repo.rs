//! Neo4j challenge repository implementation.
//!
//! # Graph-First Design
//!
//! Challenges use Neo4j edges for relationships:
//! - `(Challenge)-[:TIED_TO_SCENE]->(Scene)` - Scene this challenge appears in
//! - `(World)-[:CONTAINS_CHALLENGE]->(Challenge)` - World ownership
//!
//! Complex fields (outcomes, triggers, difficulty) are stored as JSON.

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Row};
use wrldbldr_domain::*;

use super::helpers::{parse_description_or_default, parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ChallengeRepo, RepoError};

// =============================================================================
// Stored Types for JSON serialization
// =============================================================================

/// Stored representation of Difficulty
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
enum DifficultyStored {
    Dc { value: u32 },
    Percentage { value: u32 },
    Descriptor { descriptor: String },
    Opposed,
    Custom { description: String },
}

impl From<Difficulty> for DifficultyStored {
    fn from(value: Difficulty) -> Self {
        match value {
            Difficulty::DC(v) => Self::Dc { value: v },
            Difficulty::Percentage(v) => Self::Percentage { value: v },
            Difficulty::Descriptor(d) => Self::Descriptor {
                descriptor: format!("{:?}", d),
            },
            Difficulty::Opposed => Self::Opposed,
            Difficulty::Custom(s) => Self::Custom { description: s },
        }
    }
}

impl From<DifficultyStored> for Difficulty {
    fn from(value: DifficultyStored) -> Self {
        match value {
            DifficultyStored::Dc { value } => Self::DC(value),
            DifficultyStored::Percentage { value } => Self::Percentage(value),
            DifficultyStored::Descriptor { descriptor } => {
                // Parse the descriptor string back to enum
                let d = match descriptor.as_str() {
                    "Trivial" => DifficultyDescriptor::Trivial,
                    "Easy" => DifficultyDescriptor::Easy,
                    "Moderate" => DifficultyDescriptor::Moderate,
                    "Hard" => DifficultyDescriptor::Hard,
                    "VeryHard" => DifficultyDescriptor::VeryHard,
                    "Extreme" => DifficultyDescriptor::Extreme,
                    "Impossible" => DifficultyDescriptor::Impossible,
                    "Risky" => DifficultyDescriptor::Risky,
                    "Desperate" => DifficultyDescriptor::Desperate,
                    _ => DifficultyDescriptor::Moderate,
                };
                Self::Descriptor(d)
            }
            DifficultyStored::Opposed => Self::Opposed,
            DifficultyStored::Custom { description } => Self::Custom(description),
        }
    }
}

/// Stored representation of ChallengeOutcomes
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct OutcomesStored {
    success: OutcomeStored,
    failure: OutcomeStored,
    partial: Option<OutcomeStored>,
    critical_success: Option<OutcomeStored>,
    critical_failure: Option<OutcomeStored>,
}

impl From<&ChallengeOutcomes> for OutcomesStored {
    fn from(value: &ChallengeOutcomes) -> Self {
        Self {
            success: (&value.success).into(),
            failure: (&value.failure).into(),
            partial: value.partial.as_ref().map(Into::into),
            critical_success: value.critical_success.as_ref().map(Into::into),
            critical_failure: value.critical_failure.as_ref().map(Into::into),
        }
    }
}

impl TryFrom<OutcomesStored> for ChallengeOutcomes {
    type Error = RepoError;

    fn try_from(value: OutcomesStored) -> Result<Self, Self::Error> {
        Ok(ChallengeOutcomes {
            success: value.success.try_into()?,
            failure: value.failure.try_into()?,
            partial: value.partial.map(TryInto::try_into).transpose()?,
            critical_success: value.critical_success.map(TryInto::try_into).transpose()?,
            critical_failure: value.critical_failure.map(TryInto::try_into).transpose()?,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct OutcomeStored {
    description: String,
    // Triggers stored as JSON - simplified for now
    triggers_json: Option<String>,
}

impl From<&Outcome> for OutcomeStored {
    fn from(value: &Outcome) -> Self {
        Self {
            description: value.description.clone(),
            triggers_json: serde_json::to_string(&value.triggers).ok(),
        }
    }
}

impl TryFrom<OutcomeStored> for Outcome {
    type Error = RepoError;

    fn try_from(value: OutcomeStored) -> Result<Self, Self::Error> {
        let triggers: Vec<OutcomeTrigger> = match value.triggers_json {
            Some(s) => serde_json::from_str(&s).map_err(|e| {
                RepoError::database(
                    "parse",
                    format!("Invalid triggers_json in Outcome: {} (value: '{}')", e, s),
                )
            })?,
            None => Vec::new(),
        };
        let mut outcome = Outcome::new(value.description);
        outcome.triggers = triggers;
        Ok(outcome)
    }
}

/// Stored representation of TriggerCondition - uses direct JSON object, not stringified
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TriggerConditionStored {
    condition_type: TriggerType,
    description: String,
    required: bool,
}

impl From<&TriggerCondition> for TriggerConditionStored {
    fn from(value: &TriggerCondition) -> Self {
        Self {
            condition_type: value.condition_type.clone(),
            description: value.description.clone(),
            required: value.required,
        }
    }
}

impl From<TriggerConditionStored> for TriggerCondition {
    fn from(value: TriggerConditionStored) -> Self {
        let mut tc = TriggerCondition::new(value.condition_type, value.description);
        tc.required = value.required;
        tc
    }
}

// =============================================================================
// Repository Implementation
// =============================================================================

/// Repository for Challenge operations.
pub struct Neo4jChallengeRepo {
    graph: Neo4jGraph,
}

impl Neo4jChallengeRepo {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }

    /// Convert a Neo4j row to a Challenge entity.
    fn row_to_challenge(&self, row: Row) -> Result<Challenge, RepoError> {
        let node: neo4rs::Node = row.get("c").map_err(|e| RepoError::database("query", e))?;

        let id: ChallengeId = parse_typed_id(&node, "id").map_err(|e| {
            RepoError::database("query", format!("Failed to parse ChallengeId: {}", e))
        })?;
        let world_id: WorldId = parse_typed_id(&node, "world_id").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to parse world_id for Challenge {}: {}", id, e),
            )
        })?;
        let name_str: String = node.get("name").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get 'name' for Challenge {}: {}", id, e),
            )
        })?;
        let name = ChallengeName::new(name_str).map_err(|e| RepoError::database("parse", e))?;
        let description = parse_description_or_default(node.get_optional_string("description"));

        let challenge_type_str: String = node.get_string_strict("challenge_type").map_err(|e| {
            RepoError::database(
                "query",
                format!("Missing challenge_type for Challenge {}: {}", id, e),
            )
        })?;
        let challenge_type = match challenge_type_str.as_str() {
            "SkillCheck" => ChallengeType::SkillCheck,
            "AbilityCheck" => ChallengeType::AbilityCheck,
            "SavingThrow" => ChallengeType::SavingThrow,
            "OpposedCheck" => ChallengeType::OpposedCheck,
            "ComplexChallenge" => ChallengeType::ComplexChallenge,
            _ => {
                return Err(RepoError::database(
                    "parse",
                    format!(
                        "Invalid ChallengeType for Challenge {}: '{}'",
                        id, challenge_type_str
                    ),
                ));
            }
        };

        let difficulty: Difficulty = node
            .get_json_strict::<DifficultyStored>("difficulty_json")
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!("Invalid difficulty_json for Challenge {}: {}", id, e),
                )
            })?
            .into();

        let outcomes: ChallengeOutcomes = node
            .get_json_strict::<OutcomesStored>("outcomes_json")
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!("Invalid outcomes_json for Challenge {}: {}", id, e),
                )
            })?
            .try_into()?;

        let trigger_conditions: Vec<TriggerCondition> = node
            .get_json_strict::<Vec<TriggerConditionStored>>("triggers_json")
            .map_err(|e| {
                RepoError::database(
                    "parse",
                    format!("Invalid triggers_json for Challenge {}: {}", id, e),
                )
            })?
            .into_iter()
            .map(Into::into)
            .collect();

        let active = node.get_bool_or("active", true);
        let order = node.get_i64_or("challenge_order", 0) as u32;
        let is_favorite = node.get_bool_or("is_favorite", false);
        let tags: Vec<String> = node.get_json_or_default("tags_json");
        let check_stat: Option<wrldbldr_domain::Stat> = node
            .get_optional_string("check_stat")
            .and_then(|s| s.parse().ok());

        let mut challenge = Challenge::new(world_id, name, difficulty)
            .with_id(id)
            .with_description(description)
            .with_challenge_type(challenge_type)
            .with_outcomes(outcomes)
            .with_active(active)
            .with_order(order)
            .with_is_favorite(is_favorite);

        for condition in trigger_conditions {
            challenge = challenge.with_trigger(condition);
        }
        for tag_str in tags {
            let tag =
                wrldbldr_domain::Tag::new(&tag_str).map_err(|e| RepoError::database("parse", e))?;
            challenge = challenge.with_tag(tag);
        }
        if let Some(stat) = check_stat {
            challenge = challenge.with_check_stat(stat);
        }

        Ok(challenge)
    }
}

#[async_trait]
impl ChallengeRepo for Neo4jChallengeRepo {
    async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>, RepoError> {
        let q = query("MATCH (c:Challenge {id: $id}) RETURN c").param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            Ok(Some(self.row_to_challenge(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, challenge: &Challenge) -> Result<(), RepoError> {
        let difficulty_json =
            serde_json::to_string(&DifficultyStored::from(challenge.difficulty().clone()))
                .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let outcomes_json = serde_json::to_string(&OutcomesStored::from(challenge.outcomes()))
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let triggers_json = serde_json::to_string(
            &challenge
                .trigger_conditions()
                .iter()
                .map(TriggerConditionStored::from)
                .collect::<Vec<_>>(),
        )
        .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let tags_json = serde_json::to_string(challenge.tags())
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        // MERGE for upsert behavior
        let q = query(
            "MATCH (w:World {id: $world_id})
            MERGE (c:Challenge {id: $id})
            SET c.world_id = $world_id,
                c.name = $name,
                c.description = $description,
                c.challenge_type = $challenge_type,
                c.difficulty_json = $difficulty_json,
                c.outcomes_json = $outcomes_json,
                c.triggers_json = $triggers_json,
                c.active = $active,
                c.challenge_order = $challenge_order,
                c.is_favorite = $is_favorite,
                c.tags_json = $tags_json,
                c.check_stat = $check_stat
            MERGE (w)-[:CONTAINS_CHALLENGE]->(c)
            RETURN c.id as id",
        )
        .param("id", challenge.id().to_string())
        .param("world_id", challenge.world_id().to_string())
        .param("name", challenge.name().to_string())
        .param("description", challenge.description().to_string())
        .param(
            "challenge_type",
            format!("{:?}", challenge.challenge_type()),
        )
        .param("difficulty_json", difficulty_json)
        .param("outcomes_json", outcomes_json)
        .param("triggers_json", triggers_json)
        .param("active", challenge.active())
        .param("challenge_order", challenge.order() as i64)
        .param("is_favorite", challenge.is_favorite())
        .param("tags_json", tags_json)
        .param(
            "check_stat",
            challenge
                .check_stat()
                .map_or(String::new(), |s| s.to_string()),
        );

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Saved challenge: {}", challenge.name());
        Ok(())
    }

    async fn delete(&self, id: ChallengeId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (c:Challenge {id: $id})
            DETACH DELETE c",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Deleted challenge: {}", id);
        Ok(())
    }

    async fn list_for_world(&self, world_id: WorldId) -> Result<Vec<Challenge>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_CHALLENGE]->(c:Challenge)
            RETURN c
            ORDER BY c.is_favorite DESC, c.challenge_order",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut challenges = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            challenges.push(self.row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    async fn list_for_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>, RepoError> {
        let q = query(
            "MATCH (c:Challenge)-[:TIED_TO_SCENE]->(s:Scene {id: $scene_id})
            RETURN c
            ORDER BY c.is_favorite DESC, c.challenge_order",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut challenges = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            challenges.push(self.row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    async fn list_pending_for_world(&self, world_id: WorldId) -> Result<Vec<Challenge>, RepoError> {
        // Pending challenges = active challenges in the world
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_CHALLENGE]->(c:Challenge {active: true})
            RETURN c
            ORDER BY c.is_favorite DESC, c.challenge_order",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut challenges = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            challenges.push(self.row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    async fn mark_resolved(&self, id: ChallengeId) -> Result<(), RepoError> {
        // Mark as inactive (resolved)
        let q = query(
            "MATCH (c:Challenge {id: $id})
            SET c.active = false
            RETURN c.id as id",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Marked challenge {} as resolved", id);
        Ok(())
    }

    async fn set_enabled(&self, id: ChallengeId, enabled: bool) -> Result<(), RepoError> {
        let q = query(
            "MATCH (c:Challenge {id: $id})
            SET c.active = $enabled
            RETURN c.id as id",
        )
        .param("id", id.to_string())
        .param("enabled", enabled);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Set challenge {} enabled={}", id, enabled);
        Ok(())
    }

    async fn get_resolved_challenges(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<ChallengeId>, RepoError> {
        // Get all challenges that have been resolved (active = false)
        let q = query(
            "MATCH (c:Challenge {world_id: $world_id, active: false})
            RETURN c.id AS id",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        let mut challenge_ids = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let id_str: String = row.get("id").map_err(|e| RepoError::database("query", e))?;
            if let Ok(id) = id_str.parse::<uuid::Uuid>() {
                challenge_ids.push(ChallengeId::from(id));
            }
        }

        Ok(challenge_ids)
    }
}
