//! Prompt Template Service
//!
//! HTTP client for fetching and updating LLM prompt templates.
//! Templates can be overridden at the world level, falling back to global defaults.

use crate::infrastructure::http_client::HttpClient;
use serde::{Deserialize, Serialize};

/// Prompt template metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplateInfo {
    /// Unique key for this template
    pub key: String,
    /// Human-readable label
    pub label: String,
    /// Description of what this template is used for
    pub description: String,
    /// Category for UI grouping
    pub category: String,
    /// Default value
    pub default_value: String,
}

/// Resolved prompt template value with override status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPromptTemplate {
    /// Template key
    pub key: String,
    /// Current resolved value (world override or global default)
    pub value: String,
    /// Whether this is a world-specific override
    pub is_override: bool,
    /// Default value (for reset functionality)
    pub default_value: String,
}

/// Request to save a prompt template override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePromptTemplateRequest {
    /// New template value
    pub value: String,
}

/// Prompt template service
#[derive(Clone)]
pub struct PromptTemplateService {
    _http: (),
}

impl PromptTemplateService {
    pub fn new() -> Self {
        Self { _http: () }
    }

    /// Get all available prompt template metadata
    pub async fn list_templates(&self) -> Result<Vec<PromptTemplateInfo>, String> {
        HttpClient::get("/api/prompt-templates")
            .await
            .map_err(|e| e.to_string())
    }

    /// Get resolved template value for a specific world
    pub async fn get_template(
        &self,
        world_id: &str,
        key: &str,
    ) -> Result<ResolvedPromptTemplate, String> {
        let url = format!(
            "/api/prompt-templates/resolve/{}?world_id={}",
            key, world_id
        );
        HttpClient::get(&url).await.map_err(|e| e.to_string())
    }

    /// Save a world-specific override for a template
    pub async fn save_template(
        &self,
        world_id: &str,
        key: &str,
        request: SavePromptTemplateRequest,
    ) -> Result<ResolvedPromptTemplate, String> {
        let url = format!("/api/prompt-templates/world/{}/{}", world_id, key);
        HttpClient::put(&url, &request)
            .await
            .map_err(|e| e.to_string())
    }

    /// Reset a world-specific override (delete to fall back to default)
    /// After deletion, we need to fetch the default value again
    pub async fn reset_template(
        &self,
        world_id: &str,
        key: &str,
    ) -> Result<ResolvedPromptTemplate, String> {
        let url = format!("/api/prompt-templates/world/{}/{}", world_id, key);
        HttpClient::delete(&url).await.map_err(|e| e.to_string())?;
        // After deleting, fetch the default value
        self.get_template(world_id, key).await
    }
}

// Hook for using the prompt template service
pub fn use_prompt_template_service() -> PromptTemplateService {
    PromptTemplateService::new()
}
