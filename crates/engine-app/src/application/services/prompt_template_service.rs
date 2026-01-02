//! Prompt Template Service
//!
//! Manages configurable LLM prompt templates with priority resolution:
//! 1. World-specific DB override (if world_id provided)
//! 2. Global DB override
//! 3. Environment variable (WRLDBLDR_PROMPT_{KEY})
//! 4. Hard-coded default
//!
//! Provides caching for performance and methods to get/set/reset templates.

use async_trait::async_trait;
use std::sync::Arc;

use wrldbldr_domain::value_objects::{
    get_prompt_default, key_to_env_var, prompt_template_keys, prompt_template_metadata,
    PromptTemplateMetadata,
};
use wrldbldr_domain::WorldId;
use crate::application::services::internal::PromptTemplateServicePort;
use wrldbldr_engine_ports::outbound::{
    EnvironmentPort, PromptTemplateCachePort, PromptTemplateError, PromptTemplateRepositoryPort,
    PromptTemplateSource, ResolvedPromptTemplate,
};

/// Service for managing prompt templates with priority resolution
pub struct PromptTemplateService {
    repository: Arc<dyn PromptTemplateRepositoryPort>,
    /// Environment port for reading environment variables (hexagonal architecture)
    environment: Arc<dyn EnvironmentPort>,
    cache: Arc<dyn PromptTemplateCachePort>,
}

impl PromptTemplateService {
    /// Create a new prompt template service
    pub fn new(
        repository: Arc<dyn PromptTemplateRepositoryPort>,
        environment: Arc<dyn EnvironmentPort>,
        cache: Arc<dyn PromptTemplateCachePort>,
    ) -> Self {
        Self {
            repository,
            environment,
            cache,
        }
    }

    /// Resolve a template with full priority chain (no world context)
    ///
    /// Priority: Global DB → Env → Default
    pub async fn resolve(&self, key: &str) -> String {
        self.resolve_with_source(key).await.value
    }

    /// Resolve a template with source information (no world context)
    pub async fn resolve_with_source(&self, key: &str) -> ResolvedPromptTemplate {
        // Check cache first
        if let Some(resolved) = self.cache.get_global(key).await {
            return resolved;
        }

        // Resolve and cache
        let resolved = self.do_resolve_global(key).await;
        self.cache
            .set_global(key.to_string(), resolved.clone())
            .await;
        resolved
    }

    /// Resolve a template for a specific world
    ///
    /// Priority: World DB → Global DB → Env → Default
    pub async fn resolve_for_world(&self, world_id: WorldId, key: &str) -> String {
        self.resolve_for_world_with_source(world_id, key)
            .await
            .value
    }

    /// Resolve a template, optionally for a specific world
    ///
    /// If world_id is Some, uses world-specific resolution (World DB → Global DB → Env → Default)
    /// If world_id is None, uses global resolution (Global DB → Env → Default)
    pub async fn resolve_optional_world(&self, world_id: Option<&WorldId>, key: &str) -> String {
        match world_id {
            Some(wid) => self.resolve_for_world(*wid, key).await,
            None => self.resolve(key).await,
        }
    }

    /// Resolve a template for a specific world with source information
    pub async fn resolve_for_world_with_source(
        &self,
        world_id: WorldId,
        key: &str,
    ) -> ResolvedPromptTemplate {
        // Check cache first
        if let Some(resolved) = self.cache.get_for_world(world_id, key).await {
            return resolved;
        }

        // Resolve and cache
        let resolved = self.do_resolve_for_world(world_id, key).await;
        self.cache
            .set_for_world(world_id, key.to_string(), resolved.clone())
            .await;
        resolved
    }

    /// Get all templates with their resolved values (global context)
    pub async fn get_all(&self) -> Vec<ResolvedPromptTemplate> {
        let mut results = Vec::new();
        for key in prompt_template_keys() {
            results.push(self.resolve_with_source(key).await);
        }
        results
    }

    /// Get all templates with their resolved values for a world
    pub async fn get_all_for_world(&self, world_id: WorldId) -> Vec<ResolvedPromptTemplate> {
        let mut results = Vec::new();
        for key in prompt_template_keys() {
            results.push(self.resolve_for_world_with_source(world_id, key).await);
        }
        results
    }

    /// Get template metadata (keys, labels, descriptions, defaults)
    pub fn get_metadata(&self) -> Vec<PromptTemplateMetadata> {
        prompt_template_metadata()
    }

    /// Set a global template override
    pub async fn set_global(&self, key: &str, value: &str) -> Result<(), PromptTemplateError> {
        self.repository.set_global(key, value).await?;
        // Invalidate caches
        self.cache.remove_global(key).await;
        self.cache.clear_world().await; // World caches inherit from global
        Ok(())
    }

    /// Set multiple global template overrides
    pub async fn set_all_global(
        &self,
        templates: &[(String, String)],
    ) -> Result<(), PromptTemplateError> {
        for (key, value) in templates {
            self.repository.set_global(key, value).await?;
        }
        // Invalidate all caches
        self.cache.clear_global().await;
        self.cache.clear_world().await;
        Ok(())
    }

    /// Delete a global template override (falls back to env/default)
    pub async fn delete_global(&self, key: &str) -> Result<(), PromptTemplateError> {
        self.repository.delete_global(key).await?;
        self.cache.remove_global(key).await;
        self.cache.clear_world().await;
        Ok(())
    }

    /// Reset all global overrides (delete all, fall back to env/defaults)
    pub async fn reset_global(&self) -> Result<(), PromptTemplateError> {
        self.repository.delete_all_global().await?;
        self.cache.clear_global().await;
        self.cache.clear_world().await;
        Ok(())
    }

