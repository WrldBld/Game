//! Get region exits use case.
//!
//! Resolves exits from a region to other locations, enriching them with
//! location names and determining arrival regions.

use std::sync::Arc;
use wrldbldr_domain::{LocationId, RegionId};

use crate::infrastructure::ports::{LocationRepo, RepoError};

// =============================================================================
// DTOs (Use Case Result Types)
// =============================================================================

/// An exit from a region to another location.
///
/// Used for navigation UI - enriched version of LocationConnection.
#[derive(Debug, Clone)]
pub struct RegionExit {
    pub location_id: LocationId,
    pub location_name: String,
    pub arrival_region_id: RegionId,
    pub description: Option<String>,
}

/// Result of getting exits from a region.
///
/// Includes both valid exits and any exits that were skipped due to
/// data integrity issues (for error reporting).
#[derive(Debug, Clone, Default)]
pub struct RegionExitsResult {
    /// Successfully resolved exits
    pub exits: Vec<RegionExit>,
    /// Exits that were skipped due to errors (for error reporting)
    pub skipped: Vec<SkippedExit>,
}

/// An exit that was skipped due to a data integrity issue.
#[derive(Debug, Clone)]
pub struct SkippedExit {
    /// The target location ID that was referenced
    pub to_location: LocationId,
    /// Why this exit was skipped
    pub reason: String,
}

// =============================================================================
// Use Case
// =============================================================================

/// Get exits from a region to other locations.
///
/// This finds the location for the given region, then finds connections to
/// other locations, and enriches them with location names and default arrival regions.
///
/// First checks for explicit region exits (RegionExit domain objects). If none exist,
/// falls back to location-level connections (LocationConnection) and resolves arrival
/// regions from default regions or spawn points.
pub struct GetRegionExits {
    location_repo: Arc<dyn LocationRepo>,
}

impl GetRegionExits {
    pub fn new(location_repo: Arc<dyn LocationRepo>) -> Self {
        Self { location_repo }
    }

    /// Execute the get exits use case.
    ///
    /// # Arguments
    /// * `region_id` - The region to get exits from
    ///
    /// # Returns
    /// * `Ok(RegionExitsResult)` - Successfully resolved exits (may include skipped entries)
    /// * `Err(RepoError)` - Repository operation failed
    pub async fn execute(&self, region_id: RegionId) -> Result<RegionExitsResult, RepoError> {
        // First, try explicit region exits (the preferred method)
        let region_exits = self.location_repo.get_region_exits(region_id).await?;
        if !region_exits.is_empty() {
            return self.resolve_explicit_exits(region_id, region_exits).await;
        }

        // Fall back to location-level connections
        self.resolve_location_level_exits(region_id).await
    }

    /// Resolve explicit region exits (RegionExit domain objects).
    async fn resolve_explicit_exits(
        &self,
        region_id: RegionId,
        region_exits: Vec<wrldbldr_domain::RegionExit>,
    ) -> Result<RegionExitsResult, RepoError> {
        let mut result = RegionExitsResult::default();

        for exit in region_exits {
            if let Some(target_location) = self.location_repo.get_location(exit.to_location).await?
            {
                result.exits.push(RegionExit {
                    location_id: exit.to_location,
                    location_name: target_location.name().to_string(),
                    arrival_region_id: exit.arrival_region_id,
                    description: exit.description.clone(),
                });
            } else {
                let reason = "Target location not found".to_string();
                tracing::error!(
                    from_region = %region_id,
                    to_location = %exit.to_location,
                    reason = %reason,
                    "Navigation exit skipped due to data integrity issue"
                );
                result.skipped.push(SkippedExit {
                    to_location: exit.to_location,
                    reason,
                });
            }
        }

        Ok(result)
    }

    /// Resolve exits from location-level connections (LocationConnection).
    ///
    /// This is the fallback when no explicit region exits exist.
    async fn resolve_location_level_exits(
        &self,
        region_id: RegionId,
    ) -> Result<RegionExitsResult, RepoError> {
        // Get the region to find its location
        let region = match self.location_repo.get_region(region_id).await? {
            Some(r) => r,
            None => return Ok(RegionExitsResult::default()),
        };

        // Get exits from this location
        let location_exits = self
            .location_repo
            .get_location_exits(region.location_id())
            .await?;

        let mut result = RegionExitsResult::default();
        for exit in location_exits {
            // Get the target location details
            if let Some(target_location) = self.location_repo.get_location(exit.to_location).await?
            {
                // Determine arrival region
                let arrival_region_id =
                    if let Some(default_region) = target_location.default_region_id() {
                        default_region
                    } else {
                        // Try to find a spawn point in the target location
                        let regions = self
                            .location_repo
                            .list_regions_in_location(exit.to_location)
                            .await?;
                        match regions.into_iter().find(|r| r.is_spawn_point()) {
                            Some(r) => r.id(),
                            None => {
                                let reason = format!(
                                    "Target location '{}' has no default region and no spawn point",
                                    target_location.name().as_str()
                                );
                                tracing::error!(
                                    from_region = %region_id,
                                    to_location = %exit.to_location,
                                    target_location_name = %target_location.name().as_str(),
                                    reason = %reason,
                                    "Navigation exit skipped due to data integrity issue"
                                );
                                result.skipped.push(SkippedExit {
                                    to_location: exit.to_location,
                                    reason,
                                });
                                continue;
                            }
                        }
                    };

                result.exits.push(RegionExit {
                    location_id: exit.to_location,
                    location_name: target_location.name().to_string(),
                    arrival_region_id,
                    description: exit.description.clone(),
                });
            }
        }

        Ok(result)
    }
}
