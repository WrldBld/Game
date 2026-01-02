//! DM Action Processor Service - Business logic for DM action processing
//!
//! This service handles the core business logic for processing DM actions:
//! - ApprovalDecision: Process approve/reject/edit of pending approvals
//! - DirectNPCControl: DM directly controls NPC dialogue
//! - TriggerEvent: Manually trigger narrative events
//! - TransitionScene: Transition to a new scene
//!
//! The infrastructure layer (queue workers) handles:
//! - Queue management and dequeuing
//! - Broadcasting results to WebSocket clients
//!
//! This separation keeps business logic in the application layer while
//! infrastructure concerns remain in the adapters layer.

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::{info, warn};

use crate::application::services::{
    ApprovalOutcome, InteractionService, NarrativeEventService, SceneService,
};
use wrldbldr_domain::entities::{InteractionTarget, TimeContext};
use wrldbldr_domain::{CharacterId, NarrativeEventId, SceneId, WorldId};
use wrldbldr_engine_ports::outbound::{
    ClockPort, DmActionData, DmActionPayloadType as DmActionType, DmActionProcessorPort,
    DmActionResult, DmApprovalDecision, QueueError,
};

/// Port for processing approval decisions
///
/// This trait abstracts the approval queue handling so the processor
/// service can process decisions without knowing about queue internals.
#[async_trait]
pub trait ApprovalProcessorPort: Send + Sync {
    /// Process an approval decision and return the outcome
    async fn process_decision(
        &self,
        world_id: WorldId,
        item_id: uuid::Uuid,
        decision: DmApprovalDecision,
    ) -> Result<ApprovalOutcome, QueueError>;
}

/// Service for processing DM actions
///
/// This service contains the core business logic for handling DM commands.
/// It is called by the infrastructure worker, which handles queue management
/// and broadcasting results.
pub struct DmActionProcessorService {
    /// Approval processor for handling approval decisions
    approval_processor: Arc<dyn ApprovalProcessorPort>,
    /// Narrative event service for triggering events
    narrative_event_service: Arc<dyn NarrativeEventService>,
    /// Scene service for scene transitions
    scene_service: Arc<dyn SceneService>,
    /// Interaction service for loading scene interactions
    interaction_service: Arc<dyn InteractionService>,
    /// Clock for time operations
    clock: Arc<dyn ClockPort>,
}

