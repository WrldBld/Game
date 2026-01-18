//! Pre-built test scenarios for common game flows.
//!
//! These scenario builders make it easy to set up common test situations
//! without having to manually orchestrate all the steps.
//!
//! # Example
//!
//! ```ignore
//! // Quick conversation test
//! let mut scenario = ConversationScenario::setup(addr, world_id, pc_id).await?;
//! scenario.talk_to(npc_id).await?;
//! let response = scenario.say("Tell me about the dragon").await?;
//! assert!(response.contains("dragon"));
//! ```

use std::net::SocketAddr;

use wrldbldr_domain::{CharacterId, PlayerCharacterId, RegionId, WorldId};

use super::e2e_client::{
    ConversationStarted, DialogueResponse, E2EError, JoinedWorld, WsE2EClient,
};

// =============================================================================
// Conversation Scenario
// =============================================================================

/// Pre-built scenario for testing NPC conversations.
///
/// Handles joining the world and provides a fluent API for conversation flows.
pub struct ConversationScenario {
    client: WsE2EClient,
    #[allow(dead_code)]
    world_id: WorldId,
    pc_id: PlayerCharacterId,
    current_conversation: Option<ConversationStarted>,
}

impl ConversationScenario {
    /// Set up a conversation scenario by joining as a player.
    pub async fn setup(
        addr: SocketAddr,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
    ) -> Result<Self, E2EError> {
        let mut client = WsE2EClient::connect(addr).await?;
        let _joined = client.join_as_player(world_id, pc_id).await?;

        Ok(Self {
            client,
            world_id,
            pc_id,
            current_conversation: None,
        })
    }

    /// Start a conversation with an NPC.
    pub async fn talk_to(&mut self, npc_id: CharacterId) -> Result<&ConversationStarted, E2EError> {
        let conversation = self.client.start_conversation(npc_id, "").await?;
        self.current_conversation = Some(conversation);
        Ok(self.current_conversation.as_ref().unwrap())
    }

    /// Start a conversation with an NPC with an opening message.
    pub async fn talk_to_with_message(
        &mut self,
        npc_id: CharacterId,
        message: &str,
    ) -> Result<&ConversationStarted, E2EError> {
        let conversation = self.client.start_conversation(npc_id, message).await?;
        self.current_conversation = Some(conversation);
        Ok(self.current_conversation.as_ref().unwrap())
    }

    /// Say something to the current NPC.
    ///
    /// Returns the NPC's response text.
    pub async fn say(&mut self, text: &str) -> Result<String, E2EError> {
        let conversation = self
            .current_conversation
            .as_ref()
            .ok_or_else(|| E2EError::RequestFailed("No active conversation".to_string()))?;

        let npc_uuid: uuid::Uuid = conversation
            .npc_id
            .parse()
            .map_err(|_| E2EError::RequestFailed("Invalid NPC ID in conversation".to_string()))?;

        let response = self
            .client
            .continue_conversation(
                CharacterId::from(npc_uuid),
                text,
                Some(&conversation.conversation_id),
            )
            .await?;

        Ok(response.text)
    }

    /// Get the full dialogue response from the NPC.
    pub async fn say_and_get_full_response(
        &mut self,
        text: &str,
    ) -> Result<DialogueResponse, E2EError> {
        let conversation = self
            .current_conversation
            .as_ref()
            .ok_or_else(|| E2EError::RequestFailed("No active conversation".to_string()))?;

        let npc_uuid: uuid::Uuid = conversation
            .npc_id
            .parse()
            .map_err(|_| E2EError::RequestFailed("Invalid NPC ID in conversation".to_string()))?;

        self.client
            .continue_conversation(
                CharacterId::from(npc_uuid),
                text,
                Some(&conversation.conversation_id),
            )
            .await
    }

    /// Get the current conversation state.
    pub fn current_conversation(&self) -> Option<&ConversationStarted> {
        self.current_conversation.as_ref()
    }

    /// Get a mutable reference to the underlying client for advanced operations.
    pub fn client_mut(&mut self) -> &mut WsE2EClient {
        &mut self.client
    }

    /// Get the player character ID.
    pub fn pc_id(&self) -> PlayerCharacterId {
        self.pc_id
    }
}

