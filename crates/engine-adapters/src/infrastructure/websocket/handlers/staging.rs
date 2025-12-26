//! Staging system handlers
//!
//! Handles DM approval of NPC presence staging, regeneration requests,
//! and proactive region staging.
//!
//! ## Handlers
//!
//! - [`handle_staging_approval_response`]: DM approves/modifies a staging proposal
//! - [`handle_staging_regenerate_request`]: DM requests LLM to regenerate suggestions
//! - [`handle_pre_stage_region`]: DM pre-stages a region before player arrival

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket::converters::fetch_region_items;
use wrldbldr_engine_app::application::services::staging_service::ApprovedNpcData;
use wrldbldr_protocol::{ApprovedNpcInfo, ServerMessage, StagedNpcInfo};

/// Handles a staging approval response from the DM.
///
/// **DM-only**: This handler processes DM approval of NPC staging proposals.
///
/// This handler:
/// 1. Validates the client is connected to a world
/// 2. Retrieves the pending staging approval by request ID
/// 3. Parses the staging source (rule/llm/custom)
/// 4. Builds approved NPC data from the approval response
/// 5. Calls the staging service to persist the staging
/// 6. Sends `StagingReady` and `SceneChanged` to all waiting PCs
/// 7. Removes the pending staging from world state
///
/// # Arguments
/// * `state` - The application state containing all services
/// * `client_id` - The WebSocket client ID making the request
/// * `request_id` - The ID of the staging request being approved
/// * `approved_npcs` - List of NPCs with presence decisions
/// * `ttl_hours` - Time-to-live in hours for this staging
/// * `source` - How this staging was finalized: "rule", "llm", or "custom"
///
/// # Returns
/// * `None` on success (messages are sent to waiting PCs)
/// * `Some(ServerMessage::Error)` on failure
pub async fn handle_staging_approval_response(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    approved_npcs: Vec<ApprovedNpcInfo>,
    ttl_hours: i32,
    source: String,
) -> Option<ServerMessage> {
    tracing::info!(
        request_id = %request_id,
        npc_count = approved_npcs.len(),
        ttl_hours = ttl_hours,
        source = %source,
        "Staging approval response received"
    );

    // Get client connection
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
                message: "Client is not connected".to_string(),
            });
        }
    };

    let world_id_uuid = match connection.world_id {
        Some(id) => id,
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };
    let world_id = wrldbldr_domain::WorldId::from_uuid(world_id_uuid);
    let dm_user_id = connection.user_id.clone();

    // Get the pending staging approval from WorldStateManager
    let pending = match state
        .world_state
        .get_pending_staging_by_request_id(&world_id, &request_id)
    {
        Some(p) => p,
        None => {
            return Some(ServerMessage::Error {
                code: "STAGING_NOT_FOUND".to_string(),
                message: format!("Pending staging request {} not found", request_id),
            });
        }
    };

    // Parse staging source
    let staging_source = match source.as_str() {
        "rule" => wrldbldr_domain::entities::StagingSource::RuleBased,
        "llm" => wrldbldr_domain::entities::StagingSource::LlmBased,
        "custom" => wrldbldr_domain::entities::StagingSource::DmCustomized,
        _ => wrldbldr_domain::entities::StagingSource::DmCustomized,
    };

    // Get character data for approved NPCs
    let mut approved_npc_data = Vec::new();
    for npc_info in &approved_npcs {
        let char_id = match uuid::Uuid::parse_str(&npc_info.character_id) {
            Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
            Err(_) => continue,
        };

        // Find character in proposal to get name and assets
        let (name, sprite, portrait) = pending
            .proposal
            .rule_based_npcs
            .iter()
            .chain(pending.proposal.llm_based_npcs.iter())
            .find(|n| n.character_id == npc_info.character_id)
            .map(|n| {
                (
                    n.name.clone(),
                    n.sprite_asset.clone(),
                    n.portrait_asset.clone(),
                )
            })
            .unwrap_or_else(|| ("Unknown".to_string(), None, None));

        approved_npc_data.push(ApprovedNpcData {
            character_id: char_id,
            name,
            sprite_asset: sprite,
            portrait_asset: portrait,
            is_present: npc_info.is_present,
            is_hidden_from_players: npc_info.is_hidden_from_players,
            reasoning: npc_info
                .reasoning
                .clone()
                .unwrap_or_else(|| "DM approved".to_string()),
        });
    }

    // Get game time from WorldStateManager
    let game_time = state
        .world_state
        .get_game_time(&pending.world_id)
        .unwrap_or_default();

    // Approve the staging
    let staging = match state
        .staging_service
        .approve_staging(
            pending.region_id,
            pending.location_id,
            pending.world_id,
            &game_time,
            approved_npc_data,
            ttl_hours,
            staging_source,
            &dm_user_id,
            None,
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "STAGING_APPROVAL_FAILED".to_string(),
                message: format!("Failed to approve staging: {}", e),
            });
        }
    };

    // Build the NPC presence list for players
    let npcs_present: Vec<wrldbldr_protocol::NpcPresentInfo> = staging
        .npcs
        .iter()
        .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
        .map(|npc| wrldbldr_protocol::NpcPresentInfo {
            character_id: npc.character_id.to_string(),
            name: npc.name.clone(),
            sprite_asset: npc.sprite_asset.clone(),
            portrait_asset: npc.portrait_asset.clone(),
            is_hidden_from_players: false,
        })
        .collect();

    // Send StagingReady to all waiting PCs via world connection manager
    let staging_ready = ServerMessage::StagingReady {
        region_id: pending.region_id.to_string(),
        npcs_present: npcs_present.clone(),
    };

    // Send to each waiting PC
    for waiting_pc in &pending.waiting_pcs {
        // Send StagingReady
        let _ = state
            .world_connection_manager
            .send_to_user_in_world(&world_id_uuid, &waiting_pc.user_id, staging_ready.clone())
            .await;

        // Also send SceneChanged with the NPCs
        // Get region and location data for the scene change
        let map_asset = state
            .repository
            .locations()
            .get(pending.location_id)
            .await
            .ok()
            .flatten()
            .and_then(|loc| loc.map_asset);

        if let Ok(Some(region)) = state.repository.regions().get(pending.region_id).await {
            let connections = state
                .repository
                .regions()
                .get_connections(pending.region_id)
                .await
                .unwrap_or_default();
            let exits = state
                .repository
                .regions()
                .get_exits(pending.region_id)
                .await
                .unwrap_or_default();

            let mut connected_regions = Vec::new();
            for conn in connections {
                if let Ok(Some(target)) = state.repository.regions().get(conn.to_region).await {
                    connected_regions.push(wrldbldr_protocol::NavigationTarget {
                        region_id: conn.to_region.to_string(),
                        name: target.name,
                        is_locked: conn.is_locked,
                        lock_description: conn.lock_description,
                    });
                }
            }

            let mut exit_targets = Vec::new();
            for exit in exits {
                if let Ok(Some(target_loc)) =
                    state.repository.locations().get(exit.to_location).await
                {
                    exit_targets.push(wrldbldr_protocol::NavigationExit {
                        location_id: exit.to_location.to_string(),
                        location_name: target_loc.name,
                        arrival_region_id: exit.arrival_region_id.to_string(),
                        description: exit.description,
                    });
                }
            }

            let region_items = fetch_region_items(state, pending.region_id).await;
            let scene_changed = ServerMessage::SceneChanged {
                pc_id: waiting_pc.pc_id.to_string(),
                region: wrldbldr_protocol::RegionData {
                    id: pending.region_id.to_string(),
                    name: region.name.clone(),
                    location_id: pending.location_id.to_string(),
                    location_name: pending.location_name.clone(),
                    backdrop_asset: region.backdrop_asset.clone(),
                    atmosphere: region.atmosphere.clone(),
                    map_asset: map_asset.clone(),
                },
                npcs_present: npcs_present
                    .iter()
                    .map(|npc| wrldbldr_protocol::NpcPresenceData {
                        character_id: npc.character_id.clone(),
                        name: npc.name.clone(),
                        sprite_asset: npc.sprite_asset.clone(),
                        portrait_asset: npc.portrait_asset.clone(),
                    })
                    .collect(),
                navigation: wrldbldr_protocol::NavigationData {
                    connected_regions,
                    exits: exit_targets,
                },
                region_items,
            };
            let _ = state
                .world_connection_manager
                .send_to_user_in_world(&world_id_uuid, &waiting_pc.user_id, scene_changed)
                .await;
        }
    }

    // Remove the pending staging approval
    state
        .world_state
        .remove_pending_staging(&world_id, &request_id);

    tracing::info!(
        request_id = %request_id,
        region_id = %pending.region_id,
        waiting_pcs = pending.waiting_pcs.len(),
        "Staging approved and sent to waiting PCs"
    );

    None // No direct response to DM
}

