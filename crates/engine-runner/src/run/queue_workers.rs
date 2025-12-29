//! Background workers for processing queue items
//!
//! These workers process items from the queues and handle notifications,
//! approvals, and other async operations.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use wrldbldr_domain::value_objects::{ApprovalRequestData, ChallengeOutcomeData, DmActionData, DmActionType};
use wrldbldr_domain::{NarrativeEventId, WorldId};
use wrldbldr_engine_adapters::infrastructure::queues::QueueBackendEnum;
use wrldbldr_engine_adapters::infrastructure::websocket::{
    domain_challenge_suggestion_to_proto, domain_narrative_suggestion_to_proto,
    domain_tools_to_proto,
};
use wrldbldr_engine_adapters::infrastructure::world_connection_manager::SharedWorldConnectionManager;
use wrldbldr_engine_app::application::services::{
    ApprovalOutcome, DMApprovalQueueService, DmActionQueueService, InteractionService,
    ItemServiceImpl, NarrativeEventService, SceneService,
};
use wrldbldr_engine_ports::outbound::QueueNotificationPort;
use wrldbldr_protocol::{
    CharacterData, CharacterPosition, InteractionData, ProposedToolInfo, SceneData, ServerMessage,
};

/// Worker that processes approval items and sends ApprovalRequired messages to DM
pub async fn approval_notification_worker(
    approval_queue_service: Arc<
        DMApprovalQueueService<
            wrldbldr_engine_adapters::infrastructure::queues::QueueBackendEnum<ApprovalRequestData>,
            ItemServiceImpl,
        >,
    >,
    world_connection_manager: SharedWorldConnectionManager,
    recovery_interval: Duration,
    cancel_token: CancellationToken,
) {
    tracing::info!("Starting approval notification worker");
    let notifier = approval_queue_service.queue().notifier();
    loop {
        // Check for cancellation
        if cancel_token.is_cancelled() {
            tracing::info!("Approval notification worker shutting down");
            break;
        }

        // Get all pending approvals from the queue
        // We need to check each active world for pending approvals
        let world_ids = world_connection_manager.get_all_world_ids().await;

        let mut has_work = false;
        for world_id in world_ids {
            let pending = match approval_queue_service
                .get_pending(WorldId::from(world_id))
                .await
            {
                Ok(items) => items,
                Err(e) => {
                    tracing::error!(
                        "Failed to get pending approvals for world {}: {}",
                        world_id,
                        e
                    );
                    continue;
                }
            };

            if !pending.is_empty() {
                has_work = true;
            }

            // Send ApprovalRequired messages for new approvals
            for item in pending {
                let approval_id = item.id.to_string();
                // Convert domain types to protocol types for wire transmission
                let proposed_tools: Vec<ProposedToolInfo> =
                    domain_tools_to_proto(&item.payload.proposed_tools);
                let challenge_suggestion =
                    domain_challenge_suggestion_to_proto(item.payload.challenge_suggestion.as_ref());
                let narrative_event_suggestion = domain_narrative_suggestion_to_proto(
                    item.payload.narrative_event_suggestion.as_ref(),
                );

                // Send ApprovalRequired message to DM via world connection manager
                let approval_msg = ServerMessage::ApprovalRequired {
                    request_id: approval_id.clone(),
                    npc_name: item.payload.npc_name.clone(),
                    proposed_dialogue: item.payload.proposed_dialogue.clone(),
                    internal_reasoning: item.payload.internal_reasoning.clone(),
                    proposed_tools,
                    challenge_suggestion,
                    narrative_event_suggestion,
                };

                if let Err(e) = world_connection_manager
                    .send_to_dm(&world_id, approval_msg)
                    .await
                {
                    tracing::warn!(
                        "Failed to send approval to DM for world {}: {}",
                        world_id,
                        e
                    );
                } else {
                    tracing::info!("Sent ApprovalRequired for approval {} to DM", approval_id);
                }
            }
        }

        // Wait for notification if no work, otherwise check again immediately
        if !has_work {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    tracing::info!("Approval notification worker shutting down");
                    break;
                }
                _ = notifier.wait_for_work(recovery_interval) => {}
            }
        }
    }
}