// =============================================================================
// DM Scenario
// =============================================================================

/// Pre-built scenario for testing DM (Dungeon Master) operations.
///
/// Handles joining the world as DM and provides access to DM-specific flows.
pub struct DmScenario {
    client: WsE2EClient,
    world_id: WorldId,
    #[allow(dead_code)]
    joined: JoinedWorld,
}

impl DmScenario {
    /// Set up a DM scenario by joining as the Dungeon Master.
    pub async fn setup(addr: SocketAddr, world_id: WorldId) -> Result<Self, E2EError> {
        let mut client = WsE2EClient::connect(addr).await?;
        let joined = client.join_as_dm(world_id).await?;

        Ok(Self {
            client,
            world_id,
            joined,
        })
    }

    /// Get the world ID.
    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    /// Get the world snapshot from when we joined.
    pub fn snapshot(&self) -> &serde_json::Value {
        &self.joined.snapshot
    }

    /// Get a mutable reference to the underlying client for advanced operations.
    pub fn client_mut(&mut self) -> &mut WsE2EClient {
        &mut self.client
    }
}

// =============================================================================
// Movement Scenario
// =============================================================================

/// Pre-built scenario for testing player movement between regions/locations.
pub struct MovementScenario {
    client: WsE2EClient,
    #[allow(dead_code)]
    world_id: WorldId,
    pc_id: PlayerCharacterId,
}

impl MovementScenario {
    /// Set up a movement scenario by joining as a player.
    pub async fn setup(
        addr: SocketAddr,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
    ) -> Result<Self, E2EError> {
        let mut client = WsE2EClient::connect(addr).await?;
        let _joined = client.join_as_player(world_id, pc_id).await?;

        Ok(Self {
            client,
            world_id,
            pc_id,
        })
    }

    /// Move to a different region within the same location.
    pub async fn move_to(&mut self, region_id: RegionId) -> Result<(), E2EError> {
        self.client.move_to_region(self.pc_id, region_id).await
    }

    /// Exit to a different location.
    pub async fn exit_to(
        &mut self,
        location_id: wrldbldr_domain::LocationId,
        arrival_region: Option<RegionId>,
    ) -> Result<(), E2EError> {
        self.client
            .exit_to_location(self.pc_id, location_id, arrival_region)
            .await
    }

    /// Get the player character ID.
    pub fn pc_id(&self) -> PlayerCharacterId {
        self.pc_id
    }

    /// Get a mutable reference to the underlying client for advanced operations.
    pub fn client_mut(&mut self) -> &mut WsE2EClient {
        &mut self.client
    }
}

// =============================================================================
// Multi-Client Scenario
// =============================================================================

/// Pre-built scenario for testing multiple clients in the same world.
///
/// Useful for testing multiplayer interactions and DM approval flows.
pub struct MultiClientScenario {
    pub dm: WsE2EClient,
    pub players: Vec<WsE2EClient>,
    world_id: WorldId,
}

impl MultiClientScenario {
    /// Set up a multi-client scenario with one DM and multiple players.
    pub async fn setup(
        addr: SocketAddr,
        world_id: WorldId,
        player_pcs: Vec<PlayerCharacterId>,
    ) -> Result<Self, E2EError> {
        // Connect DM first
        let mut dm = WsE2EClient::connect(addr).await?;
        dm.join_as_dm(world_id).await?;

        // Connect players
        let mut players = Vec::new();
        for pc_id in player_pcs {
            let mut player = WsE2EClient::connect(addr).await?;
            player.join_as_player(world_id, pc_id).await?;
            players.push(player);
        }

        Ok(Self {
            dm,
            players,
            world_id,
        })
    }

    /// Get the world ID.
    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    /// Get mutable reference to a specific player client.
    pub fn player_mut(&mut self, index: usize) -> Option<&mut WsE2EClient> {
        self.players.get_mut(index)
    }

    /// Get mutable reference to the DM client.
    pub fn dm_mut(&mut self) -> &mut WsE2EClient {
        &mut self.dm
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would go here, but they require a running server
    // See ws_integration_tests/ for actual tests using these scenarios
}
