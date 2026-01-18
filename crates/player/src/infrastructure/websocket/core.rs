//! Platform-agnostic core logic for the Engine WebSocket client.
//!
//! This is deliberately free of any runtime / platform dependencies (tokio, web-sys, etc).
//! Platform clients (desktop/wasm) own the actual socket and call into this core for shared
//! behaviors like pending-request tracking and reconnection backoff math.

use std::collections::HashMap;

use wrldbldr_shared::ResponseResult;

use super::shared::{
    BACKOFF_MULTIPLIER, INITIAL_RETRY_DELAY_MS, MAX_RETRY_ATTEMPTS, MAX_RETRY_DELAY_MS,
};

#[cfg(target_arch = "wasm32")]
pub type PendingCallback = Box<dyn FnOnce(ResponseResult) + 'static>;

#[cfg(not(target_arch = "wasm32"))]
pub type PendingCallback = Box<dyn FnOnce(ResponseResult) + Send + 'static>;

/// Tracks pending request callbacks keyed by request_id.
#[derive(Default)]
pub struct PendingRequests {
    inner: HashMap<String, PendingCallback>,
}

impl PendingRequests {
    /// Insert a pending request callback.
    #[allow(dead_code)]
    pub fn insert(&mut self, request_id: String, callback: PendingCallback) {
        self.inner.insert(request_id, callback);
    }

    #[allow(dead_code)]
    pub fn contains(&self, request_id: &str) -> bool {
        self.inner.contains_key(request_id)
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, request_id: &str) -> bool {
        self.inner.remove(request_id).is_some()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Resolve and remove a pending request.
    ///
    /// Returns true if a pending request was found.
    pub fn resolve(&mut self, request_id: &str, result: ResponseResult) -> bool {
        if let Some(callback) = self.inner.remove(request_id) {
            callback(result);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) -> usize {
        let count = self.inner.len();
        self.inner.clear();
        count
    }
}

/// Exponential backoff state shared by reconnect logic.
#[derive(Debug, Clone, Copy)]
pub struct BackoffState {
    attempts: u32,
    delay_ms: u64,
}

impl Default for BackoffState {
    fn default() -> Self {
        Self {
            attempts: 0,
            delay_ms: INITIAL_RETRY_DELAY_MS,
        }
    }
}

impl BackoffState {
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    #[allow(dead_code)]
    pub fn delay_ms(&self) -> u64 {
        self.delay_ms
    }

    pub fn is_exhausted(&self) -> bool {
        self.attempts >= MAX_RETRY_ATTEMPTS
    }

    /// Advance to the next attempt, updating the delay for the subsequent attempt.
    ///
    /// Returns the delay to wait *before* performing this attempt.
    pub fn next_delay_and_advance(&mut self) -> Option<u64> {
        if self.is_exhausted() {
            return None;
        }

        let current_delay = self.delay_ms;
        self.attempts += 1;
        self.delay_ms =
            ((self.delay_ms as f64) * BACKOFF_MULTIPLIER).min(MAX_RETRY_DELAY_MS as f64) as u64;
        Some(current_delay)
    }
}
