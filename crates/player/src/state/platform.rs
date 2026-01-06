//! Platform DI Container
//!
//! This module provides the `Platform` struct - a dependency injection container
//! that aggregates all platform-specific service implementations behind port traits.
//!
//! The Platform struct lives in the adapters layer because:
//! 1. It's a concrete implementation (DI container with Arc<dyn> fields)
//! 2. It contains type erasure logic (*Dyn traits and blanket impls)
//! 3. The ports layer should only contain pure interface definitions
//!
//! Usage:
//! - Created by `create_platform()` factory in platform/desktop.rs or platform/wasm.rs
//! - Injected into Dioxus context by player-runner
//! - Accessed in UI via `use_context::<Platform>()`

use std::{future::Future, pin::Pin, sync::Arc};

use crate::ports::outbound::{
    ConnectionFactoryProvider, DocumentProvider, EngineConfigProvider, GameConnectionPort,
    LogProvider, RandomProvider, SleepProvider, StorageProvider, TimeProvider,
};

/// Unified platform services container
///
/// Provides all platform abstractions through a single injectable type.
/// Use via Dioxus context: `use_context::<Platform>()`
#[derive(Clone)]
pub struct Platform {
    time: Arc<dyn TimeProviderDyn>,
    sleep: Arc<dyn SleepProviderDyn>,
    random: Arc<dyn RandomProviderDyn>,
    storage: Arc<dyn StorageProviderDyn>,
    log: Arc<dyn LogProviderDyn>,
    document: Arc<dyn DocumentProviderDyn>,
    engine_config: Arc<dyn EngineConfigProviderDyn>,
    connection_factory: Arc<dyn ConnectionFactoryProviderDyn>,
}

// =============================================================================
// Dynamic trait versions for Arc storage (need Send + Sync for Dioxus context)
// =============================================================================

trait TimeProviderDyn: Send + Sync {
    fn now_unix_secs(&self) -> u64;
    fn now_millis(&self) -> u64;
}

trait SleepProviderDyn: Send + Sync {
    fn sleep_ms(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + 'static>>;
}

trait RandomProviderDyn: Send + Sync {
    fn random_f64(&self) -> f64;
    fn random_range(&self, min: i32, max: i32) -> i32;
}

trait StorageProviderDyn: Send + Sync {
    fn save(&self, key: &str, value: &str);
    fn load(&self, key: &str) -> Option<String>;
    fn remove(&self, key: &str);
}

trait LogProviderDyn: Send + Sync {
    fn info(&self, msg: &str);
    fn error(&self, msg: &str);
    fn debug(&self, msg: &str);
    fn warn(&self, msg: &str);
}

trait DocumentProviderDyn: Send + Sync {
    fn set_page_title(&self, title: &str);
}

trait EngineConfigProviderDyn: Send + Sync {
    fn configure_engine_url(&self, ws_url: &str);
    fn ws_to_http(&self, ws_url: &str) -> String;
}

trait ConnectionFactoryProviderDyn: Send + Sync {
    fn create_game_connection(&self, server_url: &str) -> Arc<dyn GameConnectionPort>;
}

// =============================================================================
// Blanket implementations - convert port traits to dyn-safe wrappers
// =============================================================================

impl<T: TimeProvider + Send + Sync> TimeProviderDyn for T {
    fn now_unix_secs(&self) -> u64 {
        TimeProvider::now_unix_secs(self)
    }
    fn now_millis(&self) -> u64 {
        TimeProvider::now_millis(self)
    }
}

impl<T: SleepProvider + Send + Sync> SleepProviderDyn for T {
    fn sleep_ms(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + 'static>> {
        SleepProvider::sleep_ms(self, ms)
    }
}

impl<T: RandomProvider + Send + Sync> RandomProviderDyn for T {
    fn random_f64(&self) -> f64 {
        RandomProvider::random_f64(self)
    }
    fn random_range(&self, min: i32, max: i32) -> i32 {
        RandomProvider::random_range(self, min, max)
    }
}

impl<T: StorageProvider + Send + Sync> StorageProviderDyn for T {
    fn save(&self, key: &str, value: &str) {
        StorageProvider::save(self, key, value)
    }
    fn load(&self, key: &str) -> Option<String> {
        StorageProvider::load(self, key)
    }
    fn remove(&self, key: &str) {
        StorageProvider::remove(self, key)
    }
}

impl<T: LogProvider + Send + Sync> LogProviderDyn for T {
    fn info(&self, msg: &str) {
        LogProvider::info(self, msg)
    }
    fn error(&self, msg: &str) {
        LogProvider::error(self, msg)
    }
    fn debug(&self, msg: &str) {
        LogProvider::debug(self, msg)
    }
    fn warn(&self, msg: &str) {
        LogProvider::warn(self, msg)
    }
}

impl<T: DocumentProvider + Send + Sync> DocumentProviderDyn for T {
    fn set_page_title(&self, title: &str) {
        DocumentProvider::set_page_title(self, title)
    }
}

impl<T: EngineConfigProvider + Send + Sync> EngineConfigProviderDyn for T {
    fn configure_engine_url(&self, ws_url: &str) {
        EngineConfigProvider::configure_engine_url(self, ws_url)
    }

