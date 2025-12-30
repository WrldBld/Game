//! World Directorial Port - DM directorial context management.
//!
//! This port handles the DM's directorial notes for NPC guidance.

use wrldbldr_domain::value_objects::DirectorialNotes;
use wrldbldr_domain::WorldId;

/// Port for managing DM directorial context within a world.
///
/// Directorial context provides runtime guidance for NPC behavior,
/// including mood, motivations, and scene-specific instructions.
///
/// All methods are synchronous as they operate on in-memory state.
/// Implementations must be thread-safe (Send + Sync).
pub trait WorldDirectorialPort: Send + Sync {
    /// Get the DM's directorial context (runtime NPC guidance) for a world.
    fn get_directorial_context(&self, world_id: &WorldId) -> Option<DirectorialNotes>;

    /// Set the directorial context for a world.
    fn set_directorial_context(&self, world_id: &WorldId, notes: DirectorialNotes);

    /// Clear the directorial context for a world.
    fn clear_directorial_context(&self, world_id: &WorldId);
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    pub WorldDirectorialPort {}

    impl WorldDirectorialPort for WorldDirectorialPort {
        fn get_directorial_context(&self, world_id: &WorldId) -> Option<DirectorialNotes>;
        fn set_directorial_context(&self, world_id: &WorldId, notes: DirectorialNotes);
        fn clear_directorial_context(&self, world_id: &WorldId);
    }
}
