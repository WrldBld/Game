//! Scene Use Case Adapters
//!
//! Implements scene-related ports by wrapping existing services.

use std::sync::Arc;

use wrldbldr_domain::{SceneId, WorldId};
use wrldbldr_engine_app::application::services::{
    InteractionService, SceneService, DMActionQueueService,
};
use wrldbldr_engine_app::application::use_cases::{
    CharacterEntity, DirectorialContextData, DirectorialContextRepositoryPort,
    DmAction, DmActionQueuePort, InteractionEntity, InteractionServicePort, InteractionTarget,
    LocationEntity, SceneEntity, SceneServicePort, SceneWithRelations, TimeContext,
    WorldStatePort,
};
use wrldbldr_engine_ports::outbound::DirectorialContextRepositoryPort as PortDirectorialContextRepositoryPort;
use wrldbldr_protocol::DirectorialContext;

use crate::infrastructure::WorldStateManager;

/// Adapter for SceneService
pub struct SceneServiceAdapter {
    service: Arc<dyn SceneService>,
}

impl SceneServiceAdapter {
    pub fn new(service: Arc<dyn SceneService>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl SceneServicePort for SceneServiceAdapter {
    async fn get_scene_with_relations(
        &self,
        scene_id: SceneId,
    ) -> Result<Option<SceneWithRelations>, String> {
        match self.service.get_with_relations(scene_id).await {
            Ok(Some((scene, location, characters))) => {
                Ok(Some(SceneWithRelations {
                    scene: SceneEntity {
                        id: scene.id,
                        name: scene.name,
                        location_id: scene.location_id,
                        backdrop_override: scene.backdrop_override,
                        time_context: match scene.time_context {
                            wrldbldr_domain::value_objects::TimeContext::Unspecified => {
                                TimeContext::Unspecified
                            }
                            wrldbldr_domain::value_objects::TimeContext::TimeOfDay(t) => {
                                TimeContext::TimeOfDay(t)
                            }
                            wrldbldr_domain::value_objects::TimeContext::During(s) => {
                                TimeContext::During(s)
                            }
                            wrldbldr_domain::value_objects::TimeContext::Custom(s) => {
                                TimeContext::Custom(s)
                            }
                        },
                        directorial_notes: scene.directorial_notes,
                    },
                    location: LocationEntity {
                        name: location.name,
                        backdrop_asset: location.backdrop_asset,
                    },
                    featured_characters: characters
                        .into_iter()
                        .map(|c| CharacterEntity {
                            id: c.id,
                            name: c.name,
                            sprite_asset: c.sprite_asset,
                            portrait_asset: c.portrait_asset,
                        })
                        .collect(),
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Adapter for InteractionService
pub struct InteractionServiceAdapter {
    service: Arc<dyn InteractionService>,
}

impl InteractionServiceAdapter {
    pub fn new(service: Arc<dyn InteractionService>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl InteractionServicePort for InteractionServiceAdapter {
    async fn list_interactions(&self, scene_id: SceneId) -> Result<Vec<InteractionEntity>, String> {
        match self.service.list_by_scene(scene_id).await {
            Ok(interactions) => Ok(interactions
                .into_iter()
                .map(|i| InteractionEntity {
                    id: i.id,
                    name: i.name,
                    interaction_type: format!("{:?}", i.interaction_type),
                    target: match i.target_id {
                        Some(id) => InteractionTarget::Character(id.into()),
                        None => InteractionTarget::None,
                    },
                    is_available: i.is_available,
                })
                .collect()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Adapter for WorldStateManager (scene-related operations)
pub struct SceneWorldStateAdapter {
    state: Arc<WorldStateManager>,
}

impl SceneWorldStateAdapter {
    pub fn new(state: Arc<WorldStateManager>) -> Self {
        Self { state }
    }
}

impl WorldStatePort for SceneWorldStateAdapter {
    fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>) {
        self.state.set_current_scene(world_id, scene_id);
    }

    fn set_directorial_context(&self, world_id: &WorldId, context: DirectorialContextData) {
        // Convert to protocol DirectorialContext
        let protocol_context = DirectorialContext {
            npc_motivations: context
                .npc_motivations
                .into_iter()
                .map(|m| wrldbldr_protocol::NpcMotivation {
                    character_id: m.character_id,
                    motivation: m.motivation,
                    emotional_state: m.emotional_state,
                })
                .collect(),
            scene_mood: context.scene_mood,
            pacing: context.pacing,
            dm_notes: context.dm_notes,
        };

        self.state.set_directorial_context(world_id, protocol_context);
    }
}

/// Adapter for DirectorialContextRepositoryPort
pub struct DirectorialContextAdapter {
    repo: Arc<dyn PortDirectorialContextRepositoryPort>,
}

impl DirectorialContextAdapter {
    pub fn new(repo: Arc<dyn PortDirectorialContextRepositoryPort>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl DirectorialContextRepositoryPort for DirectorialContextAdapter {
    async fn save(&self, world_id: &WorldId, context: &DirectorialContextData) -> Result<(), String> {
        // Convert to domain DirectorialContext
        let domain_context = wrldbldr_domain::value_objects::DirectorialContext {
            npc_motivations: context
                .npc_motivations
                .iter()
                .map(|m| wrldbldr_domain::value_objects::NpcMotivation {
                    character_id: m.character_id.clone(),
                    motivation: m.motivation.clone(),
                    emotional_state: m.emotional_state.clone(),
                })
                .collect(),
            scene_mood: context.scene_mood.clone(),
            pacing: context.pacing.clone(),
            dm_notes: context.dm_notes.clone(),
        };

        self.repo.save(world_id, &domain_context).await
    }
}

/// Adapter for DMActionQueueService
pub struct DmActionQueueAdapter {
    service: Arc<DMActionQueueService>,
}

impl DmActionQueueAdapter {
    pub fn new(service: Arc<DMActionQueueService>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl DmActionQueuePort for DmActionQueueAdapter {
    async fn enqueue_action(
        &self,
        world_id: &WorldId,
        dm_id: String,
        action: DmAction,
    ) -> Result<(), String> {
        match action {
            DmAction::ApprovalDecision { request_id, decision } => {
                // Convert to queue action format
                let approved = !matches!(
                    decision,
                    wrldbldr_engine_app::application::use_cases::ApprovalDecision::Reject { .. }
                );
                
                self.service
                    .enqueue_approval_decision(world_id, &dm_id, &request_id, approved)
                    .await
                    .map_err(|e| e.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_context_conversion() {
        // Just verify the module compiles and types match
        let _unspec = TimeContext::Unspecified;
        let _tod = TimeContext::TimeOfDay("Morning".to_string());
    }
}
