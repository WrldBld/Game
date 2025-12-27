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
    use std::collections::HashMap;
    use std::sync::Arc;
    use futures_util::{SinkExt, StreamExt};
    use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    use wrldbldr_protocol::{RequestPayload, ResponseResult, RequestError};

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
}

// ============================================================================
// WASM (Web-sys) Implementation
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use web_sys::{MessageEvent, WebSocket};
    use wrldbldr_protocol::{RequestError, RequestPayload, ResponseResult};

    /// Storage for WebSocket event closures to prevent leaks on reconnect
    struct WasmClosures {
        #[allow(dead_code)]
        onmessage: Closure<dyn FnMut(MessageEvent)>,
        #[allow(dead_code)]
        onopen: Closure<dyn FnMut()>,
        #[allow(dead_code)]
        onclose: Closure<dyn FnMut()>,
        #[allow(dead_code)]
        onerror: Closure<dyn FnMut()>,
    }

    /// WebSocket client for communicating with the Engine (WASM)
    pub struct EngineClient {
        url: String,
        state: Rc<RefCell<ConnectionState>>,
        ws: Rc<RefCell<Option<WebSocket>>>,
        on_message: Rc<RefCell<Option<Box<dyn FnMut(ServerMessage)>>>>,
        on_state_change: Rc<RefCell<Option<Box<dyn FnMut(ConnectionState)>>>>,
        pending_requests: Rc<RefCell<HashMap<String, Box<dyn FnOnce(ResponseResult)>>>>,
        /// Stored closures for cleanup on disconnect/reconnect
        closures: Rc<RefCell<Option<WasmClosures>>>,
    }

    impl EngineClient {
        pub fn new(url: impl Into<String>) -> Self {
            Self {
                url: url.into(),
                state: Rc::new(RefCell::new(ConnectionState::Disconnected)),
                ws: Rc::new(RefCell::new(None)),
                on_message: Rc::new(RefCell::new(None)),
                on_state_change: Rc::new(RefCell::new(None)),
                pending_requests: Rc::new(RefCell::new(HashMap::new())),
                closures: Rc::new(RefCell::new(None)),
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

        /// Send a request and get a future that resolves when response arrives
        pub fn request(&self, payload: RequestPayload) -> impl std::future::Future<Output = Result<ResponseResult, RequestError>> {
            use futures_channel::oneshot;
            
            let request_id = uuid::Uuid::new_v4().to_string();
            let (tx, rx) = oneshot::channel();
            
            // Store the sender as a callback
            self.pending_requests.borrow_mut().insert(
                request_id.clone(),
                Box::new(move |result| { let _ = tx.send(result); })
            );
            
            // Send the message
            let msg = ClientMessage::Request { 
                request_id: request_id.clone(), 
                payload 
            };
            
            let send_result = self.send(msg);
            
            async move {
                send_result.map_err(|e| RequestError::SendFailed(e.to_string()))?;
                rx.await.map_err(|_| RequestError::Cancelled)
            }
        }

        /// Send a request with timeout, cleaning up on timeout
        pub fn request_with_timeout(
            &self,
            payload: RequestPayload,
            timeout_ms: u64,
        ) -> impl std::future::Future<Output = Result<ResponseResult, RequestError>> {
            use futures_channel::oneshot;
            use futures_util::future::{select, Either};
            use gloo_timers::future::TimeoutFuture;

            let request_id = uuid::Uuid::new_v4().to_string();
            let (tx, rx) = oneshot::channel();

            // Store the sender as a callback
            self.pending_requests.borrow_mut().insert(
                request_id.clone(),
                Box::new(move |result| {
                    let _ = tx.send(result);
                }),
            );

            // Send the message
            let msg = ClientMessage::Request {
                request_id: request_id.clone(),
                payload,
            };

            let send_result = self.send(msg);
            let pending_requests = Rc::clone(&self.pending_requests);
            let request_id_for_cleanup = request_id;

            async move {
                send_result.map_err(|e| RequestError::SendFailed(e.to_string()))?;

                let timeout_future = TimeoutFuture::new(timeout_ms as u32);

                match select(Box::pin(rx), Box::pin(timeout_future)).await {
                    Either::Left((result, _)) => result.map_err(|_| RequestError::Cancelled),
                    Either::Right((_, _)) => {
                        // Timeout - remove from pending requests to prevent leak
                        pending_requests.borrow_mut().remove(&request_id_for_cleanup);
                        tracing::debug!(
                            "Request {} timed out, removed from pending",
                            request_id_for_cleanup
                        );
                        Err(RequestError::Timeout)
                    }
                }
            }
        }

        pub fn connect(&self) -> Result<()> {
            // Drop existing closures before creating new ones to prevent leaks
            *self.closures.borrow_mut() = None;

            self.set_state(ConnectionState::Connecting);

            let ws = WebSocket::new(&self.url).map_err(|e| {
                anyhow::anyhow!("Failed to create WebSocket: {:?}", e)
            })?;

            ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

            // Set up message handler
            let on_message = Rc::clone(&self.on_message);
            let pending_requests_clone = Rc::clone(&self.pending_requests);
            let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
                if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                    let text: String = txt.into();
                    match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(server_msg) => {
                            // Check if it's a Response and resolve pending request
                            if let ServerMessage::Response {
                                ref request_id,
                                ref result,
                            } = server_msg
                            {
                                if let Some(callback) =
                                    pending_requests_clone.borrow_mut().remove(request_id)
                                {
                                    callback(result.clone());
                                    return; // Don't pass to regular callback
                                }
                            }

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

            // Store closures instead of forgetting them to allow cleanup on disconnect/reconnect
            *self.closures.borrow_mut() = Some(WasmClosures {
                onmessage: onmessage_callback,
                onopen: onopen_callback,
                onclose: onclose_callback,
                onerror: onerror_callback,
            });

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
            // Clear pending requests - dropping callbacks causes Cancelled errors for waiters
            {
                let mut pending = self.pending_requests.borrow_mut();
                let count = pending.len();
                pending.clear();
                if count > 0 {
                    tracing::debug!("Cleared {} pending requests on disconnect", count);
                }
            }

            // Drop closures to free memory
            *self.closures.borrow_mut() = None;

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
                pending_requests: Rc::clone(&self.pending_requests),
                closures: Rc::clone(&self.closures),
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
