//! Environment adapter using std::env for environment variable access.

use wrldbldr_engine_ports::outbound::EnvironmentPort;

/// System environment variable adapter.
///
/// Uses std::env for reading environment variables from the OS.
#[derive(Debug, Clone, Default)]
pub struct SystemEnvironmentAdapter;

impl SystemEnvironmentAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl EnvironmentPort for SystemEnvironmentAdapter {
    fn get_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}
