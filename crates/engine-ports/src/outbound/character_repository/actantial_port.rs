//! Actantial view management operations for Character entities.

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::value_objects::ActantialTarget;
use wrldbldr_domain::{ActantialRole, ActantialView, CharacterId, PlayerCharacterId, WantId};

/// Actantial view management operations for Character entities.
///
/// This trait covers:
/// - Managing VIEWS_AS_* edges between characters
/// - Actantial roles (Helper, Opponent, Sender, Receiver)
/// - Views toward both NPCs and PCs
///
/// # Used By
/// - `ActantialContextServiceImpl` - For building motivation context
/// - `CharacterServiceImpl` - For actantial view CRUD operations
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait CharacterActantialPort: Send + Sync {
    /// Add an actantial view toward an NPC (Helper, Opponent, Sender, Receiver)
    async fn add_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        view: &ActantialView,
    ) -> Result<()>;

    /// Add an actantial view toward a PC (Helper, Opponent, Sender, Receiver)
    ///
    /// This allows NPCs to view player characters in actantial roles.
    async fn add_actantial_view_to_pc(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: PlayerCharacterId,
        view: &ActantialView,
    ) -> Result<()>;

    /// Get all actantial views for a character (as subject)
    ///
    /// Returns views toward both NPCs and PCs using ActantialTarget.
    async fn get_actantial_views(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<(ActantialRole, ActantialTarget, ActantialView)>>;

    /// Remove an actantial view toward an NPC
    async fn remove_actantial_view(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: CharacterId,
        want_id: WantId,
    ) -> Result<()>;

    /// Remove an actantial view toward a PC
    async fn remove_actantial_view_to_pc(
        &self,
        subject_id: CharacterId,
        role: ActantialRole,
        target_id: PlayerCharacterId,
        want_id: WantId,
    ) -> Result<()>;
}
