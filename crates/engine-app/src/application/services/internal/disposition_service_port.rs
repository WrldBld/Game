//! Disposition service port - Interface for NPC disposition operations
//!
//! This port abstracts NPC mood/disposition logic from infrastructure,
//! allowing adapters to depend on a trait rather than the concrete
//! DispositionService implementation in engine-app.
//!
//! ## Domain Concepts
//!
//! - **Disposition**: How an NPC emotionally feels about a specific PC (Tier 1)
//! - **Relationship**: Social distance/familiarity between NPC and PC (Tier 2)
//!
//! See `disposition.rs` in domain for the full Three-Tier Emotional Model.

use anyhow::Result;
use async_trait::async_trait;

use wrldbldr_domain::value_objects::{
    DispositionLevel, InteractionOutcome, NpcDispositionState, RelationshipLevel,
};
use wrldbldr_domain::{CharacterId, PlayerCharacterId};

/// Port for disposition service operations.
///
/// This trait defines the application-level use cases for NPC disposition
/// and relationship management. Implementations coordinate domain logic
/// with repository operations.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait DispositionServicePort: Send + Sync {
    /// Get an NPC's disposition toward a specific PC.
    ///
    /// Returns the disposition state if it exists, or creates a default one
    /// from the NPC's default_disposition.
    async fn get_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<NpcDispositionState>;

    /// Set an NPC's disposition toward a PC (for DM control).
    ///
    /// This directly sets the disposition level, bypassing the normal
    /// sentiment-based calculation.
    async fn set_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        disposition: DispositionLevel,
        reason: Option<String>,
    ) -> Result<NpcDispositionState>;

    /// Apply an interaction outcome to update disposition and relationship.
    ///
    /// This is the primary way dispositions change during gameplay:
    /// - Challenge results affect both disposition and relationship
    /// - Positive/negative interactions adjust sentiment
    async fn apply_interaction(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        outcome: InteractionOutcome,
    ) -> Result<NpcDispositionState>;

    /// Get dispositions for multiple NPCs in a scene.
    ///
    /// Used when loading scene context to get all NPC dispositions
    /// toward the current PC efficiently.
    async fn get_scene_dispositions(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>>;

    /// Get all NPC relationships for a PC (for DM panel).
    ///
    /// Returns all known disposition states for NPCs that have
    /// interacted with the given PC.
    async fn get_all_relationships(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>>;

    /// Get an NPC's default disposition.
    ///
    /// This is the baseline disposition used when creating new
    /// NPC-PC disposition states.
    async fn get_default_disposition(&self, npc_id: CharacterId) -> Result<DispositionLevel>;

    /// Set an NPC's default disposition.
    ///
    /// Changes the NPC's baseline disposition for new PC interactions.
    /// Does not affect existing disposition states.
    async fn set_default_disposition(
        &self,
        npc_id: CharacterId,
        disposition: DispositionLevel,
    ) -> Result<()>;

    /// Set an NPC's relationship level toward a PC (for DM control).
    ///
    /// This directly sets the relationship level and adjusts the
    /// underlying relationship points to match.
    async fn set_relationship(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        relationship: RelationshipLevel,
    ) -> Result<NpcDispositionState>;
}
