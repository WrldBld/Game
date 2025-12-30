//! World Conversation Port - Conversation history management.
//!
//! This port handles conversation history for LLM context.

use wrldbldr_domain::value_objects::ConversationEntry;
use wrldbldr_domain::WorldId;

/// Port for managing conversation history within a world.
///
/// Conversation history is used to provide context for LLM prompts,
/// helping maintain narrative continuity in NPC responses.
///
/// All methods are synchronous as they operate on in-memory state.
/// Implementations must be thread-safe (Send + Sync).
pub trait WorldConversationPort: Send + Sync {
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
}

#[cfg(any(test, feature = "testing"))]
mockall::mock! {
    pub WorldConversationPort {}

    impl WorldConversationPort for WorldConversationPort {
        fn add_conversation(&self, world_id: &WorldId, entry: ConversationEntry);
        fn get_conversation_history(&self, world_id: &WorldId, limit: Option<usize>) -> Vec<ConversationEntry>;
        fn clear_conversation_history(&self, world_id: &WorldId);
    }
}
