//! World State Ports - ISP-compliant sub-traits for world state management.
//!
//! This module splits the former `WorldStatePort` god trait into focused,
//! single-responsibility sub-traits following the Interface Segregation Principle.
//!
//! # Sub-traits
//!
//! | Trait | Methods | Purpose |
//! |-------|---------|---------|
//! | `WorldTimePort` | 3 | Game time management |
//! | `WorldConversationPort` | 3 | Conversation history |
//! | `WorldApprovalPort` | 3 | Pending DM approvals |
//! | `WorldScenePort` | 2 | Current scene tracking |
//! | `WorldDirectorialPort` | 3 | DM directorial context |
//! | `WorldLifecyclePort` | 3 | World initialization/cleanup |
//!
//! # Usage
//!
//! Services should depend only on the specific traits they need:
//!
//! ```ignore
//! // Good - depends only on conversation history
//! fn my_service(conversation: Arc<dyn WorldConversationPort>) { ... }
//!
//! // Bad - depends on everything
//! fn my_service(state: Arc<dyn WorldStatePort>) { ... }
//! ```
//!
//! # Migration from WorldStatePort
//!
//! The original `WorldStatePort` trait is preserved as a super-trait for
//! backward compatibility. New code should use the specific sub-traits.

mod world_approval_port;
mod world_conversation_port;
mod world_directorial_port;
mod world_lifecycle_port;
mod world_scene_port;
mod world_time_port;

pub use world_approval_port::WorldApprovalPort;
pub use world_conversation_port::WorldConversationPort;
pub use world_directorial_port::WorldDirectorialPort;
pub use world_lifecycle_port::WorldLifecyclePort;
pub use world_scene_port::WorldScenePort;
pub use world_time_port::WorldTimePort;

#[cfg(any(test, feature = "testing"))]
pub use world_approval_port::MockWorldApprovalPort;
#[cfg(any(test, feature = "testing"))]
pub use world_conversation_port::MockWorldConversationPort;
#[cfg(any(test, feature = "testing"))]
pub use world_directorial_port::MockWorldDirectorialPort;
#[cfg(any(test, feature = "testing"))]
pub use world_lifecycle_port::MockWorldLifecyclePort;
#[cfg(any(test, feature = "testing"))]
pub use world_scene_port::MockWorldScenePort;
#[cfg(any(test, feature = "testing"))]
pub use world_time_port::MockWorldTimePort;

// Re-import domain types for the mockall mock! macro
#[cfg(any(test, feature = "testing"))]
use wrldbldr_domain::value_objects::{ConversationEntry, DirectorialNotes, PendingApprovalItem};
#[cfg(any(test, feature = "testing"))]
use wrldbldr_domain::{GameTime, WorldId};

/// Combined WorldStatePort trait for backward compatibility.
///
/// New code should prefer the specific sub-traits (`WorldTimePort`, etc.).
/// This trait is preserved to ease migration of existing code.
pub trait WorldStatePort:
    WorldTimePort
    + WorldConversationPort
    + WorldApprovalPort
    + WorldScenePort
    + WorldDirectorialPort
    + WorldLifecyclePort
{
}

/// Blanket implementation for any type that implements all sub-traits.
impl<T> WorldStatePort for T where
    T: WorldTimePort
        + WorldConversationPort
        + WorldApprovalPort
        + WorldScenePort
        + WorldDirectorialPort
        + WorldLifecyclePort
{
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    pub WorldStatePort {}

    impl WorldTimePort for WorldStatePort {
        fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime>;
        fn set_game_time(&self, world_id: &WorldId, time: GameTime);
        fn advance_game_time(&self, world_id: &WorldId, hours: i64, minutes: i64) -> Option<GameTime>;
    }

    impl WorldConversationPort for WorldStatePort {
        fn add_conversation(&self, world_id: &WorldId, entry: ConversationEntry);
        fn get_conversation_history(&self, world_id: &WorldId, limit: Option<usize>) -> Vec<ConversationEntry>;
        fn clear_conversation_history(&self, world_id: &WorldId);
    }

    impl WorldApprovalPort for WorldStatePort {
        fn add_pending_approval(&self, world_id: &WorldId, item: PendingApprovalItem);
        fn remove_pending_approval(&self, world_id: &WorldId, approval_id: &str) -> Option<PendingApprovalItem>;
        fn get_pending_approvals(&self, world_id: &WorldId) -> Vec<PendingApprovalItem>;
    }

    impl WorldScenePort for WorldStatePort {
        fn get_current_scene(&self, world_id: &WorldId) -> Option<String>;
        fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>);
    }

    impl WorldDirectorialPort for WorldStatePort {
        fn get_directorial_context(&self, world_id: &WorldId) -> Option<DirectorialNotes>;
        fn set_directorial_context(&self, world_id: &WorldId, notes: DirectorialNotes);
        fn clear_directorial_context(&self, world_id: &WorldId);
    }

    impl WorldLifecyclePort for WorldStatePort {
        fn initialize_world(&self, world_id: &WorldId, initial_time: GameTime);
        fn cleanup_world(&self, world_id: &WorldId);
        fn is_world_initialized(&self, world_id: &WorldId) -> bool;
    }
}
