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
        if id.is_empty() {
            return Err(DomainError::validation("UserId cannot be empty"));
        }
        Ok(Self(id))
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
