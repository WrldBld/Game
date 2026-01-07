use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GenerationRequest {
    GetGenerationQueue { world_id: String, #[serde(default)] user_id: Option<String> },
    SyncGenerationReadState { world_id: String, read_batches: Vec<String>, read_suggestions: Vec<String> },
}
