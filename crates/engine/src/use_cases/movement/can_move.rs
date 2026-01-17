// Can move use case - prepared for future movement validation
#![allow(dead_code)]

//! Can move use case.
//!
//! Checks if movement between two regions is allowed.

use std::sync::Arc;
use wrldbldr_domain::RegionId;

use crate::infrastructure::ports::{LocationRepo, RepoError};

/// Check if movement between regions is possible.
///
/// Validates that a connection exists and is not locked.
pub struct CanMove {
    location_repo: Arc<dyn LocationRepo>,
}

impl CanMove {
    pub fn new(location_repo: Arc<dyn LocationRepo>) -> Self {
        Self { location_repo }
    }

    /// Check if a region connection exists and is not locked.
    ///
    /// # Arguments
    /// * `from` - Source region ID
    /// * `to` - Target region ID
    ///
    /// # Returns
    /// * `Ok(true)` - Movement is allowed
    /// * `Ok(false)` - No connection exists or connection is locked
    /// * `Err(RepoError)` - Repository operation failed
    pub async fn execute(&self, from: RegionId, to: RegionId) -> Result<bool, RepoError> {
        let connections = self.location_repo.get_connections(from).await?;
        Ok(connections
            .iter()
            .any(|c| c.to_region == to && !c.is_locked))
    }
}
