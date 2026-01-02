//! WASM WebSocket client using web-sys

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use anyhow::Result;
use futures_channel::oneshot;
use futures_util::future::{select, Either};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{MessageEvent, WebSocket};

use wrldbldr_protocol::{
    ClientMessage, ParticipantRole, RequestError, RequestPayload, ResponseResult, ServerMessage,
};

use crate::infrastructure::session_type_converters::participant_role_to_world_role;
use crate::infrastructure::websocket::protocol::ConnectionState;

// Reconnection constants
const INITIAL_RETRY_DELAY_MS: u32 = 1000;
const MAX_RETRY_DELAY_MS: u32 = 30000;
const MAX_RETRY_ATTEMPTS: u32 = 10;
const BACKOFF_MULTIPLIER: f64 = 2.0;

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
    /// Flag to track if disconnect was intentional (vs unexpected close)
    intentional_disconnect: Rc<RefCell<bool>>,
    /// Message buffer for messages sent during reconnection
    message_buffer: Rc<RefCell<VecDeque<ClientMessage>>>,
    /// Current reconnection attempt count
    reconnect_attempts: Rc<RefCell<u32>>,
    /// Current backoff delay
    reconnect_delay: Rc<RefCell<u32>>,
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
            intentional_disconnect: Rc::new(RefCell::new(false)),
            message_buffer: Rc::new(RefCell::new(VecDeque::new())),
            reconnect_attempts: Rc::new(RefCell::new(0)),
            reconnect_delay: Rc::new(RefCell::new(INITIAL_RETRY_DELAY_MS)),
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
    pub fn request(
        &self,
        payload: RequestPayload,
    ) -> impl std::future::Future<Output = Result<ResponseResult, RequestError>> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        // If a screen deep-links and fires requests before the connection route's
        // ensure_connection() runs, lazily initiate the WebSocket connection here.
        // Outbound messages will buffer while CONNECTING and flush on OPEN.
        if matches!(
            self.state(),
            ConnectionState::Disconnected | ConnectionState::Failed
        ) {
            let _ = self.connect();
        }

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
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        // Same lazy-connect behavior as request() for deep-link safety.
        if matches!(
            self.state(),
            ConnectionState::Disconnected | ConnectionState::Failed
        ) {
            let _ = self.connect();
        }

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
                    pending_requests
                        .borrow_mut()
                        .remove(&request_id_for_cleanup);
                    tracing::debug!(
                        "Request {} timed out, removed from pending",
                        request_id_for_cleanup
                    );
                    Err(RequestError::Timeout)
                }
            }
        }
    }

    /// Internal connection logic - used by both connect() and reconnection
    fn connect_internal(&self) -> Result<()> {
        // Drop existing closures before creating new ones to prevent leaks
        *self.closures.borrow_mut() = None;

        self.set_state(ConnectionState::Connecting);

        let ws = WebSocket::new(&self.url)
            .map_err(|e| anyhow::anyhow!("Failed to create WebSocket: {:?}", e))?;

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

        // Set up open handler - flush buffered messages on successful connection
        let state = Rc::clone(&self.state);
        let on_state_change = Rc::clone(&self.on_state_change);
        let message_buffer = Rc::clone(&self.message_buffer);
        let ws_for_open = Rc::clone(&self.ws);
        let pending_requests_for_open = Rc::clone(&self.pending_requests);
        let reconnect_attempts = Rc::clone(&self.reconnect_attempts);
        let reconnect_delay = Rc::clone(&self.reconnect_delay);
        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            *state.borrow_mut() = ConnectionState::Connected;

            // Reset reconnection state on successful connection
            *reconnect_attempts.borrow_mut() = 0;
            *reconnect_delay.borrow_mut() = INITIAL_RETRY_DELAY_MS;

            if let Some(ref mut cb) = *on_state_change.borrow_mut() {
                cb(ConnectionState::Connected);
            }
            web_sys::console::log_1(&"WebSocket connected".into());

            // Flush buffered messages
            let mut buffer = message_buffer.borrow_mut();
            if !buffer.is_empty() {
                web_sys::console::log_1(
                    &format!("Flushing {} buffered messages", buffer.len()).into(),
                );
                if let Some(ref ws) = *ws_for_open.borrow() {
                    while let Some(msg) = buffer.pop_front() {
                        // If a request already timed out/cancelled, don't send it.
                        if let ClientMessage::Request { request_id, .. } = &msg {
                            if !pending_requests_for_open.borrow().contains_key(request_id) {
                                continue;
                            }
                        }

                        if let Ok(json) = serde_json::to_string(&msg) {
                            if let Err(e) = ws.send_with_str(&json) {
                                web_sys::console::warn_1(
                                    &format!("Failed to send buffered message: {:?}", e).into(),
                                );
                                // Re-add failed message to front of buffer
                                buffer.push_front(msg);
                                break;
                            }
                        }
                    }
                }
            }
        });
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));

        // Set up close handler - trigger reconnection on unexpected close
        let state = Rc::clone(&self.state);
        let on_state_change = Rc::clone(&self.on_state_change);
        let intentional_disconnect = Rc::clone(&self.intentional_disconnect);
        let client_clone = self.clone();
        let onclose_callback = Closure::<dyn FnMut()>::new(move || {
            let intentional = *intentional_disconnect.borrow();

            if intentional {
                *state.borrow_mut() = ConnectionState::Disconnected;
                if let Some(ref mut cb) = *on_state_change.borrow_mut() {
                    cb(ConnectionState::Disconnected);
                }
                web_sys::console::log_1(&"WebSocket closed (intentional)".into());
            } else {
                web_sys::console::log_1(
                    &"WebSocket closed unexpectedly, attempting reconnection".into(),
                );
                // Trigger reconnection
                let client = client_clone.clone();
                spawn_local(async move {
                    client.reconnect_with_backoff().await;
                });
            }
        });
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));

        // Set up error handler
        let state = Rc::clone(&self.state);
        let on_state_change = Rc::clone(&self.on_state_change);
        let onerror_callback = Closure::<dyn FnMut()>::new(move || {
            // Don't immediately fail - the close handler will trigger reconnection
            // Only set to Failed if we're not already reconnecting
            let current_state = *state.borrow();
            if current_state != ConnectionState::Reconnecting {
                *state.borrow_mut() = ConnectionState::Failed;
                if let Some(ref mut cb) = *on_state_change.borrow_mut() {
                    cb(ConnectionState::Failed);
                }
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

    /// Attempt to reconnect with exponential backoff
    async fn reconnect_with_backoff(&self) {
        // Check if we should attempt reconnection
        if *self.intentional_disconnect.borrow() {
            web_sys::console::log_1(&"Reconnection cancelled - intentional disconnect".into());
            return;
        }

        let attempts = *self.reconnect_attempts.borrow();
        let delay = *self.reconnect_delay.borrow();

        if attempts >= MAX_RETRY_ATTEMPTS {
            web_sys::console::error_1(&"Max reconnection attempts reached, giving up".into());
            self.set_state(ConnectionState::Failed);
            return;
        }

        self.set_state(ConnectionState::Reconnecting);
        web_sys::console::log_1(
            &format!(
                "Reconnection attempt {} of {}, waiting {}ms",
                attempts + 1,
                MAX_RETRY_ATTEMPTS,
                delay
            )
            .into(),
        );

        // Wait with exponential backoff using gloo-timers
        TimeoutFuture::new(delay).await;

        // Check again if disconnect was requested during the wait
        if *self.intentional_disconnect.borrow() {
            web_sys::console::log_1(
                &"Reconnection cancelled during wait - intentional disconnect".into(),
            );
            self.set_state(ConnectionState::Disconnected);
            return;
        }

        // Update reconnection state for next attempt
        *self.reconnect_attempts.borrow_mut() = attempts + 1;
        *self.reconnect_delay.borrow_mut() =
            ((delay as f64) * BACKOFF_MULTIPLIER).min(MAX_RETRY_DELAY_MS as f64) as u32;

        // Attempt to reconnect
        if let Err(e) = self.connect_internal() {
            web_sys::console::warn_1(&format!("Reconnection attempt failed: {:?}", e).into());
            // The close handler will trigger another reconnection attempt
        }
        // If connect_internal succeeds, the onopen handler will reset the reconnection state
    }

    pub fn connect(&self) -> Result<()> {
        // Reset intentional disconnect flag
        *self.intentional_disconnect.borrow_mut() = false;
        // Reset reconnection state
        *self.reconnect_attempts.borrow_mut() = 0;
        *self.reconnect_delay.borrow_mut() = INITIAL_RETRY_DELAY_MS;

        self.connect_internal()
    }

    pub fn send(&self, message: ClientMessage) -> Result<()> {
        let current_state = self.state();

        // Buffer messages during connecting/reconnecting.
        // In browsers, calling WebSocket::send() during CONNECTING throws InvalidStateError.
        if current_state == ConnectionState::Connecting
            || current_state == ConnectionState::Reconnecting
        {
            self.message_buffer.borrow_mut().push_back(message);
            web_sys::console::log_1(&"Message buffered until socket is open".into());
            return Ok(());
        }

        if let Some(ref ws) = *self.ws.borrow() {
            let json = serde_json::to_string(&message)?;
            ws.send_with_str(&json)
                .map_err(|e| anyhow::anyhow!("Failed to send: {:?}", e))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not connected"))
        }
    }

    pub fn join_world(&self, world_id: &str, _user_id: &str, role: ParticipantRole) -> Result<()> {
        let world_id = uuid::Uuid::parse_str(world_id)?;
        let world_role = participant_role_to_world_role(role);

        self.send(ClientMessage::JoinWorld {
            world_id,
            role: world_role,
            pc_id: None, // PC selection happens after joining
            spectate_pc_id: None,
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
        // Mark this as intentional to prevent reconnection attempts
        *self.intentional_disconnect.borrow_mut() = true;

        // Clear pending requests - dropping callbacks causes Cancelled errors for waiters
        {
            let mut pending = self.pending_requests.borrow_mut();
            let count = pending.len();
            pending.clear();
            if count > 0 {
                web_sys::console::log_1(
                    &format!("Cleared {} pending requests on disconnect", count).into(),
                );
            }
        }

        // Clear message buffer
        {
            let mut buffer = self.message_buffer.borrow_mut();
            let count = buffer.len();
            buffer.clear();
            if count > 0 {
                web_sys::console::log_1(
                    &format!("Cleared {} buffered messages on disconnect", count).into(),
                );
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
            intentional_disconnect: Rc::clone(&self.intentional_disconnect),
            message_buffer: Rc::clone(&self.message_buffer),
            reconnect_attempts: Rc::clone(&self.reconnect_attempts),
            reconnect_delay: Rc::clone(&self.reconnect_delay),
        }
    }
}
