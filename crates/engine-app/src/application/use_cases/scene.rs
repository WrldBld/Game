//! Scene Use Case
//!
//! Handles scene management operations including scene changes,
//! directorial updates, and approval decisions.
//!
//! # Responsibilities
//!
//! - Request scene changes
//! - Update directorial context (DM)
//! - Handle approval decisions (DM)
//!
//! # Architecture Note
//!
//! Scene operations affect the narrative flow of the game.
//! The directorial context influences NPC behavior and narrative generation.

use std::sync::Arc;
use tracing::{debug, info, warn};

use wrldbldr_engine_ports::inbound::{SceneUseCasePort, UseCaseContext};
use wrldbldr_engine_ports::outbound::BroadcastPort;

use super::errors::SceneError;

// Import port traits from engine-ports
pub use wrldbldr_engine_ports::inbound::{
    DirectorialContextRepositoryPort, InteractionServicePort,
    SceneDmActionQueuePort as DmActionQueuePort, SceneServicePort, WorldStatePort,
};

// Re-export types from engine-ports for backwards compatibility
pub use wrldbldr_engine_ports::outbound::{
    CharacterEntity, DirectorialContextData, DirectorialUpdateResult, InteractionEntity,
    InteractionTarget, LocationEntity, NpcMotivation, RequestSceneChangeInput, SceneChangeResult,
    SceneApprovalDecision,
    SceneApprovalDecisionInput,
    SceneApprovalDecisionResult,
    SceneCharacterData as CharacterData, SceneDmAction as DmAction, SceneEntity,
    SceneInteractionData as InteractionData, TimeContext, UpdateDirectorialInput,
    UseCaseSceneData as SceneData, UseCaseSceneWithRelations as SceneWithRelations,
};

// =============================================================================
// Scene Use Case
// =============================================================================

/// Use case for scene operations
pub struct SceneUseCase {
    scene_service: Arc<dyn SceneServicePort>,
    interaction_service: Arc<dyn InteractionServicePort>,
    world_state: Arc<dyn WorldStatePort>,
    directorial_repo: Arc<dyn DirectorialContextRepositoryPort>,
    dm_action_queue: Arc<dyn DmActionQueuePort>,
    broadcast: Arc<dyn BroadcastPort>,
}

impl SceneUseCase {
    /// Create a new SceneUseCase with all dependencies
    pub fn new(
        scene_service: Arc<dyn SceneServicePort>,
        interaction_service: Arc<dyn InteractionServicePort>,
        world_state: Arc<dyn WorldStatePort>,
        directorial_repo: Arc<dyn DirectorialContextRepositoryPort>,
        dm_action_queue: Arc<dyn DmActionQueuePort>,
        broadcast: Arc<dyn BroadcastPort>,
    ) -> Self {
        Self {
            scene_service,
            interaction_service,
            world_state,
            directorial_repo,
            dm_action_queue,
            broadcast,
        }
    }

    /// Request a scene change
    ///
    /// Any connected player can request a scene change.
    pub async fn request_scene_change(
        &self,
        ctx: UseCaseContext,
        input: RequestSceneChangeInput,
    ) -> Result<SceneChangeResult, SceneError> {
        debug!(scene_id = %input.scene_id, "Scene change requested");

        // Load scene with relations
        let scene_with_relations = self
            .scene_service
            .get_scene_with_relations(input.scene_id)
            .await
            .map_err(|e| SceneError::Database(e))?
            .ok_or_else(|| SceneError::SceneNotFound(input.scene_id.to_string()))?;

        // Load interactions
        let interactions = self
            .interaction_service
            .list_interactions(input.scene_id)
            .await
            .unwrap_or_else(|e| {
                warn!(error = %e, "Failed to load interactions");
                vec![]
            });

        // Build character data
        let characters: Vec<CharacterData> = scene_with_relations
            .featured_characters
            .iter()
            .map(|c| CharacterData {
                id: c.id.to_string(),
                name: c.name.clone(),
                sprite_asset: c.sprite_asset.clone(),
                portrait_asset: c.portrait_asset.clone(),
                position: "Center".to_string(),
                is_speaking: false,
                emotion: None,
            })
            .collect();

        // Build interaction data
        let interaction_data: Vec<InteractionData> = interactions
            .iter()
            .map(|i| {
                let target_name = match &i.target {
                    InteractionTarget::Character(_) => Some("Character".to_string()),
                    InteractionTarget::Item(_) => Some("Item".to_string()),
                    InteractionTarget::Environment(desc) => Some(desc.clone()),
                    InteractionTarget::None => None,
                };
                InteractionData {
                    id: i.id.to_string(),
                    name: i.name.clone(),
                    interaction_type: i.interaction_type.clone(),
                    target_name,
                    is_available: i.is_available,
                }
            })
            .collect();

        // Build scene data
        let scene_data = SceneData {
            id: scene_with_relations.scene.id.to_string(),
            name: scene_with_relations.scene.name.clone(),
            location_id: scene_with_relations.scene.location_id.to_string(),
            location_name: scene_with_relations.location.name.clone(),
            backdrop_asset: scene_with_relations
                .scene
                .backdrop_override
                .or(scene_with_relations.location.backdrop_asset.clone()),
            time_context: match &scene_with_relations.scene.time_context {
                TimeContext::Unspecified => "Unspecified".to_string(),
                TimeContext::TimeOfDay(tod) => tod.clone(),
                TimeContext::During(s) => s.clone(),
                TimeContext::Custom(s) => s.clone(),
            },
            directorial_notes: scene_with_relations.scene.directorial_notes.clone(),
        };

        // Update world state
        self.world_state
            .set_current_scene(&ctx.world_id, Some(input.scene_id.to_string()));

        info!(
            scene_id = %input.scene_id,
            "Scene change processed"
        );

        Ok(SceneChangeResult {
            scene_changed: true,
            scene: Some(scene_data),
            characters,
            interactions: interaction_data,
        })
    }

