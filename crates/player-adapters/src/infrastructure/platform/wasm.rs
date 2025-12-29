//! WASM platform implementations
//!
//! Provides platform-specific implementations for web browsers using
//! js_sys and web_sys crates.

use std::{future::Future, pin::Pin, sync::Arc};
use wrldbldr_player_ports::outbound::platform::{
    ConnectionFactoryProvider, DocumentProvider, EngineConfigProvider, LogProvider, RandomProvider,
    SleepProvider, StorageProvider, TimeProvider,
};
use wrldbldr_player_ports::outbound::Platform;

/// WASM time provider using js_sys::Date
#[derive(Clone, Default)]
pub struct WasmTimeProvider;

impl TimeProvider for WasmTimeProvider {
    fn now_unix_secs(&self) -> u64 {
        (js_sys::Date::now() / 1000.0) as u64
    }

    fn now_millis(&self) -> u64 {
        js_sys::Date::now() as u64
    }
}

/// WASM random provider using js_sys::Math
#[derive(Clone, Default)]
pub struct WasmRandomProvider;

impl RandomProvider for WasmRandomProvider {
    fn random_f64(&self) -> f64 {
        js_sys::Math::random()
    }

    fn random_range(&self, min: i32, max: i32) -> i32 {
        let range = (max - min + 1) as f64;
        min + (js_sys::Math::random() * range).floor() as i32
    }
}

/// WASM storage provider using localStorage
#[derive(Clone, Default)]
pub struct WasmStorageProvider;

impl StorageProvider for WasmStorageProvider {
    fn save(&self, key: &str, value: &str) {
        if let Some(storage) = web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
        {
            let _ = storage.set_item(key, value);
        }
    }

    fn load(&self, key: &str) -> Option<String> {
        web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
            .and_then(|s| s.get_item(key).ok())
            .flatten()
    }

    fn remove(&self, key: &str) {
        if let Some(storage) = web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
        {
            let _ = storage.remove_item(key);
        }
    }
}

/// WASM log provider using web_sys::console
#[derive(Clone, Default)]
pub struct WasmLogProvider;

impl LogProvider for WasmLogProvider {
    fn info(&self, msg: &str) {
        web_sys::console::log_1(&msg.into());
    }

    fn error(&self, msg: &str) {
        web_sys::console::error_1(&msg.into());
    }

    fn debug(&self, msg: &str) {
        web_sys::console::debug_1(&msg.into());
    }

    fn warn(&self, msg: &str) {
        web_sys::console::warn_1(&msg.into());
    }
}

/// WASM document provider for browser document operations
#[derive(Clone, Default)]
pub struct WasmDocumentProvider;

impl DocumentProvider for WasmDocumentProvider {
    fn set_page_title(&self, title: &str) {
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            document.set_title(&format!("{} | WrldBldr", title));
        }
    }
}

/// WASM sleep provider using gloo timers
#[derive(Clone, Default)]
pub struct WasmSleepProvider;

impl SleepProvider for WasmSleepProvider {
    fn sleep_ms(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + 'static>> {
        Box::pin(async move {
            gloo_timers::future::TimeoutFuture::new(ms as u32).await;
        })
    }
}

/// WASM engine configuration provider
#[derive(Clone, Default)]
pub struct WasmEngineConfigProvider;

impl EngineConfigProvider for WasmEngineConfigProvider {
    fn configure_engine_url(&self, ws_url: &str) {
        let http_url = self.ws_to_http(ws_url);
        crate::infrastructure::api::set_engine_url(&http_url);
    }

    fn ws_to_http(&self, ws_url: &str) -> String {
        crate::infrastructure::api::ws_to_http(ws_url)
    }
}

/// WASM connection factory provider
#[derive(Clone, Default)]
pub struct WasmConnectionFactoryProvider;

impl ConnectionFactoryProvider for WasmConnectionFactoryProvider {
    fn create_game_connection(
        &self,
        server_url: &str,
    ) -> Arc<dyn wrldbldr_player_ports::outbound::GameConnectionPort> {
        crate::infrastructure::connection_factory::ConnectionFactory::create_game_connection(
            server_url,
        )
    }
}

/// Create platform services for WASM
pub fn create_platform() -> Platform {
    Platform::new(
        WasmTimeProvider,
        WasmSleepProvider,
        WasmRandomProvider,
        WasmStorageProvider,
        WasmLogProvider,
        WasmDocumentProvider,
        WasmEngineConfigProvider,
        WasmConnectionFactoryProvider,
    )
}
