use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            pub fn to_uuid(self) -> Uuid {
                self.0
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
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.0
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

// Connection IDs
define_id!(ConnectionId);

// Skill and challenge IDs
define_id!(SkillId);
define_id!(ChallengeId);

// Event and narrative IDs
define_id!(EventId);
define_id!(StoryEventId);
define_id!(NarrativeEventId);
define_id!(EventChainId);

// Participant IDs (SessionId removed - using WorldId for connection scoping)
define_id!(ParticipantId);
define_id!(ActionId);

/// User identifier - wraps a client-provided string from browser storage.
///
/// Unlike other IDs which are UUIDs, UserId wraps a string because it comes
/// from the client (typically browser localStorage) and is not a UUID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(String);

impl UserId {
    /// Create a new UserId from a string, validating that it's not empty.
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err(DomainError::validation("UserId cannot be empty"));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Create from trusted source (DB) without validation.
    ///
    /// Use this when loading from storage where the value was already validated.
    pub fn from_trusted(id: String) -> Self {
        Self(id)
    }

    /// Get the inner string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Check if the user ID is empty.
    ///
    /// Note: This should always return false for validated UserIds,
    /// but may return true for UserIds created with `from_trusted`.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl TryFrom<String> for UserId {
    type Error = DomainError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<&str> for UserId {
    type Error = DomainError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<UserId> for String {
    fn from(id: UserId) -> String {
        id.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Asset and generation IDs
define_id!(AssetId);
define_id!(BatchId);

// Scene interaction IDs
define_id!(InteractionId);

// Queue IDs
define_id!(QueueItemId);

// Map IDs
define_id!(GridMapId);

// Staging IDs
define_id!(StagingId);

// Lore IDs
define_id!(LoreId);
define_id!(LoreChunkId);

// Visual State IDs
define_id!(LocationStateId);
define_id!(RegionStateId);

// Misc IDs (present in codebase)
define_id!(WorkflowId);

// Stat system IDs
define_id!(StatModifierId);

// Conversation IDs
define_id!(ConversationId);

// Approval and suggestion IDs
define_id!(ApprovalRequestId);
define_id!(TimeSuggestionId);
define_id!(ApprovalId);

// Conversion from QueueItemId to ActionId for player action tracking
impl From<QueueItemId> for ActionId {
    fn from(value: QueueItemId) -> Self {
        Self(value.to_uuid())
    }
}

// ============================================================================
// Tests for UserId validation
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_validation() {
        // Valid user IDs
        assert!(UserId::new("user123").is_ok());
        assert!(UserId::new("alice@example.com").is_ok());
        assert!(UserId::new("user-with-dashes").is_ok());

        // Invalid: empty string
        assert!(UserId::new("").is_err());
        assert!(matches!(UserId::new(""), Err(DomainError::Validation(_))));

        // Invalid: whitespace only
        assert!(UserId::new("   ").is_err());
    }

    #[test]
    fn test_user_id_from_trusted() {
        // from_trusted should not validate
        let empty_user_id = UserId::from_trusted("".to_string());
        assert!(empty_user_id.is_empty());

        let valid_user_id = UserId::from_trusted("user123".to_string());
        assert!(!valid_user_id.is_empty());
        assert_eq!(valid_user_id.as_str(), "user123");
    }

    #[test]
    fn test_user_id_display() {
        let user_id = UserId::new("test_user").unwrap();
        assert_eq!(user_id.to_string(), "test_user");
        assert_eq!(format!("{}", user_id), "test_user");
    }

    #[test]
    fn test_user_id_conversions() {
        let user_id = UserId::new("alice").unwrap();

        // Convert to String
        let s: String = user_id.clone().into();
        assert_eq!(s, "alice");

        // Convert from String
        let user_id2: Result<UserId, _> = "bob".try_into();
        assert!(user_id2.is_ok());
        assert_eq!(user_id2.unwrap().as_str(), "bob");
    }
}