    /// Update directorial context
    ///
    /// DM-only operation.
    pub async fn update_directorial_context(
        &self,
        ctx: UseCaseContext,
        input: UpdateDirectorialInput,
    ) -> Result<DirectorialUpdateResult, SceneError> {
        if !ctx.is_dm {
            return Err(SceneError::NotAuthorized);
        }

        debug!("Updating directorial context");

        let context = DirectorialContextData {
            npc_motivations: input.npc_motivations,
            scene_mood: input.scene_mood,
            pacing: input.pacing,
            dm_notes: input.dm_notes,
        };

        // Store in world state
        self.world_state
            .set_directorial_context(&ctx.world_id, context.clone());

        // Persist to database (non-fatal if fails)
        if let Err(e) = self.directorial_repo.save(&ctx.world_id, &context).await {
            warn!(
                error = %e,
                "Failed to persist directorial context"
            );
        }

        info!(
            npc_count = context.npc_motivations.len(),
            "Directorial context updated"
        );

        Ok(DirectorialUpdateResult { updated: true })
    }

    /// Handle approval decision
    ///
    /// DM-only operation.
    pub async fn handle_approval_decision(
        &self,
        ctx: UseCaseContext,
        input: SceneApprovalDecisionInput,
    ) -> Result<SceneApprovalDecisionResult, SceneError> {
        if !ctx.is_dm {
            return Err(SceneError::NotAuthorized);
        }

        debug!(
            request_id = %input.request_id,
            decision = ?input.decision,
            "Processing approval decision"
        );

        // Enqueue to DM action queue
        let action = DmAction::ApprovalDecision {
            request_id: input.request_id.clone(),
            decision: input.decision,
        };

        self.dm_action_queue
            .enqueue_action(&ctx.world_id, ctx.user_id.clone(), action)
            .await
            .map_err(|e| SceneError::Database(e))?;

        info!(
            request_id = %input.request_id,
            "Approval decision enqueued"
        );

        Ok(SceneApprovalDecisionResult { processed: true })
    }
}

// =============================================================================
// SceneUseCasePort Implementation
// =============================================================================

use async_trait::async_trait;
use wrldbldr_engine_ports::inbound::scene_use_case_port::SceneUseCaseError as PortSceneError;

#[async_trait]
impl SceneUseCasePort for SceneUseCase {
    async fn request_scene_change(
        &self,
        ctx: UseCaseContext,
        input: RequestSceneChangeInput,
    ) -> Result<SceneChangeResult, PortSceneError> {
        self.request_scene_change(ctx, input)
            .await
            .map_err(|e| e.to_string())
    }

    async fn update_directorial_context(
        &self,
        ctx: UseCaseContext,
        input: UpdateDirectorialInput,
    ) -> Result<DirectorialUpdateResult, PortSceneError> {
        self.update_directorial_context(ctx, input)
            .await
            .map_err(|e| e.to_string())
    }

    async fn handle_approval_decision(
        &self,
        ctx: UseCaseContext,
        input: SceneApprovalDecisionInput,
    ) -> Result<SceneApprovalDecisionResult, PortSceneError> {
        self.handle_approval_decision(ctx, input)
            .await
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_decision_variants() {
        let approve = SceneApprovalDecision::Approve;
        let reject = SceneApprovalDecision::Reject {
            reason: "Not appropriate".to_string(),
        };
        let edit = SceneApprovalDecision::ApproveWithEdits {
            modified_text: "New text".to_string(),
        };

        assert!(matches!(approve, SceneApprovalDecision::Approve));
        assert!(matches!(reject, SceneApprovalDecision::Reject { .. }));
        assert!(matches!(edit, SceneApprovalDecision::ApproveWithEdits { .. }));
    }

    #[test]
    fn test_time_context_variants() {
        let unspec = TimeContext::Unspecified;
        let tod = TimeContext::TimeOfDay("Evening".to_string());
        let during = TimeContext::During("The festival".to_string());
        let custom = TimeContext::Custom("Three hours past midnight".to_string());

        assert!(matches!(unspec, TimeContext::Unspecified));
        assert!(matches!(tod, TimeContext::TimeOfDay(_)));
        assert!(matches!(during, TimeContext::During(_)));
        assert!(matches!(custom, TimeContext::Custom(_)));
    }
}
