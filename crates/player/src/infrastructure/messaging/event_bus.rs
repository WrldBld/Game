//! Event Bus for receiving messages from the game engine.
//!
//! The EventBus provides a push-based subscription model for receiving events
//! from the game engine. Subscribers register callbacks that are invoked
//! when events arrive.

use crate::ports::outbound::player_events::PlayerEvent;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;

#[cfg(target_arch = "wasm32")]
use send_wrapper::SendWrapper;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

/// Event bus for receiving game events.
///
/// Push-based: subscribers register callbacks that are invoked when events arrive.
/// The bus holds strong references to subscribers, so they persist until explicitly
/// removed or the bus is dropped.
///
/// # Platform Support
///
/// Works on both desktop (tokio) and WASM (web-sys) platforms.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct EventBus {
    subscribers: Arc<Mutex<Vec<Box<dyn FnMut(PlayerEvent) + Send + 'static>>>>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct EventBus {
    subscribers: SendWrapper<Rc<RefCell<Vec<Box<dyn FnMut(PlayerEvent) + 'static>>>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl EventBus {
    /// Create a new EventBus with no subscribers.
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Subscribe to all events.
    ///
    /// The callback will be invoked for every event received from the engine.
    /// Returns immediately; the callback is stored and called asynchronously
    /// when events arrive.
    pub async fn subscribe(&self, callback: impl FnMut(PlayerEvent) + Send + 'static) {
        self.subscribers.lock().await.push(Box::new(callback));
    }

    /// Subscribe to all events (sync version for non-async contexts).
    ///
    /// Uses blocking lock - prefer `subscribe()` in async contexts.
    pub fn subscribe_sync(&self, callback: impl FnMut(PlayerEvent) + Send + 'static) {
        // Use try_lock to avoid blocking, or spawn a task
        let subscribers = Arc::clone(&self.subscribers);
        tokio::spawn(async move {
            subscribers.lock().await.push(Box::new(callback));
        });
    }

    /// Dispatch an event to all subscribers.
    ///
    /// This is called by the WebSocket bridge when events arrive.
    /// Each subscriber's callback is invoked with a clone of the event.
    pub async fn dispatch(&self, event: PlayerEvent) {
        let mut subscribers = self.subscribers.lock().await;
        for subscriber in subscribers.iter_mut() {
            subscriber(event.clone());
        }
    }

    /// Get the number of subscribers.
    pub async fn subscriber_count(&self) -> usize {
        self.subscribers.lock().await.len()
    }

    /// Clear all subscribers.
    pub async fn clear(&self) {
        self.subscribers.lock().await.clear();
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl EventBus {
    /// Create a new EventBus with no subscribers.
    pub fn new() -> Self {
        Self {
            subscribers: SendWrapper::new(Rc::new(RefCell::new(Vec::new()))),
        }
    }

    /// Subscribe to all events.
    ///
    /// The callback will be invoked for every event received from the engine.
    pub fn subscribe(&self, callback: impl FnMut(PlayerEvent) + 'static) {
        self.subscribers.borrow_mut().push(Box::new(callback));
    }

    /// Dispatch an event to all subscribers.
    ///
    /// This is called by the WebSocket bridge when events arrive.
    pub fn dispatch(&self, event: PlayerEvent) {
        for subscriber in self.subscribers.borrow_mut().iter_mut() {
            subscriber(event.clone());
        }
    }

    /// Get the number of subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.borrow().len()
    }

    /// Clear all subscribers.
    pub fn clear(&self) {
        self.subscribers.borrow_mut().clear();
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_subscribe_and_dispatch() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let bus = EventBus::new();
        let count = Arc::new(AtomicU32::new(0));

        let count_clone = Arc::clone(&count);
        bus.subscribe(move |_event| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        assert_eq!(bus.subscriber_count().await, 1);

        bus.dispatch(PlayerEvent::Pong).await;
        bus.dispatch(PlayerEvent::Pong).await;

        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_multiple_subscribers() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let bus = EventBus::new();
        let count1 = Arc::new(AtomicU32::new(0));
        let count2 = Arc::new(AtomicU32::new(0));

        let count1_clone = Arc::clone(&count1);
        bus.subscribe(move |_event| {
            count1_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        let count2_clone = Arc::clone(&count2);
        bus.subscribe(move |_event| {
            count2_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        bus.dispatch(PlayerEvent::Pong).await;

        assert_eq!(count1.load(Ordering::SeqCst), 1);
        assert_eq!(count2.load(Ordering::SeqCst), 1);
    }
}
