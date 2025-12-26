//! Movement handlers
//!
//! Handles PC movement between regions and locations.
//! Includes staging system integration for NPC presence approval.

use chrono::Timelike;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket::converters::fetch_region_items;
use crate::infrastructure::world_state_manager::WorldPendingStagingApproval;
use wrldbldr_engine_ports::outbound::PlayerCharacterRepositoryPort;
use wrldbldr_protocol::{
    GameTime, NavigationData, NavigationExit, NavigationTarget, NpcPresenceData, RegionData,
    ServerMessage, StagedNpcInfo, WaitingPcInfo,
};

// =============================================================================
// SelectPlayerCharacter Handler
// =============================================================================

/// Handles a request to select a player character for play.
///
/// This handler:
/// 1. Parses the PC ID
/// 2. Fetches the PC from the repository
/// 3. Returns PcSelected with the PC's current position
///
/// # Arguments
/// * `state` - The application state containing repositories
/// * `client_id` - The WebSocket client ID making the request
/// * `pc_id` - The player character ID to select
///
/// # Returns
/// * `Some(ServerMessage::PcSelected)` on success
/// * `Some(ServerMessage::Error)` on failure
pub async fn handle_select_player_character(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
) -> Option<ServerMessage> {
    tracing::debug!(pc_id = %pc_id, "SelectPlayerCharacter request received");

    // Parse pc_id UUID
    let pc_uuid = match Uuid::parse_str(&pc_id) {
        Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_PC_ID".to_string(),
                message: "Invalid PC ID format".to_string(),
            });
        }
    };

    // Get PC from repository
    let pc = match state.repository.player_characters().get(pc_uuid).await {
        Ok(Some(pc)) => pc,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "PC_NOT_FOUND".to_string(),
                message: format!("Player character {} not found", pc_id),
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch PC");
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch player character".to_string(),
            });
        }
    };

    tracing::info!(
        client_id = %client_id,
        pc_id = %pc_id,
        pc_name = %pc.name,
        "Player selected character"
    );

    Some(ServerMessage::PcSelected {
        pc_id: pc.id.to_string(),
        pc_name: pc.name,
        location_id: pc.current_location_id.to_string(),
        region_id: pc.current_region_id.map(|r| r.to_string()),
    })
}

// =============================================================================
// MoveToRegion Handler
// =============================================================================

