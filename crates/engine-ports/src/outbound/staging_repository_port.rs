//! Staging repository port - persistence interface for NPC staging approvals
//!
//! Stagings are DM-approved NPC presence configurations for regions.
//! They are cached and expire based on game time TTL.

use async_trait::async_trait;
use anyhow::Result;

use wrldbldr_domain::entities::Staging;
use wrldbldr_domain::{CharacterId, GameTime, RegionId, StagingId};

/// Row data for staged NPC (from INCLUDES_NPC edge)
#[derive(Debug, Clone)]
pub struct StagedNpcRow {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

/// Repository port for staging persistence
#[async_trait]
pub trait StagingRepositoryPort: Send + Sync {
    /// Get the current active staging for a region (if any)
    async fn get_current(&self, region_id: RegionId) -> Result<Option<Staging>>;
    
    /// Get staging history for a region (most recent first)
    async fn get_history(&self, region_id: RegionId, limit: u32) -> Result<Vec<Staging>>;
    
    /// Get a staging by ID
    async fn get(&self, id: StagingId) -> Result<Option<Staging>>;
    
    /// Save a new staging (creates node and INCLUDES_NPC edges)
    /// Also creates HAS_STAGING edge to region
    /// Returns the staging ID
    async fn save(&self, staging: &Staging) -> Result<StagingId>;
    
    /// Check if a staging is still valid based on game time
    async fn is_valid(&self, staging_id: StagingId, current_game_time: &GameTime) -> Result<bool>;
    
    /// Invalidate all stagings for a region (sets is_active = false)
    async fn invalidate_all(&self, region_id: RegionId) -> Result<()>;
    
    /// Set a staging as the current active staging for its region
    /// (creates CURRENT_STAGING edge, removes any existing)
    async fn set_current(&self, staging_id: StagingId) -> Result<()>;
    
    /// Get all NPCs in a staging with their presence status
    async fn get_staged_npcs(&self, staging_id: StagingId) -> Result<Vec<StagedNpcRow>>;
}
