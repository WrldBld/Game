//! Narrative entity CRUD operations.
//!
//! Simple repository wrappers for narrative events, event chains, story events,
//! and conversation management. Uses only NarrativeRepo.

use std::sync::Arc;

use wrldbldr_domain::{
    self as domain, CharacterId, EventChainId, NarrativeEventId, PlayerCharacterId, RegionId,
    StoryEventId, WorldId,
};

use crate::infrastructure::ports::{NarrativeRepo, RepoError};

/// Narrative entity CRUD operations.
///
/// Provides simple repository access for narrative events, event chains,
/// story events, and conversation tracking. Does not perform complex
/// trigger evaluation - see `NarrativeOps` for that.
pub struct Narrative {
    repo: Arc<dyn NarrativeRepo>,
}

impl Narrative {
    pub fn new(repo: Arc<dyn NarrativeRepo>) -> Self {
        Self { repo }
    }

    // =========================================================================
    // Narrative Events
    // =========================================================================

    pub async fn get_event(
        &self,
        id: NarrativeEventId,
    ) -> Result<Option<domain::NarrativeEvent>, RepoError> {
        self.repo.get_event(id).await
    }

    pub async fn save_event(&self, event: &domain::NarrativeEvent) -> Result<(), RepoError> {
        self.repo.save_event(event).await
    }

    pub async fn list_events(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        self.repo.list_events_for_world(world_id).await
    }

    /// Delete a narrative event by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete_event(&self, id: NarrativeEventId) -> Result<(), RepoError> {
        self.repo.delete_event(id).await
    }

    // =========================================================================
    // Event Chains
    // =========================================================================

    pub async fn get_chain(
        &self,
        id: EventChainId,
    ) -> Result<Option<domain::EventChain>, RepoError> {
        self.repo.get_chain(id).await
    }

    pub async fn save_chain(&self, chain: &domain::EventChain) -> Result<(), RepoError> {
        self.repo.save_chain(chain).await
    }

    /// Delete an event chain by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete_chain(&self, id: EventChainId) -> Result<(), RepoError> {
        self.repo.delete_chain(id).await
    }

    pub async fn list_chains_for_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<domain::EventChain>, RepoError> {
        self.repo.list_chains_for_world(world_id).await
    }

    // =========================================================================
    // Story Events
    // =========================================================================

    pub async fn get_story_event(
        &self,
        id: StoryEventId,
    ) -> Result<Option<domain::StoryEvent>, RepoError> {
        self.repo.get_story_event(id).await
    }

    pub async fn save_story_event(&self, event: &domain::StoryEvent) -> Result<(), RepoError> {
        self.repo.save_story_event(event).await
    }

    /// Delete a story event by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete_story_event(&self, id: StoryEventId) -> Result<(), RepoError> {
        self.repo.delete_story_event(id).await
    }

    pub async fn list_story_events(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> Result<Vec<domain::StoryEvent>, RepoError> {
        self.repo.list_story_events(world_id, limit).await
    }

    // =========================================================================
    // Dialogue History
    // =========================================================================

    /// Get dialogue history between a PC and NPC.
    ///
    /// Returns DialogueExchange story events in reverse chronological order.
    pub async fn get_dialogues_with_npc(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<domain::StoryEvent>, RepoError> {
        self.repo.get_dialogues_with_npc(pc_id, npc_id, limit).await
    }

    /// Get conversation turns for LLM context.
    ///
    /// Returns ConversationTurn records from the active conversation between
    /// PC and NPC, in chronological order (oldest first). These are formatted
    /// for use in LLM prompts.
    ///
    /// # Arguments
    /// * `pc_id` - The player character ID
    /// * `npc_id` - The NPC character ID
    /// * `limit` - Maximum number of turns to return
    pub async fn get_conversation_turns(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        limit: usize,
    ) -> Result<Vec<domain::ConversationTurn>, RepoError> {
        let records = self
            .repo
            .get_conversation_turns(pc_id, npc_id, limit)
            .await?;

        // Convert ConversationTurnRecord to ConversationTurn
        let turns = records
            .into_iter()
            .map(|r| domain::ConversationTurn {
                speaker: r.speaker,
                text: r.text,
            })
            .collect();

        Ok(turns)
    }

    /// Get the active conversation ID between PC and NPC (if one exists).
    pub async fn get_active_conversation_id(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<uuid::Uuid>, RepoError> {
        self.repo.get_active_conversation_id(pc_id, npc_id).await
    }

    /// Check if a specific conversation is still active (not ended).
    ///
    /// Returns true if the conversation exists and has is_active = true.
    /// Returns false if the conversation doesn't exist or has been ended.
    pub async fn is_conversation_active(
        &self,
        conversation_id: uuid::Uuid,
    ) -> Result<bool, RepoError> {
        self.repo.is_conversation_active(conversation_id).await
    }

    /// End a conversation by setting is_active = false.
    ///
    /// This marks the conversation as ended so it cannot be resumed.
    /// Returns Ok(true) if the conversation was found and ended,
    /// Ok(false) if the conversation was not found or already ended.
    pub async fn end_conversation(&self, conversation_id: uuid::Uuid) -> Result<bool, RepoError> {
        self.repo.end_conversation(conversation_id).await
    }

    /// End the active conversation between PC and NPC (if one exists).
    ///
    /// Finds the active conversation and marks it as ended.
    /// Returns the conversation ID if one was ended, None if no active conversation.
    pub async fn end_active_conversation(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<uuid::Uuid>, RepoError> {
        self.repo.end_active_conversation(pc_id, npc_id).await
    }

    // =========================================================================
    // Triggers (Simple reads only)
    // =========================================================================

    pub async fn get_triggers_for_region(
        &self,
        world_id: WorldId,
        region_id: RegionId,
    ) -> Result<Vec<domain::NarrativeEvent>, RepoError> {
        self.repo.get_triggers_for_region(world_id, region_id).await
    }

    /// Set a narrative event's active status.
    ///
    /// Used by EnableEvent/DisableEvent effects.
    pub async fn set_event_active(
        &self,
        id: NarrativeEventId,
        active: bool,
    ) -> Result<(), RepoError> {
        self.repo.set_event_active(id, active).await
    }
}