/// Handles a request to move a player character to a different region.
///
/// This handler implements the full staging system workflow:
/// 1. Validates the connection and parses IDs
/// 2. Checks for locked region connections
/// 3. Updates PC position in the database
/// 4. Checks for existing valid staging in the target region
/// 5. If valid staging exists: returns SceneChanged with NPCs
/// 6. If pending staging exists: adds PC to waiting list, sends StagingPending
/// 7. If no staging: generates proposal, sends StagingApprovalRequired to DM
///
/// # Arguments
/// * `state` - The application state containing all services
/// * `client_id` - The WebSocket client ID making the request
/// * `pc_id` - The player character ID to move
/// * `region_id` - The target region ID
/// * `sender` - Channel to send additional messages (for DM notifications)
///
/// # Returns
/// * `Some(ServerMessage::SceneChanged)` if staging exists
/// * `Some(ServerMessage::StagingPending)` if waiting for DM approval
/// * `Some(ServerMessage::Error)` on failure
pub async fn handle_move_to_region(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    region_id: String,
    sender: mpsc::UnboundedSender<ServerMessage>,
) -> Option<ServerMessage> {
    tracing::debug!(
        pc_id = %pc_id,
        region_id = %region_id,
        "MoveToRegion request received"
    );

    // Get connection context
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };

    let world_id = match connection.world_id {
        Some(id) => id,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };

    // Parse pc_id UUID
    let pc_uuid = match Uuid::parse_str(&pc_id) {
        Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_PC_ID".to_string(),
                message: "Invalid PC ID format".to_string(),
            });
        }
    };

    // Parse region_id UUID
    let region_uuid = match Uuid::parse_str(&region_id) {
        Ok(uuid) => wrldbldr_domain::RegionId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_REGION_ID".to_string(),
                message: "Invalid region ID format".to_string(),
            });
        }
    };

    // Get PC's current region to check for locked connections
    let pc = match state.repository.player_characters().get(pc_uuid).await {
        Ok(Some(pc)) => pc,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "PC_NOT_FOUND".to_string(),
                message: format!("Player character {} not found", pc_id),
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch PC");
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch player character".to_string(),
            });
        }
    };

    // Check for locked connections if PC has a current region
    if let Some(current_region_id) = pc.current_region_id {
        let connections = match state
            .repository
            .regions()
            .get_connections(current_region_id)
            .await
        {
            Ok(conns) => conns,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to fetch region connections");
                vec![]
            }
        };

        // Find the connection to the target region
        if let Some(conn) = connections.iter().find(|c| c.to_region == region_uuid) {
            if conn.is_locked {
                let reason = conn
                    .lock_description
                    .clone()
                    .unwrap_or_else(|| "The way is blocked".to_string());
                return Some(ServerMessage::MovementBlocked {
                    pc_id: pc_id.clone(),
                    reason,
                });
            }
        }
    }

    // Get target region details
    let region = match state.repository.regions().get(region_uuid).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "REGION_NOT_FOUND".to_string(),
                message: format!("Region {} not found", region_id),
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch region");
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch region".to_string(),
            });
        }
    };

    // Get location details
    let location = match state.repository.locations().get(region.location_id).await {
        Ok(Some(l)) => l,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "LOCATION_NOT_FOUND".to_string(),
                message: "Region's location not found".to_string(),
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch location");
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch location".to_string(),
            });
        }
    };

    // Update PC position
    if let Err(e) = state
        .repository
        .player_characters()
        .update_region(pc_uuid, region_uuid)
        .await
    {
        tracing::error!(error = %e, "Failed to update PC region");
        return Some(ServerMessage::Error {
            code: "DATABASE_ERROR".to_string(),
            message: "Failed to update position".to_string(),
        });
    }

    let world_id_domain = wrldbldr_domain::WorldId::from_uuid(world_id);

    // Handle staging system
    handle_staging_for_region(
        state,
        client_id,
        client_id_str,
        world_id,
        world_id_domain,
        region_uuid,
        region.location_id,
        &region.name,
        &location.name,
        &pc_id,
        &pc.name,
        connection.user_id.clone(),
        &location,
        sender,
    )
    .await
}

// =============================================================================
// ExitToLocation Handler
// =============================================================================

