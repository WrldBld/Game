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

use wrldbldr_domain::entities::StagedNpc;
use wrldbldr_domain::{CharacterId, GameTime, WorldId};
use wrldbldr_engine_ports::inbound::{StagingUseCasePort, UseCaseContext};
use wrldbldr_engine_ports::outbound::{
    BroadcastPort, CharacterCrudPort, ClockPort, GameEvent, LocationCrudPort, NpcPresenceData,
    RegionCrudPort, StagingReadyEvent, WaitingPcData,
};

use super::builders::SceneBuilder;
use super::errors::StagingError;

// Import port traits from engine-ports
pub use wrldbldr_engine_ports::outbound::{
    StagingStateExtPort, StagingMutationPort as StagingServiceExtPort,
};

// Re-export types from engine-ports for backwards compatibility
pub use wrldbldr_engine_ports::outbound::{
    ApproveInput, ApproveResult, ApprovedNpcData, ApprovedNpcInput as ApprovedNpc,
    PendingStagingInfo, PreStageInput, PreStageResult, ProposedNpc, RegenerateInput,
    RegeneratedNpc, StagingApprovalSource, StagingRegenerateResult as RegenerateResult,
    WaitingPcInfo,
};

// Note: From<StagingApprovalSource> for StagingSource is implemented in engine-ports

// =============================================================================
// Staging Approval Use Case
// =============================================================================

/// Use case for staging approval operations
///
/// Coordinates DM staging approval, regeneration, and pre-staging.
pub struct StagingApprovalUseCase {
    staging_service: Arc<dyn StagingServiceExtPort>,
    staging_state: Arc<dyn StagingStateExtPort>,
    character_crud: Arc<dyn CharacterCrudPort>,
    region_crud: Arc<dyn RegionCrudPort>,
    location_repo: Arc<dyn LocationCrudPort>,
    broadcast: Arc<dyn BroadcastPort>,
    scene_builder: Arc<SceneBuilder>,
    clock: Arc<dyn ClockPort>,
}

impl StagingApprovalUseCase {
    /// Create a new StagingApprovalUseCase with all dependencies
    pub fn new(
        staging_service: Arc<dyn StagingServiceExtPort>,
        staging_state: Arc<dyn StagingStateExtPort>,
        character_crud: Arc<dyn CharacterCrudPort>,
        region_crud: Arc<dyn RegionCrudPort>,
        location_repo: Arc<dyn LocationCrudPort>,
        broadcast: Arc<dyn BroadcastPort>,
        scene_builder: Arc<SceneBuilder>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            staging_service,
            staging_state,
            character_crud,
            region_crud,
            location_repo,
            broadcast,
            scene_builder,
            clock,
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
            .unwrap_or_else(|| GameTime::new(self.clock.now()));

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
            .map_err(StagingError::ApprovalFailed)?;

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
            .unwrap_or_else(|| GameTime::new(self.clock.now()));

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
            .map_err(StagingError::RegenerationFailed)?;

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
            .region_crud
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
            .unwrap_or_else(|| GameTime::new(self.clock.now()));

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
            .map_err(StagingError::PreStagingFailed)?;

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
        match self.character_crud.get(character_id).await {
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
        let region = match self.region_crud.get(pending.region_id).await {
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

// =============================================================================
// StagingUseCasePort Implementation
// =============================================================================

#[async_trait::async_trait]
impl StagingUseCasePort for StagingApprovalUseCase {
    async fn approve(
        &self,
        ctx: UseCaseContext,
        input: ApproveInput,
    ) -> Result<ApproveResult, StagingError> {
        self.approve(ctx, input).await
    }

    async fn regenerate(
        &self,
        ctx: UseCaseContext,
        input: RegenerateInput,
    ) -> Result<RegenerateResult, StagingError> {
        self.regenerate(ctx, input).await
    }

    async fn pre_stage(
        &self,
        ctx: UseCaseContext,
        input: PreStageInput,
    ) -> Result<PreStageResult, StagingError> {
        self.pre_stage(ctx, input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain::StagingSource;

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
