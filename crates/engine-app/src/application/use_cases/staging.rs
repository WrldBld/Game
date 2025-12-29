//! Staging Approval Use Case
//!
//! Handles DM approval of NPC presence staging, regeneration requests,
//! and proactive region staging.
//!
//! # Responsibilities
//!
//! - Process DM approval of staging proposals
//! - Regenerate LLM suggestions with DM guidance
//! - Pre-stage regions before player arrival
//! - Notify waiting PCs when staging is ready
//! - Build and broadcast scene changes

use std::sync::Arc;
use tracing::{info, warn};

use wrldbldr_domain::entities::{StagedNpc, StagingSource};
use wrldbldr_domain::{CharacterId, GameTime, LocationId, PlayerCharacterId, RegionId, WorldId};
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, CharacterRepositoryPort, GameEvent, LocationRepositoryPort, NpcPresenceData,
    RegionRepositoryPort, StagingReadyEvent, WaitingPcData,
};

use super::builders::SceneBuilder;
use super::errors::StagingError;
use super::movement::{StagingServicePort, StagingStatePort};

// =============================================================================
// Input/Output Types
// =============================================================================

/// Input for approving a staging proposal
#[derive(Debug, Clone)]
pub struct ApproveInput {
    /// Request ID of the pending staging
    pub request_id: String,
    /// Approved NPCs with presence decisions
    pub approved_npcs: Vec<ApprovedNpc>,
    /// TTL in hours for the staging
    pub ttl_hours: i32,
    /// How this staging was finalized: rule, llm, or custom
    pub source: StagingApprovalSource,
}

/// An approved NPC with presence decision
#[derive(Debug, Clone)]
pub struct ApprovedNpc {
    pub character_id: CharacterId,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: Option<String>,
}

/// Source of staging decision
#[derive(Debug, Clone, Copy)]
pub enum StagingApprovalSource {
    RuleBased,
    LlmBased,
    DmCustomized,
}

impl From<StagingApprovalSource> for StagingSource {
    fn from(source: StagingApprovalSource) -> Self {
        match source {
            StagingApprovalSource::RuleBased => StagingSource::RuleBased,
            StagingApprovalSource::LlmBased => StagingSource::LlmBased,
            StagingApprovalSource::DmCustomized => StagingSource::DmCustomized,
        }
    }
}

/// Result of approving staging
#[derive(Debug, Clone)]
pub struct ApproveResult {
    /// NPCs now present in the region
    pub npcs_present: Vec<NpcPresenceData>,
    /// Number of waiting PCs that were notified
    pub notified_pc_count: usize,
}

/// Input for regenerating staging suggestions
#[derive(Debug, Clone)]
pub struct RegenerateInput {
    /// Request ID of the pending staging
    pub request_id: String,
    /// DM guidance for the LLM
    pub guidance: String,
}

/// Regenerated NPC suggestion
#[derive(Debug, Clone)]
pub struct RegeneratedNpc {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

/// Result of regenerating suggestions
#[derive(Debug, Clone)]
pub struct RegenerateResult {
    /// New LLM-based suggestions
    pub llm_based_npcs: Vec<RegeneratedNpc>,
}

/// Input for pre-staging a region
#[derive(Debug, Clone)]
pub struct PreStageInput {
    /// Region to pre-stage
    pub region_id: RegionId,
    /// NPCs to stage
    pub npcs: Vec<ApprovedNpc>,
    /// TTL in hours
    pub ttl_hours: i32,
}

/// Result of pre-staging
#[derive(Debug, Clone)]
pub struct PreStageResult {
    /// NPCs now present in the region
    pub npcs_present: Vec<NpcPresenceData>,
}

// =============================================================================
// Pending Staging Port Extension
// =============================================================================

/// Extended port for staging state management (adds operations needed by this use case)
///
/// ARCHITECTURE NOTE: This port is defined in engine-app rather than engine-ports
/// because it depends on use-case-specific DTOs (PendingStagingInfo, RegeneratedNpc,
/// WaitingPcInfo, ProposedNpc) that are defined in this crate. Moving to engine-ports
/// would create circular dependencies. This is an approved deviation from the
/// standard hexagonal port placement.
#[async_trait::async_trait]
pub trait StagingStateExtPort: StagingStatePort {
    /// Get a pending staging by request ID
    fn get_pending_staging(
        &self,
        world_id: &WorldId,
        request_id: &str,
    ) -> Option<PendingStagingInfo>;

