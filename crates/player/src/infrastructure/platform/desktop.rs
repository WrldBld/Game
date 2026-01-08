//! Desktop platform implementations
//!
//! Provides platform-specific implementations for desktop using
//! standard library and native crates.

use crate::ports::outbound::platform::{
    ConnectionFactoryProvider, DocumentProvider, EngineConfigProvider, LogProvider, RandomProvider,
    SleepProvider, StorageProvider, TimeProvider,
};
use crate::state::Platform;
use directories::ProjectDirs;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
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

/// Desktop storage provider with file-based persistence
///
/// Stores key-value pairs in a JSON file at:
/// - Linux: ~/.config/wrldbldr/player/storage.json
/// - macOS: ~/Library/Application Support/io.wrldbldr.player/storage.json
/// - Windows: C:\Users\<User>\AppData\Roaming\wrldbldr\player\storage.json
#[derive(Clone)]
pub struct DesktopStorageProvider {
    /// Path to the storage file
    storage_path: PathBuf,
    /// In-memory cache of stored values
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl Default for DesktopStorageProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopStorageProvider {
    /// Create a new desktop storage provider
    ///
    /// Loads existing data from the storage file if it exists.
    pub fn new() -> Self {
        // Get platform-specific config directory
        let storage_path = if let Some(dirs) = ProjectDirs::from("io", "wrldbldr", "player") {
            dirs.config_dir().join("storage.json")
        } else {
            // Fallback to current directory if project dirs unavailable
            PathBuf::from("wrldbldr_storage.json")
        };

        // Load existing data from file
        let cache = if storage_path.exists() {
            match fs::read_to_string(&storage_path) {
                Ok(data) => match serde_json::from_str::<HashMap<String, String>>(&data) {
                    Ok(map) => map,
                    Err(e) => {
                        tracing::warn!("Failed to parse storage file: {}", e);
                        HashMap::new()
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read storage file: {}", e);
                    HashMap::new()
                }
            }
        } else {
            HashMap::new()
        };

        tracing::debug!("Desktop storage initialized at: {:?}", storage_path);

        Self {
            storage_path,
            cache: Arc::new(RwLock::new(cache)),
        }
    }

    /// Persist the cache to disk
    fn persist(&self) {
        // Ensure parent directory exists
        if let Some(parent) = self.storage_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                tracing::error!("Failed to create storage directory: {}", e);
                return;
            }
        }

        // Write cache to file
        let cache = match self.cache.read() {
            Ok(guard) => guard,
            Err(e) => {
                tracing::error!("Failed to acquire read lock for storage: {}", e);
                return;
            }
        };

        match serde_json::to_string_pretty(&*cache) {
            Ok(data) => {
                if let Err(e) = fs::write(&self.storage_path, data) {
                    tracing::error!("Failed to write storage file: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize storage data: {}", e);
            }
        }
    }
}

impl StorageProvider for DesktopStorageProvider {
    fn save(&self, key: &str, value: &str) {
        match self.cache.write() {
            Ok(mut guard) => {
                guard.insert(key.to_string(), value.to_string());
                drop(guard); // Release lock before I/O
                self.persist();
            }
            Err(e) => {
                tracing::error!("Failed to acquire write lock for storage: {}", e);
            }
        }
    }

    fn load(&self, key: &str) -> Option<String> {
        match self.cache.read() {
            Ok(guard) => guard.get(key).cloned(),
            Err(e) => {
                tracing::error!("Failed to acquire read lock for storage: {}", e);
                None
            }
        }
    }

    fn remove(&self, key: &str) {
        match self.cache.write() {
            Ok(mut guard) => {
                guard.remove(key);
                drop(guard); // Release lock before I/O
                self.persist();
            }
            Err(e) => {
                tracing::error!("Failed to acquire write lock for storage: {}", e);
            }
        }
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
    fn create_game_connection(
        &self,
        server_url: &str,
    ) -> Arc<dyn crate::ports::outbound::GameConnectionPort> {
        crate::infrastructure::connection_factory::ConnectionFactory::create_game_connection(
            server_url,
        )
    }
}

/// Create platform services for desktop
pub fn create_platform() -> Platform {
    Platform::new(
        DesktopTimeProvider,
        DesktopSleepProvider,
        DesktopRandomProvider,
        DesktopStorageProvider::new(),
        DesktopLogProvider,
        DesktopDocumentProvider,
        DesktopEngineConfigProvider,
        DesktopConnectionFactoryProvider,
    )
}
