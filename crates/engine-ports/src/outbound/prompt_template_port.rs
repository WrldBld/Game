//! Prompt Template Repository Port
//!
//! Port for storing and retrieving configurable LLM prompt templates.
//! Templates follow a priority resolution order:
//! 1. World-specific DB override
//! 2. Global DB override  
//! 3. Environment variable (WRLDBLDR_PROMPT_{KEY})
//! 4. Hard-coded default

use async_trait::async_trait;
use wrldbldr_domain::WorldId;

/// Error type for prompt template operations
#[derive(Debug, thiserror::Error)]
pub enum PromptTemplateError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Template not found: {0}")]
    NotFound(String),
}

/// A resolved prompt template with source information
#[derive(Debug, Clone)]
pub struct ResolvedPromptTemplate {
    /// The template key
    pub key: String,
    /// The effective template value
    pub value: String,
    /// Where this value came from
    pub source: PromptTemplateSource,
    /// The hard-coded default (for reference/reset)
    pub default_value: String,
}

/// Source of a prompt template value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptTemplateSource {
    /// From world-specific DB override
    WorldOverride,
    /// From global DB override
    GlobalOverride,
    /// From environment variable
    Environment,
    /// Using hard-coded default
    Default,
}

impl PromptTemplateSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WorldOverride => "world_override",
            Self::GlobalOverride => "global_override",
            Self::Environment => "environment",
            Self::Default => "default",
        }
    }
}

/// Repository port for prompt template storage
#[async_trait]
pub trait PromptTemplateRepositoryPort: Send + Sync {
    /// Get a global override for a template key (DB only, no fallback)
    async fn get_global(&self, key: &str) -> Result<Option<String>, PromptTemplateError>;
    
    /// Get all global overrides
    async fn get_all_global(&self) -> Result<Vec<(String, String)>, PromptTemplateError>;
    
    /// Set a global override for a template key
    async fn set_global(&self, key: &str, value: &str) -> Result<(), PromptTemplateError>;
    
    /// Delete a global override (falls back to env/default)
    async fn delete_global(&self, key: &str) -> Result<(), PromptTemplateError>;
    
    /// Delete all global overrides
    async fn delete_all_global(&self) -> Result<(), PromptTemplateError>;
    
    /// Get a world-specific override for a template key (DB only, no fallback)
    async fn get_for_world(&self, world_id: WorldId, key: &str) -> Result<Option<String>, PromptTemplateError>;
    
    /// Get all world-specific overrides
    async fn get_all_for_world(&self, world_id: WorldId) -> Result<Vec<(String, String)>, PromptTemplateError>;
    
    /// Set a world-specific override for a template key
    async fn set_for_world(&self, world_id: WorldId, key: &str, value: &str) -> Result<(), PromptTemplateError>;
    
    /// Delete a world-specific override (falls back to global/env/default)
    async fn delete_for_world(&self, world_id: WorldId, key: &str) -> Result<(), PromptTemplateError>;
    
    /// Delete all world-specific overrides
    async fn delete_all_for_world(&self, world_id: WorldId) -> Result<(), PromptTemplateError>;
}