/// Handles a staging regeneration request from the DM.
///
/// **DM-only**: This handler allows DM to request new LLM suggestions with guidance.
///
/// This handler:
/// 1. Validates the client is connected to a world
/// 2. Retrieves the pending staging approval by request ID
/// 3. Calls the staging service to regenerate LLM suggestions
/// 4. Returns `StagingRegenerated` with the new suggestions
///
/// # Arguments
/// * `state` - The application state containing all services
/// * `client_id` - The WebSocket client ID making the request
/// * `request_id` - The ID of the staging request to regenerate
/// * `guidance` - Guidance text for the LLM regeneration
///
/// # Returns
/// * `Some(ServerMessage::StagingRegenerated)` on success
/// * `Some(ServerMessage::Error)` on failure
pub async fn handle_staging_regenerate_request(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    guidance: String,
) -> Option<ServerMessage> {
    tracing::info!(
        request_id = %request_id,
        guidance = %guidance,
        "Staging regenerate request received"
    );

    // Get client connection
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
                message: "Client is not connected".to_string(),
            });
        }
    };

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };

    // Get the pending staging approval from WorldStateManager
    let pending = match state
        .world_state
        .get_pending_staging_by_request_id(&world_id, &request_id)
    {
        Some(p) => p,
        None => {
            return Some(ServerMessage::Error {
                code: "STAGING_NOT_FOUND".to_string(),
                message: format!("Pending staging request {} not found", request_id),
            });
        }
    };

    // Get game time from WorldStateManager
    let game_time = state
        .world_state
        .get_game_time(&pending.world_id)
        .unwrap_or_default();

    // Regenerate LLM suggestions
    let new_suggestions = match state
        .staging_service
        .regenerate_suggestions(
            pending.world_id,
            pending.region_id,
            &pending.location_name,
            &game_time,
            &guidance,
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "REGENERATION_FAILED".to_string(),
                message: format!("Failed to regenerate suggestions: {}", e),
            });
        }
    };

    // Convert to protocol format
    let llm_based_npcs: Vec<StagedNpcInfo> = new_suggestions
        .into_iter()
        .map(|npc| StagedNpcInfo {
            character_id: npc.character_id,
            name: npc.name,
            sprite_asset: npc.sprite_asset,
            portrait_asset: npc.portrait_asset,
            is_present: npc.is_present,
            reasoning: npc.reasoning,
            is_hidden_from_players: npc.is_hidden_from_players,
        })
        .collect();

    tracing::info!(
        request_id = %request_id,
        new_count = llm_based_npcs.len(),
        "Staging suggestions regenerated"
    );

    Some(ServerMessage::StagingRegenerated {
        request_id,
        llm_based_npcs,
    })
}

