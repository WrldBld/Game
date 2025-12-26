//! Miscellaneous WebSocket message handlers.
//!
//! This module contains handlers for various utility and DM-specific operations:
//! - ComfyUI health checks
//! - NPC location sharing (DM only)
//! - Approach event triggering (DM only)
//! - Location event triggering (DM only)

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use wrldbldr_engine_ports::outbound::{
    CharacterRepositoryPort, ObservationRepositoryPort, PlayerCharacterRepositoryPort,
};
use wrldbldr_protocol::ServerMessage;

/// Handles a ComfyUI health check request.
///
/// This handler spawns an async task to perform the health check and broadcast
/// the result to all connected clients. The health check result is sent as a
/// `ComfyUIStateChanged` message to all worlds.
///
/// # Arguments
/// * `state` - The application state containing ComfyUI client and connection manager
///
/// # Returns
/// * `None` - Response is sent asynchronously via broadcast
pub async fn handle_check_comfyui_health(state: &AppState) -> Option<ServerMessage> {
    let comfyui_client = state.comfyui_client.clone();
    let world_connection_manager = state.world_connection_manager.clone();

    tokio::spawn(async move {
        let (state_str, message) = match comfyui_client.health_check().await {
            Ok(true) => ("connected".to_string(), None),
            Ok(false) => (
                "disconnected".to_string(),
                Some("ComfyUI is not responding".to_string()),
            ),
            Err(e) => (
                "disconnected".to_string(),
                Some(format!("Health check failed: {}", e)),
            ),
        };

        // Broadcast to all connected clients
        let msg = ServerMessage::ComfyUIStateChanged {
            state: state_str,
            message,
            retry_in_seconds: None,
        };

        // Broadcast to all worlds
        let world_ids = world_connection_manager.get_all_world_ids().await;
        for world_id in world_ids {
            world_connection_manager
                .broadcast_to_world(world_id, msg.clone())
                .await;
        }
    });

    None // Response sent asynchronously
}

/// Handles sharing an NPC's location with a player character.
///
/// **DM-only**: This handler requires the client to be the DM of the world.
///
/// Creates a "HeardAbout" observation for the PC, indicating they have learned
/// about an NPC's location through the DM sharing this information (e.g., via
/// an NPC telling them, or through investigation results).
///
/// # Arguments
/// * `state` - The application state containing repositories and connection manager
/// * `client_id` - The WebSocket client ID making the request
/// * `pc_id` - The player character ID to share the information with
/// * `npc_id` - The NPC whose location is being shared
/// * `location_id` - The location where the NPC was observed
/// * `region_id` - The specific region within the location
/// * `notes` - Optional notes about how the PC learned this information
///
/// # Returns
/// * `None` on success
/// * `Some(ServerMessage::Error)` if not authorized, not connected, or database error
pub async fn handle_share_npc_location(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    npc_id: String,
    location_id: String,
    region_id: String,
    notes: Option<String>,
) -> Option<ServerMessage> {
    tracing::info!("DM sharing NPC {} location with PC {}", npc_id, pc_id);

    // Only DMs can share NPC locations
    let client_id_str = client_id.to_string();

    // Get connection
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    // Extract world_id (used for logging, not directly needed for observation)
    let _world_id = match connection.world_id {
        Some(id) => id,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can share NPC locations".to_string(),
        });
    }

    // Parse IDs
    let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
        Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_PC_ID".to_string(),
                message: "Invalid PC ID format".to_string(),
            });
        }
    };
    let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
        Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_NPC_ID".to_string(),
                message: "Invalid NPC ID format".to_string(),
            });
        }
    };
    let location_uuid = match uuid::Uuid::parse_str(&location_id) {
        Ok(uuid) => wrldbldr_domain::LocationId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_LOCATION_ID".to_string(),
                message: "Invalid location ID format".to_string(),
            });
        }
    };
    let region_uuid = match uuid::Uuid::parse_str(&region_id) {
        Ok(uuid) => wrldbldr_domain::RegionId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_REGION_ID".to_string(),
                message: "Invalid region ID format".to_string(),
            });
        }
    };

    // Get game time - for now use current time
    // TODO: Once game_time is migrated to world-based, fetch from world
    let game_time = chrono::Utc::now();

    // Create HeardAbout observation
    let observation = wrldbldr_domain::entities::NpcObservation::heard_about(
        pc_uuid,
        npc_uuid,
        location_uuid,
        region_uuid,
        game_time,
        notes,
    );

    // Store the observation
    match state.repository.observations().upsert(&observation).await {
        Ok(()) => {
            tracing::info!(
                "Created HeardAbout observation: PC {} now knows NPC {} was at region {}",
                pc_id,
                npc_id,
                region_id
            );
            // Could broadcast to the player here if we had their client ID
            None
        }
        Err(e) => {
            tracing::error!("Failed to create observation: {}", e);
            Some(ServerMessage::Error {
                code: "OBSERVATION_ERROR".to_string(),
                message: format!("Failed to record observation: {}", e),
            })
        }
    }
}

