// Settings use cases - port trait injection per ADR-009
#![allow(dead_code)]

//! Settings management use cases.
//!
//! Handles application settings at both global and per-world levels.
//! Provides fallback logic from world-specific settings to global defaults.

use std::sync::Arc;

use wrldbldr_domain::WorldId;
use wrldbldr_shared::settings::settings_metadata;
use wrldbldr_shared::settings::SettingsFieldMetadata;

use crate::infrastructure::app_settings::AppSettings;
use crate::infrastructure::ports::SettingsRepo;

/// Settings operations use case.
///
/// Encapsulates all settings-related queries and mutations, providing:
/// - Global settings management
/// - Per-world settings with fallback to global
/// - Settings metadata for UI/configuration
pub struct SettingsOps {
    repo: Arc<dyn SettingsRepo>,
}

impl SettingsOps {
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
    pub async fn update_global(&self, settings: AppSettings) -> Result<AppSettings, SettingsError> {
        let settings = settings.with_world_id(None);
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
        if let Some(settings) = self.repo.get_for_world(world_id).await? {
            let settings = settings.with_world_id(Some(world_id));
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
        settings: AppSettings,
    ) -> Result<AppSettings, SettingsError> {
        let settings = settings.with_world_id(Some(world_id));
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

    /// Load settings with environment variable overrides applied.
    ///
    /// Takes base settings (from file or database) and applies overrides
    /// from environment variables.
    pub fn load_settings_from_env(base_settings: AppSettings) -> AppSettings {
        let mut settings = base_settings;
        Self::apply_env_list_limits(&mut settings);
        settings
    }

    /// Apply list limit environment variable overrides to settings.
    ///
    /// Supported environment variables:
    /// - WRLDBLDR_LIST_DEFAULT_PAGE_SIZE: Override default list page size (range: 10-200)
    /// - WRLDBLDR_LIST_MAX_PAGE_SIZE: Override max list page size (range: 50-1000)
    pub fn apply_env_list_limits(settings: &mut AppSettings) {
        let mut updated = std::mem::take(settings);

        if let Ok(val) = std::env::var("WRLDBLDR_LIST_DEFAULT_PAGE_SIZE") {
            if let Ok(size) = val.parse::<u32>() {
                if size >= 10 && size <= 200 {
                    updated = updated.with_list_default_page_size_override(Some(size));
                    tracing::info!(size, "Applied WRLDBLDR_LIST_DEFAULT_PAGE_SIZE environment variable");
                } else {
                    tracing::warn!(
                        size,
                        "WRLDBLDR_LIST_DEFAULT_PAGE_SIZE out of range [10, 200], ignoring"
                    );
                }
            } else {
                tracing::warn!(
                    val = %val,
                    "WRLDBLDR_LIST_DEFAULT_PAGE_SIZE is not a valid u32, ignoring"
                );
            }
        }

        if let Ok(val) = std::env::var("WRLDBLDR_LIST_MAX_PAGE_SIZE") {
            if let Ok(size) = val.parse::<u32>() {
                if size >= 50 && size <= 1000 {
                    updated = updated.with_list_max_page_size_override(Some(size));
                    tracing::info!(size, "Applied WRLDBLDR_LIST_MAX_PAGE_SIZE environment variable");
                } else {
                    tracing::warn!(
                        size,
                        "WRLDBLDR_LIST_MAX_PAGE_SIZE out of range [50, 1000], ignoring"
                    );
                }
            } else {
                tracing::warn!(
                    val = %val,
                    "WRLDBLDR_LIST_MAX_PAGE_SIZE is not a valid u32, ignoring"
                );
            }
        }

        *settings = updated;
    }
}

/// Errors that can occur during settings operations.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Repository error: {0}")]
    Repo(#[from] crate::infrastructure::ports::RepoError),
}
