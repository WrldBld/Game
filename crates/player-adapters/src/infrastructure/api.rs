//! Centralized API configuration
//!
//! Provides the Engine API base URL for all HTTP requests.

use crate::infrastructure::storage;

/// Storage key for the Engine HTTP URL
pub const STORAGE_KEY_ENGINE_URL: &str = "wrldbldr_engine_url";

/// Default Engine HTTP base URL
pub const DEFAULT_ENGINE_HTTP_URL: &str = "http://localhost:3000";

/// Get the Engine HTTP base URL
///
/// Tries to load from localStorage first, falls back to default.
pub fn get_engine_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(url) = storage::load(STORAGE_KEY_ENGINE_URL) {
            if !url.is_empty() {
                return url;
            }
        }
    }
    DEFAULT_ENGINE_HTTP_URL.to_string()
}

/// Set the Engine HTTP base URL
pub fn set_engine_url(url: &str) {
    storage::save(STORAGE_KEY_ENGINE_URL, url);
}

/// Convert WebSocket URL to HTTP URL
///
/// Handles both ws:// and wss:// protocols
pub fn ws_to_http(ws_url: &str) -> String {
    let url = ws_url
        .replace("wss://", "https://")
        .replace("ws://", "http://");

    // Remove /ws path suffix if present
    if url.ends_with("/ws") {
        url[..url.len() - 3].to_string()
    } else {
        url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_to_http() {
        assert_eq!(ws_to_http("ws://localhost:3000/ws"), "http://localhost:3000");
        assert_eq!(ws_to_http("wss://example.com/ws"), "https://example.com");
        assert_eq!(ws_to_http("ws://192.168.1.1:3000"), "http://192.168.1.1:3000");
    }
}
