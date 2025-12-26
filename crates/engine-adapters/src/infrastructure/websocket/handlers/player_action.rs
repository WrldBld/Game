//! Player action message handler.
//!
//! This handler processes PlayerAction messages from clients. It has two distinct behaviors:
//!
//! ## Travel Actions
//! When `action_type` is "travel", the action is processed immediately:
//! 1. Updates the player character's location in the database
//! 2. Resolves the scene for the new location
//! 3. Sends a SceneUpdate to the player with the new scene data
//! 4. Checks for split party conditions and notifies the DM if applicable
//! 5. Returns an ActionReceived acknowledgment
//!
//! ## All Other Actions
//! For non-travel actions, the handler:
//! 1. Enqueues the action to the PlayerActionQueue for later processing
//! 2. Sends an ActionQueued event to the DM with queue depth info
//! 3. Returns an ActionReceived acknowledgment to the player
//!
//! The queued actions are processed asynchronously by the action processor,
//! which may involve LLM processing or DM approval workflows.

use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use wrldbldr_domain::ActionId;
use wrldbldr_protocol::{CharacterData, CharacterPosition, InteractionData, SceneData, ServerMessage};

/// Handles a PlayerAction message from a client.
///
/// # Arguments
/// * `state` - The application state containing all services
/// * `client_id` - The unique identifier of the client sending the action
/// * `action_type` - The type of action (e.g., "travel", "interact", "speak")
/// * `target` - Optional target of the action (e.g., location ID, character ID)
/// * `dialogue` - Optional dialogue text for speech actions
/// * `sender` - Channel sender for sending the ActionReceived acknowledgment
///
/// # Returns
/// * `Some(ServerMessage)` - An error message if the action failed
/// * `None` - If the action was successfully processed or enqueued
pub async fn handle_player_action(
    state: &AppState,
    client_id: Uuid,
    action_type: String,
    target: Option<String>,
    dialogue: Option<String>,
    sender: mpsc::UnboundedSender<ServerMessage>,
) -> Option<ServerMessage> {
    tracing::debug!("Received player action: {} -> {:?}", action_type, target);

    // Generate a unique action ID for tracking
    let action_id = ActionId::new();
    let action_id_str = action_id.to_string();

    // Get the client's connection info via WorldConnectionManager
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            tracing::warn!("Client {} sent action but is not connected", client_id);
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            });
        }
    };

    let world_id = match connection.world_id {
        Some(id) => id,
        None => {
            tracing::warn!("Client {} sent action but is not in a world", client_id);
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };

    let player_id = connection.user_id.clone();

    // Convert world_id to domain WorldId for service calls
    let world_id_domain = wrldbldr_domain::WorldId::from_uuid(world_id);

    // Handle Travel actions immediately (update location and resolve scene)
    if action_type == "travel" {
        if let Some(location_id_str) = target.as_ref() {
            // Parse location ID
            let location_uuid = match uuid::Uuid::parse_str(location_id_str) {
                Ok(uuid) => wrldbldr_domain::LocationId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_LOCATION_ID".to_string(),
                        message: "Invalid location ID format".to_string(),
                    });
                }
            };

            // Get PC for this user
            match state
                .player
                .player_character_service
                .get_pc_by_user_and_world(&player_id, &world_id_domain)
                .await
            {
                Ok(Some(pc)) => {
                    // Update PC location
                    if let Err(e) = state
                        .player
                        .player_character_service
                        .update_pc_location(pc.id, location_uuid)
                        .await
                    {
                        tracing::error!("Failed to update PC location: {}", e);
                        return Some(ServerMessage::Error {
                            code: "LOCATION_UPDATE_FAILED".to_string(),
                            message: format!("Failed to update location: {}", e),
                        });
                    }

                    // Resolve scene for the new location
                    match state
                        .player
                        .scene_resolution_service
                        .resolve_scene_for_pc(pc.id)
                        .await
                    {
                        Ok(Some(scene)) => {
                            // Load scene with relations to build SceneUpdate
                            match state.core.scene_service.get_scene_with_relations(scene.id).await
                            {
                                Ok(Some(scene_with_relations)) => {
                                    // Load interactions for the scene
                                    let interaction_templates = match state
                                        .core
                                        .interaction_service
                                        .list_interactions(scene.id)
                                        .await
                                    {
                                        Ok(templates) => templates,
                                        Err(_) => vec![],
                                    };

                                    // Build interactions
                                    let interactions: Vec<InteractionData> = interaction_templates
                                        .iter()
                                        .map(|i| {
                                            let target_name = match &i.target {
                                                wrldbldr_domain::entities::InteractionTarget::Character(char_id) => {
                                                    Some(format!("Character {}", char_id))
                                                }
                                                wrldbldr_domain::entities::InteractionTarget::Item(item_id) => {
                                                    Some(format!("Item {}", item_id))
                                                }
                                                wrldbldr_domain::entities::InteractionTarget::Environment(desc) => {
                                                    Some(desc.clone())
                                                }
                                                wrldbldr_domain::entities::InteractionTarget::None => None,
                                            };
                                            InteractionData {
                                                id: i.id.to_string(),
                                                name: i.name.clone(),
                                                target_name,
                                                interaction_type: format!("{:?}", i.interaction_type),
                                                is_available: i.is_available,
                                            }
                                        })
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
                                            location_id: scene_with_relations
                                                .scene
                                                .location_id
                                                .to_string(),
                                            location_name: scene_with_relations.location.name.clone(),
                                            backdrop_asset: scene_with_relations
                                                .scene
                                                .backdrop_override
                                                .or(scene_with_relations
                                                    .location
                                                    .backdrop_asset
                                                    .clone()),
                                            time_context: match &scene_with_relations
                                                .scene
                                                .time_context
                                            {
                                                wrldbldr_domain::entities::TimeContext::Unspecified => {
                                                    "Unspecified".to_string()
                                                }
                                                wrldbldr_domain::entities::TimeContext::TimeOfDay(
                                                    tod,
                                                ) => format!("{:?}", tod),
                                                wrldbldr_domain::entities::TimeContext::During(s) => {
                                                    s.clone()
                                                }
                                                wrldbldr_domain::entities::TimeContext::Custom(s) => {
                                                    s.clone()
                                                }
                                            },
                                            directorial_notes: scene_with_relations
                                                .scene
                                                .directorial_notes
                                                .clone(),
                                        },
                                        characters,
                                        interactions,
                                    };

                                    // Send scene update to player via WorldConnectionManager
                                    state
                                        .world_connection_manager
                                        .send_to_user(&player_id, world_id, scene_update.clone())
                                        .await;
                                    tracing::info!(
                                        "Sent scene update to player {} after travel to location {}",
                                        player_id,
                                        location_id_str
                                    );

                                    // Check for split party and notify DM
                                    if let Ok(resolution_result) = state
                                        .player
                                        .scene_resolution_service
                                        .resolve_scene_for_world(&world_id_domain)
                                        .await
                                    {
                                        if resolution_result.is_split_party {
                                            // Get location details for notification
                                            let pcs = match state
                                                .player
                                                .player_character_service
                                                .get_pcs_by_world(&world_id_domain)
                                                .await
                                            {
                                                Ok(pcs) => pcs,
                                                Err(_) => vec![],
                                            };

                                            // Group PCs by location
                                            let mut location_pcs: HashMap<String, Vec<&_>> =
                                                HashMap::new();
                                            for pc in &pcs {
                                                location_pcs
                                                    .entry(pc.current_location_id.to_string())
                                                    .or_insert_with(Vec::new)
                                                    .push(pc);
                                            }

                                            // Build location info
                                            let mut split_locations = Vec::new();
                                            for (loc_id_str, pcs_at_loc) in location_pcs.iter() {
                                                if let Ok(location) = state
                                                    .core
                                                    .location_service
                                                    .get_location(
                                                        wrldbldr_domain::LocationId::from_uuid(
                                                            uuid::Uuid::parse_str(loc_id_str)
                                                                .unwrap_or_default(),
                                                        ),
                                                    )
                                                    .await
                                                {
                                                    if let Some(loc) = location {
                                                        split_locations.push(
                                                            wrldbldr_protocol::SplitPartyLocation {
                                                                location_id: loc_id_str.to_string(),
                                                                location_name: loc.name,
                                                                pc_count: pcs_at_loc.len(),
                                                                pc_names: pcs_at_loc
                                                                    .iter()
                                                                    .map(|pc| pc.name.clone())
                                                                    .collect(),
                                                            },
                                                        );
                                                    }
                                                }
                                            }

                                            // Send notification to DM via WorldConnectionManager
                                            let dm_msg = ServerMessage::SplitPartyNotification {
                                                location_count: split_locations.len(),
                                                locations: split_locations,
                                            };
                                            let _ = state
                                                .world_connection_manager
                                                .send_to_dm(&world_id, dm_msg)
                                                .await;
                                        }
                                    }

                                    // Return acknowledgment
                                    return Some(ServerMessage::ActionReceived {
                                        action_id: action_id_str,
                                        player_id: player_id.clone(),
                                        action_type: action_type.clone(),
                                    });
                                }
                                Ok(None) => {
                                    tracing::warn!(
                                        "Scene {} not found after resolution",
                                        scene.id
                                    );
                                }
                                Err(e) => {
                                    tracing::error!("Failed to load scene with relations: {}", e);
                                }
                            }
                        }
                        Ok(None) => {
                            // No scene found, but location updated - still acknowledge
                            tracing::warn!(
                                "No scene found for location {} after travel",
                                location_id_str
                            );
                        }
                        Err(e) => {
                            tracing::error!("Failed to resolve scene: {}", e);
                        }
                    }
                }
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "NO_PC".to_string(),
                        message: "You must create a character before traveling".to_string(),
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to get PC: {}", e);
                    return Some(ServerMessage::Error {
                        code: "PC_LOOKUP_FAILED".to_string(),
                        message: format!("Failed to find your character: {}", e),
                    });
                }
            }
        } else {
            return Some(ServerMessage::Error {
                code: "MISSING_TARGET".to_string(),
                message: "Travel action requires a target location".to_string(),
            });
        }
    }

    // Look up the player's character ID for challenge targeting
    let pc_id = match state
        .player
        .player_character_service
        .get_pc_by_user_and_world(&player_id, &world_id_domain)
        .await
    {
        Ok(Some(pc)) => Some(pc.id),
        Ok(None) => {
            tracing::debug!(
                "Player {} has no character selected in world {}",
                player_id,
                world_id
            );
            None
        }
        Err(e) => {
            tracing::warn!("Failed to look up PC for player {}: {}", player_id, e);
            None
        }
    };

    // Enqueue to PlayerActionQueue - returns immediately
    match state
        .queues
        .player_action_queue_service
        .enqueue_action(
            &world_id_domain,
            player_id.clone(),
            pc_id,
            action_type.clone(),
            target.clone(),
            dialogue.clone(),
        )
        .await
    {
        Ok(_) => {
            // Get queue depth for status update
            let depth = state
                .queues
                .player_action_queue_service
                .depth()
                .await
                .unwrap_or(0);

            // Send ActionQueued event to DM via WorldConnectionManager
            let dm_msg = ServerMessage::ActionQueued {
                action_id: action_id_str.clone(),
                player_name: player_id.clone(),
                action_type: action_type.clone(),
                queue_depth: depth,
            };
            let _ = state.world_connection_manager.send_to_dm(&world_id, dm_msg).await;

            tracing::info!(
                "Enqueued action {} from player {} in world {}: {} -> {:?}",
                action_id_str,
                player_id,
                world_id,
                action_type,
                target
            );

            // Send ActionReceived acknowledgment to the player
            let _ = sender.send(ServerMessage::ActionReceived {
                action_id: action_id_str,
                player_id,
                action_type: action_type.clone(),
            });
        }
        Err(e) => {
            tracing::error!("Failed to enqueue player action: {}", e);
            return Some(ServerMessage::Error {
                code: "QUEUE_ERROR".to_string(),
                message: format!("Failed to queue action: {}", e),
            });
        }
    }

    None // No response from here; responses come from LLM processing or DM approval
}
