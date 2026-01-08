use serde::{Deserialize, Serialize};

use crate::messages::{ActantialRoleData, ActorTypeData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActantialRequest {
    GetActantialContext {
        character_id: String,
    },
    AddActantialView {
        character_id: String,
        want_id: String,
        target_id: String,
        target_type: ActorTypeData,
        role: ActantialRoleData,
        reason: String,
    },
    RemoveActantialView {
        character_id: String,
        want_id: String,
        target_id: String,
        target_type: ActorTypeData,
        role: ActantialRoleData,
    },
}
