//! Settings use cases.

use std::sync::Arc;

use wrldbldr_domain::{settings_metadata, AppSettings, SettingsFieldMetadata, WorldId};

use crate::infrastructure::ports::{RepoError, SettingsRepo};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

pub struct SettingsUseCases {
    pub ops: Arc<SettingsOps>,
}

impl SettingsUseCases {
    pub fn new(ops: Arc<SettingsOps>) -> Self {
        Self { ops }
    }
}

pub struct SettingsOps {
    repo: Arc<dyn SettingsRepo>,
}

impl SettingsOps {
    pub fn new(repo: Arc<dyn SettingsRepo>) -> Self {
        Self { repo }
    }

    pub async fn get_global(&self) -> Result<AppSettings, SettingsError> {
        Ok(self
            .repo
            .get_global()
            .await?
            .unwrap_or_default())
    }

    pub async fn update_global(&self, mut settings: AppSettings) -> Result<AppSettings, SettingsError> {
        settings.world_id = None;
        self.repo.save_global(&settings).await?;
        Ok(settings)
    }

    pub async fn reset_global(&self) -> Result<AppSettings, SettingsError> {
        let settings = AppSettings::default();
        self.repo.save_global(&settings).await?;
        Ok(settings)
    }

    pub async fn get_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
        if let Some(mut settings) = self.repo.get_for_world(world_id).await? {
            settings.world_id = Some(world_id);
            return Ok(settings);
        }

        let global = self.get_global().await?;
        Ok(AppSettings::for_world(global, world_id))
    }

    pub async fn update_for_world(
        &self,
        world_id: WorldId,
        mut settings: AppSettings,
    ) -> Result<AppSettings, SettingsError> {
        settings.world_id = Some(world_id);
        self.repo.save_for_world(world_id, &settings).await?;
        Ok(settings)
    }

    pub async fn reset_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
        self.repo.delete_for_world(world_id).await?;
        self.get_for_world(world_id).await
    }

    pub fn metadata(&self) -> Vec<SettingsFieldMetadata> {
        settings_metadata()
    }
}
