//! Settings entity operations.
//!
//! Handles application settings at both global and per-world levels.
//! Provides fallback logic from world-specific settings to global defaults.

use std::sync::Arc;

use wrldbldr_domain::{AppSettings, WorldId};
use wrldbldr_protocol::settings::settings_metadata;
use wrldbldr_protocol::settings::SettingsFieldMetadata;

use crate::infrastructure::ports::{RepoError, SettingsRepo};

/// Settings entity operations.
///
/// Encapsulates all settings-related queries and mutations, providing:
/// - Global settings management
/// - Per-world settings with fallback to global
/// - Settings metadata for UI/configuration
pub struct Settings {
    repo: Arc<dyn SettingsRepo>,
}

impl Settings {
    pub fn new(repo: Arc<dyn SettingsRepo>) -> Self {
        Self { repo }
    }

    /// Get global application settings.
    ///
    /// Returns default settings if none have been saved.
    pub async fn get_global(&self) -> Result<AppSettings, SettingsError> {
        Ok(self.repo.get_global().await?.unwrap_or_default())
    }

    /// Update global application settings.
    ///
    /// Clears any world_id to ensure settings are truly global.
    pub async fn update_global(
        &self,
        mut settings: AppSettings,
    ) -> Result<AppSettings, SettingsError> {
        settings.world_id = None;
        self.repo.save_global(&settings).await?;
        Ok(settings)
    }

    /// Reset global settings to defaults.
    pub async fn reset_global(&self) -> Result<AppSettings, SettingsError> {
        let settings = AppSettings::default();
        self.repo.save_global(&settings).await?;
        Ok(settings)
    }

    /// Get settings for a specific world.
    ///
    /// Falls back to global settings if no world-specific settings exist.
    /// The returned settings will have the world_id set appropriately.
    pub async fn get_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
        if let Some(mut settings) = self.repo.get_for_world(world_id).await? {
            settings.world_id = Some(world_id);
            return Ok(settings);
        }

        let global = self.get_global().await?;
        Ok(AppSettings::for_world(global, world_id))
    }

    /// Update settings for a specific world.
    ///
    /// Sets the world_id to ensure settings are associated with the correct world.
    pub async fn update_for_world(
        &self,
        world_id: WorldId,
        mut settings: AppSettings,
    ) -> Result<AppSettings, SettingsError> {
        settings.world_id = Some(world_id);
        self.repo.save_for_world(world_id, &settings).await?;
        Ok(settings)
    }

    /// Reset world-specific settings.
    ///
    /// Deletes the world-specific settings, causing future reads to fall back
    /// to global settings.
    pub async fn reset_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
        self.repo.delete_for_world(world_id).await?;
        self.get_for_world(world_id).await
    }

    /// Get metadata about available settings fields.
    ///
    /// Used by UI to render settings forms with descriptions, types, and defaults.
    pub fn metadata(&self) -> Vec<SettingsFieldMetadata> {
        settings_metadata()
    }
}

/// Errors that can occur during settings operations.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
