//! High-level WebSocket E2E test client.
//!
//! Provides a fluent API for WebSocket E2E testing, wrapping the low-level
//! `test_support` helpers with domain-aware methods.
//!
//! # Example
//!
//! ```ignore
//! let mut client = WsE2EClient::connect(addr).await?;
//! let snapshot = client.join_as_dm(world_id).await?;
//!
//! // Start a conversation
//! let conversation = client.start_conversation(npc_id, "Hello there!").await?;
//! let response = client.say("I need information about the dragon").await?;
//!
//! // Move to a new region
//! client.move_to_region(pc_id, region_id).await?;
//! ```

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use thiserror::Error;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage, MaybeTlsStream};
use uuid::Uuid;

use wrldbldr_domain::{CharacterId, PlayerCharacterId, RegionId, WorldId};
use wrldbldr_protocol::{
    ClientMessage, RequestPayload, ResponseResult, ServerMessage, WorldRole as ProtoWorldRole,
};

/// Error type for E2E client operations.
#[derive(Debug, Error)]
pub enum E2EError {
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Timeout waiting for message: {0}")]
    Timeout(&'static str),

    #[error("Unexpected message received: {0:?}")]
    UnexpectedMessage(ServerMessage),

    #[error("Server error: {code} - {message}")]
    ServerError { code: String, message: String },

    #[error("Join world failed: {0}")]
    JoinFailed(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result of joining a world.
#[derive(Debug, Clone)]
pub struct JoinedWorld {
    pub world_id: Uuid,
    pub snapshot: serde_json::Value,
    pub your_role: ProtoWorldRole,
    pub your_pc: Option<serde_json::Value>,
}

/// Result of starting a conversation.
#[derive(Debug, Clone)]
pub struct ConversationStarted {
    pub conversation_id: String,
    pub npc_id: String,
    pub npc_name: String,
    pub npc_disposition: Option<String>,
}

/// Result of NPC dialogue.
#[derive(Debug, Clone)]
pub struct DialogueResponse {
    pub speaker_id: String,
    pub speaker_name: String,
    pub text: String,
    pub conversation_id: Option<String>,
}

/// Result of entering a region.
#[derive(Debug, Clone)]
pub struct RegionEntered {
    pub region_id: String,
    pub region_name: String,
}

/// High-level WebSocket E2E test client.
///
/// Wraps a WebSocket connection and provides domain-aware methods
/// for testing game flows.
pub struct WsE2EClient {
    ws: tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    timeout: Duration,
    pending_messages: Vec<ServerMessage>,
}

impl WsE2EClient {
    /// Connect to a WebSocket server.
    pub async fn connect(addr: SocketAddr) -> Result<Self, E2EError> {
        let url = format!("ws://{}/ws", addr);
        let (ws, _resp) = connect_async(url)
            .await
            .map_err(|e| E2EError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            ws,
            timeout: Duration::from_secs(5),
            pending_messages: Vec::new(),
        })
    }

    /// Set the timeout for waiting for messages.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    // =========================================================================
    // Session Operations
    // =========================================================================

    /// Join a world as the DM (Dungeon Master).
    pub async fn join_as_dm(&mut self, world_id: WorldId) -> Result<JoinedWorld, E2EError> {
        self.send_client_message(&ClientMessage::JoinWorld {
            world_id: *world_id.as_uuid(),
            role: ProtoWorldRole::Dm,
            user_id: format!("test-dm-{}", Uuid::new_v4()),
            pc_id: None,
            spectate_pc_id: None,
        })
        .await?;

        self.expect_world_joined().await
    }

    /// Join a world as a player with a specific player character.
    pub async fn join_as_player(
        &mut self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
    ) -> Result<JoinedWorld, E2EError> {
        self.send_client_message(&ClientMessage::JoinWorld {
            world_id: *world_id.as_uuid(),
            role: ProtoWorldRole::Player,
            user_id: format!("test-player-{}", Uuid::new_v4()),
            pc_id: Some(*pc_id.as_uuid()),
            spectate_pc_id: None,
        })
        .await?;

        self.expect_world_joined().await
    }

    // =========================================================================
    // Conversation Operations
    // =========================================================================

    /// Start a conversation with an NPC.
    pub async fn start_conversation(
        &mut self,
        npc_id: CharacterId,
        message: &str,
    ) -> Result<ConversationStarted, E2EError> {
        self.send_client_message(&ClientMessage::StartConversation {
            npc_id: npc_id.to_string(),
            message: message.to_string(),
        })
        .await?;

        self.expect_conversation_started().await
    }

    /// Continue a conversation with an NPC.
    ///
    /// Returns the NPC's dialogue response.
    pub async fn continue_conversation(
        &mut self,
        npc_id: CharacterId,
        message: &str,
        conversation_id: Option<&str>,
    ) -> Result<DialogueResponse, E2EError> {
        self.send_client_message(&ClientMessage::ContinueConversation {
            npc_id: npc_id.to_string(),
            message: message.to_string(),
            conversation_id: conversation_id.map(String::from),
        })
        .await?;

        // Wait for dialogue response (may have thinking indicators first)
        self.expect_dialogue_response().await
    }

    // =========================================================================
    // Movement Operations
    // =========================================================================

    /// Move a player character to a different region within the same location.
    pub async fn move_to_region(
        &mut self,
        pc_id: PlayerCharacterId,
        region_id: RegionId,
    ) -> Result<(), E2EError> {
        self.send_client_message(&ClientMessage::MoveToRegion {
            pc_id: pc_id.to_string(),
            region_id: region_id.to_string(),
        })
        .await?;

        // Wait for either StagingPending or StagingReady
        let deadline = Instant::now() + self.timeout;

        while Instant::now() < deadline {
            let remaining = deadline - Instant::now();
            match self.recv_with_timeout(remaining).await {
                Ok(msg) => match &msg {
                    ServerMessage::StagingPending { .. } => {
                        // Store for later, wait for StagingReady
                        self.pending_messages.push(msg);
                    }
                    ServerMessage::StagingReady { .. } => {
                        return Ok(());
                    }
                    ServerMessage::Error { code, message } => {
                        return Err(E2EError::ServerError {
                            code: code.clone(),
                            message: message.clone(),
                        });
                    }
                    _ => {
                        self.pending_messages.push(msg);
                    }
                },
                Err(E2EError::Timeout(_)) => break,
                Err(e) => return Err(e),
            }
        }

        Err(E2EError::Timeout("StagingReady after move"))
    }

    /// Exit to a different location.
    pub async fn exit_to_location(
        &mut self,
        pc_id: PlayerCharacterId,
        location_id: wrldbldr_domain::LocationId,
        arrival_region_id: Option<RegionId>,
    ) -> Result<(), E2EError> {
        self.send_client_message(&ClientMessage::ExitToLocation {
            pc_id: pc_id.to_string(),
            location_id: location_id.to_string(),
            arrival_region_id: arrival_region_id.map(|r| r.to_string()),
        })
        .await?;

        // Similar to move_to_region, wait for staging to complete
        let deadline = Instant::now() + self.timeout;

        while Instant::now() < deadline {
            let remaining = deadline - Instant::now();
            match self.recv_with_timeout(remaining).await {
                Ok(msg) => match &msg {
                    ServerMessage::StagingPending { .. } => {
                        self.pending_messages.push(msg);
                    }
                    ServerMessage::StagingReady { .. } => {
                        return Ok(());
                    }
                    ServerMessage::Error { code, message } => {
                        return Err(E2EError::ServerError {
                            code: code.clone(),
                            message: message.clone(),
                        });
                    }
                    _ => {
                        self.pending_messages.push(msg);
                    }
                },
                Err(E2EError::Timeout(_)) => break,
                Err(e) => return Err(e),
            }
        }

        Err(E2EError::Timeout("StagingReady after location exit"))
    }

    // =========================================================================
    // Request/Response Operations
    // =========================================================================

    /// Send a request and wait for the response.
    ///
    /// This handles the Request/Response pattern with automatic request ID generation.
    pub async fn request(
        &mut self,
        payload: RequestPayload,
    ) -> Result<serde_json::Value, E2EError> {
        let request_id = Uuid::new_v4().to_string();

        self.send_client_message(&ClientMessage::Request {
            request_id: request_id.clone(),
            payload,
        })
        .await?;

        // Wait for matching response
        let deadline = Instant::now() + self.timeout;

        while Instant::now() < deadline {
            let remaining = deadline - Instant::now();
            match self.recv_with_timeout(remaining).await {
                Ok(msg) => match msg {
                    ServerMessage::Response {
                        request_id: rid,
                        result,
                    } if rid == request_id => {
                        return match result {
                            ResponseResult::Success { data } => Ok(data.unwrap_or_default()),
                            ResponseResult::Error { code, message, .. } => {
                                Err(E2EError::RequestFailed(format!("{:?}: {}", code, message)))
                            }
                            _ => Err(E2EError::RequestFailed("Unknown result type".to_string())),
                        };
                    }
                    _ => {
                        self.pending_messages.push(msg);
                    }
                },
                Err(E2EError::Timeout(_)) => break,
                Err(e) => return Err(e),
            }
        }

        Err(E2EError::Timeout("Response"))
    }

    // =========================================================================
    // Assertion Helpers
    // =========================================================================

    /// Wait for a message matching the predicate.
    ///
    /// Returns the first message that matches, storing non-matching messages
    /// for later retrieval.
    pub async fn expect_message<F>(&mut self, mut matcher: F) -> Result<ServerMessage, E2EError>
    where
        F: FnMut(&ServerMessage) -> bool,
    {
        // Check pending messages first
        for (i, msg) in self.pending_messages.iter().enumerate() {
            if matcher(msg) {
                return Ok(self.pending_messages.remove(i));
            }
        }

        // Wait for new messages
        let deadline = Instant::now() + self.timeout;

        while Instant::now() < deadline {
            let remaining = deadline - Instant::now();
            match self.recv_with_timeout(remaining).await {
                Ok(msg) => {
                    if matcher(&msg) {
                        return Ok(msg);
                    }
                    self.pending_messages.push(msg);
                }
                Err(E2EError::Timeout(_)) => break,
                Err(e) => return Err(e),
            }
        }

        Err(E2EError::Timeout("Expected message not received"))
    }

    /// Assert that no message matching the predicate is received within the wait duration.
    pub async fn expect_no_message<F>(&mut self, mut matcher: F, wait: Duration) -> Result<(), E2EError>
    where
        F: FnMut(&ServerMessage) -> bool,
    {
        let deadline = Instant::now() + wait;

        while Instant::now() < deadline {
            let remaining = deadline - Instant::now();
            match self.recv_with_timeout(remaining).await {
                Ok(msg) => {
                    if matcher(&msg) {
                        return Err(E2EError::UnexpectedMessage(msg));
                    }
                    self.pending_messages.push(msg);
                }
                Err(E2EError::Timeout(_)) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    /// Drain all pending messages that were received but not yet processed.
    pub fn drain_pending(&mut self) -> Vec<ServerMessage> {
        std::mem::take(&mut self.pending_messages)
    }

    /// Get the number of pending messages.
    pub fn pending_count(&self) -> usize {
        self.pending_messages.len()
    }

    // =========================================================================
    // Specific Message Expectations
    // =========================================================================

    async fn expect_world_joined(&mut self) -> Result<JoinedWorld, E2EError> {
        let msg = self
            .expect_message(|m| matches!(m, ServerMessage::WorldJoined { .. }))
            .await?;

        match msg {
            ServerMessage::WorldJoined {
                world_id,
                snapshot,
                your_role,
                your_pc,
                ..
            } => Ok(JoinedWorld {
                world_id,
                snapshot,
                your_role,
                your_pc,
            }),
            _ => unreachable!(),
        }
    }

    async fn expect_conversation_started(&mut self) -> Result<ConversationStarted, E2EError> {
        let msg = self
            .expect_message(|m| matches!(m, ServerMessage::ConversationStarted { .. }))
            .await?;

        match msg {
            ServerMessage::ConversationStarted {
                conversation_id,
                npc_id,
                npc_name,
                npc_disposition,
            } => Ok(ConversationStarted {
                conversation_id,
                npc_id,
                npc_name,
                npc_disposition,
            }),
            _ => unreachable!(),
        }
    }

    async fn expect_dialogue_response(&mut self) -> Result<DialogueResponse, E2EError> {
        let msg = self
            .expect_message(|m| matches!(m, ServerMessage::DialogueResponse { .. }))
            .await?;

        match msg {
            ServerMessage::DialogueResponse {
                speaker_id,
                speaker_name,
                text,
                conversation_id,
                ..
            } => Ok(DialogueResponse {
                speaker_id,
                speaker_name,
                text,
                conversation_id,
            }),
            _ => unreachable!(),
        }
    }

    // =========================================================================
    // Low-level Helpers
    // =========================================================================

    async fn send_client_message(&mut self, msg: &ClientMessage) -> Result<(), E2EError> {
        let json = serde_json::to_string(msg)?;
        self.ws
            .send(WsMessage::Text(json.into()))
            .await
            .map_err(|e| E2EError::SendFailed(e.to_string()))
    }

    async fn recv_with_timeout(&mut self, timeout_duration: Duration) -> Result<ServerMessage, E2EError> {
        match timeout(timeout_duration, self.recv_server_message()).await {
            Ok(result) => result,
            Err(_) => Err(E2EError::Timeout("receive")),
        }
    }

    async fn recv_server_message(&mut self) -> Result<ServerMessage, E2EError> {
        loop {
            let msg = self
                .ws
                .next()
                .await
                .ok_or_else(|| E2EError::ReceiveFailed("Connection closed".to_string()))?
                .map_err(|e| E2EError::ReceiveFailed(e.to_string()))?;

            match msg {
                WsMessage::Text(text) => {
                    return serde_json::from_str::<ServerMessage>(&text)
                        .map_err(E2EError::from);
                }
                WsMessage::Binary(bin) => {
                    let text = String::from_utf8(bin)
                        .map_err(|e| E2EError::ReceiveFailed(e.to_string()))?;
                    return serde_json::from_str::<ServerMessage>(&text)
                        .map_err(E2EError::from);
                }
                WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Close(_) => {
                    // Skip control frames
                }
                WsMessage::Frame(_) => {
                    // Skip raw frames
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2e_error_display() {
        let err = E2EError::Timeout("test operation");
        assert!(err.to_string().contains("Timeout"));

        let err = E2EError::ServerError {
            code: "NOT_FOUND".to_string(),
            message: "Character not found".to_string(),
        };
        assert!(err.to_string().contains("NOT_FOUND"));
        assert!(err.to_string().contains("Character not found"));
    }
}
