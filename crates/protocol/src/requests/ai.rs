use serde::{Deserialize, Serialize};

use super::SuggestionContextData;
use crate::messages::ActantialRoleData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiRequest {
    SuggestDeflectionBehavior {
        npc_id: String,
        want_id: String,
        want_description: String,
    },
    SuggestBehavioralTells {
        npc_id: String,
        want_id: String,
        want_description: String,
    },
    SuggestWantDescription {
        npc_id: String,
        #[serde(default)]
        context: Option<String>,
    },
    SuggestActantialReason {
        npc_id: String,
        want_id: String,
        target_id: String,
        role: ActantialRoleData,
    },

    EnqueueContentSuggestion {
        world_id: String,
        suggestion_type: String,
        context: SuggestionContextData,
    },
    CancelContentSuggestion {
        request_id: String,
    },
}
