//! Flag entity module.
//!
//! Handles game flag operations for scene conditions and narrative triggers.

use std::collections::HashSet;
use std::sync::Arc;

use wrldbldr_domain::{PlayerCharacterId, WorldId};

use crate::infrastructure::ports::{FlagRepo, RepoError};

/// Flag entity - handles game flag operations.
pub struct Flag {
    repo: Arc<dyn FlagRepo>,
}

impl Flag {
    pub fn new(repo: Arc<dyn FlagRepo>) -> Self {
        Self { repo }
    }

    /// Get all set flags for a world (world-scoped flags).
    pub async fn get_world_flags(&self, world_id: WorldId) -> Result<Vec<String>, RepoError> {
        self.repo.get_world_flags(world_id).await
    }

    /// Get all set flags for a player character (PC-scoped flags).
    pub async fn get_pc_flags(&self, pc_id: PlayerCharacterId) -> Result<Vec<String>, RepoError> {
        self.repo.get_pc_flags(pc_id).await
    }

    /// Get all flags relevant to a PC (combines world and PC-scoped flags).
    /// 
    /// Uses HashSet to deduplicate flags that may exist at both scopes.
    pub async fn get_all_flags_for_pc(
        &self,
        world_id: WorldId,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<String>, RepoError> {
        let world_flags = self.get_world_flags(world_id).await?;
        let pc_flags = self.get_pc_flags(pc_id).await?;
        
        // Deduplicate using HashSet
        let mut unique_flags: HashSet<String> = HashSet::with_capacity(world_flags.len() + pc_flags.len());
        unique_flags.extend(world_flags);
        unique_flags.extend(pc_flags);
        
        Ok(unique_flags.into_iter().collect())
    }

    /// Set a world-scoped flag.
    pub async fn set_world_flag(&self, world_id: WorldId, flag_name: &str) -> Result<(), RepoError> {
        self.repo.set_world_flag(world_id, flag_name).await
    }

    /// Unset a world-scoped flag.
    pub async fn unset_world_flag(&self, world_id: WorldId, flag_name: &str) -> Result<(), RepoError> {
        self.repo.unset_world_flag(world_id, flag_name).await
    }

    /// Set a PC-scoped flag.
    pub async fn set_pc_flag(&self, pc_id: PlayerCharacterId, flag_name: &str) -> Result<(), RepoError> {
        self.repo.set_pc_flag(pc_id, flag_name).await
    }

    /// Unset a PC-scoped flag.
    pub async fn unset_pc_flag(&self, pc_id: PlayerCharacterId, flag_name: &str) -> Result<(), RepoError> {
        self.repo.unset_pc_flag(pc_id, flag_name).await
    }

    /// Check if a world-scoped flag is set.
    pub async fn is_world_flag_set(&self, world_id: WorldId, flag_name: &str) -> Result<bool, RepoError> {
        self.repo.is_world_flag_set(world_id, flag_name).await
    }

    /// Check if a PC-scoped flag is set.
    pub async fn is_pc_flag_set(&self, pc_id: PlayerCharacterId, flag_name: &str) -> Result<bool, RepoError> {
        self.repo.is_pc_flag_set(pc_id, flag_name).await
    }
}
