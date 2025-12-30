//! Domain ID value objects
//!
//! Strongly-typed IDs for domain entities to prevent mixing up different ID types.

use std::fmt;

/// Macro to define a newtype ID wrapper
macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            /// Create a new ID from a string
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// Get the raw string value
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Convert to owned String
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self::new(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self::new(s)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

// Define all domain IDs
define_id!(WorldId);
define_id!(ActId);
define_id!(SceneId);
define_id!(CharacterId);
define_id!(LocationId);
define_id!(InteractionId);
define_id!(SkillId);
define_id!(ChallengeId);
define_id!(AssetId);
define_id!(SessionId);
define_id!(UserId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_id_creation() {
        let id = WorldId::new("world-123");
        assert_eq!(id.as_str(), "world-123");
    }

    #[test]
    fn test_id_from_string() {
        let id: CharacterId = "char-456".into();
        assert_eq!(id.to_string(), "char-456");
    }

    #[test]
    fn test_id_equality() {
        let id1 = LocationId::new("loc-1");
        let id2 = LocationId::new("loc-1");
        let id3 = LocationId::new("loc-2");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }
}