    /// Remove a pending staging
    fn remove_pending_staging(&self, world_id: &WorldId, request_id: &str);

    /// Update the LLM suggestions for a pending staging
    fn update_llm_suggestions(
        &self,
        world_id: &WorldId,
        request_id: &str,
        npcs: Vec<RegeneratedNpc>,
    );
}

/// Information about a pending staging
#[derive(Debug, Clone)]
pub struct PendingStagingInfo {
    pub request_id: String,
    pub world_id: WorldId,
    pub region_id: RegionId,
    pub location_id: LocationId,
    pub region_name: String,
    pub location_name: String,
    pub waiting_pcs: Vec<WaitingPcInfo>,
    pub rule_based_npcs: Vec<ProposedNpc>,
    pub llm_based_npcs: Vec<ProposedNpc>,
}

/// A waiting PC
#[derive(Debug, Clone)]
pub struct WaitingPcInfo {
    pub pc_id: PlayerCharacterId,
    pub pc_name: String,
    pub user_id: String,
}

/// A proposed NPC from the staging proposal
#[derive(Debug, Clone)]
pub struct ProposedNpc {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

// =============================================================================
// Staging Service Port Extension
// =============================================================================

/// Extended staging service port with additional operations
#[async_trait::async_trait]
pub trait StagingServiceExtPort: StagingServicePort {
    /// Approve staging and persist it
    async fn approve_staging(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: &GameTime,
        approved_npcs: Vec<ApprovedNpcData>,
        ttl_hours: i32,
        source: StagingSource,
        approved_by: &str,
    ) -> Result<Vec<StagedNpc>, String>;

    /// Regenerate LLM suggestions with guidance
    async fn regenerate_suggestions(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        location_name: &str,
        game_time: &GameTime,
        guidance: &str,
    ) -> Result<Vec<RegeneratedNpc>, String>;

    /// Pre-stage a region
    async fn pre_stage_region(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
        game_time: &GameTime,
        npcs: Vec<ApprovedNpcData>,
        ttl_hours: i32,
        dm_user_id: &str,
    ) -> Result<Vec<StagedNpc>, String>;
}

/// Approved NPC data for the service
#[derive(Debug, Clone)]
pub struct ApprovedNpcData {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
}

// =============================================================================
// Staging Approval Use Case
// =============================================================================

/// Use case for staging approval operations
///
/// Coordinates DM staging approval, regeneration, and pre-staging.
pub struct StagingApprovalUseCase {
    staging_service: Arc<dyn StagingServiceExtPort>,
    staging_state: Arc<dyn StagingStateExtPort>,
    character_repo: Arc<dyn CharacterRepositoryPort>,
    region_repo: Arc<dyn RegionRepositoryPort>,
    location_repo: Arc<dyn LocationRepositoryPort>,
    broadcast: Arc<dyn BroadcastPort>,
    scene_builder: Arc<SceneBuilder>,
}

impl StagingApprovalUseCase {
    /// Create a new StagingApprovalUseCase with all dependencies
    pub fn new(
        staging_service: Arc<dyn StagingServiceExtPort>,
        staging_state: Arc<dyn StagingStateExtPort>,
        character_repo: Arc<dyn CharacterRepositoryPort>,
        region_repo: Arc<dyn RegionRepositoryPort>,
        location_repo: Arc<dyn LocationRepositoryPort>,
        broadcast: Arc<dyn BroadcastPort>,
        scene_builder: Arc<SceneBuilder>,
    ) -> Self {
        Self {
            staging_service,
            staging_state,
            character_repo,
            region_repo,
            location_repo,
            broadcast,
            scene_builder,
        }
    }

