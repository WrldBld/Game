//! Browser localStorage service for persisting user preferences
//!
//! This module provides a platform-agnostic interface for storing and retrieving
//! user preferences and session data. On WASM targets, it uses browser localStorage.
//! On desktop targets, it provides no-op stubs (persistence can be handled by other means).

#[cfg(target_arch = "wasm32")]
use web_sys::window;

/// Keys for localStorage entries
pub const STORAGE_KEY_SERVER_URL: &str = "wrldbldr_server_url";
pub const STORAGE_KEY_ROLE: &str = "wrldbldr_role";
pub const STORAGE_KEY_USER_ID: &str = "wrldbldr_user_id";

/// Save a value to localStorage (WASM only)
///
/// On WASM targets, persists the value in the browser's localStorage.
/// On desktop targets, this is a no-op.
///
/// # Arguments
/// * `key` - The localStorage key
/// * `value` - The value to store
///
/// # Example
/// ```ignore
/// storage::save(storage::STORAGE_KEY_SERVER_URL, "ws://localhost:8080");
/// ```
#[cfg(target_arch = "wasm32")]
pub fn save(key: &str, value: &str) {
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(key, value);
        }
    }
}

/// Load a value from localStorage (WASM only)
///
/// On WASM targets, retrieves the value from browser's localStorage.
/// On desktop targets, returns None.
///
/// # Arguments
/// * `key` - The localStorage key
///
/// # Returns
/// Some(value) if the key exists, None otherwise
///
/// # Example
/// ```ignore
/// if let Some(url) = storage::load(storage::STORAGE_KEY_SERVER_URL) {
///     println!("Saved server URL: {}", url);
/// }
/// ```
#[cfg(target_arch = "wasm32")]
pub fn load(key: &str) -> Option<String> {
    window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|s| s.get_item(key).ok())
        .flatten()
}

/// Remove a value from localStorage (WASM only)
///
/// On WASM targets, removes the key from browser's localStorage.
/// On desktop targets, this is a no-op.
///
/// # Arguments
/// * `key` - The localStorage key
///
/// # Example
/// ```ignore
/// storage::remove(storage::STORAGE_KEY_SERVER_URL);
/// ```
#[cfg(target_arch = "wasm32")]
pub fn remove(key: &str) {
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(key);
        }
    }
}

// Desktop stubs - no-op implementations for non-WASM targets
#[cfg(not(target_arch = "wasm32"))]
pub fn save(_key: &str, _value: &str) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn load(_key: &str) -> Option<String> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn remove(_key: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_constants() {
        assert_eq!(STORAGE_KEY_SERVER_URL, "wrldbldr_server_url");
        assert_eq!(STORAGE_KEY_ROLE, "wrldbldr_role");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_desktop_stubs() {
        // Desktop stubs should be no-op
        save("key", "value");
        assert_eq!(load("key"), None);
        remove("key");
    }
}
