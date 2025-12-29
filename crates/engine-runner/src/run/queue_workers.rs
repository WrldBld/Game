//! Background workers for processing queue items
//!
//! These workers process items from the queues and handle notifications,
//! approvals, and other async operations.
//!
//! # Architecture
//!
//! The workers in this module are **infrastructure** code - they handle:
//! - Queue polling and dequeuing
//! - Cancellation token handling
//! - Broadcasting results to WebSocket clients
//!
//! Business logic is delegated to the application layer via ports:
//! - `DmActionProcessorPort` for DM action processing
//! - Queue services for queue management

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio_util::sync::CancellationToken;
use wrldbldr_domain::value_objects::{ApprovalRequestData, ChallengeOutcomeData, DmActionData};
use wrldbldr_domain::WorldId;
use wrldbldr_engine_adapters::infrastructure::queues::QueueBackendEnum;
use wrldbldr_engine_adapters::infrastructure::websocket::{
    domain_challenge_suggestion_to_proto, domain_narrative_suggestion_to_proto,
    domain_tools_to_proto,
};
use wrldbldr_engine_adapters::infrastructure::world_connection_manager::SharedWorldConnectionManager;
use wrldbldr_engine_app::application::services::{
    DMApprovalQueueService, DmActionQueueService, ItemServiceImpl,
};
use wrldbldr_engine_ports::outbound::{DmActionProcessorPort, DmActionResult, QueueNotificationPort};
use wrldbldr_protocol::{ProposedToolInfo, ServerMessage};

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
                let challenge_suggestion = domain_challenge_suggestion_to_proto(
                    item.payload.challenge_suggestion.as_ref(),
                );
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
///
/// This worker handles the infrastructure concerns of DM action processing:
/// - Dequeuing actions from the queue
/// - Delegating business logic to `DmActionProcessorPort`
/// - Broadcasting results to WebSocket clients
///
/// The business logic is fully contained in the application layer's
/// `DmActionProcessorService`, keeping this worker pure infrastructure.
pub async fn dm_action_worker(
    dm_action_queue_service: Arc<
        DmActionQueueService<
            wrldbldr_engine_adapters::infrastructure::queues::QueueBackendEnum<DmActionData>,
        >,
    >,
    dm_action_processor: Arc<dyn DmActionProcessorPort>,
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

        let dm_action_processor_clone = dm_action_processor.clone();
        let world_connection_manager_clone = world_connection_manager.clone();
        match dm_action_queue_service
            .process_next(|action| {
                let processor = dm_action_processor_clone.clone();
                let world_connection_manager = world_connection_manager_clone.clone();
                async move {
                    // Delegate business logic to application layer via port
                    let action_type = format!("{:?}", action.action);
                    let action_data = serde_json::to_value(&action.action)
                        .map_err(|e: serde_json::Error| wrldbldr_engine_ports::outbound::QueueError::Backend(e.to_string()))?;

                    match processor
                        .process_action(&action_type, action_data, action.world_id, &action.dm_id)
                        .await
                    {
                        Ok(result) => {
                            // Broadcast the result (infrastructure concern)
                            broadcast_dm_action_result(
                                &world_connection_manager,
                                action.world_id,
                                result,
                            )
                            .await;
                            Ok(())
                        }
                        Err(e) => {
                            tracing::error!("DM action processing failed: {}", e);
                            Err(wrldbldr_engine_ports::outbound::QueueError::Backend(
                                e.to_string(),
                            ))
                        }
                    }
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

/// Broadcast the result of a DM action to appropriate WebSocket clients
///
/// This is infrastructure code - it converts application-layer results
/// into protocol messages and broadcasts them via the connection manager.
async fn broadcast_dm_action_result(
    world_connection_manager: &SharedWorldConnectionManager,
    world_id: WorldId,
    result: DmActionResult,
) {
    match result {
        DmActionResult::ApprovalProcessed {
            broadcast_messages,
            dm_feedback,
        } => {
            // Broadcast each message to players
            for msg_json in broadcast_messages {
                if let Ok(message) = serde_json::from_value::<ServerMessage>(msg_json.clone()) {
                    world_connection_manager
                        .broadcast_to_players(world_id.into(), message)
                        .await;
                }
            }
            // Send feedback to DM if present
            if let Some(feedback) = dm_feedback {
                let feedback_msg = ServerMessage::Error {
                    code: "DM_ACTION_FEEDBACK".to_string(),
                    message: feedback,
                };
                let _ = world_connection_manager
                    .send_to_dm(&world_id.into(), feedback_msg)
                    .await;
            }
        }
        DmActionResult::DialogueGenerated {
            npc_id: _,
            npc_name,
            dialogue,
        } => {
            let message = ServerMessage::DialogueResponse {
                speaker_id: npc_name.clone(),
                speaker_name: npc_name,
                text: dialogue,
                choices: vec![],
            };
            world_connection_manager
                .broadcast_to_players(world_id.into(), message)
                .await;
        }
        DmActionResult::EventTriggered {
            event_id: _,
            event_name,
            outcome,
        } => {
            let message = ServerMessage::Error {
                code: "NARRATIVE_EVENT_TRIGGERED".to_string(),
                message: format!(
                    "Narrative event '{}' has been triggered{}",
                    event_name,
                    outcome
                        .map(|o| format!(": {}", o))
                        .unwrap_or_default()
                ),
            };
            world_connection_manager
                .broadcast_to_players(world_id.into(), message)
                .await;
        }
        DmActionResult::SceneTransitioned {
            scene_id: _,
            scene_data,
        } => {
            // Parse the scene_data JSON and construct ServerMessage::SceneUpdate
            if let Ok(update) = parse_scene_update_from_json(scene_data) {
                world_connection_manager
                    .broadcast_to_world(world_id.into(), update)
                    .await;
            } else {
                tracing::error!("Failed to parse scene data for SceneUpdate message");
            }
        }
    }
}

/// Parse a SceneUpdate message from the scene_data JSON returned by the processor
fn parse_scene_update_from_json(scene_data: Value) -> Result<ServerMessage, ()> {
    use wrldbldr_protocol::{CharacterData, CharacterPosition, InteractionData, SceneData};

    let scene = scene_data.get("scene").ok_or(())?;
    let characters = scene_data.get("characters").ok_or(())?;
    let interactions = scene_data.get("interactions").ok_or(())?;

    let scene_data_result = SceneData {
        id: scene.get("id").and_then(Value::as_str).ok_or(())?.to_string(),
        name: scene.get("name").and_then(Value::as_str).ok_or(())?.to_string(),
        location_id: scene.get("location_id").and_then(Value::as_str).ok_or(())?.to_string(),
        location_name: scene.get("location_name").and_then(Value::as_str).ok_or(())?.to_string(),
        backdrop_asset: scene.get("backdrop_asset").and_then(Value::as_str).map(String::from),
        time_context: scene.get("time_context").and_then(Value::as_str).unwrap_or("Unspecified").to_string(),
        directorial_notes: scene.get("directorial_notes").and_then(Value::as_str).unwrap_or("").to_string(),
    };

    let characters: Vec<CharacterData> = characters
        .as_array()
        .ok_or(())?
        .iter()
        .filter_map(|c: &Value| {
            Some(CharacterData {
                id: c.get("id")?.as_str()?.to_string(),
                name: c.get("name")?.as_str()?.to_string(),
                sprite_asset: c.get("sprite_asset").and_then(Value::as_str).map(String::from),
                portrait_asset: c.get("portrait_asset").and_then(Value::as_str).map(String::from),
                position: CharacterPosition::Center,
                is_speaking: c.get("is_speaking").and_then(Value::as_bool).unwrap_or(false),
                emotion: c.get("emotion").and_then(Value::as_str).map(String::from),
            })
        })
        .collect();

    let interactions: Vec<InteractionData> = interactions
        .as_array()
        .ok_or(())?
        .iter()
        .filter_map(|i: &Value| {
            Some(InteractionData {
                id: i.get("id")?.as_str()?.to_string(),
                name: i.get("name")?.as_str()?.to_string(),
                interaction_type: i.get("interaction_type")?.as_str()?.to_string(),
                target_name: i.get("target_name").and_then(Value::as_str).map(String::from),
                is_available: i.get("is_available").and_then(Value::as_bool).unwrap_or(true),
            })
        })
        .collect();

    Ok(ServerMessage::SceneUpdate {
        scene: scene_data_result,
        characters,
        interactions,
    })
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
