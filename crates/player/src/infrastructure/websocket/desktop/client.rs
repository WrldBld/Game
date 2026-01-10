//! Desktop WebSocket client using tokio-tungstenite

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use wrldbldr_protocol::{ClientMessage, ServerMessage};

use crate::infrastructure::messaging::ConnectionState;
use crate::infrastructure::websocket::shared::{parse_server_message, ParsedServerMessage};
use crate::infrastructure::websocket::{BackoffState, PendingRequests};

/// WebSocket client for communicating with the Engine (Desktop)
pub struct EngineClient {
    url: String,
    state: Arc<RwLock<ConnectionState>>,
    tx: Arc<Mutex<Option<mpsc::Sender<ClientMessage>>>>,
    on_message: Arc<Mutex<Option<Box<dyn Fn(ServerMessage) + Send + Sync>>>>,
    on_state_change: Arc<Mutex<Option<Box<dyn Fn(ConnectionState) + Send + Sync>>>>,
    pending_requests: Arc<Mutex<PendingRequests>>,
    /// Flag to track if disconnect was intentional (vs unexpected close)
    intentional_disconnect: Arc<RwLock<bool>>,
}

impl EngineClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            tx: Arc::new(Mutex::new(None)),
            on_message: Arc::new(Mutex::new(None)),
            on_state_change: Arc::new(Mutex::new(None)),
            pending_requests: Arc::new(Mutex::new(PendingRequests::default())),
            intentional_disconnect: Arc::new(RwLock::new(false)),
        }
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

    /// Internal connect logic - returns whether connection closed unexpectedly
    async fn connect_internal(&self) -> Result<bool> {
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
                let on_state_change = Arc::clone(&self.on_state_change);
                let pending_requests_clone = Arc::clone(&self.pending_requests);
                let intentional_disconnect = Arc::clone(&self.intentional_disconnect);

                let read_handle = tokio::spawn(async move {
                    let mut unexpected_close = false;
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => match parse_server_message(&text) {
                                Ok(ParsedServerMessage::Response { request_id, result }) => {
                                    let _ = pending_requests_clone
                                        .lock()
                                        .await
                                        .resolve(&request_id, result);
                                }
                                Ok(ParsedServerMessage::Other(server_msg)) => {
                                    let callback = on_message.lock().await;
                                    if let Some(ref cb) = *callback {
                                        cb(server_msg);
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to parse server message: {}", e);
                                }
                            },
                            Ok(Message::Close(_)) => {
                                tracing::info!("Server closed connection");
                                // Check if this was intentional
                                let intentional = *intentional_disconnect.read().await;
                                unexpected_close = !intentional;
                                break;
                            }
                            Ok(Message::Ping(_data)) => {}
                            Err(e) => {
                                tracing::error!("WebSocket error: {}", e);
                                // Connection errors are always unexpected
                                unexpected_close = true;
                                break;
                            }
                            _ => {}
                        }
                    }

                    // Update state
                    {
                        let mut s = state.write().await;
                        *s = ConnectionState::Disconnected;
                    }
                    // Notify state change
                    {
                        let callback = on_state_change.lock().await;
                        if let Some(ref cb) = *callback {
                            cb(ConnectionState::Disconnected);
                        }
                    }

                    unexpected_close
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

                let unexpected_close = tokio::select! {
                    result = read_handle => {
                        tracing::info!("Read task completed");
                        result.unwrap_or(false)
                    }
                    _ = write_handle => {
                        tracing::info!("Write task completed");
                        // Write task ended first - likely a disconnect
                        true
                    }
                };

                Ok(unexpected_close)
            }
            Err(e) => {
                tracing::error!("Failed to connect to Engine: {}", e);
                self.set_state(ConnectionState::Failed).await;
                Err(e.into())
            }
        }
    }

    /// Attempt to reconnect with exponential backoff
    async fn reconnect_with_backoff(&self) {
        let mut backoff = BackoffState::default();

        loop {
            self.set_state(ConnectionState::Reconnecting).await;
            let Some(delay) = backoff.next_delay_and_advance() else {
                tracing::error!("Max reconnection attempts reached, giving up");
                self.set_state(ConnectionState::Failed).await;
                return;
            };
            tracing::info!(
                "Reconnection attempt {} of {}, waiting {}ms",
                backoff.attempts(),
                crate::infrastructure::websocket::shared::MAX_RETRY_ATTEMPTS,
                delay
            );

            tokio::time::sleep(Duration::from_millis(delay)).await;

            // Check if disconnect was requested during the wait
            if *self.intentional_disconnect.read().await {
                tracing::info!("Reconnection cancelled - intentional disconnect");
                self.set_state(ConnectionState::Disconnected).await;
                return;
            }

            match self.connect_internal().await {
                Ok(unexpected_close) => {
                    if unexpected_close && !*self.intentional_disconnect.read().await {
                        // Connection was established but closed unexpectedly, retry
                        continue;
                    }
                    // Either clean disconnect or intentional - stop reconnecting
                    return;
                }
                Err(e) => {
                    tracing::warn!("Reconnection attempt {} failed: {}", backoff.attempts(), e);
                }
            }
        }
    }

    pub async fn connect(&self) -> Result<()> {
        // Reset intentional disconnect flag
        {
            let mut flag = self.intentional_disconnect.write().await;
            *flag = false;
        }

        match self.connect_internal().await {
            Ok(unexpected_close) => {
                if unexpected_close && !*self.intentional_disconnect.read().await {
                    // Connection closed unexpectedly, attempt reconnection
                    tracing::info!("Connection closed unexpectedly, initiating reconnection");
                    self.reconnect_with_backoff().await;
                }
                Ok(())
            }
            Err(e) => Err(e),
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

    pub async fn disconnect(&self) {
        // Mark this as intentional to prevent reconnection attempts
        {
            let mut flag = self.intentional_disconnect.write().await;
            *flag = true;
        }
        // Clear pending requests - dropping senders causes Cancelled errors for waiters
        {
            let mut pending = self.pending_requests.lock().await;
            let count = pending.clear();
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
            intentional_disconnect: Arc::clone(&self.intentional_disconnect),
        }
    }
}
