//! World Connection Manager ports following Interface Segregation Principle.
//!
//! # Architecture
//!
//! The connection manager functionality is split into 4 focused traits:
//!
//! 1. `ConnectionQueryPort` - Query connection state (8 methods)
//! 2. `ConnectionContextPort` - Resolve client/connection context (4 methods)
//! 3. `ConnectionBroadcastPort` - Broadcast messages (4 methods)
//! 4. `ConnectionLifecyclePort` - Connection lifecycle (1 method)
//!
//! # Clean ISP Design
//!
//! Services should depend only on the traits they actually need:
//! - Queue workers: `ConnectionQueryPort` + `ConnectionBroadcastPort`
//! - WebSocket handlers: `ConnectionContextPort`
//! - Event subscribers: `ConnectionQueryPort` + `ConnectionBroadcastPort`
//! - Cleanup tasks: `ConnectionLifecyclePort`
//!
//! # Supporting Types
//!
//! This module also exports the DTOs used by these traits:
//! - `DmInfo` - Information about a connected DM
//! - `ConnectedUserInfo` - Information about a connected user
//! - `ConnectionStats` - Connection statistics
//! - `ConnectionContext` - Full connection context for handlers
//! - `ConnectionManagerError` - Error types

mod broadcast_port;
mod context_port;
mod lifecycle_port;
mod query_port;
mod types;

pub use broadcast_port::ConnectionBroadcastPort;
pub use context_port::ConnectionContextPort;
pub use lifecycle_port::ConnectionLifecyclePort;
pub use query_port::ConnectionQueryPort;
pub use types::{
    ConnectedUserInfo, ConnectionContext, ConnectionManagerError, ConnectionStats, DmInfo,
};

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use crate::outbound::use_case_types::WorldRole;
    use async_trait::async_trait;
    use mockall::mock;
    use uuid::Uuid;
    use wrldbldr_domain::WorldId;

    mock! {
        /// Mock implementation of all connection manager traits for testing.
        pub WorldConnectionManager {}

        #[async_trait]
        impl ConnectionQueryPort for WorldConnectionManager {
            async fn has_dm(&self, world_id: &WorldId) -> bool;
            async fn get_dm_info(&self, world_id: &WorldId) -> Option<DmInfo>;
            async fn get_connected_users(&self, world_id: WorldId) -> Vec<ConnectedUserInfo>;
            async fn get_user_role(&self, world_id: &WorldId, user_id: &str) -> Option<WorldRole>;
            async fn find_player_for_pc(&self, world_id: &WorldId, pc_id: &Uuid) -> Option<String>;
            async fn get_world_pcs(&self, world_id: &WorldId) -> Vec<(Uuid, String)>;
            async fn get_all_world_ids(&self) -> Vec<Uuid>;
            async fn stats(&self) -> ConnectionStats;
        }

        #[async_trait]
        impl ConnectionContextPort for WorldConnectionManager {
            async fn get_user_id_by_client_id(&self, client_id: &str) -> Option<String>;
            async fn is_dm_by_client_id(&self, client_id: &str) -> bool;
            async fn get_world_id_by_client_id(&self, client_id: &str) -> Option<Uuid>;
            async fn is_spectator_by_client_id(&self, client_id: &str) -> bool;
            async fn get_connection_context(&self, connection_id: Uuid) -> Option<ConnectionContext>;
            async fn get_connection_by_client_id(&self, client_id: &str) -> Option<ConnectionContext>;
            async fn get_pc_id_by_client_id(&self, client_id: &str) -> Option<Uuid>;
        }

        #[async_trait]
        impl ConnectionBroadcastPort for WorldConnectionManager {
            async fn broadcast_to_world(&self, world_id: Uuid, message: serde_json::Value);
            async fn broadcast_to_dms(&self, world_id: Uuid, message: serde_json::Value);
            async fn broadcast_to_players(&self, world_id: Uuid, message: serde_json::Value);
            async fn broadcast_to_all_worlds(&self, message: serde_json::Value);
        }

        #[async_trait]
        impl ConnectionLifecyclePort for WorldConnectionManager {
            async fn unregister_connection(&self, connection_id: Uuid);
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockWorldConnectionManager;