/// Handles triggering an NPC approach event.
///
/// **DM-only**: This handler requires the client to be the DM of the world.
///
/// Triggers an approach event where an NPC approaches a player character.
/// This creates a direct observation for the PC (if reveal is true, the NPC's
/// identity is known; otherwise, they appear as "Unknown Figure").
///
/// # Arguments
/// * `state` - The application state containing repositories and connection manager
/// * `client_id` - The WebSocket client ID making the request
/// * `npc_id` - The NPC who is approaching
/// * `target_pc_id` - The player character being approached
/// * `description` - Description of the approach event
/// * `reveal` - Whether to reveal the NPC's identity to the player
///
/// # Returns
/// * `None` on success (event is sent to the target player)
/// * `Some(ServerMessage::Error)` if not authorized, not connected, or entity not found
pub async fn handle_trigger_approach_event(
    state: &AppState,
    client_id: Uuid,
    npc_id: String,
    target_pc_id: String,
    description: String,
    reveal: bool,
) -> Option<ServerMessage> {
    tracing::info!(
        "DM triggering approach event: NPC {} approaching PC {}",
        npc_id,
        target_pc_id
    );

    // Only DMs can trigger approach events
    let client_id_str = client_id.to_string();

    // Get connection
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    // Extract world_id
    let world_id = match connection.world_id {
        Some(id) => id,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can trigger approach events".to_string(),
        });
    }

    // Parse NPC ID and get NPC details
    let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
        Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_NPC_ID".to_string(),
                message: "Invalid NPC ID format".to_string(),
            });
        }
    };

    // Get NPC details
    let npc = match state.repository.characters().get(npc_uuid).await {
        Ok(Some(npc)) => npc,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "NPC_NOT_FOUND".to_string(),
                message: "NPC not found".to_string(),
            });
        }
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to fetch NPC: {}", e),
            });
        }
    };

    // Parse PC ID and get PC details (for region)
    let pc_uuid = match uuid::Uuid::parse_str(&target_pc_id) {
        Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_PC_ID".to_string(),
                message: "Invalid PC ID format".to_string(),
            });
        }
    };

    let pc = match state.repository.player_characters().get(pc_uuid).await {
        Ok(Some(pc)) => pc,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "PC_NOT_FOUND".to_string(),
                message: "Player character not found".to_string(),
            });
        }
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to fetch PC: {}", e),
            });
        }
    };

    // Create Direct observation for the PC (they now see the NPC)
    if let Some(region_id) = pc.current_region_id {
        // Get game time - for now use current time
        // TODO: Once game_time is migrated to world-based, fetch from world
        let game_time = chrono::Utc::now();

        let observation = if reveal {
            wrldbldr_domain::entities::NpcObservation::direct(
                pc_uuid,
                npc_uuid,
                pc.current_location_id,
                region_id,
                game_time,
            )
        } else {
            wrldbldr_domain::entities::NpcObservation::direct_unrevealed(
                pc_uuid,
                npc_uuid,
                pc.current_location_id,
                region_id,
                game_time,
            )
        };

        if let Err(e) = state.repository.observations().upsert(&observation).await {
            tracing::warn!("Failed to create observation for approach event: {}", e);
        }
    }

    // Build the ApproachEvent message
    let (npc_name, npc_sprite) = if reveal {
        (npc.name.clone(), npc.sprite_asset.clone())
    } else {
        ("Unknown Figure".to_string(), None)
    };

    let approach_event = ServerMessage::ApproachEvent {
        npc_id: npc_id.clone(),
        npc_name,
        npc_sprite,
        description,
        reveal,
    };

    // Send to the target PC's user only (not broadcast to all)
    state
        .world_connection_manager
        .send_to_user(&pc.user_id, world_id, approach_event)
        .await;

    tracing::info!(
        "Approach event triggered: {} approached by {}",
        target_pc_id,
        npc.name
    );
    None
}

/// Handles triggering a location-wide event.
///
/// **DM-only**: This handler requires the client to be the DM of the world.
///
/// Triggers a location event that is broadcast to all players in the world.
/// Clients are responsible for filtering based on their current region.
///
/// # Arguments
/// * `state` - The application state containing connection manager
/// * `client_id` - The WebSocket client ID making the request
/// * `region_id` - The region where the event occurs
/// * `description` - Description of the event
///
/// # Returns
/// * `None` on success (event is broadcast to the world)
/// * `Some(ServerMessage::Error)` if not authorized or not connected
pub async fn handle_trigger_location_event(
    state: &AppState,
    client_id: Uuid,
    region_id: String,
    description: String,
) -> Option<ServerMessage> {
    tracing::info!("DM triggering location event in region {}", region_id);

    // Only DMs can trigger location events
    let client_id_str = client_id.to_string();

    // Get connection
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Connection not found".to_string(),
            })
        }
    };

    // Extract world_id
    let world_id = match connection.world_id {
        Some(id) => id,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            })
        }
    };

    // Check DM authorization
    if !connection.is_dm() {
        return Some(ServerMessage::Error {
            code: "NOT_AUTHORIZED".to_string(),
            message: "Only the DM can trigger location events".to_string(),
        });
    }

    // Build the LocationEvent message
    let location_event = ServerMessage::LocationEvent {
        region_id: region_id.clone(),
        description,
    };

    // Broadcast to all in world - clients filter by their current region
    state
        .world_connection_manager
        .broadcast_to_world(world_id, location_event)
        .await;

    tracing::info!("Location event triggered in region {}", region_id);
    None
}