/// Handles a pre-stage region request from the DM.
///
/// **DM-only**: This handler allows DM to proactively stage a region before players arrive.
///
/// This handler:
/// 1. Validates the client is connected to a world
/// 2. Parses and validates the region ID
/// 3. Fetches region and location data
/// 4. Builds approved NPC data from the request
/// 5. Calls the staging service to pre-stage the region
///
/// # Arguments
/// * `state` - The application state containing all services
/// * `client_id` - The WebSocket client ID making the request
/// * `region_id` - The ID of the region to pre-stage
/// * `npcs` - List of NPCs to pre-stage
/// * `ttl_hours` - Time-to-live in hours for this staging
///
/// # Returns
/// * `None` on success
/// * `Some(ServerMessage::Error)` on failure
pub async fn handle_pre_stage_region(
    state: &AppState,
    client_id: Uuid,
    region_id: String,
    npcs: Vec<ApprovedNpcInfo>,
    ttl_hours: i32,
) -> Option<ServerMessage> {
    tracing::info!(
        region_id = %region_id,
        npc_count = npcs.len(),
        ttl_hours = ttl_hours,
        "Pre-stage region request received"
    );

    // Get client connection
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
                message: "Client is not connected".to_string(),
            });
        }
    };

    let world_id = match connection.world_id {
        Some(id) => wrldbldr_domain::WorldId::from_uuid(id),
        None => {
            return Some(ServerMessage::Error {
                code: "NO_WORLD".to_string(),
                message: "Not connected to a world".to_string(),
            });
        }
    };
    let dm_user_id = connection.user_id.clone();

    // Parse region ID
    let region_uuid = match uuid::Uuid::parse_str(&region_id) {
        Ok(uuid) => wrldbldr_domain::RegionId::from_uuid(uuid),
        Err(_) => {
            return Some(ServerMessage::Error {
                code: "INVALID_REGION_ID".to_string(),
                message: "Invalid region ID format".to_string(),
            });
        }
    };

    // Get region and location
    let region = match state.repository.regions().get(region_uuid).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "REGION_NOT_FOUND".to_string(),
                message: "Region not found".to_string(),
            });
        }
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to fetch region: {}", e),
            });
        }
    };

    let location = match state.repository.locations().get(region.location_id).await {
        Ok(Some(l)) => l,
        Ok(None) => {
            return Some(ServerMessage::Error {
                code: "LOCATION_NOT_FOUND".to_string(),
                message: "Location not found".to_string(),
            });
        }
        Err(e) => {
            return Some(ServerMessage::Error {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to fetch location: {}", e),
            });
        }
    };

    // Get game time from WorldStateManager
    let game_time = state.world_state.get_game_time(&world_id).unwrap_or_default();

    // Build approved NPC data
    let mut approved_npc_data = Vec::new();
    for npc_info in &npcs {
        let char_id = match uuid::Uuid::parse_str(&npc_info.character_id) {
            Ok(uuid) => wrldbldr_domain::CharacterId::from_uuid(uuid),
            Err(_) => continue,
        };

        // Fetch character for name and assets
        let (name, sprite, portrait) = match state.repository.characters().get(char_id).await {
            Ok(Some(c)) => (c.name, c.sprite_asset, c.portrait_asset),
            _ => ("Unknown".to_string(), None, None),
        };

        approved_npc_data.push(ApprovedNpcData {
            character_id: char_id,
            name,
            sprite_asset: sprite,
            portrait_asset: portrait,
            is_present: npc_info.is_present,
            is_hidden_from_players: npc_info.is_hidden_from_players,
            reasoning: npc_info
                .reasoning
                .clone()
                .unwrap_or_else(|| "Pre-staged by DM".to_string()),
        });
    }

    // Pre-stage the region
    match state
        .staging_service
        .pre_stage_region(
            region_uuid,
            region.location_id,
            location.world_id,
            &game_time,
            approved_npc_data,
            ttl_hours,
            &dm_user_id,
        )
        .await
    {
        Ok(staging) => {
            tracing::info!(
                staging_id = %staging.id,
                region_id = %region_id,
                npc_count = staging.npcs.len(),
                "Region pre-staged successfully"
            );
            None // Success, no response needed
        }
        Err(e) => Some(ServerMessage::Error {
            code: "PRESTAGE_FAILED".to_string(),
            message: format!("Failed to pre-stage region: {}", e),
        }),
    }
}
