use serde::{Deserialize, Serialize};

use super::{CreateSkillData, UpdateSkillData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SkillRequest {
    ListSkills {
        world_id: String,
    },
    GetSkill {
        skill_id: String,
    },
    CreateSkill {
        world_id: String,
        data: CreateSkillData,
    },
    UpdateSkill {
        skill_id: String,
        data: UpdateSkillData,
    },
    DeleteSkill {
        skill_id: String,
    },
}
