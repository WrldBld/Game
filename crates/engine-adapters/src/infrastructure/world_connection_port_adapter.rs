//! Adapter implementing WorldConnectionPort using WorldConnectionManager

use std::sync::Arc;
use async_trait::async_trait;
use wrldbldr_domain::{WorldId, CharacterId};
use wrldbldr_engine_ports::outbound::{WorldConnectionPort, WorldConnectionError};
use wrldbldr_protocol::ServerMessage;
use crate::infrastructure::{WorldConnectionManager, BroadcastError};

/// Adapter implementing WorldConnectionPort using WorldConnectionManager
pub struct WorldConnectionPortAdapter {
    manager: Arc<WorldConnectionManager>,
}

impl WorldConnectionPortAdapter {
    pub fn new(manager: Arc<WorldConnectionManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl WorldConnectionPort for WorldConnectionPortAdapter {
    async fn broadcast_to_world(
        &self,
        world_id: &WorldId,
        message: ServerMessage,
    ) -> Result<(), WorldConnectionError> {
        // The underlying broadcast_to_world doesn't return an error, so we just call it
        // It silently ignores if the world doesn't exist
        self.manager.broadcast_to_world(*world_id.as_uuid(), message).await;
        Ok(())
    }
    
    async fn broadcast_to_world_except(
        &self,
        world_id: &WorldId,
        exclude_user_id: &str,
        message: ServerMessage,
    ) -> Result<(), WorldConnectionError> {
        self.manager.broadcast_to_world_except(world_id.as_uuid(), exclude_user_id, message).await
            .map_err(|e| match e {
                BroadcastError::WorldNotFound(id) => WorldConnectionError::WorldNotFound(WorldId::from(id)),
                BroadcastError::UserNotFound(user) => WorldConnectionError::UserNotFound(user),
                _ => WorldConnectionError::WorldNotFound(*world_id),
            })
    }
    
    async fn send_to_dm(
        &self,
        world_id: &WorldId,
        message: ServerMessage,
    ) -> Result<(), WorldConnectionError> {
        self.manager.send_to_dm(world_id.as_uuid(), message).await
            .map_err(|e| match e {
                BroadcastError::DmNotConnected(_) => WorldConnectionError::DmNotConnected,
                BroadcastError::WorldNotFound(id) => WorldConnectionError::WorldNotFound(WorldId::from(id)),
                _ => WorldConnectionError::DmNotConnected,
            })
    }
    
    async fn send_to_user(
        &self,
        world_id: &WorldId,
        user_id: &str,
        message: ServerMessage,
    ) -> Result<(), WorldConnectionError> {
        self.manager.send_to_user_in_world(world_id.as_uuid(), user_id, message).await
            .map_err(|e| match e {
                BroadcastError::UserNotFound(user) => WorldConnectionError::UserNotFound(user),
                BroadcastError::WorldNotFound(id) => WorldConnectionError::WorldNotFound(WorldId::from(id)),
                _ => WorldConnectionError::UserNotFound(user_id.to_string()),
            })
    }
    
    async fn send_to_player(
        &self,
        world_id: &WorldId,
        pc_id: &CharacterId,
        message: ServerMessage,
    ) -> Result<(), WorldConnectionError> {
        self.manager.send_to_player(world_id.as_uuid(), pc_id.as_uuid(), message).await
            .map_err(|e| match e {
                BroadcastError::PlayerNotFound(_) => WorldConnectionError::PlayerNotFound,
                BroadcastError::WorldNotFound(id) => WorldConnectionError::WorldNotFound(WorldId::from(id)),
                _ => WorldConnectionError::PlayerNotFound,
            })
    }
    
    fn has_dm(&self, world_id: &WorldId) -> bool {
        self.manager.has_dm(world_id.as_uuid())
    }
    
    fn get_dm_user_id(&self, world_id: &WorldId) -> Option<String> {
        // get_dm_info is async, but we need sync here
        // We'll use try_read instead via the has_dm pattern
        // For now, we'll need to make this async-aware or block
        // The safest approach is to use tokio::runtime::Handle::current().block_on
        // but that's not ideal. Let me check if we can restructure.
        
        // Actually, looking at the WorldConnectionManager, get_dm_info is async
        // but we need this to be sync. We'll need to spawn a blocking task or
        // make the trait method async. Since the spec says this should be sync,
        // let's use block_on cautiously.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.manager.get_dm_info(world_id.as_uuid()).await.map(|info| info.user_id)
            })
        })
    }
    
    fn find_player_for_pc(
        &self,
        world_id: &WorldId,
        pc_id: &CharacterId,
    ) -> Option<String> {
        // Same issue as get_dm_user_id - underlying method is async but trait requires sync
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.manager.find_player_for_pc(world_id.as_uuid(), pc_id.as_uuid()).await
            })
        })
    }
}
