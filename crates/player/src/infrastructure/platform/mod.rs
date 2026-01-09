//! Platform-specific implementations
//!
//! This module provides platform-specific implementations of the
//! platform abstraction traits defined in application/ports/outbound/platform.rs.
//!
//! The correct platform is selected at compile time based on the target architecture.

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;

pub mod mock;

// Re-export the platform-specific types explicitly
#[cfg(target_arch = "wasm32")]
pub use wasm::{
    create_platform, WasmDocumentProvider, WasmEngineConfigProvider, WasmLogProvider,
    WasmRandomProvider, WasmSleepProvider, WasmStorageProvider, WasmTimeProvider,
};

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::{
    create_platform, DesktopDocumentProvider, DesktopEngineConfigProvider, DesktopLogProvider,
    DesktopRandomProvider, DesktopSleepProvider, DesktopStorageProvider, DesktopTimeProvider,
};

// Mock platform remains available via `crate::infrastructure::platform::mock`.