    /// Approve a staging proposal
    ///
    /// DM approves a staging proposal with their chosen NPCs.
    /// Sends SceneChanged to all waiting PCs.
    pub async fn approve(
        &self,
        ctx: UseCaseContext,
        input: ApproveInput,
    ) -> Result<ApproveResult, StagingError> {
        // Get the pending staging
        let pending = self
            .staging_state
            .get_pending_staging(&ctx.world_id, &input.request_id)
            .ok_or_else(|| StagingError::PendingNotFound(input.request_id.clone()))?;

        // Get game time
        let game_time = self
            .staging_state
            .get_game_time(&ctx.world_id)
            .unwrap_or_default();

        // Build approved NPC data with enriched character info
        let approved_npc_data = self
            .build_approved_npc_data(&input.approved_npcs, &pending)
            .await;

        // Approve the staging
        let staged_npcs = self
            .staging_service
            .approve_staging(
                pending.region_id,
                pending.location_id,
                pending.world_id,
                &game_time,
                approved_npc_data,
                input.ttl_hours,
                input.source.into(),
                &ctx.user_id,
            )
            .await
            .map_err(|e| StagingError::ApprovalFailed(e))?;

        // Build NPC presence list
        let npcs_present: Vec<NpcPresenceData> = staged_npcs
            .iter()
            .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
            .map(|npc| NpcPresenceData {
                character_id: npc.character_id,
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
            })
            .collect();

        // Broadcast StagingReady to DMs
        let staging_ready_event = StagingReadyEvent {
            region_id: pending.region_id,
            npcs_present: npcs_present.clone(),
            waiting_pcs: pending
                .waiting_pcs
                .iter()
                .map(|pc| WaitingPcData {
                    pc_id: pc.pc_id,
                    pc_name: pc.pc_name.clone(),
                    user_id: pc.user_id.clone(),
                })
                .collect(),
        };
        self.broadcast
            .broadcast(ctx.world_id, GameEvent::StagingReady(staging_ready_event))
            .await;

        // Send SceneChanged to each waiting PC
        let notified_count = self
            .notify_waiting_pcs(&ctx.world_id, &pending, &staged_npcs)
            .await;

        // Remove the pending staging
        self.staging_state
            .remove_pending_staging(&ctx.world_id, &input.request_id);

        info!(
            request_id = %input.request_id,
            region_id = %pending.region_id,
            waiting_pcs = notified_count,
            "Staging approved and sent to waiting PCs"
        );

        Ok(ApproveResult {
            npcs_present,
            notified_pc_count: notified_count,
        })
    }

    /// Regenerate LLM suggestions with DM guidance
    pub async fn regenerate(
        &self,
        ctx: UseCaseContext,
        input: RegenerateInput,
    ) -> Result<RegenerateResult, StagingError> {
        // Get the pending staging
        let pending = self
            .staging_state
            .get_pending_staging(&ctx.world_id, &input.request_id)
            .ok_or_else(|| StagingError::PendingNotFound(input.request_id.clone()))?;

        // Get game time
        let game_time = self
            .staging_state
            .get_game_time(&ctx.world_id)
            .unwrap_or_default();

        // Regenerate suggestions
        let new_suggestions = self
            .staging_service
            .regenerate_suggestions(
                pending.world_id,
                pending.region_id,
                &pending.location_name,
                &game_time,
                &input.guidance,
            )
            .await
            .map_err(|e| StagingError::RegenerationFailed(e))?;

        // Update the pending staging with new suggestions
        self.staging_state.update_llm_suggestions(
            &ctx.world_id,
            &input.request_id,
            new_suggestions.clone(),
        );

        info!(
            request_id = %input.request_id,
            new_count = new_suggestions.len(),
            "Staging suggestions regenerated"
        );

        Ok(RegenerateResult {
            llm_based_npcs: new_suggestions,
        })
    }

    /// Pre-stage a region before player arrival
    pub async fn pre_stage(
        &self,
        ctx: UseCaseContext,
        input: PreStageInput,
    ) -> Result<PreStageResult, StagingError> {
        // Get region and location
        let region = self
            .region_repo
            .get(input.region_id)
            .await
            .map_err(|e| StagingError::Database(e.to_string()))?
            .ok_or(StagingError::RegionNotFound(input.region_id))?;

        let location = self
            .location_repo
            .get(region.location_id)
            .await
            .map_err(|e| StagingError::Database(e.to_string()))?
            .ok_or(StagingError::RegionNotFound(input.region_id))?; // Location error

        // Get game time
        let game_time = self
            .staging_state
            .get_game_time(&ctx.world_id)
            .unwrap_or_default();

        // Build approved NPC data with character info
        let mut approved_npc_data = Vec::new();
        for npc in &input.npcs {
            let (name, sprite, portrait) = self.fetch_character_info(npc.character_id).await;
            approved_npc_data.push(ApprovedNpcData {
                character_id: npc.character_id,
                name,
                sprite_asset: sprite,
                portrait_asset: portrait,
                is_present: npc.is_present,
                is_hidden_from_players: npc.is_hidden_from_players,
                reasoning: npc
                    .reasoning
                    .clone()
                    .unwrap_or_else(|| "Pre-staged by DM".to_string()),
            });
        }

        // Pre-stage the region
        let staged_npcs = self
            .staging_service
            .pre_stage_region(
                input.region_id,
                region.location_id,
                location.world_id,
                &game_time,
                approved_npc_data,
                input.ttl_hours,
                &ctx.user_id,
            )
            .await
            .map_err(|e| StagingError::PreStagingFailed(e))?;

        // Build NPC presence list
        let npcs_present: Vec<NpcPresenceData> = staged_npcs
            .iter()
            .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
            .map(|npc| NpcPresenceData {
                character_id: npc.character_id,
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
            })
            .collect();

        // Broadcast StagingReady to DMs
        let staging_ready_event = StagingReadyEvent {
            region_id: input.region_id,
            npcs_present: npcs_present.clone(),
            waiting_pcs: vec![], // No waiting PCs for pre-staging
        };
        self.broadcast
            .broadcast(ctx.world_id, GameEvent::StagingReady(staging_ready_event))
            .await;

        info!(
            region_id = %input.region_id,
            npc_count = staged_npcs.len(),
            "Region pre-staged successfully"
        );

        Ok(PreStageResult { npcs_present })
    }

