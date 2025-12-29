//! Port for persisting DM directorial context.
//!
//! Directorial context is set by the DM and provides guidance for NPC behavior
//! and narrative generation. This port allows the context to be persisted
//! so it survives server restarts.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::value_objects::DirectorialNotes;
use wrldbldr_domain::WorldId;

/// Port for persisting DM directorial context
#[async_trait]
pub trait DirectorialContextRepositoryPort: Send + Sync {
    /// Get the directorial context for a world
    async fn get(&self, world_id: &WorldId) -> Result<Option<DirectorialNotes>>;

    /// Save/update the directorial context for a world
    async fn save(&self, world_id: &WorldId, context: &DirectorialNotes) -> Result<()>;

    /// Delete the directorial context for a world
    async fn delete(&self, world_id: &WorldId) -> Result<()>;
}