    fn ws_to_http(&self, ws_url: &str) -> String {
        EngineConfigProvider::ws_to_http(self, ws_url)
    }
}

impl<T: ConnectionFactoryProvider + Send + Sync> ConnectionFactoryProviderDyn for T {
    fn create_game_connection(&self, server_url: &str) -> Arc<dyn GameConnectionPort> {
        ConnectionFactoryProvider::create_game_connection(self, server_url)
    }
}

// =============================================================================
// Platform implementation
// =============================================================================

impl Platform {
    /// Create a new Platform with the given providers
    pub fn new<Tm, Sl, R, S, L, D, E, C>(
        time: Tm,
        sleep: Sl,
        random: R,
        storage: S,
        log: L,
        document: D,
        engine_config: E,
        connection_factory: C,
    ) -> Self
    where
        Tm: TimeProvider + Send + Sync,
        Sl: SleepProvider + Send + Sync,
        R: RandomProvider + Send + Sync,
        S: StorageProvider + Send + Sync,
        L: LogProvider + Send + Sync,
        D: DocumentProvider + Send + Sync,
        E: EngineConfigProvider + Send + Sync,
        C: ConnectionFactoryProvider + Send + Sync,
    {
        Self {
            time: Arc::new(time),
            sleep: Arc::new(sleep),
            random: Arc::new(random),
            storage: Arc::new(storage),
            log: Arc::new(log),
            document: Arc::new(document),
            engine_config: Arc::new(engine_config),
            connection_factory: Arc::new(connection_factory),
        }
    }

    // -------------------------------------------------------------------------
    // Time operations
    // -------------------------------------------------------------------------

    /// Get current time as Unix timestamp in seconds
    pub fn now_unix_secs(&self) -> u64 {
        self.time.now_unix_secs()
    }

    /// Get current time in milliseconds since epoch
    pub fn now_millis(&self) -> u64 {
        self.time.now_millis()
    }

    // -------------------------------------------------------------------------
    // Sleep operations
    // -------------------------------------------------------------------------

    /// Sleep for the given number of milliseconds.
    pub fn sleep_ms(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + 'static>> {
        self.sleep.sleep_ms(ms)
    }

    // -------------------------------------------------------------------------
    // Random operations
    // -------------------------------------------------------------------------

    /// Generate random f64 in range [0.0, 1.0)
    pub fn random_f64(&self) -> f64 {
        self.random.random_f64()
    }

    /// Generate random i32 in range [min, max] (inclusive)
    pub fn random_range(&self, min: i32, max: i32) -> i32 {
        self.random.random_range(min, max)
    }

    // -------------------------------------------------------------------------
    // Storage operations
    // -------------------------------------------------------------------------

    /// Save a string value with the given key
    pub fn storage_save(&self, key: &str, value: &str) {
        self.storage.save(key, value)
    }

    /// Load a string value by key, returns None if not found
    pub fn storage_load(&self, key: &str) -> Option<String> {
        self.storage.load(key)
    }

    /// Remove a value by key
    pub fn storage_remove(&self, key: &str) {
        self.storage.remove(key)
    }

    /// Get a StorageProvider adapter for use with application services
    ///
    /// This allows application-layer services like UserService to use
    /// Platform's storage without exposing internal implementation details.
    ///
    /// # Example
    /// ```ignore
    /// let user_service = UserService::new(platform.storage_adapter());
    /// let user_id = user_service.get_user_id();
    /// ```
    pub fn storage_adapter(&self) -> PlatformStorageAdapter {
        PlatformStorageAdapter {
            platform: self.clone(),
        }
    }

    // -------------------------------------------------------------------------
    // User identity operations (convenience method)
    // -------------------------------------------------------------------------

    /// Get or create a stable anonymous user ID.
    ///
    /// This ID is persisted in storage and reused across sessions until local
    /// storage is cleared, effectively acting as an anonymous user identity.
    ///
    /// NOTE: This is a convenience method. The business logic lives in
    /// `player-app/services/user_service.rs`. For more control, use:
    /// ```ignore
    /// let user_service = UserService::new(platform.storage_adapter());
    /// let user_id = user_service.get_user_id();
    /// ```
    pub fn get_user_id(&self) -> String {
        use crate::ports::outbound::storage_keys;

        if let Some(existing) = self.storage_load(storage_keys::USER_ID) {
            return existing;
        }

        let new_id = format!("user-{}", uuid::Uuid::new_v4());
        self.storage_save(storage_keys::USER_ID, &new_id);
        new_id
    }

