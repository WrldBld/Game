//! Domain types for challenge use cases.
//!
//! These types represent challenge data in a structured way, avoiding raw JSON.
//! JSON serialization happens at the API boundary layer.
//!
//! Note: These DTOs are designed for serialization (use case -> API -> JSON wire format).
//! The `#[serde(flatten)]` + `#[serde(untagged)]` pattern produces the same JSON as the
//! original `serde_json::json!` approach while providing type safety.

use serde::{Deserialize, Serialize};
use wrldbldr_domain::{ChallengeId, SceneId, WorldId};

/// Summary of a challenge for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSummary {
    pub id: ChallengeId,
    pub world_id: WorldId,
    pub scene_id: Option<SceneId>,
    pub name: String,
    pub description: String,
    pub challenge_type: String,
    pub skill_id: String,
    pub difficulty: DifficultySummary,
    pub outcomes: OutcomesSummary,
    pub trigger_conditions: Vec<TriggerConditionSummary>,
    pub prerequisite_challenges: Vec<String>,
    pub active: bool,
    pub order: u32,
    pub is_favorite: bool,
    pub tags: Vec<String>,
}

/// Summary of challenge difficulty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultySummary {
    #[serde(rename = "type")]
    pub difficulty_type: String,
    pub value: Option<String>,
}

/// Summary of challenge outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomesSummary {
    pub success: OutcomeSummary,
    pub failure: OutcomeSummary,
    pub partial: Option<OutcomeSummary>,
    pub critical_success: Option<OutcomeSummary>,
    pub critical_failure: Option<OutcomeSummary>,
}

/// Summary of a single outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeSummary {
    pub description: String,
    pub triggers: Vec<OutcomeTriggerSummary>,
}

/// Summary of an outcome trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeTriggerSummary {
    #[serde(rename = "type")]
    pub trigger_type: String,
    #[serde(flatten)]
    pub data: OutcomeTriggerData,
}

/// Data for an outcome trigger (varies by type).
///
/// Uses `#[serde(untagged)]` because the discriminator is in the parent struct's
/// `trigger_type` field. The variant fields are flattened into the parent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutcomeTriggerData {
    RevealInformation {
        info: String,
        persist: bool,
    },
    EnableChallenge {
        challenge_id: String,
    },
    DisableChallenge {
        challenge_id: String,
    },
    ModifyCharacterStat {
        stat: String,
        modifier: i32,
    },
    TriggerScene {
        scene_id: String,
    },
    GiveItem {
        item_name: String,
        item_description: Option<String>,
    },
    Custom {
        description: String,
    },
}

/// Summary of a trigger condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConditionSummary {
    pub condition_type: TriggerTypeSummary,
    pub description: String,
    pub required: bool,
}

/// Summary of a trigger type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerTypeSummary {
    #[serde(rename = "type")]
    pub trigger_type: String,
    #[serde(flatten)]
    pub data: TriggerTypeData,
}

/// Data for a trigger type (varies by type).
///
/// Uses `#[serde(untagged)]` because the discriminator is in the parent struct's
/// `trigger_type` field. The variant fields are flattened into the parent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TriggerTypeData {
    ObjectInteraction {
        keywords: Vec<String>,
    },
    EnterArea {
        area_keywords: Vec<String>,
    },
    DialogueTopic {
        topic_keywords: Vec<String>,
    },
    ChallengeComplete {
        challenge_id: String,
        requires_success: Option<bool>,
    },
    TimeBased {
        turns: u32,
    },
    NpcPresent {
        npc_keywords: Vec<String>,
    },
    Custom {
        description: String,
    },
}
