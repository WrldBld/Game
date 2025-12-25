//! Prompt Template Service
//!
//! Manages configurable LLM prompt templates with priority resolution:
//! 1. World-specific DB override (if world_id provided)
//! 2. Global DB override
//! 3. Environment variable (WRLDBLDR_PROMPT_{KEY})
//! 4. Hard-coded default
//!
//! Provides caching for performance and methods to get/set/reset templates.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use wrldbldr_engine_ports::outbound::{
    PromptTemplateError, PromptTemplateRepositoryPort, PromptTemplateSource, ResolvedPromptTemplate,
};
use wrldbldr_domain::value_objects::{
    get_prompt_default, key_to_env_var, prompt_template_keys, prompt_template_metadata,
    PromptTemplateMetadata,
};
use wrldbldr_domain::WorldId;

/// Service for managing prompt templates with priority resolution
pub struct PromptTemplateService {
    repository: Arc<dyn PromptTemplateRepositoryPort>,
    /// Cache for global resolved templates
    global_cache: RwLock<HashMap<String, ResolvedPromptTemplate>>,
    /// Cache for per-world resolved templates
    world_cache: RwLock<HashMap<WorldId, HashMap<String, ResolvedPromptTemplate>>>,
}

impl PromptTemplateService {
    /// Create a new prompt template service
    pub fn new(repository: Arc<dyn PromptTemplateRepositoryPort>) -> Self {
        Self {
            repository,
            global_cache: RwLock::new(HashMap::new()),
            world_cache: RwLock::new(HashMap::new()),
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
        {
            let cache = self.global_cache.read().await;
            if let Some(resolved) = cache.get(key) {
                return resolved.clone();
            }
        }

        // Resolve and cache
        let resolved = self.do_resolve_global(key).await;
        self.global_cache.write().await.insert(key.to_string(), resolved.clone());
        resolved
    }

    /// Resolve a template for a specific world
    ///
    /// Priority: World DB → Global DB → Env → Default
    pub async fn resolve_for_world(&self, world_id: WorldId, key: &str) -> String {
        self.resolve_for_world_with_source(world_id, key).await.value
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
    pub async fn resolve_for_world_with_source(&self, world_id: WorldId, key: &str) -> ResolvedPromptTemplate {
        // Check cache first
        {
            let cache = self.world_cache.read().await;
            if let Some(world_templates) = cache.get(&world_id) {
                if let Some(resolved) = world_templates.get(key) {
                    return resolved.clone();
                }
            }
        }

        // Resolve and cache
        let resolved = self.do_resolve_for_world(world_id, key).await;
        let mut cache = self.world_cache.write().await;
        cache.entry(world_id)
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), resolved.clone());
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
        self.global_cache.write().await.remove(key);
        self.world_cache.write().await.clear(); // World caches inherit from global
        Ok(())
    }

    /// Set multiple global template overrides
    pub async fn set_all_global(&self, templates: &[(String, String)]) -> Result<(), PromptTemplateError> {
        for (key, value) in templates {
            self.repository.set_global(key, value).await?;
        }
        // Invalidate all caches
        self.global_cache.write().await.clear();
        self.world_cache.write().await.clear();
        Ok(())
    }

    /// Delete a global template override (falls back to env/default)
    pub async fn delete_global(&self, key: &str) -> Result<(), PromptTemplateError> {
        self.repository.delete_global(key).await?;
        self.global_cache.write().await.remove(key);
        self.world_cache.write().await.clear();
        Ok(())
    }

    /// Reset all global overrides (delete all, fall back to env/defaults)
    pub async fn reset_global(&self) -> Result<(), PromptTemplateError> {
        self.repository.delete_all_global().await?;
        self.global_cache.write().await.clear();
        self.world_cache.write().await.clear();
        Ok(())
    }

    /// Set a world-specific template override
    pub async fn set_for_world(&self, world_id: WorldId, key: &str, value: &str) -> Result<(), PromptTemplateError> {
        self.repository.set_for_world(world_id, key, value).await?;
        // Invalidate world cache for this key
        if let Some(world_templates) = self.world_cache.write().await.get_mut(&world_id) {
            world_templates.remove(key);
        }
        Ok(())
    }

    /// Set multiple world-specific template overrides
    pub async fn set_all_for_world(&self, world_id: WorldId, templates: &[(String, String)]) -> Result<(), PromptTemplateError> {
        for (key, value) in templates {
            self.repository.set_for_world(world_id, key, value).await?;
        }
        // Invalidate world cache
        self.world_cache.write().await.remove(&world_id);
        Ok(())
    }

    /// Delete a world-specific template override
    pub async fn delete_for_world(&self, world_id: WorldId, key: &str) -> Result<(), PromptTemplateError> {
        self.repository.delete_for_world(world_id, key).await?;
        if let Some(world_templates) = self.world_cache.write().await.get_mut(&world_id) {
            world_templates.remove(key);
        }
        Ok(())
    }

    /// Reset all world-specific overrides (falls back to global/env/defaults)
    pub async fn reset_for_world(&self, world_id: WorldId) -> Result<(), PromptTemplateError> {
        self.repository.delete_all_for_world(world_id).await?;
        self.world_cache.write().await.remove(&world_id);
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

        // 2. Check environment variable
        let env_var = key_to_env_var(key);
        if let Ok(value) = std::env::var(&env_var) {
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

        // 3. Check environment variable
        let env_var = key_to_env_var(key);
        if let Ok(value) = std::env::var(&env_var) {
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
