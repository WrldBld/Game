//! Strongly-typed identifiers for domain entities
//!
//! These ID types are shared between Engine and Player to ensure type-safe
//! communication. All IDs are UUID-based for consistency.
//!
//! # WASM Compatibility
//!
//! The `uuid` crate is configured with the `js` feature in the workspace,
//! enabling proper random number generation in browser environments.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Macro to define a strongly-typed ID wrapper around UUID
macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Create a new random ID
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Create from an existing UUID
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Get the underlying UUID reference
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            /// Get the UUID value
            pub fn to_uuid(self) -> Uuid {
                self.0
            }

            /// Get string representation
            pub fn as_str(&self) -> String {
                self.0.to_string()
            }

            /// Parse from string (returns None if invalid)
            pub fn parse(s: &str) -> Option<Self> {
                Uuid::parse_str(s).ok().map(Self)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Uuid {
                id.0
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }

        impl AsRef<Uuid> for $name {
            fn as_ref(&self) -> &Uuid {
                &self.0
            }
        }
    };
}

// Core entity IDs
define_id!(WorldId);
define_id!(ActId);
define_id!(SceneId);
define_id!(LocationId);
define_id!(RegionId);
define_id!(CharacterId);
define_id!(PlayerCharacterId);

// Item and inventory IDs
define_id!(ItemId);
define_id!(WantId);
define_id!(GoalId);

// Relationship IDs
define_id!(RelationshipId);

// Skill and challenge IDs
define_id!(SkillId);
define_id!(ChallengeId);

// Event and narrative IDs
define_id!(EventId);
define_id!(StoryEventId);
define_id!(NarrativeEventId);
define_id!(EventChainId);

// Session and participant IDs
define_id!(SessionId);
define_id!(ParticipantId);
define_id!(UserId);
define_id!(ActionId);

// Asset and generation IDs
define_id!(AssetId);
define_id!(BatchId);
define_id!(WorkflowConfigId);

// Scene interaction IDs
define_id!(InteractionId);

// Queue IDs
define_id!(QueueItemId);

// Map IDs (for future tactical combat)
define_id!(GridMapId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation() {
        let id = WorldId::new();
        assert!(!id.to_string().is_empty());
    }

    #[test]
    fn test_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = CharacterId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn test_id_parse() {
        let original = WorldId::new();
        let str_rep = original.to_string();
        let parsed = WorldId::parse(&str_rep).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_id_from_str() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id: LocationId = uuid_str.parse().unwrap();
        assert_eq!(id.to_string(), uuid_str);
    }

    #[test]
    fn test_id_serialization() {
        let id = SceneId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: SceneId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_id_equality() {
        let id1 = CharacterId::new();
        let id2 = CharacterId::from_uuid(*id1.as_uuid());
        let id3 = CharacterId::new();

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }
}