    /// Set a world-specific template override
    pub async fn set_for_world(
        &self,
        world_id: WorldId,
        key: &str,
        value: &str,
    ) -> Result<(), PromptTemplateError> {
        self.repository.set_for_world(world_id, key, value).await?;
        // Invalidate world cache for this key
        self.cache.remove_for_world(world_id, key).await;
        Ok(())
    }

    /// Set multiple world-specific template overrides
    pub async fn set_all_for_world(
        &self,
        world_id: WorldId,
        templates: &[(String, String)],
    ) -> Result<(), PromptTemplateError> {
        for (key, value) in templates {
            self.repository.set_for_world(world_id, key, value).await?;
        }
        // Invalidate world cache
        self.cache.remove_world(world_id).await;
        Ok(())
    }

    /// Delete a world-specific template override
    pub async fn delete_for_world(
        &self,
        world_id: WorldId,
        key: &str,
    ) -> Result<(), PromptTemplateError> {
        self.repository.delete_for_world(world_id, key).await?;
        self.cache.remove_for_world(world_id, key).await;
        Ok(())
    }

    /// Reset all world-specific overrides (falls back to global/env/defaults)
    pub async fn reset_for_world(&self, world_id: WorldId) -> Result<(), PromptTemplateError> {
        self.repository.delete_all_for_world(world_id).await?;
        self.cache.remove_world(world_id).await;
        Ok(())
    }

    // =========================================================================
    // Internal resolution logic
    // =========================================================================

    /// Resolve a template without world context
    async fn do_resolve_global(&self, key: &str) -> ResolvedPromptTemplate {
        let default_value = get_prompt_default(key)
            .map(|s| s.to_string())
            .unwrap_or_default();

        // 1. Check global DB override
        if let Ok(Some(value)) = self.repository.get_global(key).await {
            return ResolvedPromptTemplate {
                key: key.to_string(),
                value,
                source: PromptTemplateSource::GlobalOverride,
                default_value,
            };
        }

        // 2. Check environment variable (via port - no direct I/O in app layer)
        let env_var = key_to_env_var(key);
        if let Some(value) = self.environment.get_var(&env_var) {
            return ResolvedPromptTemplate {
                key: key.to_string(),
                value,
                source: PromptTemplateSource::Environment,
                default_value,
            };
        }

        // 3. Use default
        ResolvedPromptTemplate {
            key: key.to_string(),
            value: default_value.clone(),
            source: PromptTemplateSource::Default,
            default_value,
        }
    }

    /// Resolve a template with world context
    async fn do_resolve_for_world(&self, world_id: WorldId, key: &str) -> ResolvedPromptTemplate {
        let default_value = get_prompt_default(key)
            .map(|s| s.to_string())
            .unwrap_or_default();

        // 1. Check world-specific DB override
        if let Ok(Some(value)) = self.repository.get_for_world(world_id, key).await {
            return ResolvedPromptTemplate {
                key: key.to_string(),
                value,
                source: PromptTemplateSource::WorldOverride,
                default_value,
            };
        }

        // 2. Check global DB override
        if let Ok(Some(value)) = self.repository.get_global(key).await {
            return ResolvedPromptTemplate {
                key: key.to_string(),
                value,
                source: PromptTemplateSource::GlobalOverride,
                default_value,
            };
        }

        // 3. Check environment variable (via port - no direct I/O in app layer)
        let env_var = key_to_env_var(key);
        if let Some(value) = self.environment.get_var(&env_var) {
            return ResolvedPromptTemplate {
                key: key.to_string(),
                value,
                source: PromptTemplateSource::Environment,
                default_value,
            };
        }

        // 4. Use default
        ResolvedPromptTemplate {
            key: key.to_string(),
            value: default_value.clone(),
            source: PromptTemplateSource::Default,
            default_value,
        }
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

#[async_trait]
impl PromptTemplateServicePort for PromptTemplateService {
    async fn get_all(&self) -> Vec<ResolvedPromptTemplate> {
        self.get_all().await
    }

    async fn set_global(&self, key: &str, value: &str) -> Result<(), PromptTemplateError> {
        self.set_global(key, value).await
    }

    async fn delete_global(&self, key: &str) -> Result<(), PromptTemplateError> {
        self.delete_global(key).await
    }

    async fn reset_global(&self) -> Result<(), PromptTemplateError> {
        self.reset_global().await
    }

    async fn get_all_for_world(&self, world_id: WorldId) -> Vec<ResolvedPromptTemplate> {
        self.get_all_for_world(world_id).await
    }

    async fn set_for_world(
        &self,
        world_id: WorldId,
        key: &str,
        value: &str,
    ) -> Result<(), PromptTemplateError> {
        self.set_for_world(world_id, key, value).await
    }

    async fn delete_for_world(
        &self,
        world_id: WorldId,
        key: &str,
    ) -> Result<(), PromptTemplateError> {
        self.delete_for_world(world_id, key).await
    }

    async fn reset_for_world(&self, world_id: WorldId) -> Result<(), PromptTemplateError> {
        self.reset_for_world(world_id).await
    }

    async fn resolve_with_source(&self, key: &str) -> ResolvedPromptTemplate {
        self.resolve_with_source(key).await
    }

    async fn resolve_for_world_with_source(
        &self,
        world_id: WorldId,
        key: &str,
    ) -> ResolvedPromptTemplate {
        self.resolve_for_world_with_source(world_id, key).await
    }

    fn get_metadata(&self) -> Vec<PromptTemplateMetadata> {
        self.get_metadata()
    }
}
