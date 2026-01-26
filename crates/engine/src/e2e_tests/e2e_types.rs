//! Type-safe identifiers for E2E test infrastructure.
//!
//! This module contains test-only newtypes that are used across the
//! E2E test logging infrastructure to maintain type safety while
//! avoiding architecture violations.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an LLM request in E2E tests.
///
/// This type is used to correlate LLM prompt events with response events
/// in the E2E event log. It is test-only and has no domain equivalent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TestRequestId(Uuid);

impl TestRequestId {
    /// Generate a new unique test request ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID value.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Convert into the inner UUID value.
    pub fn to_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for TestRequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TestRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TestRequestId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<TestRequestId> for Uuid {
    fn from(value: TestRequestId) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_new() {
        let id = TestRequestId::new();
        // Should generate a non-nil UUID
        assert_ne!(id.to_uuid(), Uuid::nil());
    }

    #[test]
    fn test_request_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = TestRequestId::from(uuid);
        assert_eq!(id.to_uuid(), uuid);
    }

    #[test]
    fn test_request_id_default() {
        let id = TestRequestId::default();
        // Default should also be non-nil (calls new())
        assert_ne!(id.to_uuid(), Uuid::nil());
    }

    #[test]
    fn test_request_id_display() {
        let uuid = Uuid::nil();
        let id = TestRequestId::from(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }
}
