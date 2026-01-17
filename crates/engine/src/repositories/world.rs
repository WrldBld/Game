//! World entity operations.

use std::sync::Arc;
use wrldbldr_domain::{
    self as domain, GameTime, TimeAdvanceReason, TimeAdvanceResult, TimeMode, WorldId,
};

use crate::infrastructure::ports::{ClockPort, RepoError, WorldRepo};

/// World entity operations.
pub struct WorldRepository {
    repo: Arc<dyn WorldRepo>,
    clock: Arc<dyn ClockPort>,
}

/// Error type for world operations.
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("World not found: {0}")]
    NotFound(WorldId),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

impl WorldRepository {
    pub fn new(repo: Arc<dyn WorldRepo>, clock: Arc<dyn ClockPort>) -> Self {
        Self { repo, clock }
    }

    pub async fn get(&self, id: WorldId) -> Result<Option<domain::World>, RepoError> {
        self.repo.get(id).await
    }

    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.clock.now()
    }

    pub async fn save(&self, world: &domain::World) -> Result<(), RepoError> {
        self.repo.save(world).await
    }

    pub async fn list_all(&self) -> Result<Vec<domain::World>, RepoError> {
        self.repo.list_all().await
    }

    pub async fn delete(&self, id: WorldId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }

    // =========================================================================
    // Game Time Operations
    // =========================================================================

    /// Get the current game time for a world.
    pub async fn get_current_time(&self, id: WorldId) -> Result<GameTime, WorldError> {
        let world = self.repo.get(id).await?.ok_or(WorldError::NotFound(id))?;
        Ok(world.game_time().clone())
    }

    /// Advance game time by a number of minutes.
    ///
    /// Returns the time advance result with previous/new time info.
    pub async fn advance_time(
        &self,
        id: WorldId,
        minutes: u32,
        reason: TimeAdvanceReason,
    ) -> Result<TimeAdvanceResult, WorldError> {
        let mut world = self.repo.get(id).await?.ok_or(WorldError::NotFound(id))?;
        let now = self.clock.now();

        let result = world.advance_time(minutes, reason, now);

        self.repo.save(&world).await?;

        Ok(result)
    }

    /// Set the game time to a specific value.
    ///
    /// Use with caution - this can cause time to go backwards.
    pub async fn set_time(&self, id: WorldId, game_time: GameTime) -> Result<(), WorldError> {
        let mut world = self.repo.get(id).await?.ok_or(WorldError::NotFound(id))?;

        *world.game_time_mut() = game_time;

        self.repo.save(&world).await?;

        Ok(())
    }

    /// Set the time mode (Manual, Suggested, Auto).
    pub async fn set_time_mode(&self, id: WorldId, mode: TimeMode) -> Result<(), WorldError> {
        let mut world = self.repo.get(id).await?.ok_or(WorldError::NotFound(id))?;

        world.set_time_mode(mode, self.clock.now());

        self.repo.save(&world).await?;

        Ok(())
    }

    /// Get the current time mode for a world.
    pub async fn get_time_mode(&self, id: WorldId) -> Result<TimeMode, WorldError> {
        let world = self.repo.get(id).await?.ok_or(WorldError::NotFound(id))?;
        Ok(world.time_config().mode)
    }
}
