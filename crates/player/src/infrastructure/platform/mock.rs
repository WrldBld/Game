//! Mock platform implementations for testing
//!
//! Provides controllable implementations of all platform providers
//! for deterministic testing.

use crate::ports::outbound::platform::{
    ConnectionFactoryProvider, DocumentProvider, EngineConfigProvider, LogProvider, RandomProvider,
    SleepProvider, StorageProvider, TimeProvider,
};
use crate::state::Platform;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::{future::Future, pin::Pin};

/// Mock time provider with controllable time
#[derive(Clone)]
pub struct MockTimeProvider {
    current_time: Arc<RwLock<u64>>,
}

impl Default for MockTimeProvider {
    fn default() -> Self {
        Self::new(1700000000) // Nov 2023
    }
}

impl MockTimeProvider {
    pub fn new(initial_time: u64) -> Self {
        Self {
            current_time: Arc::new(RwLock::new(initial_time)),
        }
    }

    /// Advance time by the given number of seconds
    pub fn advance(&self, seconds: u64) {
        let mut time = self.current_time.write().unwrap();
        *time += seconds;
    }

    /// Set the current time
    pub fn set(&self, time: u64) {
        let mut current = self.current_time.write().unwrap();
        *current = time;
    }
}

impl TimeProvider for MockTimeProvider {
    fn now_unix_secs(&self) -> u64 {
        *self.current_time.read().unwrap()
    }

    fn now_millis(&self) -> u64 {
        *self.current_time.read().unwrap() * 1000
    }
}

/// Mock random provider with predetermined values
#[derive(Clone)]
pub struct MockRandomProvider {
    next_values: Arc<RwLock<Vec<f64>>>,
    default_value: f64,
}

impl Default for MockRandomProvider {
    fn default() -> Self {
        Self::fixed(0.5)
    }
}

impl MockRandomProvider {
    /// Create a mock that returns values from the given sequence
    pub fn new(values: Vec<f64>) -> Self {
        Self {
            next_values: Arc::new(RwLock::new(values)),
            default_value: 0.5,
        }
    }

    /// Create a mock that always returns the same value
    pub fn fixed(value: f64) -> Self {
        Self {
            next_values: Arc::new(RwLock::new(vec![])),
            default_value: value,
        }
    }

    /// Queue up values to be returned in order
    pub fn queue(&self, values: Vec<f64>) {
        let mut queue = self.next_values.write().unwrap();
        queue.extend(values);
    }
}

impl RandomProvider for MockRandomProvider {
    fn random_f64(&self) -> f64 {
        let mut values = self.next_values.write().unwrap();
        if !values.is_empty() {
            values.remove(0)
        } else {
            self.default_value
        }
    }

    fn random_range(&self, min: i32, max: i32) -> i32 {
        let range = (max - min + 1) as f64;
        min + (self.random_f64() * range).floor() as i32
    }
}

/// Mock storage provider with in-memory storage
#[derive(Clone, Default)]
pub struct MockStorageProvider {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl MockStorageProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all stored data for inspection
    pub fn get_all(&self) -> HashMap<String, String> {
        self.data.read().unwrap().clone()
    }

    /// Clear all stored data
    pub fn clear(&self) {
        self.data.write().unwrap().clear();
    }
}

impl StorageProvider for MockStorageProvider {
    fn save(&self, key: &str, value: &str) {
        self.data
            .write()
            .unwrap()
            .insert(key.to_string(), value.to_string());
    }

    fn load(&self, key: &str) -> Option<String> {
        self.data.read().unwrap().get(key).cloned()
    }

    fn remove(&self, key: &str) {
        self.data.write().unwrap().remove(key);
    }
}

/// Mock log provider that captures all log messages
#[derive(Clone, Default)]
pub struct MockLogProvider {
    logs: Arc<RwLock<Vec<(String, String)>>>,
}

impl MockLogProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all captured logs as (level, message) pairs
    pub fn get_logs(&self) -> Vec<(String, String)> {
        self.logs.read().unwrap().clone()
    }

    /// Clear all captured logs
    pub fn clear(&self) {
        self.logs.write().unwrap().clear();
    }

    /// Check if any log contains the given message
    pub fn contains(&self, msg: &str) -> bool {
        self.logs
            .read()
            .unwrap()
            .iter()
            .any(|(_, m)| m.contains(msg))
    }

    /// Check if any error log contains the given message
    pub fn has_error(&self, msg: &str) -> bool {
        self.logs
            .read()
            .unwrap()
            .iter()
            .any(|(level, m)| level == "ERROR" && m.contains(msg))
    }
}

impl LogProvider for MockLogProvider {
    fn info(&self, msg: &str) {
        self.logs
            .write()
            .unwrap()
            .push(("INFO".to_string(), msg.to_string()));
    }