/// Handles a request to exit to a different location.
///
/// This handler implements location-to-location transitions with staging:
/// 1. Validates the connection and parses IDs
/// 2. Determines the arrival region (from param, location default, or first spawn)
/// 3. Updates PC position (location and region)
/// 4. Applies the same staging workflow as MoveToRegion
///
/// # Arguments
/// * `state` - The application state containing all services
/// * `client_id` - The WebSocket client ID making the request
/// * `pc_id` - The player character ID to move
/// * `location_id` - The target location ID
/// * `arrival_region_id` - Optional specific region to arrive in
/// * `sender` - Channel to send additional messages (for DM notifications)
///
/// # Returns
/// * `Some(ServerMessage::SceneChanged)` if staging exists
/// * `Some(ServerMessage::StagingPending)` if waiting for DM approval
/// * `Some(ServerMessage::Error)` on failure
pub async fn handle_exit_to_location(
    state: &AppState,
    client_id: Uuid,
    pc_id: String,
    location_id: String,
    arrival_region_id: Option<String>,
    sender: mpsc::UnboundedSender<ServerMessage>,
) -> Option<ServerMessage> {
    tracing::debug!(
        pc_id = %pc_id,
        location_id = %location_id,
        arrival_region_id = ?arrival_region_id,
        "ExitToLocation request received"
    );

    // Get connection context
    let client_id_str = client_id.to_string();
    let connection = match state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
    {
        Some(conn) => conn,
        None => {
            return Some(ServerMessage::Error {
                code: "NOT_CONNECTED".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };

    let world_id = match connection.world_id {
        Some(id) => id,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };

    // Parse pc_id UUID
    let pc_uuid = match Uuid::parse_str(&pc_id) {
        Ok(uuid) => wrldbldr_domain::PlayerCharacterId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_PC_ID".to_string(),
                message: "Invalid PC ID format".to_string(),
            });
        }
    };

    // Parse location_id UUID
    let location_uuid = match Uuid::parse_str(&location_id) {
        Ok(uuid) => wrldbldr_domain::LocationId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_LOCATION_ID".to_string(),
                message: "Invalid location ID format".to_string(),
            });
        }
    };

    // Parse optional arrival_region_id
    let arrival_region_uuid = if let Some(ref arrival_id) = arrival_region_id {
        match Uuid::parse_str(arrival_id) {
            Ok(uuid) => Some(wrldbldr_domain::RegionId::from_uuid(uuid)),
            Err(_) => {
                return Some(ServerMessage::Error {
                    code: "INVALID_REGION_ID".to_string(),
                    message: "Invalid arrival region ID format".to_string(),
                });
            }
        }
    } else {
        None
    };

    // Get PC
    let pc = match state.repository.player_characters().get(pc_uuid).await {
        Ok(Some(pc)) => pc,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "PC_NOT_FOUND".to_string(),
                message: format!("Player character {} not found", pc_id),
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch PC");
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch player character".to_string(),
            });
        }
    };

    // Get target location
    let location = match state.repository.locations().get(location_uuid).await {
        Ok(Some(l)) => l,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "LOCATION_NOT_FOUND".to_string(),
                message: format!("Location {} not found", location_id),
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch location");
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch location".to_string(),
            });
        }
    };

    // Determine arrival region: explicit > location default > first spawn point
    let final_arrival_region = match arrival_region_uuid {
        Some(region_id) => {
            // Verify the region exists and belongs to this location
            match state.repository.regions().get(region_id).await {
                Ok(Some(region)) if region.location_id == location_uuid => Some(region),
                Ok(Some(_)) => {
                    return Some(ServerMessage::Error {
                        code: "REGION_MISMATCH".to_string(),
                        message: "Arrival region does not belong to target location".to_string(),
                    });
                }
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "REGION_NOT_FOUND".to_string(),
                        message: "Arrival region not found".to_string(),
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to fetch arrival region");
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: "Failed to fetch arrival region".to_string(),
                    });
                }
            }
        }
        None => {
            // Try location's default arrival region, then first spawn point
            if let Some(default_region_id) = location.default_region_id {
                match state.repository.regions().get(default_region_id).await {
                    Ok(Some(region)) => Some(region),
                    Ok(None) | Err(_) => {
                        // Fallback to first spawn point
                        find_first_spawn_point(state, location_uuid).await
                    }
                }
            } else {
                // No default, use first spawn point
                find_first_spawn_point(state, location_uuid).await
            }
        }
    };

    let arrival_region = match final_arrival_region {
        Some(r) => r,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_ARRIVAL_REGION".to_string(),
                message: "Could not determine arrival region for location".to_string(),
            });
        }
    };

    let arrival_region_id = arrival_region.id;

    // Update PC position (both location and region)
    if let Err(e) = state
        .repository
        .player_characters()
        .update_position(pc_uuid, location_uuid, Some(arrival_region_id))
        .await
    {
        tracing::error!(error = %e, "Failed to update PC position");
        return Some(ServerMessage::Error {
            code: "DATABASE_ERROR".to_string(),
            message: "Failed to update position".to_string(),
        });
    }

    let world_id_domain = wrldbldr_domain::WorldId::from_uuid(world_id);

    // Handle staging system
    handle_staging_for_region(
        state,
        client_id,
        client_id_str,
        world_id,
        world_id_domain,
        arrival_region_id,
        location_uuid,
        &arrival_region.name,
        &location.name,
        &pc_id,
        &pc.name,
        connection.user_id.clone(),
        &location,
        sender,
    )
    .await
}

