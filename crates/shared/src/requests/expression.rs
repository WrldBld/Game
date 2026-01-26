use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExpressionRequest {
    GenerateExpressionSheet {
        character_id: String,
        workflow: String,
        #[serde(default)]
        expressions: Option<Vec<String>>,
        #[serde(default)]
        grid_layout: Option<String>,
        #[serde(default)]
        style_prompt: Option<String>,
    },
}
