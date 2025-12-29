//! Settings service port - Interface for settings operations
//!
//! This port abstracts settings business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::value_objects::AppSettings;
use wrldbldr_domain::WorldId;

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

/// Port for settings service operations
///
/// This trait defines the operations for retrieving and updating
/// application settings, both globally and per-world.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait SettingsServicePort: Send + Sync {
    /// Get settings for a specific world
    ///
    /// Returns the settings for the specified world, falling back to
    /// global settings if no world-specific settings exist.
    async fn get_settings(&self, world_id: WorldId) -> Result<AppSettings>;

    /// Update settings for a specific world
    ///
    /// Saves the provided settings for the specified world.
    async fn update_settings(&self, world_id: WorldId, settings: AppSettings) -> Result<()>;

    /// Get LLM configuration for a world
    ///
    /// Extracts and returns the LLM-specific configuration from the
    /// world's settings.
    async fn get_llm_config(&self, world_id: WorldId) -> Result<LlmConfig>;
}