// =============================================================================
// Common Staging Logic
// =============================================================================

/// Handles the staging system workflow for a region arrival.
///
/// This is extracted as common logic used by both MoveToRegion and ExitToLocation.
///
/// Workflow:
/// 1. Check for existing valid staging
/// 2. If valid staging exists: return SceneChanged with NPCs
/// 3. Check for pending staging approval
/// 4. If pending: add PC to waiting list, return StagingPending
/// 5. If no staging: generate proposal, send to DM, return StagingPending
#[allow(clippy::too_many_arguments)]
async fn handle_staging_for_region(
    state: &AppState,
    client_id: Uuid,
    client_id_str: String,
    world_id: Uuid,
    world_id_domain: wrldbldr_domain::WorldId,
    region_id: wrldbldr_domain::RegionId,
    location_id: wrldbldr_domain::LocationId,
    region_name: &str,
    location_name: &str,
    pc_id: &str,
    pc_name: &str,
    user_id: String,
    location: &wrldbldr_domain::entities::Location,
    sender: mpsc::UnboundedSender<ServerMessage>,
) -> Option<ServerMessage> {
    // Get current game time
    let game_time = state.world_state.get_game_time(&world_id_domain).unwrap_or_default();

    // Check for existing valid staging
    match state
        .staging_service
        .get_current_staging(region_id, &game_time)
        .await
    {
        Ok(Some(staging)) => {
            // Valid staging exists - return SceneChanged
            tracing::debug!(
                region_id = %region_id,
                staging_id = %staging.id,
                "Using existing valid staging"
            );

            return build_scene_changed_response(
                state,
                pc_id,
                region_id,
                location_id,
                region_name,
                location_name,
                location,
                &staging.npcs,
            )
            .await;
        }
        Ok(None) => {
            // No valid staging - check for pending
            tracing::debug!(
                region_id = %region_id,
                "No valid staging, checking for pending"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to check staging, continuing without staging");
        }
    }

    // Check for pending staging approval
    let has_pending = state
        .world_state
        .get_pending_staging_for_region(&world_id_domain, &region_id)
        .is_some();

    if has_pending {
        // Add PC to waiting list
        state.world_state.with_pending_staging_for_region_mut(
            &world_id_domain,
            &region_id,
            |approval| {
                approval.add_waiting_pc(
                    Uuid::parse_str(pc_id).unwrap_or_else(|_| Uuid::new_v4()),
                    pc_name.to_string(),
                    user_id.clone(),
                    client_id_str.clone(),
                );
            },
        );

        tracing::info!(
            pc_id = %pc_id,
            region_id = %region_id,
            "PC added to staging wait list"
        );

        return Some(ServerMessage::StagingPending {
            region_id: region_id.to_string(),
            region_name: region_name.to_string(),
        });
    }

    // No staging exists - generate a proposal
    let ttl_hours = location.presence_cache_ttl_hours;

    let proposal = match state
        .staging_service
        .generate_proposal(
            world_id_domain,
            region_id,
            location_id,
            location_name,
            &game_time,
            ttl_hours,
            None, // No DM guidance yet
        )
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "Failed to generate staging proposal");
            // Return scene changed with no NPCs as fallback
            return build_scene_changed_response(
                state,
                pc_id,
                region_id,
                location_id,
                region_name,
                location_name,
                location,
                &[],
            )
            .await;
        }
    };

    // Get previous staging for DM reference
    let previous_staging = state
        .staging_service
        .get_previous_staging(region_id)
        .await
        .ok()
        .flatten();

    // Create pending approval
    let pending_approval = WorldPendingStagingApproval::new(
        proposal.request_id.clone(),
        region_id,
        location_id,
        world_id_domain,
        region_name.to_string(),
        location_name.to_string(),
        proposal.clone(),
    );

    // Add waiting PC
    let mut pending_with_pc = pending_approval;
    pending_with_pc.add_waiting_pc(
        Uuid::parse_str(pc_id).unwrap_or_else(|_| Uuid::new_v4()),
        pc_name.to_string(),
        user_id,
        client_id_str,
    );

    // Store pending approval
    state
        .world_state
        .add_pending_staging(&world_id_domain, pending_with_pc.clone());

    // Convert proposal NPCs to protocol format
    let rule_based_npcs: Vec<StagedNpcInfo> = proposal
        .rule_based_npcs
        .iter()
        .map(|n| StagedNpcInfo {
            character_id: n.character_id.clone(),
            name: n.name.clone(),
            sprite_asset: n.sprite_asset.clone(),
            portrait_asset: n.portrait_asset.clone(),
            is_present: n.is_present,
            reasoning: n.reasoning.clone(),
            is_hidden_from_players: n.is_hidden_from_players,
        })
        .collect();

    let llm_based_npcs: Vec<StagedNpcInfo> = proposal
        .llm_based_npcs
        .iter()
        .map(|n| StagedNpcInfo {
            character_id: n.character_id.clone(),
            name: n.name.clone(),
            sprite_asset: n.sprite_asset.clone(),
            portrait_asset: n.portrait_asset.clone(),
            is_present: n.is_present,
            reasoning: n.reasoning.clone(),
            is_hidden_from_players: n.is_hidden_from_players,
        })
        .collect();

    let waiting_pcs: Vec<WaitingPcInfo> = pending_with_pc
        .waiting_pcs
        .iter()
        .map(|w| WaitingPcInfo {
            pc_id: w.pc_id.to_string(),
            pc_name: w.pc_name.clone(),
            player_id: w.user_id.clone(),
        })
        .collect();

    let previous_staging_info = previous_staging.map(|s| wrldbldr_protocol::PreviousStagingInfo {
        staging_id: s.id.to_string(),
        approved_at: s.approved_at.to_rfc3339(),
        npcs: s
            .npcs
            .iter()
            .map(|n| StagedNpcInfo {
                character_id: n.character_id.to_string(),
                name: n.name.clone(),
                sprite_asset: n.sprite_asset.clone(),
                portrait_asset: n.portrait_asset.clone(),
                is_present: n.is_present,
                reasoning: n.reasoning.clone(),
                is_hidden_from_players: n.is_hidden_from_players,
            })
            .collect(),
    });

    // Convert game time to protocol format
    let game_time_proto = GameTime {
        day: game_time.day_ordinal(),
        hour: game_time.current().hour() as u8,
        minute: game_time.current().minute() as u8,
        is_paused: game_time.is_paused(),
    };

    // Build StagingApprovalRequired message for DM
    let dm_message = ServerMessage::StagingApprovalRequired {
        request_id: proposal.request_id.clone(),
        region_id: region_id.to_string(),
        region_name: region_name.to_string(),
        location_id: location_id.to_string(),
        location_name: location_name.to_string(),
        game_time: game_time_proto,
        previous_staging: previous_staging_info,
        rule_based_npcs,
        llm_based_npcs,
        default_ttl_hours: ttl_hours,
        waiting_pcs,
    };

    // Send to DM via the sender channel
    if let Err(e) = sender.send(dm_message) {
        tracing::error!(error = %e, "Failed to send staging approval to DM");
    }

    // Also broadcast to DM via world connection manager
    let _ = state
        .world_connection_manager
        .send_to_dm(
            &world_id,
            ServerMessage::StagingApprovalRequired {
                request_id: proposal.request_id,
                region_id: region_id.to_string(),
                region_name: region_name.to_string(),
                location_id: location_id.to_string(),
                location_name: location_name.to_string(),
                game_time: GameTime {
                    day: game_time.day_ordinal(),
                    hour: game_time.current().hour() as u8,
                    minute: game_time.current().minute() as u8,
                    is_paused: game_time.is_paused(),
                },
                previous_staging: None, // Already sent above
                rule_based_npcs: vec![],
                llm_based_npcs: vec![],
                default_ttl_hours: ttl_hours,
                waiting_pcs: vec![],
            },
        )
        .await;

    tracing::info!(
        pc_id = %pc_id,
        region_id = %region_id,
        "Staging approval requested from DM"
    );

    Some(ServerMessage::StagingPending {
        region_id: region_id.to_string(),
        region_name: region_name.to_string(),
    })
}

