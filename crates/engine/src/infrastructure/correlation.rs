//! Correlation ID tracking for request tracing.

use std::fmt;
use uuid::Uuid;

/// Correlation ID for tracking requests across the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CorrelationId(Uuid);

impl CorrelationId {
    /// Generate a new correlation ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Short format (first 8 characters) for logging.
    pub fn short(&self) -> String {
        self.0.to_string()[..8].to_string()
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl serde::Serialize for CorrelationId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for CorrelationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        Ok(Self(Uuid::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_generates_unique_ids() {
        let id1 = CorrelationId::new();
        let id2 = CorrelationId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_short_format_is_8_chars() {
        let id = CorrelationId::new();
        assert_eq!(id.short().len(), 8);
    }

    #[test]
    fn test_display_format() {
        let id = CorrelationId::new();
        let display = format!("{}", id);
        // UUID format: 8-4-4-4-12 hex chars with dashes
        assert_eq!(display.len(), 36);
    }

    #[test]
    fn test_serialize_deserialize() {
        let id1 = CorrelationId::new();
        let serialized = serde_json::to_string(&id1).unwrap();
        let id2: CorrelationId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(id1, id2);
    }
}
