// Prompt template use case - port trait injection per ADR-009
#![allow(dead_code)]

//! Prompt template management use cases.
//!
//! Handles prompt template configuration at both global and per-world levels.
//! Provides resolution logic with priority: World DB > Global DB > Environment Variable > Default.

use std::sync::Arc;

use wrldbldr_domain::WorldId;

use crate::infrastructure::ports::PromptTemplateRepo;
use crate::prompt_templates::prompt_template_metadata;

/// Prompt template operations use case.
///
/// Encapsulates all prompt template-related queries and mutations, providing:
/// - Global template override management
/// - Per-world template overrides with fallback to global
/// - Template resolution with priority chain
/// - Template metadata for UI/configuration
pub struct PromptTemplateOps {
    repo: Arc<dyn PromptTemplateRepo>,
}

impl PromptTemplateOps {
    pub fn new(repo: Arc<dyn PromptTemplateRepo>) -> Self {
        Self { repo }
    }

    // =========================================================================
    // Global Overrides
    // =========================================================================

    /// Get all global template overrides.
    pub async fn list_global_overrides(&self) -> Result<Vec<TemplateOverride>, PromptTemplateError> {
        let overrides = self.repo.list_global_overrides().await?;
        Ok(overrides
            .into_iter()
            .map(|(key, value)| TemplateOverride { key, value })
            .collect())
    }

    /// Set a global template override.
    pub async fn set_global_override(
        &self,
        key: String,
        value: String,
    ) -> Result<(), PromptTemplateError> {
        // Validate that the key is a known template
        if crate::prompt_templates::get_default(&key).is_none() {
            return Err(PromptTemplateError::UnknownKey(key));
        }

        self.repo.set_global_override(&key, &value).await?;
        Ok(())
    }

    /// Delete a global template override.
    pub async fn delete_global_override(&self, key: String) -> Result<(), PromptTemplateError> {
        self.repo.delete_global_override(&key).await?;
        Ok(())
    }

    // =========================================================================
    // World Overrides
    // =========================================================================

    /// Get all template overrides for a specific world.
    pub async fn list_world_overrides(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<TemplateOverride>, PromptTemplateError> {
        let overrides = self.repo.list_world_overrides(world_id).await?;
        Ok(overrides
            .into_iter()
            .map(|(key, value)| TemplateOverride { key, value })
            .collect())
    }

    /// Set a world-specific template override.
    pub async fn set_world_override(
        &self,
        world_id: WorldId,
        key: String,
        value: String,
    ) -> Result<(), PromptTemplateError> {
        // Validate that the key is a known template
        if crate::prompt_templates::get_default(&key).is_none() {
            return Err(PromptTemplateError::UnknownKey(key));
        }

        self.repo
            .set_world_override(world_id, &key, &value)
            .await?;
        Ok(())
    }

    /// Delete a world-specific template override.
    pub async fn delete_world_override(
        &self,
        world_id: WorldId,
        key: String,
    ) -> Result<(), PromptTemplateError> {
        self.repo.delete_world_override(world_id, &key).await?;
        Ok(())
    }

    // =========================================================================
    // Template Resolution
    // =========================================================================

    /// Resolve a template value for a specific world.
    ///
    /// Resolution priority:
    /// 1. World-specific override (if world_id provided)
    /// 2. Global override
    /// 3. Environment variable
    /// 4. Default value
    pub async fn resolve_template(
        &self,
        world_id: Option<WorldId>,
        key: &str,
    ) -> Result<Option<String>, PromptTemplateError> {
        self.repo.resolve_template(world_id, key).await.map_err(Into::into)
    }

    /// Resolve a template value for a specific world (returns error if not found).
    pub async fn resolve_template_required(
        &self,
        world_id: Option<WorldId>,
        key: &str,
    ) -> Result<String, PromptTemplateError> {
        match self.resolve_template(world_id, key).await? {
            Some(value) => Ok(value),
            None => Err(PromptTemplateError::NotFound(key.to_string())),
        }
    }

    // =========================================================================
    // Metadata
    // =========================================================================

    /// Get metadata for all prompt templates.
    ///
    /// Used by UI to render template lists and forms.
    pub fn all_template_metadata(&self) -> Vec<TemplateMetadata> {
        prompt_template_metadata()
            .into_iter()
            .map(|m| TemplateMetadata {
                key: m.key,
                label: m.label,
                description: m.description,
                category: m.category,
                default_value: m.default_value,
                env_var: m.env_var,
            })
            .collect()
    }

    /// Get metadata for a specific template.
    pub fn get_template_metadata(&self, key: &str) -> Option<TemplateMetadata> {
        prompt_template_metadata()
            .into_iter()
            .find(|m| m.key == key)
            .map(|m| TemplateMetadata {
                key: m.key,
                label: m.label,
                description: m.description,
                category: m.category,
                default_value: m.default_value,
                env_var: m.env_var,
            })
    }
}

// =============================================================================
// Result Types (domain-level, not wire format)
// =============================================================================

/// A template override value.
#[derive(Debug, Clone)]
pub struct TemplateOverride {
    pub key: String,
    pub value: String,
}

/// Metadata about a prompt template.
#[derive(Debug, Clone)]
pub struct TemplateMetadata {
    pub key: String,
    pub label: String,
    pub description: String,
    pub category: crate::prompt_templates::PromptTemplateCategory,
    pub default_value: String,
    pub env_var: String,
}

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum PromptTemplateError {
    #[error("Repository error: {0}")]
    Repo(#[from] crate::infrastructure::ports::RepoError),

    #[error("Unknown template key: {0}")]
    UnknownKey(String),

    #[error("Template not found: {0}")]
    NotFound(String),
}
