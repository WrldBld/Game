// Location event use cases - methods for future features
#![allow(dead_code)]

//! Location event use cases.
//!
//! Handles DM-triggered location events.

use std::sync::Arc;

use crate::infrastructure::ports::{LocationRepo, RepoError};
use wrldbldr_domain::RegionId;

/// Container for location event use cases.
pub struct LocationEventUseCases {
    pub trigger: Arc<TriggerLocationEvent>,
}

impl LocationEventUseCases {
    pub fn new(trigger: Arc<TriggerLocationEvent>) -> Self {
        Self { trigger }
    }
}

/// Trigger a location event (DM broadcast).
pub struct TriggerLocationEvent {
    location: Arc<dyn LocationRepo>,
}

impl TriggerLocationEvent {
    pub fn new(location: Arc<dyn LocationRepo>) -> Self {
        Self { location }
    }

    pub async fn execute(
        &self,
        region_id: RegionId,
        description: String,
    ) -> Result<LocationEventResult, LocationEventError> {
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(LocationEventError::RegionNotFound)?;

        Ok(LocationEventResult {
            region_id,
            region_name: region.name().to_string(),
            description,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LocationEventResult {
    pub region_id: RegionId,
    pub region_name: String,
    pub description: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LocationEventError {
    #[error("Region not found")]
    RegionNotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
