//! Value objects
//!
//! Immutable types that represent concepts in the domain.

pub mod ids;

// Re-export local String-based IDs (for backward compatibility)
pub use ids::{
    LocationId, WorldId,
};

// Re-export protocol's UUID-based IDs for components that need them
// These are the canonical types shared with Engine
pub mod protocol_ids {
    pub use wrldbldr_protocol::{
        WorldId as UuidWorldId,
        LocationId as UuidLocationId,
        CharacterId as UuidCharacterId,
        SceneId as UuidSceneId,
        SessionId as UuidSessionId,
        // Add more as needed
    };
}
