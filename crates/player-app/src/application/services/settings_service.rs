//! Settings Service - Application service for managing Engine settings
//!
//! This service provides use case implementations for fetching and
//! updating the Engine's application settings. It abstracts away
//! the HTTP client details from the presentation layer.

use crate::application::dto::{AppSettings, SettingsMetadataResponse};
use wrldbldr_player_ports::outbound::{ApiError, ApiPort};

/// Settings service for managing Engine application settings
///
/// This service provides methods for settings-related operations
/// while depending only on the `ApiPort` trait, not concrete
/// infrastructure implementations.
pub struct SettingsService<A: ApiPort> {
    api: A,
}

impl<A: ApiPort> SettingsService<A> {
    /// Create a new SettingsService with the given API port
    pub fn new(api: A) -> Self {
        Self { api }
    }

    // =========================================================================
    // Global Settings
    // =========================================================================

    /// Get current global application settings from the Engine
    pub async fn get(&self) -> Result<AppSettings, ApiError> {
        self.api.get("/api/settings").await
    }

    /// Update global application settings on the Engine
    ///
    /// # Arguments
    /// * `settings` - The updated settings to save
    ///
    /// # Returns
    /// The updated settings as confirmed by the Engine
    pub async fn update(&self, settings: &AppSettings) -> Result<AppSettings, ApiError> {
        self.api.put("/api/settings", settings).await
    }

    /// Reset global application settings to defaults
    ///
    /// # Returns
    /// The default settings as confirmed by the Engine
    pub async fn reset(&self) -> Result<AppSettings, ApiError> {
        self.api.post("/api/settings/reset", &()).await
    }

    // =========================================================================
    // Per-World Settings
    // =========================================================================

    /// Get settings for a specific world
    ///
    /// Returns world-specific settings if configured, otherwise falls back
    /// to global settings with the world_id set.
    ///
    /// # Arguments
    /// * `world_id` - The UUID of the world
    pub async fn get_for_world(&self, world_id: &str) -> Result<AppSettings, ApiError> {
        let path = format!("/api/worlds/{}/settings", world_id);
        self.api.get(&path).await
    }

    /// Update settings for a specific world
    ///
    /// # Arguments
    /// * `world_id` - The UUID of the world
    /// * `settings` - The updated settings to save
    ///
    /// # Returns
    /// The updated settings as confirmed by the Engine
    pub async fn update_for_world(&self, world_id: &str, settings: &AppSettings) -> Result<AppSettings, ApiError> {
        let path = format!("/api/worlds/{}/settings", world_id);
        self.api.put(&path, settings).await
    }

    /// Reset world settings to global defaults
    ///
    /// This removes all world-specific overrides and reverts to using
    /// the global settings for this world.
    ///
    /// # Arguments
    /// * `world_id` - The UUID of the world
    ///
    /// # Returns
    /// The global settings (now applied to this world)
    pub async fn reset_for_world(&self, world_id: &str) -> Result<AppSettings, ApiError> {
        let path = format!("/api/worlds/{}/settings/reset", world_id);
        self.api.post(&path, &()).await
    }

    // =========================================================================
    // Metadata
    // =========================================================================

    /// Get settings field metadata for UI rendering
    ///
    /// Returns information about all configurable settings fields including:
    /// - Display names and descriptions
    /// - Field types and validation constraints
    /// - Category groupings
    /// - Whether fields require restart
    pub async fn get_metadata(&self) -> Result<SettingsMetadataResponse, ApiError> {
        self.api.get("/api/settings/metadata").await
    }
}
