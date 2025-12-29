//! Location and region availability management for Challenge entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::{ChallengeLocationAvailability, ChallengeRegionAvailability};
use wrldbldr_domain::{Challenge, ChallengeId, LocationId, RegionId};

/// Location and region availability operations for Challenge entities.
///
/// This trait manages:
/// - AVAILABLE_AT edges between Challenge and Location nodes
/// - AVAILABLE_AT_REGION edges between Challenge and Region nodes
/// - ON_SUCCESS_UNLOCKS edges for location unlocks on challenge completion
///
/// # Used By
/// - `ChallengeServiceImpl` - For managing where challenges are available
/// - `NavigationService` - For finding challenges at current location/region
/// - `ChallengeResolutionService` - For unlocking locations on success
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ChallengeAvailabilityPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Location Availability (AVAILABLE_AT edges)
    // -------------------------------------------------------------------------

    /// Add a location where this challenge is available (creates AVAILABLE_AT edge)
    async fn add_location_availability(
        &self,
        challenge_id: ChallengeId,
        availability: ChallengeLocationAvailability,
    ) -> Result<()>;

    /// Get all locations where a challenge is available
    async fn get_location_availabilities(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengeLocationAvailability>>;

    /// Remove a location availability from a challenge
    async fn remove_location_availability(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()>;

    // -------------------------------------------------------------------------
    // Region Availability (AVAILABLE_AT_REGION edges)
    // -------------------------------------------------------------------------

    /// List challenges available at a specific region (via AVAILABLE_AT_REGION edge)
    async fn list_by_region(&self, region_id: RegionId) -> Result<Vec<Challenge>>;

    /// Add a region where this challenge is available (creates AVAILABLE_AT_REGION edge)
    async fn add_region_availability(
        &self,
        challenge_id: ChallengeId,
        availability: ChallengeRegionAvailability,
    ) -> Result<()>;

    /// Get all regions where a challenge is available
    async fn get_region_availabilities(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengeRegionAvailability>>;

    /// Remove a region availability from a challenge
    async fn remove_region_availability(
        &self,
        challenge_id: ChallengeId,
        region_id: RegionId,
    ) -> Result<()>;

    // -------------------------------------------------------------------------
    // Unlock Edges (ON_SUCCESS_UNLOCKS)
    // -------------------------------------------------------------------------

    /// Add a location that gets unlocked on successful challenge completion
    async fn add_unlock_location(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()>;

    /// Get locations that get unlocked when this challenge succeeds
    async fn get_unlock_locations(&self, challenge_id: ChallengeId) -> Result<Vec<LocationId>>;

    /// Remove an unlock from a challenge
    async fn remove_unlock_location(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()>;
}
