//! Desktop WebSocket client using tokio-tungstenite

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use wrldbldr_protocol::{ClientMessage, ParticipantRole, RequestError, RequestPayload, ResponseResult, ServerMessage};

use crate::infrastructure::session_type_converters::participant_role_to_world_role;
use crate::infrastructure::websocket::protocol::ConnectionState;

/// WebSocket client for communicating with the Engine (Desktop)
pub struct EngineClient {
    url: String,
    state: Arc<RwLock<ConnectionState>>,
    tx: Arc<Mutex<Option<mpsc::Sender<ClientMessage>>>>,
    on_message: Arc<Mutex<Option<Box<dyn Fn(ServerMessage) + Send + Sync>>>>,
    on_state_change: Arc<Mutex<Option<Box<dyn Fn(ConnectionState) + Send + Sync>>>>,
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<ResponseResult>>>>,
}

impl EngineClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            tx: Arc::new(Mutex::new(None)),
            on_message: Arc::new(Mutex::new(None)),
            on_state_change: Arc::new(Mutex::new(None)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the URL this client is configured for
    pub fn url(&self) -> &str {
        &self.url
    }

    pub async fn set_on_message<F>(&self, callback: F)
    where
        F: Fn(ServerMessage) + Send + Sync + 'static,
    {
        let mut on_message = self.on_message.lock().await;
        *on_message = Some(Box::new(callback));
    }

    pub async fn set_on_state_change<F>(&self, callback: F)
    where
        F: Fn(ConnectionState) + Send + Sync + 'static,
    {
        let mut on_state_change = self.on_state_change.lock().await;
        *on_state_change = Some(Box::new(callback));
    }

    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    async fn set_state(&self, new_state: ConnectionState) {
        {
            let mut state = self.state.write().await;
            *state = new_state;
        }

        let callback = self.on_state_change.lock().await;
        if let Some(ref cb) = *callback {
            cb(new_state);
        }
    }

    pub async fn connect(&self) -> Result<()> {
        self.set_state(ConnectionState::Connecting).await;

        match connect_async(&self.url).await {
            Ok((ws_stream, _)) => {
                tracing::info!("Connected to Engine at {}", self.url);
                self.set_state(ConnectionState::Connected).await;

                let (mut write, mut read) = ws_stream.split();

                let (tx, mut rx) = mpsc::channel::<ClientMessage>(32);
                {
                    let mut tx_lock = self.tx.lock().await;
                    *tx_lock = Some(tx);
                }

                let on_message = Arc::clone(&self.on_message);
                let state = Arc::clone(&self.state);
                let pending_requests_clone = Arc::clone(&self.pending_requests);

                let read_handle = tokio::spawn(async move {
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                match serde_json::from_str::<ServerMessage>(&text) {
                                    Ok(server_msg) => {
                                        // Check if it's a Response and resolve pending request
                                        if let ServerMessage::Response { request_id, result } = &server_msg {
                                            let mut pending = pending_requests_clone.lock().await;
                                            if let Some(tx) = pending.remove(request_id) {
                                                let _ = tx.send(result.clone());
                                                continue; // Don't pass to callback
                                            }
                                        }

                                        let callback = on_message.lock().await;
                                        if let Some(ref cb) = *callback {
                                            cb(server_msg);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to parse server message: {}", e);
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => {
                                tracing::info!("Server closed connection");
                                break;
                            }
                            Ok(Message::Ping(_data)) => {}
                            Err(e) => {
                                tracing::error!("WebSocket error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }

                    let mut s = state.write().await;
                    *s = ConnectionState::Disconnected;
                });

                let write_handle = tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        let json = match serde_json::to_string(&msg) {
                            Ok(j) => j,
                            Err(e) => {
                                tracing::error!("Failed to serialize WebSocket message: {}", e);
                                continue;
                            }
                        };
                        if let Err(e) = write.send(Message::Text(json)).await {
                            tracing::error!("Failed to send message: {}", e);
                            break;
                        }
                    }
                });

                tokio::select! {
                    _ = read_handle => {
                        tracing::info!("Read task completed");
                    }
                    _ = write_handle => {
                        tracing::info!("Write task completed");
                    }
                }

                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to connect to Engine: {}", e);
                self.set_state(ConnectionState::Failed).await;
                Err(e.into())
            }
        }
    }

    pub async fn send(&self, message: ClientMessage) -> Result<()> {
        // Clone the sender to avoid holding the lock across await
        let tx = {
            let tx_lock = self.tx.lock().await;
            tx_lock.clone()
        };
        if let Some(tx) = tx {
            tx.send(message).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not connected"))
        }
    }

    pub async fn join_world(
        &self,
        world_id: &str,
        _user_id: &str,
        role: ParticipantRole,
    ) -> Result<()> {
        let world_id = uuid::Uuid::parse_str(world_id)?;
        let world_role = participant_role_to_world_role(role);

        self.send(ClientMessage::JoinWorld {
            world_id,
            role: world_role,
            pc_id: None, // PC selection happens after joining
            spectate_pc_id: None,
        })
        .await
    }

    pub async fn send_action(
        &self,
        action_type: &str,
        target: Option<&str>,
        dialogue: Option<&str>,
    ) -> Result<()> {
        self.send(ClientMessage::PlayerAction {
            action_type: action_type.to_string(),
            target: target.map(|s| s.to_string()),
            dialogue: dialogue.map(|s| s.to_string()),
        })
        .await
    }

    pub async fn heartbeat(&self) -> Result<()> {
        self.send(ClientMessage::Heartbeat).await
    }

    /// Send a request and await the response
    pub async fn request(&self, payload: RequestPayload) -> Result<ResponseResult, RequestError> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        
        // Register pending request
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), tx);
        }
        
        // Send the message
        let msg = ClientMessage::Request { 
            request_id: request_id.clone(), 
            payload 
        };
        
        self.send(msg).await.map_err(|e| RequestError::SendFailed(e.to_string()))?;
        
        // Await response
        rx.await.map_err(|_| RequestError::Cancelled)
    }

    /// Send a request with a timeout
    pub async fn request_with_timeout(&self, payload: RequestPayload, timeout_ms: u64) -> Result<ResponseResult, RequestError> {
        tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            self.request(payload)
        ).await.map_err(|_| RequestError::Timeout)?
    }

    pub async fn disconnect(&self) {
        // Clear pending requests - dropping senders causes Cancelled errors for waiters
        {
            let mut pending = self.pending_requests.lock().await;
            let count = pending.len();
            pending.clear();
            if count > 0 {
                tracing::debug!("Cleared {} pending requests on disconnect", count);
            }
        }
        {
            let mut tx_lock = self.tx.lock().await;
            *tx_lock = None;
        }
        self.set_state(ConnectionState::Disconnected).await;
    }
}

impl Clone for EngineClient {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            state: Arc::clone(&self.state),
            tx: Arc::clone(&self.tx),
            on_message: Arc::clone(&self.on_message),
            on_state_change: Arc::clone(&self.on_state_change),
            pending_requests: Arc::clone(&self.pending_requests),
        }
    }
}
