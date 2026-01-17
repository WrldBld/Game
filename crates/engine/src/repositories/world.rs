// World repository - methods for future world management
#![allow(dead_code)]

//! World entity operations.

use std::sync::Arc;
use wrldbldr_domain::{self as domain, WorldId};

use crate::infrastructure::ports::{ClockPort, RepoError, WorldRepo};

/// World entity operations.
///
/// Provides pure CRUD operations for World entities.
/// Time-related business logic is in `use_cases::time::TimeControl`.
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

    /// Access the underlying WorldRepo port for direct database operations.
    pub fn port(&self) -> &dyn WorldRepo {
        self.repo.as_ref()
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
}
