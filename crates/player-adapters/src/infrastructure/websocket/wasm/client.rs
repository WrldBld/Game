//! WASM WebSocket client using web-sys

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::Result;
use futures_channel::oneshot;
use futures_util::future::{select, Either};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use wrldbldr_protocol::{ClientMessage, ParticipantRole, RequestError, RequestPayload, ResponseResult, ServerMessage};

use crate::infrastructure::session_type_converters::participant_role_to_world_role;
use crate::infrastructure::websocket::protocol::ConnectionState;

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

    pub fn join_world(
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
