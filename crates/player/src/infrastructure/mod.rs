pub mod api;
pub mod http_client;
pub mod message_translator;
pub mod messaging;
pub mod platform;
pub mod session_type_converters;
pub mod storage;
pub mod url_handler;
pub mod websocket;

pub mod testing;

// Re-export messaging types
pub use messaging::{CommandBus, ConnectionState, EventBus};

/// Spawn an async task that works on both WASM and desktop.
///
/// On WASM, uses `wasm_bindgen_futures::spawn_local`.
/// On desktop, uses Dioxus's `spawn`.
#[cfg(target_arch = "wasm32")]
pub fn spawn_task<F>(fut: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(fut);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_task<F>(fut: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    dioxus::prelude::spawn(fut);
}

/// Platform-agnostic async sleep.
///
/// On WASM, uses `gloo_timers::future::TimeoutFuture`.
/// On desktop, uses `tokio::time::sleep`.
#[cfg(target_arch = "wasm32")]
pub async fn sleep_ms(millis: u32) {
    use gloo_timers::future::TimeoutFuture;
    TimeoutFuture::new(millis).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep_ms(millis: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(millis as u64)).await;
}
