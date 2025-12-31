//! Character-Location relationship operations.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::value_objects::{RegionRelationship, RegionShift};
use wrldbldr_domain::{Character, CharacterId, FrequencyLevel, LocationId, RegionId};

/// Character-Location relationship operations.
///
/// This trait covers:
/// - Home and work location management
/// - Frequented and avoided locations
/// - NPC presence queries based on time
/// - Character-Region relationships
///
/// # Used By
/// - `LocationServiceImpl` - For NPC presence queries
/// - `CharacterServiceImpl` - For location relationship management
/// - `StagingServiceImpl` - For determining NPC availability
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait CharacterLocationPort: Send + Sync {
    /// Set character's home location
    async fn set_home_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        description: Option<String>,
    ) -> Result<()>;

    /// Remove character's home location
    async fn remove_home_location(&self, character_id: CharacterId) -> Result<()>;

    /// Set character's work location
    async fn set_work_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        role: String,
        schedule: Option<String>,
    ) -> Result<()>;

    /// Remove character's work location
    async fn remove_work_location(&self, character_id: CharacterId) -> Result<()>;

    /// Add a frequented location
    async fn add_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        frequency: FrequencyLevel,
        time_of_day: String,
        day_of_week: Option<String>,
        reason: Option<String>,
    ) -> Result<()>;

    /// Remove a frequented location
    async fn remove_frequented_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()>;

    /// Add an avoided location
    async fn add_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
        reason: String,
    ) -> Result<()>;

    /// Remove an avoided location
    async fn remove_avoided_location(
        &self,
        character_id: CharacterId,
        location_id: LocationId,
    ) -> Result<()>;

    /// Get NPCs who might be at a location (based on home, work, frequents)
    async fn get_npcs_at_location(
        &self,
        location_id: LocationId,
        time_of_day: Option<String>,
    ) -> Result<Vec<Character>>;

    /// Get all region relationships for a character
    async fn get_region_relationships(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<RegionRelationship>>;

    /// Set character's home region (creates/replaces HOME_REGION edge)
    async fn set_home_region(&self, character_id: CharacterId, region_id: RegionId) -> Result<()>;

    /// Set character's work region with shift (creates/replaces WORKS_AT_REGION edge)
    async fn set_work_region(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        shift: RegionShift,
    ) -> Result<()>;

    /// Remove a specific region relationship by type
    ///
    /// relationship_type should be one of: "home", "work", "frequents", "avoids"
    async fn remove_region_relationship(
        &self,
        character_id: CharacterId,
        region_id: RegionId,
        relationship_type: String,
    ) -> Result<()>;
}
