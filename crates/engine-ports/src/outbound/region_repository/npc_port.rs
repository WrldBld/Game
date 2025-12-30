//! Region NPC port for querying NPC relationships to regions.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::ids::RegionId;
use wrldbldr_domain::value_objects::RegionRelationshipType;
use wrldbldr_domain::Character;

/// Port for querying NPC relationships to regions.
///
/// This trait handles queries for NPCs that have relationships to regions,
/// used for determining NPC presence and availability in a region.
///
/// # Used By
/// - `PresenceService` - For determining which NPCs are in a region
/// - `NpcService` - For NPC location lookups
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait RegionNpcPort: Send + Sync {
    /// Get all NPCs with relationships to a region.
    ///
    /// Returns NPCs linked to the region via various relationship types
    /// (e.g., FREQUENTS, WORKS_AT, LIVES_IN) along with the relationship type.
    /// Used for presence determination and NPC availability checks.
    async fn get_npcs_related_to_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>>;
}
