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

use super::errors::SceneError;

// Import internal service ports (domain-returning)
use crate::application::services::internal::{
    InteractionServicePort, SceneServicePort, SceneWithRelations as DomainSceneWithRelations,
};
use wrldbldr_domain::{
    InteractionTarget as DomainInteractionTarget, TimeContext as DomainTimeContext,
};

// Import remaining port traits from engine-ports
pub use wrldbldr_engine_ports::outbound::{
    DirectorialContextDtoRepositoryPort, SceneDmActionQueuePort as DmActionQueuePort,
    WorldStateUpdatePort as WorldStatePort,
};

// Re-export types from engine-ports for backwards compatibility
pub use wrldbldr_engine_ports::outbound::{
    CharacterEntity, DirectorialContextData, DirectorialUpdateResult, InteractionEntity,
    InteractionTarget, LocationEntity, NpcMotivation, RequestSceneChangeInput,
    SceneApprovalDecision, SceneApprovalDecisionInput, SceneApprovalDecisionResult,
    SceneChangeResult, SceneCharacterData as CharacterData, SceneDmAction as DmAction, SceneEntity,
    SceneInteractionData as InteractionData, TimeContext, UpdateDirectorialInput,
    UseCaseSceneData as SceneData, UseCaseSceneWithRelations,
};

// =============================================================================
// Scene Use Case
// =============================================================================

/// Use case for scene operations
pub struct SceneUseCase {
    scene_service: Arc<dyn SceneServicePort>,
    interaction_service: Arc<dyn InteractionServicePort>,
    world_state: Arc<dyn WorldStatePort>,
    directorial_repo: Arc<dyn DirectorialContextDtoRepositoryPort>,
    dm_action_queue: Arc<dyn DmActionQueuePort>,
}

impl SceneUseCase {
    /// Create a new SceneUseCase with all dependencies
    pub fn new(
        scene_service: Arc<dyn SceneServicePort>,
        interaction_service: Arc<dyn InteractionServicePort>,
        world_state: Arc<dyn WorldStatePort>,
        directorial_repo: Arc<dyn DirectorialContextDtoRepositoryPort>,
        dm_action_queue: Arc<dyn DmActionQueuePort>,
    ) -> Self {
        Self {
            scene_service,
            interaction_service,
            world_state,
            directorial_repo,
            dm_action_queue,
        }
    }

    /// Convert domain SceneWithRelations to use-case DTO
    fn convert_scene_with_relations(swr: DomainSceneWithRelations) -> UseCaseSceneWithRelations {
        UseCaseSceneWithRelations {
            scene: SceneEntity {
                id: swr.scene.id,
                name: swr.scene.name,
                location_id: swr.scene.location_id,
                backdrop_override: swr.scene.backdrop_override,
                time_context: match swr.scene.time_context {
                    DomainTimeContext::Unspecified => TimeContext::Unspecified,
                    DomainTimeContext::TimeOfDay(t) => {
                        TimeContext::TimeOfDay(t.display_name().to_string())
                    }
                    DomainTimeContext::During(s) => TimeContext::During(s),
                    DomainTimeContext::Custom(s) => TimeContext::Custom(s),
                },
                directorial_notes: if swr.scene.directorial_notes.is_empty() {
                    None
                } else {
                    Some(swr.scene.directorial_notes)
                },
            },
            location: LocationEntity {
                name: swr.location.name,
                backdrop_asset: swr.location.backdrop_asset,
            },
            featured_characters: swr
                .featured_characters
                .into_iter()
                .map(|c| CharacterEntity {
                    id: c.id,
                    name: c.name,
                    sprite_asset: c.sprite_asset,
                    portrait_asset: c.portrait_asset,
                })
                .collect(),
        }
    }

    /// Convert domain InteractionTemplate to use-case DTO
    fn convert_interaction(i: wrldbldr_domain::entities::InteractionTemplate) -> InteractionEntity {
        let target = match &i.target {
            DomainInteractionTarget::Character(id) => InteractionTarget::Character(*id),
            DomainInteractionTarget::Item(id) => InteractionTarget::Item(*id),
            DomainInteractionTarget::Environment(desc) => {
                InteractionTarget::Environment(desc.clone())
            }
            DomainInteractionTarget::None => InteractionTarget::None,
        };
        InteractionEntity {
            id: i.id,
            name: i.name,
            interaction_type: format!("{:?}", i.interaction_type),
            target,
            is_available: i.is_available,
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

        // Load scene with relations (from internal service returning domain types)
        let domain_scene = self
            .scene_service
            .get_scene_with_relations(input.scene_id)
            .await
            .map_err(|e| SceneError::Database(e.to_string()))?
            .ok_or_else(|| SceneError::SceneNotFound(input.scene_id.to_string()))?;

        // Convert domain type to use-case DTO
        let scene_with_relations = Self::convert_scene_with_relations(domain_scene);

        // Load interactions (from internal service returning domain types)
        let domain_interactions = self
            .interaction_service
            .list_by_scene(input.scene_id)
            .await
            .unwrap_or_else(|e| {
                warn!(error = %e, "Failed to load interactions");
                vec![]
            });

        // Convert domain interactions to use-case DTOs
        let interactions: Vec<InteractionEntity> = domain_interactions
            .into_iter()
            .map(Self::convert_interaction)
            .collect();

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
                .clone()
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
            .map_err(SceneError::Database)?;

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
        self.request_scene_change(ctx, input).await
    }

    async fn update_directorial_context(
        &self,
        ctx: UseCaseContext,
        input: UpdateDirectorialInput,
    ) -> Result<DirectorialUpdateResult, PortSceneError> {
        self.update_directorial_context(ctx, input).await
    }

    async fn handle_approval_decision(
        &self,
        ctx: UseCaseContext,
        input: SceneApprovalDecisionInput,
    ) -> Result<SceneApprovalDecisionResult, PortSceneError> {
        self.handle_approval_decision(ctx, input).await
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
        assert!(matches!(
            edit,
            SceneApprovalDecision::ApproveWithEdits { .. }
        ));
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
