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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::MockLocationRepo;
    use std::sync::Arc;
    use wrldbldr_domain::{LocationId, RegionName};

    fn create_test_region(location_id: LocationId) -> wrldbldr_domain::Region {
        wrldbldr_domain::Region::new(location_id, RegionName::new("Test Region").unwrap())
    }

    mod trigger_location_event {
        use super::*;

        #[tokio::test]
        async fn when_location_not_found_returns_error() {
            let region_id = RegionId::new();

            let mut location_repo = MockLocationRepo::new();
            location_repo
                .expect_get_region()
                .withf(move |id| *id == region_id)
                .returning(|_| Ok(None));

            let use_case =
                TriggerLocationEvent::new(Arc::new(location_repo) as Arc<dyn LocationRepo>);

            let result = use_case
                .execute(region_id, "Test event description".to_string())
                .await;

            assert!(matches!(result, Err(LocationEventError::RegionNotFound)));
        }

        #[tokio::test]
        async fn when_valid_location_succeeds() {
            let location_id = LocationId::new();
            let region = create_test_region(location_id);
            let region_id = region.id();

            let mut location_repo = MockLocationRepo::new();
            location_repo
                .expect_get_region()
                .withf(move |id| *id == region_id)
                .returning(move |_| Ok(Some(region.clone())));

            let use_case =
                TriggerLocationEvent::new(Arc::new(location_repo) as Arc<dyn LocationRepo>);

            let result = use_case
                .execute(region_id, "Something happened!".to_string())
                .await;

            assert!(result.is_ok());
            let event_result = result.unwrap();
            assert_eq!(event_result.region_id, region_id);
            assert_eq!(event_result.region_name, "Test Region");
            assert_eq!(event_result.description, "Something happened!");
        }
    }
}