/// Builds a SceneChanged response with NPCs and navigation data.
#[allow(clippy::too_many_arguments)]
async fn build_scene_changed_response(
    state: &AppState,
    pc_id: &str,
    region_id: wrldbldr_domain::RegionId,
    location_id: wrldbldr_domain::LocationId,
    region_name: &str,
    location_name: &str,
    location: &wrldbldr_domain::entities::Location,
    staged_npcs: &[wrldbldr_domain::entities::StagedNpc],
) -> Option<ServerMessage> {
    // Build NPC presence data (only visible NPCs)
    let npcs_present: Vec<NpcPresenceData> = staged_npcs
        .iter()
        .filter(|n| n.is_present && !n.is_hidden_from_players)
        .map(|n| NpcPresenceData {
            character_id: n.character_id.to_string(),
            name: n.name.clone(),
            sprite_asset: n.sprite_asset.clone(),
            portrait_asset: n.portrait_asset.clone(),
        })
        .collect();

    // Build navigation data
    let navigation = build_navigation_data(state, region_id).await;

    // Fetch region items
    let region_items = fetch_region_items(state, region_id).await;

    // Get region backdrop
    let region = state.repository.regions().get(region_id).await.ok().flatten();
    let backdrop = region
        .as_ref()
        .and_then(|r| r.backdrop_asset.clone())
        .or_else(|| location.backdrop_asset.clone());

    Some(ServerMessage::SceneChanged {
        pc_id: pc_id.to_string(),
        region: RegionData {
            id: region_id.to_string(),
            name: region_name.to_string(),
            location_id: location_id.to_string(),
            location_name: location_name.to_string(),
            backdrop_asset: backdrop,
            atmosphere: region.and_then(|r| r.atmosphere),
            map_asset: location.map_asset.clone(),
        },
        npcs_present,
        navigation,
        region_items,
    })
}

