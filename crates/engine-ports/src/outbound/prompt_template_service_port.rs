//! Prompt template service port - Interface for prompt template operations
//!
//! This port abstracts prompt template business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;

/// A resolved prompt template with its value and metadata
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// The template key/name
    pub key: String,
    /// The resolved template value (with priority resolution applied)
    pub value: String,
    /// The source from which this template was resolved
    pub source: PromptTemplateSource,
}

/// Source from which a prompt template was resolved
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptTemplateSource {
    /// World-specific database override
    WorldOverride,
    /// Global database override
    GlobalOverride,
    /// Environment variable
    Environment,
    /// Hard-coded default
    Default,
}

/// Port for prompt template service operations
///
/// This trait defines the operations for retrieving and rendering
/// prompt templates used for LLM interactions.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait PromptTemplateServicePort: Send + Sync {
    /// Get a template by name
    ///
    /// Returns the resolved template if found, or None if the
    /// template name is not recognized.
    async fn get_template(&self, name: &str) -> Result<Option<PromptTemplate>>;

    /// Render a template with the given context
    ///
    /// Resolves the template by name and applies the context values
    /// to produce the final rendered string.
    ///
    /// # Arguments
    ///
    /// * `name` - The template name/key
    /// * `context` - JSON object containing values to substitute into the template
    ///
    /// # Errors
    ///
    /// Returns an error if the template is not found or rendering fails.
    async fn render_template(&self, name: &str, context: serde_json::Value) -> Result<String>;
}
