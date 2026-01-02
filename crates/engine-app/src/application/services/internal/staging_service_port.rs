//! Staging service port - Interface for NPC staging operations
//!
//! This port abstracts NPC staging business logic from infrastructure,
//! allowing adapters to depend on the port trait rather than
//! concrete service implementations.
//!
//! # Architecture Note
//!
//! The staging system manages NPC presence in regions. It involves:
//! - Checking for existing valid stagings
//! - Generating proposals (rule-based and LLM-assisted)
//! - DM approval workflow
//! - Persisting approved stagings

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::entities::{Staging, StagingSource};
use wrldbldr_domain::value_objects::StagingContext;
use wrldbldr_domain::{CharacterId, GameTime, LocationId, RegionId, WorldId};

/// A staging proposal with both rule-based and LLM options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingProposal {
    /// Request ID for tracking this proposal through the approval flow
    pub request_id: String,
    /// Region this staging is for
    pub region_id: String,
    /// Location containing the region
    pub location_id: String,
    /// World ID
    pub world_id: String,
    /// Rule-based NPC suggestions
    pub rule_based_npcs: Vec<StagedNpcProposal>,
    /// LLM-based NPC suggestions (may be same as rule-based if LLM agrees)
    pub llm_based_npcs: Vec<StagedNpcProposal>,
    /// Default TTL from location settings
    pub default_ttl_hours: i32,
    /// Staging context used for generation
    pub context: StagingContext,
}

/// A proposed NPC for staging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedNpcProposal {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    #[serde(default)]
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

/// Data for an approved NPC in staging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovedNpc {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

/// Port for staging service operations
///
/// This trait defines the application use cases for NPC staging management,
/// including checking current stagings, generating proposals, and approving
/// stagings.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait StagingServicePort: Send + Sync {
    /// Get the current valid staging for a region
    ///
    /// Returns None if no staging exists or the current staging has expired.
    async fn get_current_staging(
        &self,
        region_id: RegionId,
        game_time: GameTime,
    ) -> Result<Option<Staging>>;

    /// Generate a staging proposal for a region
    ///
    /// Creates both rule-based and LLM-based suggestions for DM approval.
    /// The proposal includes context about the region, time, and recent events.
    async fn generate_proposal(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_id: LocationId,
        location_name: String,
        game_time: GameTime,
        ttl_hours: i32,
        dm_guidance: Option<String>,
    ) -> Result<StagingProposal>;

    /// Approve a staging proposal and persist it
    ///
    /// Called when DM approves a staging with their chosen NPCs.
    /// Invalidates any existing stagings for the region.
    async fn approve_staging(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: GameTime,
        approved_npcs: Vec<ApprovedNpc>,
        ttl_hours: i32,
        source: StagingSource,
        approved_by: String,
        dm_guidance: Option<String>,
    ) -> Result<Staging>;
}
