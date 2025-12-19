//! Desktop platform implementations
//!
//! Provides platform-specific implementations for desktop using
//! standard library and native crates.

use crate::application::ports::outbound::platform::{
    DocumentProvider, EngineConfigProvider, ConnectionFactoryProvider, LogProvider,
    Platform, RandomProvider, SleepProvider, StorageProvider, TimeProvider,
};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{future::Future, pin::Pin, sync::Arc};

/// Desktop time provider using std::time
#[derive(Clone, Default)]
pub struct DesktopTimeProvider;

impl TimeProvider for DesktopTimeProvider {
    fn now_unix_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Desktop random provider using rand crate
#[derive(Clone, Default)]
pub struct DesktopRandomProvider;

impl RandomProvider for DesktopRandomProvider {
    fn random_f64(&self) -> f64 {
        use rand::Rng;
        rand::thread_rng().gen()
    }

    fn random_range(&self, min: i32, max: i32) -> i32 {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    }
}

/// Desktop storage provider
///
/// For desktop, we use a simple in-memory implementation.
/// Could be extended to use directories crate + JSON file for persistence.
#[derive(Clone, Default)]
pub struct DesktopStorageProvider;

impl StorageProvider for DesktopStorageProvider {
    fn save(&self, _key: &str, _value: &str) {
        // Desktop: could write to ~/.config/wrldbldr/
        // For now, no-op since desktop doesn't need localStorage equivalent
        // The desktop app maintains state in memory during session
    }

    fn load(&self, _key: &str) -> Option<String> {
        // No persistence on desktop for now
        None
    }

    fn remove(&self, _key: &str) {
        // No-op
    }
}

/// Desktop log provider using tracing
#[derive(Clone, Default)]
pub struct DesktopLogProvider;

impl LogProvider for DesktopLogProvider {
    fn info(&self, msg: &str) {
        tracing::info!("{}", msg);
    }

    fn error(&self, msg: &str) {
        tracing::error!("{}", msg);
    }

    fn debug(&self, msg: &str) {
        tracing::debug!("{}", msg);
    }

    fn warn(&self, msg: &str) {
        tracing::warn!("{}", msg);
    }
}

/// Desktop document provider (no-op for page title)
#[derive(Clone, Default)]
pub struct DesktopDocumentProvider;

impl DocumentProvider for DesktopDocumentProvider {
    fn set_page_title(&self, _title: &str) {
        // No-op on desktop - window title is managed by OS/Dioxus desktop
    }
}

/// Desktop sleep provider using tokio timer
#[derive(Clone, Default)]
pub struct DesktopSleepProvider;

impl SleepProvider for DesktopSleepProvider {
    fn sleep_ms(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + 'static>> {
        Box::pin(async move {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        })
    }
}

/// Desktop engine configuration provider
#[derive(Clone, Default)]
pub struct DesktopEngineConfigProvider;

impl EngineConfigProvider for DesktopEngineConfigProvider {
    fn configure_engine_url(&self, _ws_url: &str) {
        // Desktop doesn't use the same API configuration as WASM
        // This is a no-op for desktop builds
    }

    fn ws_to_http(&self, ws_url: &str) -> String {
        // Reuse the same conversion logic as infrastructure/api.rs
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
}

/// Desktop connection factory provider
#[derive(Clone, Default)]
pub struct DesktopConnectionFactoryProvider;

impl ConnectionFactoryProvider for DesktopConnectionFactoryProvider {
    fn create_game_connection(&self, server_url: &str) -> Arc<dyn crate::application::ports::outbound::GameConnectionPort> {
        crate::infrastructure::connection_factory::ConnectionFactory::create_game_connection(server_url)
    }
}

/// Create platform services for desktop
pub fn create_platform() -> Platform {
    Platform::new(
        DesktopTimeProvider,
        DesktopSleepProvider,
        DesktopRandomProvider,
        DesktopStorageProvider,
        DesktopLogProvider,
        DesktopDocumentProvider,
        DesktopEngineConfigProvider,
        DesktopConnectionFactoryProvider,
    )
}
