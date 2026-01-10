//! Command Bus for sending messages to the game engine.
//!
//! The CommandBus provides a unified interface for sending commands to the engine,
//! supporting both fire-and-forget and request-response patterns.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use wrldbldr_protocol::{ClientMessage, RequestError, RequestPayload, ResponseResult};

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::{mpsc, oneshot, Mutex};

#[cfg(target_arch = "wasm32")]
use futures_channel::{mpsc, oneshot};
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

/// Message types sent through the command bus to the WebSocket bridge.
#[derive(Debug)]
pub enum BusMessage {
    /// Fire-and-forget command
    Send(ClientMessage),
    /// Request expecting a response (response comes back via PendingRequests)
    Request { id: String, payload: RequestPayload },
}

/// Pending request tracker for request-response correlation
#[cfg(not(target_arch = "wasm32"))]
pub struct PendingRequests {
    inner: HashMap<String, oneshot::Sender<ResponseResult>>,
}

#[cfg(target_arch = "wasm32")]
pub struct PendingRequests {
    inner: HashMap<String, oneshot::Sender<ResponseResult>>,
}

impl Default for PendingRequests {
    fn default() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
}

impl PendingRequests {
    pub fn insert(&mut self, request_id: String, tx: oneshot::Sender<ResponseResult>) {
        self.inner.insert(request_id, tx);
    }

    pub fn resolve(&mut self, request_id: &str, result: ResponseResult) -> bool {
        if let Some(tx) = self.inner.remove(request_id) {
            let _ = tx.send(result);
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, request_id: &str) -> bool {
        self.inner.remove(request_id).is_some()
    }

    pub fn clear(&mut self) -> usize {
        let count = self.inner.len();
        self.inner.clear();
        count
    }
}

/// Command bus for sending messages to the game engine.
///
/// This is a concrete struct (not a trait) that can be cloned and shared.
/// Services depend on this directly rather than through a trait object.
///
/// # Platform Support
///
/// Works on both desktop (tokio) and WASM (web-sys) platforms.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct CommandBus {
    tx: mpsc::Sender<BusMessage>,
    pending: Arc<Mutex<PendingRequests>>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct CommandBus {
    tx: mpsc::UnboundedSender<BusMessage>,
    pending: Rc<RefCell<PendingRequests>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl CommandBus {
    /// Create a new CommandBus with the given channel sender.
    ///
    /// The pending requests tracker is shared with the bridge for response correlation.
    pub fn new(tx: mpsc::Sender<BusMessage>, pending: Arc<Mutex<PendingRequests>>) -> Self {
        Self { tx, pending }
    }

    /// Send a fire-and-forget command.
    ///
    /// Returns immediately after queueing the message. Use `request()` if you
    /// need a response from the server.
    pub fn send(&self, message: ClientMessage) -> Result<()> {
        self.tx
            .try_send(BusMessage::Send(message))
            .map_err(|e| anyhow::anyhow!("CommandBus send failed: {}", e))
    }

    /// Send a request and await the response.
    ///
    /// This creates a unique request ID, sends the request, and awaits the
    /// correlated response from the server.
    pub async fn request(&self, payload: RequestPayload) -> Result<ResponseResult, RequestError> {
        let id = uuid::Uuid::new_v4().to_string();
        let (response_tx, response_rx) = oneshot::channel();

        // Register pending request before sending
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id.clone(), response_tx);
        }

        // Send the request - bridge will create ClientMessage::Request
        self.tx
            .send(BusMessage::Request {
                id: id.clone(),
                payload,
            })
            .await
            .map_err(|_| RequestError::SendFailed("channel closed".into()))?;

        // Await response
        response_rx.await.map_err(|_| RequestError::Cancelled)
    }

    /// Send a request with a custom timeout.
    pub async fn request_with_timeout(
        &self,
        payload: RequestPayload,
        timeout_ms: u64,
    ) -> Result<ResponseResult, RequestError> {
        tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            self.request(payload),
        )
        .await
        .map_err(|_| RequestError::Timeout)?
    }

    /// Get access to pending requests (for bridge use)
    pub fn pending(&self) -> Arc<Mutex<PendingRequests>> {
        Arc::clone(&self.pending)
    }
}

#[cfg(target_arch = "wasm32")]
impl CommandBus {
    /// Create a new CommandBus with the given channel sender.
    pub fn new(tx: mpsc::UnboundedSender<BusMessage>, pending: Rc<RefCell<PendingRequests>>) -> Self {
        Self { tx, pending }
    }

    /// Send a fire-and-forget command.
    pub fn send(&self, message: ClientMessage) -> Result<()> {
        self.tx
            .unbounded_send(BusMessage::Send(message))
            .map_err(|e| anyhow::anyhow!("CommandBus send failed: {}", e))
    }

    /// Send a request and await the response.
    pub fn request(
        &self,
        payload: RequestPayload,
    ) -> impl std::future::Future<Output = Result<ResponseResult, RequestError>> {
        self.request_with_id(uuid::Uuid::new_v4().to_string(), payload)
    }

    /// Send a request with a pre-generated ID and await the response.
    ///
    /// This is useful when the caller needs to track the request ID for cleanup,
    /// such as in timeout handling.
    fn request_with_id(
        &self,
        id: String,
        payload: RequestPayload,
    ) -> impl std::future::Future<Output = Result<ResponseResult, RequestError>> {
        let (response_tx, response_rx) = oneshot::channel();

        // Register pending request
        self.pending.borrow_mut().insert(id.clone(), response_tx);

        // Send the request - bridge will create ClientMessage::Request
        let send_result = self.tx.unbounded_send(BusMessage::Request {
            id,
            payload,
        });

        async move {
            send_result.map_err(|_| RequestError::SendFailed("channel closed".into()))?;
            response_rx.await.map_err(|_| RequestError::Cancelled)
        }
    }

    /// Send a request with a custom timeout.
    pub fn request_with_timeout(
        &self,
        payload: RequestPayload,
        timeout_ms: u64,
    ) -> impl std::future::Future<Output = Result<ResponseResult, RequestError>> {
        use futures_util::future::{select, Either};
        use gloo_timers::future::TimeoutFuture;

        // Generate ID before creating the request so we can use it for cleanup
        let request_id = uuid::Uuid::new_v4().to_string();
        let request_future = self.request_with_id(request_id.clone(), payload);
        let pending = Rc::clone(&self.pending);

        async move {
            let timeout_future = TimeoutFuture::new(timeout_ms as u32);

            match select(Box::pin(request_future), Box::pin(timeout_future)).await {
                Either::Left((result, _)) => result,
                Either::Right((_, _)) => {
                    // Timeout - clean up the pending request
                    pending.borrow_mut().remove(&request_id);
                    Err(RequestError::Timeout)
                }
            }
        }
    }

    /// Get access to pending requests (for bridge use)
    pub fn pending(&self) -> Rc<RefCell<PendingRequests>> {
        Rc::clone(&self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_send_command() {
        let (tx, mut rx) = mpsc::channel(10);
        let pending = Arc::new(Mutex::new(PendingRequests::default()));
        let bus = CommandBus::new(tx, pending);

        let msg = ClientMessage::Heartbeat;
        bus.send(msg).unwrap();

        let received = rx.recv().await.unwrap();
        assert!(matches!(received, BusMessage::Send(ClientMessage::Heartbeat)));
    }
}
