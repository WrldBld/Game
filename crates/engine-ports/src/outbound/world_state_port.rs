//! Port for world state management (game time, conversations, approvals).
//!
//! This port abstracts in-memory per-world state management, allowing different
//! implementations (e.g., in-memory, Redis-backed, etc.).

use wrldbldr_domain::value_objects::{ConversationEntry, DirectorialNotes, PendingApprovalItem};
use wrldbldr_domain::{GameTime, WorldId};

/// Port for managing per-world runtime state.
///
/// This includes transient state that doesn't need persistence but must be
/// tracked during active game sessions:
/// - Game time progression
/// - Conversation history (for context in LLM prompts)
/// - Pending DM approvals
///
/// All methods are synchronous as they operate on in-memory state.
/// Implementations must be thread-safe (Send + Sync).
pub trait WorldStatePort: Send + Sync {
    // === Game Time ===

    /// Get the current game time for a world.
    ///
    /// Returns `None` if the world hasn't been initialized.
    fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime>;

    /// Set the game time for a world.
    ///
    /// This will initialize the world state if it doesn't exist.
    fn set_game_time(&self, world_id: &WorldId, time: GameTime);

    /// Advance game time by the specified hours and minutes.
    ///
    /// Returns the new game time, or `None` if the world doesn't exist.
    fn advance_game_time(&self, world_id: &WorldId, hours: i64, minutes: i64) -> Option<GameTime>;

    // === Conversation History ===

    /// Add a conversation entry to the world's history.
    ///
    /// Implementations should limit history size (e.g., keep last 30 entries).
    fn add_conversation(&self, world_id: &WorldId, entry: ConversationEntry);

    /// Get conversation history, optionally limited.
    ///
    /// If `limit` is `Some(n)`, returns at most `n` most recent entries.
    /// If `limit` is `None`, returns all entries.
    fn get_conversation_history(
        &self,
        world_id: &WorldId,
        limit: Option<usize>,
    ) -> Vec<ConversationEntry>;

    /// Clear all conversation history for a world.
    fn clear_conversation_history(&self, world_id: &WorldId);

    // === Pending Approvals ===

    /// Add an item pending DM approval.
    fn add_pending_approval(&self, world_id: &WorldId, item: PendingApprovalItem);

    /// Remove a pending approval by its ID.
    ///
    /// Returns the removed item if found.
    fn remove_pending_approval(
        &self,
        world_id: &WorldId,
        approval_id: &str,
    ) -> Option<PendingApprovalItem>;

    /// Get all pending approvals for a world.
    fn get_pending_approvals(&self, world_id: &WorldId) -> Vec<PendingApprovalItem>;

    // === Current Scene ===

    /// Get the current scene ID for a world.
    fn get_current_scene(&self, world_id: &WorldId) -> Option<String>;

    /// Set the current scene for a world.
    fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>);

    // === Directorial Context ===

    /// Get the DM's directorial context (runtime NPC guidance) for a world.
    fn get_directorial_context(&self, world_id: &WorldId) -> Option<DirectorialNotes>;

    /// Set the directorial context for a world.
    fn set_directorial_context(&self, world_id: &WorldId, notes: DirectorialNotes);

    /// Clear the directorial context for a world.
    fn clear_directorial_context(&self, world_id: &WorldId);

    // === Lifecycle ===

    /// Initialize state for a new world with the given starting time.
    ///
    /// This should be called when a world connection is established.
    fn initialize_world(&self, world_id: &WorldId, initial_time: GameTime);

    /// Clean up all state for a world.
    ///
    /// This should be called when a world connection is closed.
    fn cleanup_world(&self, world_id: &WorldId);

    /// Check if a world has been initialized.
    fn is_world_initialized(&self, world_id: &WorldId) -> bool;
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    pub WorldStatePort {}

    impl WorldStatePort for WorldStatePort {
        fn get_game_time(&self, world_id: &WorldId) -> Option<GameTime>;
        fn set_game_time(&self, world_id: &WorldId, time: GameTime);
        fn advance_game_time(&self, world_id: &WorldId, hours: i64, minutes: i64) -> Option<GameTime>;
        fn add_conversation(&self, world_id: &WorldId, entry: ConversationEntry);
        fn get_conversation_history(&self, world_id: &WorldId, limit: Option<usize>) -> Vec<ConversationEntry>;
        fn clear_conversation_history(&self, world_id: &WorldId);
        fn add_pending_approval(&self, world_id: &WorldId, item: PendingApprovalItem);
        fn remove_pending_approval(&self, world_id: &WorldId, approval_id: &str) -> Option<PendingApprovalItem>;
        fn get_pending_approvals(&self, world_id: &WorldId) -> Vec<PendingApprovalItem>;
        fn get_current_scene(&self, world_id: &WorldId) -> Option<String>;
        fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>);
        fn get_directorial_context(&self, world_id: &WorldId) -> Option<DirectorialNotes>;
        fn set_directorial_context(&self, world_id: &WorldId, notes: DirectorialNotes);
        fn clear_directorial_context(&self, world_id: &WorldId);
        fn initialize_world(&self, world_id: &WorldId, initial_time: GameTime);
        fn cleanup_world(&self, world_id: &WorldId);
        fn is_world_initialized(&self, world_id: &WorldId) -> bool;
    }
}