    fn error(&self, msg: &str) {
        self.logs
            .write()
            .unwrap()
            .push(("ERROR".to_string(), msg.to_string()));
    }

    fn debug(&self, msg: &str) {
        self.logs
            .write()
            .unwrap()
            .push(("DEBUG".to_string(), msg.to_string()));
    }

    fn warn(&self, msg: &str) {
        self.logs
            .write()
            .unwrap()
            .push(("WARN".to_string(), msg.to_string()));
    }
}

/// Mock document provider that tracks page title changes
#[derive(Clone, Default)]
pub struct MockDocumentProvider {
    title: Arc<RwLock<Option<String>>>,
}

impl MockDocumentProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current page title
    pub fn get_title(&self) -> Option<String> {
        self.title.read().unwrap().clone()
    }
}

impl DocumentProvider for MockDocumentProvider {
    fn set_page_title(&self, title: &str) {
        *self.title.write().unwrap() = Some(title.to_string());
    }
}

/// Mock sleep provider (immediate)
#[derive(Clone, Default)]
pub struct MockSleepProvider;

impl SleepProvider for MockSleepProvider {
    fn sleep_ms(&self, _ms: u64) -> Pin<Box<dyn Future<Output = ()> + 'static>> {
        Box::pin(async move {})
    }
}

/// Mock engine configuration provider
#[derive(Clone, Default)]
pub struct MockEngineConfigProvider {
    configured_url: Arc<RwLock<Option<String>>>,
}

impl MockEngineConfigProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the configured engine URL
    pub fn get_configured_url(&self) -> Option<String> {
        self.configured_url.read().unwrap().clone()
    }
}

impl EngineConfigProvider for MockEngineConfigProvider {
    fn configure_engine_url(&self, ws_url: &str) {
        *self.configured_url.write().unwrap() = Some(self.ws_to_http(ws_url));
    }

    fn ws_to_http(&self, ws_url: &str) -> String {
        ws_url
            .replace("wss://", "https://")
            .replace("ws://", "http://")
            .trim_end_matches("/ws")
            .to_string()
    }
}

/// Mock connection factory provider
#[derive(Clone, Default)]
pub struct MockConnectionFactoryProvider;

impl MockConnectionFactoryProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ConnectionFactoryProvider for MockConnectionFactoryProvider {
    fn create_game_connection(
        &self,
        _server_url: &str,
    ) -> Arc<dyn crate::ports::outbound::GameConnectionPort> {
        // For testing, we would need a mock GameConnectionPort implementation
        // For now, this will panic if called - tests that need connection should mock it separately
        panic!("MockConnectionFactoryProvider::create_game_connection called - this should be mocked in tests that need it")
    }
}

/// Create a mock platform with default settings for testing
pub fn create_mock_platform() -> Platform {
    Platform::new(
        MockTimeProvider::default(),
        MockSleepProvider,
        MockRandomProvider::default(),
        MockStorageProvider::default(),
        MockLogProvider::default(),
        MockDocumentProvider::default(),
        MockEngineConfigProvider::default(),
        MockConnectionFactoryProvider,
    )
}

/// Builder for creating customized mock platforms
pub struct MockPlatformBuilder {
    time: MockTimeProvider,
    sleep: MockSleepProvider,
    random: MockRandomProvider,
    storage: MockStorageProvider,
    log: MockLogProvider,
    document: MockDocumentProvider,
    engine_config: MockEngineConfigProvider,
    connection_factory: MockConnectionFactoryProvider,
}

impl Default for MockPlatformBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPlatformBuilder {
    pub fn new() -> Self {
        Self {
            time: MockTimeProvider::default(),
            sleep: MockSleepProvider,
            random: MockRandomProvider::default(),
            storage: MockStorageProvider::default(),
            log: MockLogProvider::default(),
            document: MockDocumentProvider::default(),
            engine_config: MockEngineConfigProvider::default(),
            connection_factory: MockConnectionFactoryProvider,
        }
    }

    pub fn with_time(mut self, initial_time: u64) -> Self {
        self.time = MockTimeProvider::new(initial_time);
        self
    }

    pub fn with_fixed_random(mut self, value: f64) -> Self {
        self.random = MockRandomProvider::fixed(value);
        self
    }

    pub fn with_random_sequence(mut self, values: Vec<f64>) -> Self {
        self.random = MockRandomProvider::new(values);
        self
    }

    pub fn build(self) -> Platform {
        Platform::new(
            self.time,
            self.sleep,
            self.random,
            self.storage,
            self.log,
            self.document,
            self.engine_config,
            self.connection_factory,
        )
    }
}
