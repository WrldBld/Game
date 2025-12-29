//! NPC disposition and relationship operations.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::{CharacterId, PlayerCharacterId};
use wrldbldr_domain::value_objects::{DispositionLevel, NpcDispositionState};

/// NPC disposition and relationship operations.
///
/// This trait covers:
/// - Managing DISPOSITION_TOWARD edges to PlayerCharacter nodes
/// - Per-PC disposition tracking
/// - Default/global disposition management
///
/// # Used By
/// - `DispositionServiceImpl` - For disposition operations
/// - `DialogueServiceImpl` - For context building
/// - `StagingServiceImpl` - For NPC behavior determination
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait CharacterDispositionPort: Send + Sync {
    /// Get an NPC's disposition state toward a specific PC
    async fn get_disposition_toward_pc(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
    ) -> Result<Option<NpcDispositionState>>;

    /// Set/update an NPC's disposition state toward a specific PC
    async fn set_disposition_toward_pc(
        &self,
        disposition_state: &NpcDispositionState,
    ) -> Result<()>;

    /// Get disposition states for multiple NPCs toward a PC (for scene context)
    async fn get_scene_dispositions(
        &self,
        npc_ids: &[CharacterId],
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>>;

    /// Get all NPCs who have a relationship with a PC (for DM panel)
    async fn get_all_npc_dispositions_for_pc(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionState>>;

    /// Get the NPC's default/global disposition (from Character node)
    async fn get_default_disposition(&self, npc_id: CharacterId) -> Result<DispositionLevel>;

    /// Set the NPC's default/global disposition (on Character node)
    async fn set_default_disposition(
        &self,
        npc_id: CharacterId,
        disposition: DispositionLevel,
    ) -> Result<()>;
}