    // -------------------------------------------------------------------------
    // Logging operations
    // -------------------------------------------------------------------------

    /// Log an info message
    pub fn log_info(&self, msg: &str) {
        self.log.info(msg)
    }

    /// Log an error message
    pub fn log_error(&self, msg: &str) {
        self.log.error(msg)
    }

    /// Log a debug message
    pub fn log_debug(&self, msg: &str) {
        self.log.debug(msg)
    }

    /// Log a warning message
    pub fn log_warn(&self, msg: &str) {
        self.log.warn(msg)
    }

    // -------------------------------------------------------------------------
    // Document operations
    // -------------------------------------------------------------------------

    /// Set the browser page title (no-op on desktop)
    pub fn set_page_title(&self, title: &str) {
        self.document.set_page_title(title)
    }

    // -------------------------------------------------------------------------
    // Engine config operations
    // -------------------------------------------------------------------------

    /// Configure the base Engine URL for API calls (from WebSocket URL)
    pub fn configure_engine_url(&self, ws_url: &str) {
        self.engine_config.configure_engine_url(ws_url)
    }

    /// Convert WebSocket URL to HTTP URL
    pub fn ws_to_http(&self, ws_url: &str) -> String {
        self.engine_config.ws_to_http(ws_url)
    }

    // -------------------------------------------------------------------------
    // Connection factory operations
    // -------------------------------------------------------------------------

    /// Create a game connection to the engine
    pub fn create_game_connection(&self, server_url: &str) -> Arc<dyn GameConnectionPort> {
        self.connection_factory.create_game_connection(server_url)
    }
}

// =============================================================================
// Storage adapter for application services
// =============================================================================

/// Adapter that allows application services to use Platform's storage
///
/// This implements the StorageProvider port trait, delegating to Platform's
/// internal storage. This allows proper hexagonal architecture where:
/// - Platform (adapters layer) provides the storage infrastructure
/// - Application services like UserService use the StorageProvider port trait
#[derive(Clone)]
pub struct PlatformStorageAdapter {
    platform: Platform,
}

impl StorageProvider for PlatformStorageAdapter {
    fn save(&self, key: &str, value: &str) {
        self.platform.storage_save(key, value)
    }

    fn load(&self, key: &str) -> Option<String> {
        self.platform.storage_load(key)
    }

    fn remove(&self, key: &str) {
        self.platform.storage_remove(key)
    }
}

// =============================================================================
// PlatformPort implementation - enables player-ui to use trait abstraction
// =============================================================================

use crate::ports::outbound::PlatformPort;

impl PlatformPort for Platform {
    fn now_unix_secs(&self) -> u64 {
        self.time.now_unix_secs()
    }

    fn now_millis(&self) -> u64 {
        self.time.now_millis()
    }

    fn sleep_ms(&self, ms: u64) -> Pin<Box<dyn Future<Output = ()> + 'static>> {
        self.sleep.sleep_ms(ms)
    }

    fn random_f64(&self) -> f64 {
        self.random.random_f64()
    }

    fn random_range(&self, min: i32, max: i32) -> i32 {
        self.random.random_range(min, max)
    }

    fn storage_save(&self, key: &str, value: &str) {
        self.storage.save(key, value)
    }

    fn storage_load(&self, key: &str) -> Option<String> {
        self.storage.load(key)
    }

    fn storage_remove(&self, key: &str) {
        self.storage.remove(key)
    }

    fn get_user_id(&self) -> String {
        Platform::get_user_id(self)
    }

    fn log_info(&self, msg: &str) {
        self.log.info(msg)
    }

    fn log_error(&self, msg: &str) {
        self.log.error(msg)
    }

    fn log_debug(&self, msg: &str) {
        self.log.debug(msg)
    }

    fn log_warn(&self, msg: &str) {
        self.log.warn(msg)
    }

    fn set_page_title(&self, title: &str) {
        self.document.set_page_title(title)
    }

    fn configure_engine_url(&self, ws_url: &str) {
        self.engine_config.configure_engine_url(ws_url)
    }

    fn ws_to_http(&self, ws_url: &str) -> String {
        self.engine_config.ws_to_http(ws_url)
    }

    fn create_game_connection(&self, server_url: &str) -> Arc<dyn GameConnectionPort> {
        self.connection_factory.create_game_connection(server_url)
    }
}
