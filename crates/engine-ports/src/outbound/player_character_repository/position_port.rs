//! Position/movement operations for PlayerCharacter entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId};

/// Position and movement operations for player characters.
///
/// This trait covers updating the spatial position of player characters,
/// including location and region changes.
#[async_trait]
pub trait PlayerCharacterPositionPort: Send + Sync {
    /// Update a player character's location (clears region)
    async fn update_location(&self, id: PlayerCharacterId, location_id: LocationId) -> Result<()>;

    /// Update a player character's region (within current location)
    async fn update_region(&self, id: PlayerCharacterId, region_id: RegionId) -> Result<()>;

    /// Update both location and region at once
    async fn update_position(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
        region_id: Option<RegionId>,
    ) -> Result<()>;
}