/// Worker that processes DM action queue items
pub async fn dm_action_worker(
    dm_action_queue_service: Arc<
        DmActionQueueService<wrldbldr_engine_adapters::infrastructure::queues::QueueBackendEnum<DmActionData>>,
    >,
    approval_queue_service: Arc<
        DMApprovalQueueService<
            wrldbldr_engine_adapters::infrastructure::queues::QueueBackendEnum<ApprovalRequestData>,
            ItemServiceImpl,
        >,
    >,
    narrative_event_service: Arc<dyn NarrativeEventService>,
    scene_service: Arc<dyn SceneService>,
    interaction_service: Arc<dyn InteractionService>,
    world_connection_manager: SharedWorldConnectionManager,
    recovery_interval: Duration,
    cancel_token: CancellationToken,
) {
    tracing::info!("Starting DM action queue worker");
    let notifier = dm_action_queue_service.queue().notifier();
    loop {
        // Check for cancellation
        if cancel_token.is_cancelled() {
            tracing::info!("DM action queue worker shutting down");
            break;
        }

        let world_connection_manager_clone = world_connection_manager.clone();
        let approval_queue_service_clone = approval_queue_service.clone();
        let narrative_event_service_clone = narrative_event_service.clone();
        let scene_service_clone = scene_service.clone();
        let interaction_service_clone = interaction_service.clone();
        match dm_action_queue_service
            .process_next(|action| {
                let world_connection_manager = world_connection_manager_clone.clone();
                let approval_queue_service = approval_queue_service_clone.clone();
                let narrative_event_service = narrative_event_service_clone.clone();
                let scene_service = scene_service_clone.clone();
                let interaction_service = interaction_service_clone.clone();
                async move {
                    process_dm_action(
                        &world_connection_manager,
                        &approval_queue_service,
                        &*narrative_event_service,
                        &*scene_service,
                        &*interaction_service,
                        &action,
                    )
                    .await
                }
            })
            .await
        {
            Ok(Some(_)) => {
                // Action processed successfully
            }
            Ok(None) => {
                // Queue empty - wait for notification or recovery timeout
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        tracing::info!("DM action queue worker shutting down");
                        break;
                    }
                    _ = notifier.wait_for_work(recovery_interval) => {}
                }
            }
            Err(e) => {
                tracing::error!("Error processing DM action: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn process_dm_action(
    world_connection_manager: &SharedWorldConnectionManager,
    approval_queue_service: &Arc<
        DMApprovalQueueService<
            wrldbldr_engine_adapters::infrastructure::queues::QueueBackendEnum<ApprovalRequestData>,
            ItemServiceImpl,
        >,
    >,
    narrative_event_service: &dyn NarrativeEventService,
    scene_service: &dyn SceneService,
    interaction_service: &dyn InteractionService,
    action: &DmActionData,
) -> Result<(), wrldbldr_engine_ports::outbound::QueueError> {
    match &action.action {
        DmActionType::ApprovalDecision {
            request_id,
            decision,
        } => {
            // Parse request_id as QueueItemId (UUID string)
            let approval_item_id = match uuid::Uuid::parse_str(&request_id) {
                Ok(uuid) => uuid,
                Err(_) => {
                    tracing::error!("Invalid approval item ID: {}", request_id);
                    return Err(wrldbldr_engine_ports::outbound::QueueError::NotFound(
                        request_id.clone(),
                    ));
                }
            };

            // The approval service's process_decision expects domain ApprovalDecision
            // which matches what we have from the DmActionType

            // Process the decision using the approval queue service
            match approval_queue_service
                .process_decision(action.world_id, approval_item_id, decision.clone())
                .await
            {
                Ok(outcome) => match outcome {
                    ApprovalOutcome::Broadcast {
                        dialogue,
                        npc_name,
                        executed_tools,
                    } => {
                        let message = ServerMessage::DialogueResponse {
                            speaker_id: npc_name.clone(),
                            speaker_name: npc_name,
                            text: dialogue,
                            choices: vec![],
                        };
                        world_connection_manager
                            .broadcast_to_players(action.world_id.into(), message)
                            .await;
                        tracing::info!("Broadcast approved dialogue, tools: {:?}", executed_tools);
                    }
                    ApprovalOutcome::Rejected {
                        feedback,
                        needs_reprocessing,
                    } => {
                        tracing::info!(
                            "Approval rejected: {}, reprocess: {}",
                            feedback,
                            needs_reprocessing
                        );
                    }
                    ApprovalOutcome::MaxRetriesExceeded { feedback } => {
                        tracing::warn!("Approval max retries exceeded: {}", feedback);
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to process approval decision: {}", e);
                    return Err(e);
                }
            }
        }
        DmActionType::DirectNpcControl {
            npc_id: _,
            dialogue,
        } => {
            // Broadcast direct NPC control via world connection manager
            let response = ServerMessage::DialogueResponse {
                speaker_id: "NPC".to_string(),
                speaker_name: "NPC".to_string(),
                text: dialogue.clone(),
                choices: vec![],
            };
            world_connection_manager
                .broadcast_to_players(action.world_id.into(), response)
                .await;
        }
        DmActionType::TriggerEvent { event_id } => {
            // Parse event ID
            let event_uuid = match uuid::Uuid::parse_str(&event_id) {
                Ok(uuid) => NarrativeEventId::from_uuid(uuid),
                Err(_) => {
                    tracing::error!("Invalid event ID: {}", event_id);
                    return Err(wrldbldr_engine_ports::outbound::QueueError::Backend(
                        format!("Invalid event ID: {}", event_id),
                    ));
                }
            };

            // Load the narrative event
            let narrative_event = match narrative_event_service.get(event_uuid).await {
                Ok(Some(event)) => event,
                Ok(None) => {
                    tracing::error!("Narrative event not found: {}", event_id);
                    return Err(wrldbldr_engine_ports::outbound::QueueError::NotFound(
                        event_id.clone(),
                    ));
                }
                Err(e) => {
                    tracing::error!("Failed to load narrative event: {}", e);
                    return Err(wrldbldr_engine_ports::outbound::QueueError::Backend(
                        format!("Failed to load narrative event: {}", e),
                    ));
                }
            };

            // Mark event as triggered
            if let Err(e) = narrative_event_service
                .mark_triggered(event_uuid, None)
                .await
            {
                tracing::error!("Failed to mark narrative event as triggered: {}", e);
                return Err(wrldbldr_engine_ports::outbound::QueueError::Backend(
                    format!("Failed to mark event as triggered: {}", e),
                ));
            }

            // Broadcast notification to session via world connection manager
            let notification = ServerMessage::Error {
                code: "NARRATIVE_EVENT_TRIGGERED".to_string(),
                message: format!(
                    "Narrative event '{}' has been triggered",
                    narrative_event.name
                ),
            };
            world_connection_manager
                .broadcast_to_players(action.world_id.into(), notification)
                .await;

            tracing::info!(
                "Triggered narrative event {} ({}) in world {}",
                event_id,
                narrative_event.name,
                action.world_id.to_uuid()
            );
        }
        DmActionType::TransitionScene { scene_id } => {
            // Load scene with relations
            let scene_with_relations = match scene_service.get_scene_with_relations(*scene_id).await
            {
                Ok(Some(scene_data)) => scene_data,
                Ok(None) => {
                    tracing::error!("Scene not found: {}", scene_id);
                    return Err(wrldbldr_engine_ports::outbound::QueueError::NotFound(
                        scene_id.to_string(),
                    ));
                }
                Err(e) => {
                    tracing::error!("Failed to load scene: {}", e);
                    return Err(wrldbldr_engine_ports::outbound::QueueError::Backend(
                        format!("Failed to load scene: {}", e),
                    ));
                }
            };

            // Load interactions for the scene
            let interactions = match interaction_service.list_interactions(*scene_id).await {
                Ok(interactions) => interactions
                    .into_iter()
                    .map(|i| {
                        let target_name = match &i.target {
                            wrldbldr_domain::entities::InteractionTarget::Character(_) => {
                                Some("Character".to_string())
                            }
                            wrldbldr_domain::entities::InteractionTarget::Item(_) => {
                                Some("Item".to_string())
                            }
                            wrldbldr_domain::entities::InteractionTarget::Environment(name) => {
                                Some(name.clone())
                            }
                            wrldbldr_domain::entities::InteractionTarget::None => None,
                        };
                        InteractionData {
                            id: i.id.to_string(),
                            name: i.name.clone(),
                            interaction_type: format!("{:?}", i.interaction_type),
                            target_name,
                            is_available: i.is_available,
                        }
                    })
                    .collect(),
                Err(e) => {
                    tracing::warn!("Failed to load interactions for scene: {}", e);
                    vec![]
                }
            };

            // Build character data
            let characters: Vec<CharacterData> = scene_with_relations
                .featured_characters
                .iter()
                .map(|c| CharacterData {
                    id: c.id.to_string(),
                    name: c.name.clone(),
                    sprite_asset: c.sprite_asset.clone(),
                    portrait_asset: c.portrait_asset.clone(),
                    position: CharacterPosition::Center,
                    is_speaking: false,
                    emotion: None, // Engine doesn't track emotion state yet
                })
                .collect();

            // Build SceneUpdate message
            let scene_update = ServerMessage::SceneUpdate {
                scene: SceneData {
                    id: scene_with_relations.scene.id.to_string(),
                    name: scene_with_relations.scene.name.clone(),
                    location_id: scene_with_relations.scene.location_id.to_string(),
                    location_name: scene_with_relations.location.name.clone(),
                    backdrop_asset: scene_with_relations
                        .scene
                        .backdrop_override
                        .or(scene_with_relations.location.backdrop_asset.clone()),
                    time_context: match &scene_with_relations.scene.time_context {
                        wrldbldr_domain::entities::TimeContext::Unspecified => {
                            "Unspecified".to_string()
                        }
                        wrldbldr_domain::entities::TimeContext::TimeOfDay(tod) => {
                            format!("{:?}", tod)
                        }
                        wrldbldr_domain::entities::TimeContext::During(s) => s.clone(),
                        wrldbldr_domain::entities::TimeContext::Custom(s) => s.clone(),
                    },
                    directorial_notes: scene_with_relations.scene.directorial_notes.clone(),
                },
                characters,
                interactions,
            };

            // Broadcast scene update via world connection manager
            // Note: Scene state is tracked per-world in WorldStateManager now
            world_connection_manager
                .broadcast_to_world(action.world_id.into(), scene_update)
                .await;

            tracing::info!(
                "DM transitioned scene to {} in world {}",
                scene_id,
                action.world_id.to_uuid()
            );
        }
    }

    Ok(())
}

/// Worker that sends pending challenge outcomes to DM for approval
///
/// This worker polls the challenge outcome queue and sends `ChallengeOutcomePending`
/// messages to connected DMs. It handles DM reconnection automatically by continuously
/// polling for pending items.
pub async fn challenge_outcome_notification_worker(
    challenge_queue: Arc<QueueBackendEnum<ChallengeOutcomeData>>,
    world_connection_manager: SharedWorldConnectionManager,
    recovery_interval: Duration,
    cancel_token: CancellationToken,
) {
    use wrldbldr_engine_ports::outbound::ApprovalQueuePort;

    tracing::info!("Starting challenge outcome notification worker");
    let notifier = challenge_queue.notifier();

    loop {
        // Check for cancellation
        if cancel_token.is_cancelled() {
            tracing::info!("Challenge outcome notification worker shutting down");
            break;
        }

        let world_ids = world_connection_manager.get_all_world_ids().await;
        let mut has_work = false;

        for world_id in world_ids {
            // Only process if world has a DM connected
            if !world_connection_manager.has_dm(&world_id).await {
                continue;
            }

            let pending = match challenge_queue.list_by_world(WorldId::from(world_id)).await {
                Ok(items) => items,
                Err(e) => {
                    tracing::error!(
                        "Failed to get pending challenge outcomes for world {}: {}",
                        world_id,
                        e
                    );
                    continue;
                }
            };

            if !pending.is_empty() {
                has_work = true;
            }

            for queue_item in pending {
                let item = queue_item.payload;
                // Convert domain ProposedTool to protocol ProposedToolInfo
                let outcome_triggers = domain_tools_to_proto(&item.outcome_triggers);
                let message = ServerMessage::ChallengeOutcomePending {
                    resolution_id: item.resolution_id.clone(),
                    challenge_id: item.challenge_id,
                    challenge_name: item.challenge_name,
                    character_id: item.character_id.to_string(),
                    character_name: item.character_name,
                    roll: item.roll,
                    modifier: item.modifier,
                    total: item.total,
                    outcome_type: item.outcome_type,
                    outcome_description: item.outcome_description,
                    outcome_triggers,
                    roll_breakdown: item.roll_breakdown,
                };

                if let Err(e) = world_connection_manager
                    .send_to_dm(&world_id, message)
                    .await
                {
                    tracing::warn!(
                        "Failed to send challenge outcome to DM for world {}: {}",
                        world_id,
                        e
                    );
                } else {
                    tracing::debug!("Sent ChallengeOutcomePending {} to DM", item.resolution_id);
                }
            }
        }

        // Wait for notification if no work, otherwise check again immediately
        if !has_work {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    tracing::info!("Challenge outcome notification worker shutting down");
                    break;
                }
                _ = notifier.wait_for_work(recovery_interval) => {}
            }
        }
    }
}