/// Builds navigation data for a region (connected regions and exits).
async fn build_navigation_data(
    state: &AppState,
    region_id: wrldbldr_domain::RegionId,
) -> NavigationData {
    let mut connected_regions = Vec::new();
    let mut exits = Vec::new();

    // Get region connections
    match state.repository.regions().get_connections(region_id).await {
        Ok(connections) => {
            for conn in connections {
                // Get target region name
                let target_region = state
                    .repository
                    .regions()
                    .get(conn.to_region)
                    .await
                    .ok()
                    .flatten();

                if let Some(region) = target_region {
                    connected_regions.push(NavigationTarget {
                        region_id: conn.to_region.to_string(),
                        name: region.name,
                        is_locked: conn.is_locked,
                        lock_description: conn.lock_description,
                    });
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch region connections");
        }
    }

    // Get exits to other locations
    match state.repository.regions().get_exits(region_id).await {
        Ok(region_exits) => {
            for exit in region_exits {
                // Get target location name
                let target_location = state
                    .repository
                    .locations()
                    .get(exit.to_location)
                    .await
                    .ok()
                    .flatten();

                if let Some(loc) = target_location {
                    // Determine arrival region for the exit (use exit's arrival_region_id or location default)
                    let arrival_region_id = loc.default_region_id
                        .unwrap_or(exit.arrival_region_id);

                    exits.push(NavigationExit {
                        location_id: exit.to_location.to_string(),
                        location_name: loc.name,
                        arrival_region_id: arrival_region_id.to_string(),
                        description: exit.description,
                    });
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch region exits");
        }
    }

    NavigationData {
        connected_regions,
        exits,
    }
}

/// Finds the first spawn point region in a location.
async fn find_first_spawn_point(
    state: &AppState,
    location_id: wrldbldr_domain::LocationId,
) -> Option<wrldbldr_domain::entities::Region> {
    match state.repository.locations().get_regions(location_id).await {
        Ok(regions) => regions.into_iter().find(|r| r.is_spawn_point),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch regions for spawn point");
            None
        }
    }
}
