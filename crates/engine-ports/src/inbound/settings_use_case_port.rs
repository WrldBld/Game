//! Settings use case port - Inbound interface for settings operations
//!
//! This port is called by HTTP handlers to manage application settings.
//! The implementation lives in engine-app.

use async_trait::async_trait;
use wrldbldr_domain::value_objects::AppSettings;
use wrldbldr_domain::WorldId;

use crate::outbound::SettingsError;

/// LLM configuration extracted from settings
///
/// Contains the specific LLM-related settings needed by adapters.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Base URL for the LLM API
    pub api_base_url: String,
    /// Model name to use
    pub model: String,
    /// API key for authentication (if required)
    pub api_key: Option<String>,
    /// Maximum tokens for responses
    pub max_tokens: Option<u32>,
    /// Temperature for sampling
    pub temperature: Option<f32>,
}

/// Port for settings use case operations
///
/// This trait defines the operations for retrieving and updating
/// application settings, both globally and per-world.
///
/// Called by: HTTP handlers in settings_routes.rs
/// Implemented by: SettingsService in engine-app
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait SettingsUseCasePort: Send + Sync {
    // =========================================================================
    // Global Settings
    // =========================================================================

    /// Get global settings
    ///
    /// Returns the global application settings, falling back to
    /// environment-based defaults if no settings are stored.
    async fn get(&self) -> AppSettings;

    /// Update global settings
    ///
    /// Saves the provided settings as the global application settings.
    async fn update(&self, settings: AppSettings) -> Result<(), SettingsError>;

    /// Reset global settings to defaults
    ///
    /// Clears stored settings and returns the environment-based defaults.
    async fn reset(&self) -> Result<AppSettings, SettingsError>;

    // =========================================================================
    // Per-World Settings
    // =========================================================================

    /// Get settings for a specific world
    ///
    /// Returns the settings for the specified world, falling back to
    /// global settings if no world-specific settings exist.
    async fn get_for_world(&self, world_id: WorldId) -> AppSettings;

    /// Update settings for a specific world
    ///
    /// Saves the provided settings for the specified world.
    async fn update_for_world(
        &self,
        world_id: WorldId,
        settings: AppSettings,
    ) -> Result<(), SettingsError>;

    /// Reset per-world settings to global defaults
    ///
    /// Clears world-specific settings and returns the global settings
    /// with the world_id set.
    async fn reset_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError>;

    /// Delete per-world settings
    ///
    /// Removes all stored settings for the specified world.
    /// Typically called when a world is deleted.
    async fn delete_for_world(&self, world_id: WorldId) -> Result<(), SettingsError>;

    // =========================================================================
    // Derived Configuration
    // =========================================================================

    /// Get LLM configuration for a world
    ///
    /// Extracts and returns the LLM-specific configuration from the
    /// world's settings.
    async fn get_llm_config(&self, world_id: WorldId) -> Result<LlmConfig, SettingsError>;
}
