//! Navigation Port - Handles player movement operations
//!
//! This trait defines operations for moving the player character
//! between regions within a location and between different locations.

use crate::outbound::GameConnectionPort;

/// Port for player navigation operations
///
/// Handles movement between regions and locations in the game world.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait NavigationPort: Send + Sync {
    /// Move PC to a different region within the same location
    fn move_to_region(&self, pc_id: &str, region_id: &str) -> anyhow::Result<()>;

    /// Exit to a different location
    ///
    /// # Arguments
    /// * `pc_id` - The player character ID
    /// * `location_id` - The target location ID
    /// * `arrival_region_id` - Optional specific region to arrive at within the location
    fn exit_to_location(
        &self,
        pc_id: &str,
        location_id: &str,
        arrival_region_id: Option<String>,
    ) -> anyhow::Result<()>;
}

// =============================================================================
// Blanket implementation: GameConnectionPort -> NavigationPort
// =============================================================================

/// Blanket implementation allowing any `GameConnectionPort` to be used as `NavigationPort`
impl<T: GameConnectionPort + ?Sized> NavigationPort for T {
    fn move_to_region(&self, pc_id: &str, region_id: &str) -> anyhow::Result<()> {
        GameConnectionPort::move_to_region(self, pc_id, region_id)
    }

    fn exit_to_location(
        &self,
        pc_id: &str,
        location_id: &str,
        arrival_region_id: Option<String>,
    ) -> anyhow::Result<()> {
        GameConnectionPort::exit_to_location(self, pc_id, location_id, arrival_region_id.as_deref())
    }
}
