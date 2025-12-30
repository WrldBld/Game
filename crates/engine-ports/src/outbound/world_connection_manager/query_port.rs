//! Query operations for connection state.

use async_trait::async_trait;
use uuid::Uuid;

use crate::outbound::use_case_types::WorldRole;
use wrldbldr_domain::WorldId;

use super::{ConnectedUserInfo, ConnectionStats, DmInfo};

/// Query operations for connection state.
///
/// This trait provides read-only access to connection information:
/// - DM presence and info
/// - Connected users list
/// - User roles
/// - PC-to-player mapping
/// - Statistics
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait ConnectionQueryPort: Send + Sync {
    /// Check if a DM is connected to the specified world
    async fn has_dm(&self, world_id: &WorldId) -> bool;

    /// Get information about the DM in a world
    ///
    /// Returns `None` if no DM is connected.
    async fn get_dm_info(&self, world_id: &WorldId) -> Option<DmInfo>;

    /// Get all connected users in a world
    async fn get_connected_users(&self, world_id: WorldId) -> Vec<ConnectedUserInfo>;

    /// Get a user's role in a world
    ///
    /// Returns `None` if the user is not in the world.
    async fn get_user_role(&self, world_id: &WorldId, user_id: &str) -> Option<WorldRole>;

    /// Find which user is playing a specific PC
    ///
    /// Returns the user ID if a player is controlling the PC.
    async fn find_player_for_pc(&self, world_id: &WorldId, pc_id: &Uuid) -> Option<String>;

    /// Get all PCs in a world with their controlling users
    ///
    /// Returns a list of (pc_id, user_id) pairs.
    async fn get_world_pcs(&self, world_id: &WorldId) -> Vec<(Uuid, String)>;

    /// Get all world IDs that have active connections
    async fn get_all_world_ids(&self) -> Vec<Uuid>;

    /// Get connection statistics
    async fn stats(&self) -> ConnectionStats;
}