    /// Build approved NPC data with enriched character info
    async fn build_approved_npc_data(
        &self,
        approved_npcs: &[ApprovedNpc],
        pending: &PendingStagingInfo,
    ) -> Vec<ApprovedNpcData> {
        let mut result = Vec::new();

        for npc in approved_npcs {
            // Try to find in proposal first for cached name/assets
            let (name, sprite, portrait) = pending
                .rule_based_npcs
                .iter()
                .chain(pending.llm_based_npcs.iter())
                .find(|n| n.character_id == npc.character_id.to_string())
                .map(|n| {
                    (
                        n.name.clone(),
                        n.sprite_asset.clone(),
                        n.portrait_asset.clone(),
                    )
                })
                .unwrap_or_else(|| {
                    // Fall back to fetching from repo (blocking in async context is ok for fallback)
                    ("Unknown".to_string(), None, None)
                });

            result.push(ApprovedNpcData {
                character_id: npc.character_id,
                name,
                sprite_asset: sprite,
                portrait_asset: portrait,
                is_present: npc.is_present,
                is_hidden_from_players: npc.is_hidden_from_players,
                reasoning: npc
                    .reasoning
                    .clone()
                    .unwrap_or_else(|| "DM approved".to_string()),
            });
        }

        result
    }

    /// Fetch character info from repository
    async fn fetch_character_info(
        &self,
        character_id: CharacterId,
    ) -> (String, Option<String>, Option<String>) {
        match self.character_repo.get(character_id).await {
            Ok(Some(c)) => (c.name, c.sprite_asset, c.portrait_asset),
            _ => ("Unknown".to_string(), None, None),
        }
    }

    /// Notify waiting PCs with scene changed events
    async fn notify_waiting_pcs(
        &self,
        world_id: &WorldId,
        pending: &PendingStagingInfo,
        staged_npcs: &[StagedNpc],
    ) -> usize {
        let mut notified = 0;

        // Get region and location for scene building
        let region = match self.region_repo.get(pending.region_id).await {
            Ok(Some(r)) => r,
            _ => {
                warn!(
                    region_id = %pending.region_id,
                    "Failed to get region for scene building"
                );
                return 0;
            }
        };

        let location = match self.location_repo.get(pending.location_id).await {
            Ok(Some(l)) => l,
            _ => {
                warn!(
                    location_id = %pending.location_id,
                    "Failed to get location for scene building"
                );
                return 0;
            }
        };

        // Send SceneChanged to each waiting PC
        for waiting_pc in &pending.waiting_pcs {
            let scene_event = self
                .scene_builder
                .build_with_entities(waiting_pc.pc_id, &region, &location, staged_npcs)
                .await;

            self.broadcast
                .broadcast(
                    *world_id,
                    GameEvent::SceneChanged {
                        user_id: waiting_pc.user_id.clone(),
                        event: scene_event,
                    },
                )
                .await;

            notified += 1;
        }

        notified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_staging_approval_source_conversion() {
        assert!(matches!(
            StagingSource::from(StagingApprovalSource::RuleBased),
            StagingSource::RuleBased
        ));
        assert!(matches!(
            StagingSource::from(StagingApprovalSource::LlmBased),
            StagingSource::LlmBased
        ));
        assert!(matches!(
            StagingSource::from(StagingApprovalSource::DmCustomized),
            StagingSource::DmCustomized
        ));
    }
}
