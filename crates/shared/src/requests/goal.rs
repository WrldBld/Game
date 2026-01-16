use serde::{Deserialize, Serialize};

use crate::messages::{CreateGoalData, UpdateGoalData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GoalRequest {
    ListGoals {
        world_id: String,
    },
    GetGoal {
        goal_id: String,
    },
    CreateGoal {
        world_id: String,
        data: CreateGoalData,
    },
    UpdateGoal {
        goal_id: String,
        data: UpdateGoalData,
    },
    DeleteGoal {
        goal_id: String,
    },
}
