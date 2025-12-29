//! Environment port for abstracting environment variable access.
//!
//! This allows application layer services to read configuration from environment
//! variables without direct I/O dependencies, enabling testing and alternative
//! configuration sources.

/// Port for environment variable access.
///
/// Abstracts environment variable reading so application services don't depend
/// on std::env directly. Implementations can provide OS environment, test mocks,
/// or alternative configuration sources.
pub trait EnvironmentPort: Send + Sync {
    /// Get an environment variable value.
    /// Returns None if the variable is not set.
    fn get_var(&self, key: &str) -> Option<String>;

    /// Get an environment variable or return a default value.
    fn get_var_or(&self, key: &str, default: &str) -> String {
        self.get_var(key).unwrap_or_else(|| default.to_string())
    }
}
