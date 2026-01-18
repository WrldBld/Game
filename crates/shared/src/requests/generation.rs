use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GenerationRequest {
    GetGenerationQueue {
        world_id: String,
        #[serde(default)]
        user_id: Option<String>,
    },
    SyncGenerationReadState {
        world_id: String,
        read_batches: Vec<String>,
        read_suggestions: Vec<String>,
    },
    /// Dismiss a suggestion, removing it from the queue permanently
    DismissSuggestion { request_id: String },
}
