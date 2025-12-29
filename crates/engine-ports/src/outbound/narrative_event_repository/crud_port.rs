//! Core CRUD and state management for NarrativeEvent entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{NarrativeEvent, NarrativeEventId, WorldId};

/// Core CRUD and state management operations for NarrativeEvent entities.
///
/// This trait covers:
/// - Basic entity operations (create, get, update, delete)
/// - List operations by world (all, active, favorites, pending)
/// - State toggles (favorite, active, triggered)
///
/// # Used By
/// - `NarrativeEventServiceImpl` - For all CRUD operations
/// - `TriggerEvaluationService` - For marking events as triggered
/// - `EventEffectExecutorService` - For getting and updating events
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait NarrativeEventCrudPort: Send + Sync {
    /// Create a new narrative event
    async fn create(&self, event: &NarrativeEvent) -> Result<()>;

    /// Get a narrative event by ID
    async fn get(&self, id: NarrativeEventId) -> Result<Option<NarrativeEvent>>;

    /// Update a narrative event
    async fn update(&self, event: &NarrativeEvent) -> Result<bool>;

    /// List all narrative events for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// List active narrative events for a world
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// List favorite narrative events for a world
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// List untriggered active events (for LLM context)
    async fn list_pending(&self, world_id: WorldId) -> Result<Vec<NarrativeEvent>>;

    /// Toggle favorite status
    async fn toggle_favorite(&self, id: NarrativeEventId) -> Result<bool>;

    /// Set active status
    async fn set_active(&self, id: NarrativeEventId, is_active: bool) -> Result<bool>;

    /// Mark event as triggered
    async fn mark_triggered(
        &self,
        id: NarrativeEventId,
        outcome_name: Option<String>,
    ) -> Result<bool>;

    /// Reset triggered status (for repeatable events)
    async fn reset_triggered(&self, id: NarrativeEventId) -> Result<bool>;

    /// Delete a narrative event
    async fn delete(&self, id: NarrativeEventId) -> Result<bool>;
}
