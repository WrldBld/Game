//! Goal entity operations.

use std::sync::Arc;

use wrldbldr_domain::{self as domain, GoalId, WorldId};

use crate::infrastructure::ports::{GoalDetails, GoalRepo, RepoError};

/// Goal entity operations.
pub struct GoalRepository {
    repo: Arc<dyn GoalRepo>,
}

impl GoalRepository {
    pub fn new(repo: Arc<dyn GoalRepo>) -> Self {
        Self { repo }
    }

    // =========================================================================
    // Goal CRUD
    // =========================================================================

    /// Get a goal by ID with usage count.
    pub async fn get(&self, id: GoalId) -> Result<Option<GoalDetails>, RepoError> {
        self.repo.get(id).await
    }

    /// List goals in a world with usage counts.
    pub async fn list_in_world(&self, world_id: WorldId) -> Result<Vec<GoalDetails>, RepoError> {
        self.repo.list_in_world(world_id).await
    }

    /// Save (upsert) a goal.
    pub async fn save(&self, goal: &domain::Goal) -> Result<(), RepoError> {
        self.repo.save(goal).await
    }

    /// Delete a goal by ID.
    ///
    /// Uses DETACH DELETE to remove all relationships.
    pub async fn delete(&self, id: GoalId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }
}
