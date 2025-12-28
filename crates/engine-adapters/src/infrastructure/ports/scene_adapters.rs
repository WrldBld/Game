//! Scene Use Case Adapters
//!
//! Implements scene-related ports by wrapping existing services.
//!
//! # Implementation Notes
//!
//! Some adapters are simplified/placeholder implementations due to type mismatches
//! between the use case port definitions and existing services. Handlers may need
//! to call services directly for complex operations until the full refactoring
//! is complete.

use std::sync::Arc;

use wrldbldr_domain::{InteractionTarget as DomainInteractionTarget, SceneId, TimeContext as DomainTimeContext, WorldId};
use wrldbldr_engine_app::application::services::{InteractionService, SceneService};
use wrldbldr_engine_app::application::use_cases::{
    CharacterEntity, DirectorialContextData, DirectorialContextRepositoryPort, DmAction,
    InteractionEntity, InteractionServicePort, InteractionTarget, LocationEntity, NpcMotivation,
    SceneEntity, SceneServicePort, SceneWithRelations as UseCaseSceneWithRelations, TimeContext,
    SceneDmActionQueuePort, SceneWorldStatePort,
};
use wrldbldr_engine_ports::outbound::DirectorialContextRepositoryPort as PortDirectorialContextRepositoryPort;
use wrldbldr_protocol::{DirectorialContext, NpcMotivationData};

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
    ) -> Result<Option<UseCaseSceneWithRelations>, String> {
        match self.service.get_scene_with_relations(scene_id).await {
            Ok(Some(swr)) => Ok(Some(UseCaseSceneWithRelations {
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
            })),
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
        match self.service.list_interactions(scene_id).await {
            Ok(interactions) => Ok(interactions
                .into_iter()
                .map(|i| {
                    // Convert domain InteractionTarget to use case InteractionTarget
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

impl SceneWorldStatePort for SceneWorldStateAdapter {
    fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>) {
        self.state.set_current_scene(world_id, scene_id);
    }

    fn set_directorial_context(&self, world_id: &WorldId, context: DirectorialContextData) {
        // Convert to protocol DirectorialContext
        // Note: Protocol DirectorialContext has different fields than use case DirectorialContextData
        let protocol_context = DirectorialContext {
            scene_notes: context.dm_notes.unwrap_or_default(),
            tone: context.scene_mood.unwrap_or_default(),
            npc_motivations: context
                .npc_motivations
                .into_iter()
                .map(|m| NpcMotivationData {
                    character_id: m.character_id,
                    emotional_guidance: m.emotional_state.unwrap_or_default(),
                    immediate_goal: m.motivation,
                    secret_agenda: None,
                })
                .collect(),
            forbidden_topics: vec![],
        };

        self.state
            .set_directorial_context(world_id, protocol_context);
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
    async fn save(
        &self,
        world_id: &WorldId,
        context: &DirectorialContextData,
    ) -> Result<(), String> {
        // Convert to protocol DirectorialContext (engine-ports uses protocol types)
        let protocol_context = DirectorialContext {
            scene_notes: context.dm_notes.clone().unwrap_or_default(),
            tone: context.scene_mood.clone().unwrap_or_default(),
            npc_motivations: context
                .npc_motivations
                .iter()
                .map(|m| NpcMotivationData {
                    character_id: m.character_id.clone(),
                    emotional_guidance: m.emotional_state.clone().unwrap_or_default(),
                    immediate_goal: m.motivation.clone(),
                    secret_agenda: None,
                })
                .collect(),
            forbidden_topics: vec![],
        };

        self.repo
            .save(world_id, &protocol_context)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Placeholder adapter for DM Action Queue
///
/// The SceneUseCase's DmAction type differs from the DTO DMAction type used by
/// DMActionQueueService. This placeholder returns an error suggesting handlers
/// should call the service directly until the type alignment is complete.
pub struct DmActionQueuePlaceholder;

impl DmActionQueuePlaceholder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DmActionQueuePlaceholder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SceneDmActionQueuePort for DmActionQueuePlaceholder {
    async fn enqueue_action(
        &self,
        _world_id: &WorldId,
        _dm_id: String,
        _action: DmAction,
    ) -> Result<(), String> {
        // Scene approval actions use a different approval flow than the DM action queue.
        // Handlers should process approval decisions directly or use the appropriate
        // approval service.
        Err("DM action queue adapter not implemented for scene approvals. \
             Handlers should call the approval service directly.".to_string())
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

    #[test]
    fn test_directorial_context_data() {
        let context = DirectorialContextData {
            npc_motivations: vec![NpcMotivation {
                character_id: "npc1".to_string(),
                motivation: "Seek treasure".to_string(),
                emotional_state: Some("Excited".to_string()),
            }],
            scene_mood: Some("Tense".to_string()),
            pacing: Some("Fast".to_string()),
            dm_notes: Some("Important scene".to_string()),
        };

        assert_eq!(context.npc_motivations.len(), 1);
        assert_eq!(context.scene_mood, Some("Tense".to_string()));
    }

    #[test]
    fn test_placeholder_adapter() {
        let adapter = DmActionQueuePlaceholder::default();
        // Just verify construction works
        assert!(std::mem::size_of_val(&adapter) == 0);
    }
}