impl DmActionProcessorService {
    /// Create a new DM action processor service
    pub fn new(
        approval_processor: Arc<dyn ApprovalProcessorPort>,
        narrative_event_service: Arc<dyn NarrativeEventService>,
        scene_service: Arc<dyn SceneService>,
        interaction_service: Arc<dyn InteractionService>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            approval_processor,
            narrative_event_service,
            scene_service,
            interaction_service,
            clock,
        }
    }

    /// Process a DM action item from the queue
    ///
    /// This is the main entry point for processing DM actions.
    /// Returns a `DmActionResult` that the infrastructure layer
    /// should use to broadcast messages to clients.
    pub async fn process_action_item(&self, action: &DmActionData) -> Result<DmActionResult> {
        let world_id = action.world_id;

        match &action.action {
            DmActionType::ApprovalDecision {
                request_id,
                decision,
            } => {
                // Convert domain DmApprovalDecision to engine-dto DmApprovalDecision
                let dto_decision: wrldbldr_engine_ports::outbound::DmApprovalDecision =
                    decision.clone().into();
                self.process_approval_decision(world_id, request_id, dto_decision)
                    .await
            }
            DmActionType::DirectNpcControl { npc_id, dialogue } => {
                self.process_direct_npc_control(npc_id, dialogue).await
            }
            DmActionType::TriggerEvent { event_id } => {
                self.process_trigger_event(world_id, event_id).await
            }
            DmActionType::TransitionScene { scene_id } => {
                self.process_transition_scene(world_id, *scene_id).await
            }
        }
    }

    /// Process an approval decision
    async fn process_approval_decision(
        &self,
        world_id: WorldId,
        request_id: &str,
        decision: DmApprovalDecision,
    ) -> Result<DmActionResult> {
        // Parse request_id as UUID
        let approval_item_id = uuid::Uuid::parse_str(request_id)
            .context(format!("Invalid approval item ID: {}", request_id))?;

        // Process the decision using the approval processor
        let outcome = self
            .approval_processor
            .process_decision(world_id, approval_item_id, decision)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to process approval decision: {}", e))?;

        match outcome {
            ApprovalOutcome::Broadcast {
                dialogue,
                npc_name,
                executed_tools,
            } => {
                info!(
                    "Approval accepted, broadcasting dialogue. Executed tools: {:?}",
                    executed_tools
                );

                // Build broadcast message as JSON for the worker
                let message = serde_json::json!({
                    "type": "DialogueResponse",
                    "speaker_id": npc_name.clone(),
                    "speaker_name": npc_name,
                    "text": dialogue,
                    "choices": []
                });

                Ok(DmActionResult::ApprovalProcessed {
                    broadcast_messages: vec![message],
                    dm_feedback: None,
                })
            }
            ApprovalOutcome::Rejected {
                feedback,
                needs_reprocessing,
            } => {
                info!(
                    "Approval rejected: {}, reprocess: {}",
                    feedback, needs_reprocessing
                );

                Ok(DmActionResult::ApprovalProcessed {
                    broadcast_messages: vec![],
                    dm_feedback: Some(format!(
                        "Rejected: {}. Reprocessing: {}",
                        feedback, needs_reprocessing
                    )),
                })
            }
            ApprovalOutcome::MaxRetriesExceeded { feedback } => {
                warn!("Approval max retries exceeded: {}", feedback);

                Ok(DmActionResult::ApprovalProcessed {
                    broadcast_messages: vec![],
                    dm_feedback: Some(format!("Max retries exceeded: {}", feedback)),
                })
            }
        }
    }

    /// Process direct NPC control (DM override)
    async fn process_direct_npc_control(
        &self,
        npc_id: &CharacterId,
        dialogue: &str,
    ) -> Result<DmActionResult> {
        // For now, we use the character ID as the name
        // In a more complete implementation, we'd look up the character
        let npc_name = npc_id.to_string();

        info!("DM directly controlling NPC '{}': {}", npc_name, dialogue);

        Ok(DmActionResult::DialogueGenerated {
            npc_id: *npc_id,
            npc_name,
            dialogue: dialogue.to_string(),
        })
    }

    /// Process triggering a narrative event
    async fn process_trigger_event(
        &self,
        world_id: WorldId,
        event_id: &str,
    ) -> Result<DmActionResult> {
        // Parse event ID
        let event_uuid =
            uuid::Uuid::parse_str(event_id).context(format!("Invalid event ID: {}", event_id))?;
        let narrative_event_id = NarrativeEventId::from_uuid(event_uuid);

        // Load the narrative event
        let narrative_event = self
            .narrative_event_service
            .get(narrative_event_id)
            .await
            .context("Failed to load narrative event")?
            .ok_or_else(|| anyhow::anyhow!("Narrative event not found: {}", event_id))?;

        // Mark event as triggered
        self.narrative_event_service
            .mark_triggered(narrative_event_id, None)
            .await
            .context("Failed to mark narrative event as triggered")?;

        info!(
            "Triggered narrative event {} ({}) in world {}",
            event_id, narrative_event.name, world_id
        );

        Ok(DmActionResult::EventTriggered {
            event_id: narrative_event_id,
            event_name: narrative_event.name,
            outcome: None,
        })
    }

    /// Process transitioning to a new scene
    async fn process_transition_scene(
        &self,
        world_id: WorldId,
        scene_id: SceneId,
    ) -> Result<DmActionResult> {
        let scene_id_typed = scene_id;

        // Load scene with relations
        let scene_with_relations = self
            .scene_service
            .get_scene_with_relations(scene_id_typed)
            .await
            .context("Failed to load scene")?
            .ok_or_else(|| anyhow::anyhow!("Scene not found: {}", scene_id))?;

        // Load interactions for the scene
        let interactions = self
            .interaction_service
            .list_interactions(scene_id_typed)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to load interactions for scene: {}", e);
                vec![]
            });

        // Build scene data JSON for broadcasting
        let scene_data = self.build_scene_data(&scene_with_relations, &interactions);

        info!(
            "DM transitioned scene to {} in world {}",
            scene_id, world_id
        );

        Ok(DmActionResult::SceneTransitioned {
            scene_id: scene_id_typed,
            scene_data,
        })
    }

    /// Build scene data JSON for the SceneUpdate message
    fn build_scene_data(
        &self,
        scene_with_relations: &crate::application::services::scene_service::SceneWithRelations,
        interactions: &[wrldbldr_domain::entities::InteractionTemplate],
    ) -> serde_json::Value {
        // Build character data
        let characters: Vec<serde_json::Value> = scene_with_relations
            .featured_characters
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id.to_string(),
                    "name": c.name,
                    "sprite_asset": c.sprite_asset,
                    "portrait_asset": c.portrait_asset,
                    "position": "Center",
                    "is_speaking": false,
                    "emotion": null
                })
            })
            .collect();

        // Build interaction data
        let interaction_data: Vec<serde_json::Value> = interactions
            .iter()
            .map(|i| {
                let target_name = match &i.target {
                    InteractionTarget::Character(_) => Some("Character".to_string()),
                    InteractionTarget::Item(_) => Some("Item".to_string()),
                    InteractionTarget::Environment(name) => Some(name.clone()),
                    InteractionTarget::None => None,
                };
                serde_json::json!({
                    "id": i.id.to_string(),
                    "name": i.name,
                    "interaction_type": format!("{:?}", i.interaction_type),
                    "target_name": target_name,
                    "is_available": i.is_available
                })
            })
            .collect();

        // Build time context string
        let time_context = match &scene_with_relations.scene.time_context {
            TimeContext::Unspecified => "Unspecified".to_string(),
            TimeContext::TimeOfDay(tod) => format!("{:?}", tod),
            TimeContext::During(s) => s.clone(),
            TimeContext::Custom(s) => s.clone(),
        };

        // Build the scene data
        serde_json::json!({
            "scene": {
                "id": scene_with_relations.scene.id.to_string(),
                "name": scene_with_relations.scene.name,
                "location_id": scene_with_relations.scene.location_id.to_string(),
                "location_name": scene_with_relations.location.name,
                "backdrop_asset": scene_with_relations.scene.backdrop_override
                    .clone()
                    .or(scene_with_relations.location.backdrop_asset.clone()),
                "time_context": time_context,
                "directorial_notes": scene_with_relations.scene.directorial_notes
            },
            "characters": characters,
            "interactions": interaction_data
        })
    }
}

#[async_trait]
impl DmActionProcessorPort for DmActionProcessorService {
    async fn process_action(
        &self,
        _action_type: &str,
        action_data: serde_json::Value,
        world_id: WorldId,
        dm_user_id: &str,
    ) -> Result<DmActionResult> {
        // Parse the action from JSON
        let action: DmActionType = serde_json::from_value(action_data.clone())
            .context("Failed to parse DM action data")?;

        // Create a DmActionData for processing
        let action_item = DmActionData {
            world_id,
            dm_id: dm_user_id.to_string(),
            action,
            timestamp: self.clock.now(),
        };

        // Process using the existing method
        self.process_action_item(&action_item).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests would go here, mocking the dependencies
    // For now, just a placeholder to ensure the module compiles

    #[test]
    fn test_module_compiles() {
        // This test ensures the module structure is correct
        assert!(true);
    }
}
