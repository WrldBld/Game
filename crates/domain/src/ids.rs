use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
define_id!(UserId);
define_id!(ActionId);

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
