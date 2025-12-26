//! WebSocket client for Engine connection
//!
//! Platform-specific implementations for desktop (tokio) and WASM (web-sys).

use anyhow::Result;

use wrldbldr_protocol::{ClientMessage, ParticipantRole, ServerMessage};

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

// ============================================================================
// Desktop (Tokio) Implementation
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use super::*;
    use std::sync::Arc;
    use futures_util::{SinkExt, StreamExt};
    use tokio::sync::{mpsc, Mutex, RwLock};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    /// WebSocket client for communicating with the Engine (Desktop)
    pub struct EngineClient {
        url: String,
        state: Arc<RwLock<ConnectionState>>,
        tx: Arc<Mutex<Option<mpsc::Sender<ClientMessage>>>>,
        on_message: Arc<Mutex<Option<Box<dyn Fn(ServerMessage) + Send + Sync>>>>,
        on_state_change: Arc<Mutex<Option<Box<dyn Fn(ConnectionState) + Send + Sync>>>>,
    }

    impl EngineClient {
        pub fn new(url: impl Into<String>) -> Self {
            Self {
                url: url.into(),
                state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
                tx: Arc::new(Mutex::new(None)),
                on_message: Arc::new(Mutex::new(None)),
                on_state_change: Arc::new(Mutex::new(None)),
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

                    let read_handle = tokio::spawn(async move {
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    match serde_json::from_str::<ServerMessage>(&text) {
                                        Ok(server_msg) => {
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
                            let json = serde_json::to_string(&msg).unwrap();
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

        pub async fn join_session(
            &self,
            user_id: &str,
            role: ParticipantRole,
            world_id: Option<String>,
        ) -> Result<()> {
            let world_id = match world_id.as_deref() {
                Some(s) => Some(uuid::Uuid::parse_str(s)?),
                None => None,
            };

            self.send(ClientMessage::JoinSession {
                user_id: user_id.to_string(),
                role,
                world_id,
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

        pub async fn disconnect(&self) {
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
            }
        }
    }
}

// ============================================================================
// WASM (Web-sys) Implementation
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use web_sys::{MessageEvent, WebSocket};

    /// WebSocket client for communicating with the Engine (WASM)
    pub struct EngineClient {
        url: String,
        state: Rc<RefCell<ConnectionState>>,
        ws: Rc<RefCell<Option<WebSocket>>>,
        on_message: Rc<RefCell<Option<Box<dyn FnMut(ServerMessage)>>>>,
        on_state_change: Rc<RefCell<Option<Box<dyn FnMut(ConnectionState)>>>>,
    }

    impl EngineClient {
        pub fn new(url: impl Into<String>) -> Self {
            Self {
                url: url.into(),
                state: Rc::new(RefCell::new(ConnectionState::Disconnected)),
                ws: Rc::new(RefCell::new(None)),
                on_message: Rc::new(RefCell::new(None)),
                on_state_change: Rc::new(RefCell::new(None)),
            }
        }

        /// Get the URL this client is configured for
        pub fn url(&self) -> &str {
            &self.url
        }

        pub fn set_on_message<F>(&self, callback: F)
        where
            F: FnMut(ServerMessage) + 'static,
        {
            *self.on_message.borrow_mut() = Some(Box::new(callback));
        }

        pub fn set_on_state_change<F>(&self, callback: F)
        where
            F: FnMut(ConnectionState) + 'static,
        {
            *self.on_state_change.borrow_mut() = Some(Box::new(callback));
        }

        pub fn state(&self) -> ConnectionState {
            *self.state.borrow()
        }

        fn set_state(&self, new_state: ConnectionState) {
            *self.state.borrow_mut() = new_state;

            if let Some(ref mut cb) = *self.on_state_change.borrow_mut() {
                cb(new_state);
            }
        }

        pub fn connect(&self) -> Result<()> {
            self.set_state(ConnectionState::Connecting);

            let ws = WebSocket::new(&self.url).map_err(|e| {
                anyhow::anyhow!("Failed to create WebSocket: {:?}", e)
            })?;

            ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

            // Set up message handler
            let on_message = Rc::clone(&self.on_message);
            let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
                if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                    let text: String = txt.into();
                    match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(server_msg) => {
                            if let Some(ref mut cb) = *on_message.borrow_mut() {
                                cb(server_msg);
                            }
                        }
                        Err(e) => {
                            web_sys::console::warn_1(
                                &format!("Failed to parse server message: {}", e).into(),
                            );
                        }
                    }
                }
            });
            ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();

            // Set up open handler
            let state = Rc::clone(&self.state);
            let on_state_change = Rc::clone(&self.on_state_change);
            let onopen_callback = Closure::<dyn FnMut()>::new(move || {
                *state.borrow_mut() = ConnectionState::Connected;
                if let Some(ref mut cb) = *on_state_change.borrow_mut() {
                    cb(ConnectionState::Connected);
                }
                web_sys::console::log_1(&"WebSocket connected".into());
            });
            ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();

            // Set up close handler
            let state = Rc::clone(&self.state);
            let on_state_change = Rc::clone(&self.on_state_change);
            let onclose_callback = Closure::<dyn FnMut()>::new(move || {
                *state.borrow_mut() = ConnectionState::Disconnected;
                if let Some(ref mut cb) = *on_state_change.borrow_mut() {
                    cb(ConnectionState::Disconnected);
                }
                web_sys::console::log_1(&"WebSocket closed".into());
            });
            ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
            onclose_callback.forget();

            // Set up error handler
            let state = Rc::clone(&self.state);
            let on_state_change = Rc::clone(&self.on_state_change);
            let onerror_callback = Closure::<dyn FnMut()>::new(move || {
                *state.borrow_mut() = ConnectionState::Failed;
                if let Some(ref mut cb) = *on_state_change.borrow_mut() {
                    cb(ConnectionState::Failed);
                }
                web_sys::console::error_1(&"WebSocket error".into());
            });
            ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
            onerror_callback.forget();

            *self.ws.borrow_mut() = Some(ws);

            Ok(())
        }

        pub fn send(&self, message: ClientMessage) -> Result<()> {
            if let Some(ref ws) = *self.ws.borrow() {
                let json = serde_json::to_string(&message)?;
                ws.send_with_str(&json)
                    .map_err(|e| anyhow::anyhow!("Failed to send: {:?}", e))?;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Not connected"))
            }
        }

        pub fn join_session(
            &self,
            user_id: &str,
            role: ParticipantRole,
            world_id: Option<String>,
        ) -> Result<()> {
            let world_id = match world_id.as_deref() {
                Some(s) => Some(uuid::Uuid::parse_str(s)?),
                None => None,
            };

            self.send(ClientMessage::JoinSession {
                user_id: user_id.to_string(),
                role,
                world_id,
            })
        }

        pub fn send_action(
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
        }

        pub fn heartbeat(&self) -> Result<()> {
            self.send(ClientMessage::Heartbeat)
        }

        pub fn disconnect(&self) {
            if let Some(ref ws) = *self.ws.borrow() {
                let _ = ws.close();
            }
            *self.ws.borrow_mut() = None;
            self.set_state(ConnectionState::Disconnected);
        }
    }

    impl Clone for EngineClient {
        fn clone(&self) -> Self {
            Self {
                url: self.url.clone(),
                state: Rc::clone(&self.state),
                ws: Rc::clone(&self.ws),
                on_message: Rc::clone(&self.on_message),
                on_state_change: Rc::clone(&self.on_state_change),
            }
        }
    }
}

// ============================================================================
// Re-export the correct implementation
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::EngineClient;

#[cfg(target_arch = "wasm32")]
pub use wasm::EngineClient;
