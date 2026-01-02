use async_trait::async_trait;
use std::sync::Arc;
use wrldbldr_domain::value_objects::AppSettings;
use wrldbldr_domain::WorldId;
use crate::application::services::internal::{LlmConfig, SettingsUseCasePort};
use wrldbldr_engine_ports::outbound::{SettingsCachePort, SettingsError, SettingsRepositoryPort};

/// Callback type for loading settings from environment.
/// This allows the service to remain decoupled from the adapters layer.
pub type SettingsLoaderFn = Arc<dyn Fn() -> AppSettings + Send + Sync>;

/// Callback type for loading LLM configuration from environment.
/// This allows the service to remain decoupled from the adapters layer.
pub type LlmConfigLoaderFn = Arc<dyn Fn() -> LlmConfig + Send + Sync>;

pub struct SettingsService {
    repository: Arc<dyn SettingsRepositoryPort>,
    /// Callback to load settings from environment (injected from adapters layer)
    settings_loader: SettingsLoaderFn,
    /// Callback to load LLM config from environment (injected from adapters layer)
    llm_config_loader: Option<LlmConfigLoaderFn>,
    cache: Arc<dyn SettingsCachePort>,
}

impl SettingsService {
    /// Create a new SettingsService with the given repository and settings loader.
    ///
    /// The `settings_loader` should be `load_settings_from_env` from the adapters layer,
    /// wrapped in an Arc. This keeps environment I/O in the adapters layer while allowing
    /// the application service to fall back to environment-based defaults when needed.
    pub fn new(
        repository: Arc<dyn SettingsRepositoryPort>,
        settings_loader: SettingsLoaderFn,
        cache: Arc<dyn SettingsCachePort>,
    ) -> Self {
        Self {
            repository,
            settings_loader,
            llm_config_loader: None,
            cache,
        }
    }

    /// Set the LLM config loader callback.
    ///
    /// This should be called during service construction to inject the
    /// environment-based LLM configuration loader from the adapters layer.
    pub fn with_llm_config_loader(mut self, loader: LlmConfigLoaderFn) -> Self {
        self.llm_config_loader = Some(loader);
        self
    }

    /// Get global settings (cached)
    pub async fn get(&self) -> AppSettings {
        if let Some(settings) = self.cache.get_global().await {
            return settings;
        }

        // Load from DB
        match self.repository.get().await {
            Ok(settings) => {
                self.cache.set_global(Some(settings.clone())).await;
                settings
            }
            Err(_) => (self.settings_loader)(),
        }
    }

    /// Update global settings and invalidate cache
    pub async fn update(&self, settings: AppSettings) -> Result<(), SettingsError> {
        self.repository.save(&settings).await?;
        self.cache.set_global(Some(settings)).await;
        // Also invalidate world caches since they inherit from global
        self.cache.clear_world().await;
        Ok(())
    }

    /// Reset global settings to env/defaults and clear DB values
    pub async fn reset(&self) -> Result<AppSettings, SettingsError> {
        let settings = self.repository.reset().await?;
        self.cache.set_global(Some(settings.clone())).await;
        // Also invalidate world caches
        self.cache.clear_world().await;
        Ok(settings)
    }

    /// Get settings for a specific world (cached)
    pub async fn get_for_world(&self, world_id: WorldId) -> AppSettings {
        // Check cache first
        if let Some(settings) = self.cache.get_world(world_id).await {
            return settings;
        }

        // Load from DB
        match self.repository.get_for_world(world_id).await {
            Ok(settings) => {
                self.cache.set_world(world_id, settings.clone()).await;
                settings
            }
            Err(_) => {
                // Fall back to global settings with world_id set
                let mut settings = self.get().await;
                settings.world_id = Some(world_id.into());
                settings
            }
        }
    }

    /// Update per-world settings
    pub async fn update_for_world(
        &self,
        world_id: WorldId,
        mut settings: AppSettings,
    ) -> Result<(), SettingsError> {
        settings.world_id = Some(world_id.into());
        self.repository.save_for_world(world_id, &settings).await?;
        self.cache.set_world(world_id, settings).await;
        Ok(())
    }

    /// Reset per-world settings to global defaults
    pub async fn reset_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
        let settings = self.repository.reset_for_world(world_id).await?;
        self.cache.set_world(world_id, settings.clone()).await;
        Ok(settings)
    }

    /// Delete per-world settings when world is deleted
    pub async fn delete_for_world(&self, world_id: WorldId) -> Result<(), SettingsError> {
        self.repository.delete_for_world(world_id).await?;
        self.cache.remove_world(world_id).await;
        Ok(())
    }

    /// Invalidate all caches (useful when settings are modified externally)
    pub async fn invalidate_cache(&self) {
        self.cache.set_global(None).await;
        self.cache.clear_world().await;
    }

    /// Get LLM configuration for a world
    ///
    /// Currently returns global LLM config from environment.
    /// Per-world LLM overrides could be added in the future.
    pub async fn get_llm_config(&self, _world_id: WorldId) -> Result<LlmConfig, SettingsError> {
        match &self.llm_config_loader {
            Some(loader) => Ok((loader)()),
            None => {
                // Return default config if no loader is set
                // This provides reasonable defaults for testing
                Ok(LlmConfig {
                    api_base_url: "http://localhost:11434/v1".to_string(),
                    model: "qwen3-vl:30b".to_string(),
                    api_key: None,
                    max_tokens: None,
                    temperature: None,
                })
            }
        }
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

#[async_trait]
impl SettingsUseCasePort for SettingsService {
    async fn get(&self) -> AppSettings {
        self.get().await
    }

    async fn update(&self, settings: AppSettings) -> Result<(), SettingsError> {
        self.update(settings).await
    }

    async fn reset(&self) -> Result<AppSettings, SettingsError> {
        self.reset().await
    }

    async fn get_for_world(&self, world_id: WorldId) -> AppSettings {
        self.get_for_world(world_id).await
    }

    async fn update_for_world(
        &self,
        world_id: WorldId,
        settings: AppSettings,
    ) -> Result<(), SettingsError> {
        self.update_for_world(world_id, settings).await
    }

    async fn reset_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
        self.reset_for_world(world_id).await
    }

    async fn delete_for_world(&self, world_id: WorldId) -> Result<(), SettingsError> {
        self.delete_for_world(world_id).await
    }

    async fn get_llm_config(&self, world_id: WorldId) -> Result<LlmConfig, SettingsError> {
        self.get_llm_config(world_id).await
    }
}
