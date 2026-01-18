use super::*;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use crate::api::websocket::ws_time::time_suggestion_to_protocol;
use crate::use_cases::movement::scene_change::{
    NavigationExitInfo, NavigationInfo, NavigationTargetInfo, NpcPresenceInfo, RegionInfo,
    RegionItemInfo,
};
use crate::use_cases::movement::{EnterRegionError, StagingStatus};
use wrldbldr_shared::{
    CharacterData, CharacterPosition, ErrorCode, InteractionData, NavigationData, NavigationExit,
    NavigationTarget, NpcPresenceData, RegionData, RegionItemData, SceneData,
};

// =============================================================================
// Domain -> Protocol Conversions for Scene Change
// =============================================================================

fn region_to_proto(region: RegionInfo) -> RegionData {
    RegionData {
        id: region.id,
        name: region.name,
        location_id: region.location_id,
        location_name: region.location_name,
        backdrop_asset: region.backdrop_asset,
        atmosphere: region.atmosphere,
        map_asset: region.map_asset,
    }
}

fn npc_presence_to_proto(npc: NpcPresenceInfo) -> NpcPresenceData {
    NpcPresenceData {
        character_id: npc.character_id,
        name: npc.name,
        sprite_asset: npc.sprite_asset,
        portrait_asset: npc.portrait_asset,
    }
}

fn navigation_to_proto(nav: NavigationInfo) -> NavigationData {
    NavigationData {
        connected_regions: nav
            .connected_regions
            .into_iter()
            .map(nav_target_to_proto)
            .collect(),
        exits: nav.exits.into_iter().map(nav_exit_to_proto).collect(),
    }
}

fn nav_target_to_proto(target: NavigationTargetInfo) -> NavigationTarget {
    NavigationTarget {
        region_id: target.region_id,
        name: target.name,
        is_locked: target.is_locked,
        lock_description: target.lock_description,
    }
}

fn nav_exit_to_proto(exit: NavigationExitInfo) -> NavigationExit {
    NavigationExit {
        location_id: exit.location_id,
        location_name: exit.location_name,
        arrival_region_id: exit.arrival_region_id,
        description: exit.description,
    }
}

fn region_item_to_proto(item: RegionItemInfo) -> RegionItemData {
    RegionItemData {
        id: item.id,
        name: item.name,
        description: item.description,
        item_type: item.item_type,
    }
}

/// Convert staging approval domain data to protocol message.
fn staging_approval_to_server_message(
    approval: &crate::use_cases::staging::StagingApprovalData,
) -> ServerMessage {
    ServerMessage::StagingApprovalRequired {
        request_id: approval.request_id.clone(),
        region_id: approval.region_id.to_string(),
        region_name: approval.region_name.clone(),
        location_id: approval.location_id.to_string(),
        location_name: approval.location_name.clone(),
        game_time: approval.game_time.to_protocol(),
        previous_staging: approval.previous_staging.as_ref().map(|s| s.to_protocol()),
        rule_based_npcs: approval
            .rule_based_npcs
            .iter()
            .map(|n| n.to_protocol())
            .collect(),
        llm_based_npcs: approval
            .llm_based_npcs
            .iter()
            .map(|n| n.to_protocol())
            .collect(),
        default_ttl_hours: approval.default_ttl_hours,
        waiting_pcs: approval
            .waiting_pcs
            .iter()
            .map(|pc| pc.to_protocol())
            .collect(),
        resolved_visual_state: approval
            .resolved_visual_state
            .as_ref()
            .map(|vs| vs.to_protocol()),
        available_location_states: approval
            .available_location_states
            .iter()
            .map(|s| s.to_protocol())
            .collect(),
        available_region_states: approval
            .available_region_states
            .iter()
            .map(|s| s.to_protocol())
            .collect(),
    }
}

pub(super) async fn handle_move_to_region(
    state: &WsState,
    connection_id: Uuid,
    pc_id: String,
    region_id: String,
) -> Option<ServerMessage> {
    // Parse IDs
    let pc_uuid = match parse_pc_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let region_uuid = match parse_region_id(&region_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };

    // Get connection info to verify authorization
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    // Verify the PC belongs to this connection (or is DM)
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response(
            ErrorCode::Unauthorized,
            "Cannot control this PC",
        ));
    }

    // Execute movement use case
    match state
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_uuid, region_uuid)
        .await
    {
        Ok(result) => {
            let world_id = result.pc.world_id();

            match result.staging_status {
                StagingStatus::Pending { previous_staging } => {
                    let pending_time_suggestions = crate::stores::TimeSuggestionStore::new(
                        state.pending_time_suggestions.clone(),
                    );
                    let pending_staging_requests = crate::stores::PendingStagingStore::new(
                        state.pending_staging_requests.clone(),
                    );
                    let ctx = crate::use_cases::staging::StagingApprovalContext {
                        pending_time_suggestions: &pending_time_suggestions,
                        pending_staging_requests: &pending_staging_requests,
                    };
                    let input = crate::use_cases::staging::StagingApprovalInput {
                        world_id,
                        region: result.region.clone(),
                        pc: result.pc.clone(),
                        previous_staging: *previous_staging,
                        time_suggestion: result.time_suggestion.clone(),
                        guidance: None,
                    };

                    match state
                        .app
                        .use_cases
                        .staging
                        .request_approval
                        .execute(&ctx, input)
                        .await
                    {
                        Ok(staging_result) => {
                            // Convert domain types to protocol and notify DMs
                            let approval_msg =
                                staging_approval_to_server_message(&staging_result.approval);
                            state
                                .connections
                                .broadcast_to_dms(world_id, approval_msg)
                                .await;

                            // Send time suggestion to DMs if present
                            if let Some(ref time_suggestion) = staging_result.time_suggestion {
                                let suggestion_msg = ServerMessage::TimeSuggestion {
                                    data: time_suggestion_to_protocol(time_suggestion),
                                };
                                state
                                    .connections
                                    .broadcast_to_dms(world_id, suggestion_msg)
                                    .await;
                            }

                            // Send StagingPending to player
                            let pending_msg = ServerMessage::StagingPending {
                                region_id: staging_result.pending.region_id.to_string(),
                                region_name: staging_result.pending.region_name,
                                timeout_seconds: staging_result.pending.timeout_seconds,
                            };
                            state.connections.send_to_pc(pc_uuid, pending_msg).await;

                            None
                        }
                        Err(e) => Some(error_response(
                            ErrorCode::InternalError,
                            &sanitize_repo_error(&e, "process staging"),
                        )),
                    }
                }
                StagingStatus::Ready => {
                    maybe_broadcast_time_suggestion(state, world_id, &result.time_suggestion).await;

                    let npcs = result.npcs.clone();
                    let scene_change = match state
                        .app
                        .use_cases
                        .scene_change
                        .build_scene_change(&result.region, npcs.clone(), conn_info.is_dm())
                        .await
                    {
                        Ok(sc) => sc,
                        Err(e) => {
                            return Some(error_response(
                                ErrorCode::InternalError,
                                &sanitize_repo_error(&e, "build scene"),
                            ));
                        }
                    };

                    if let Some(scene) = result.resolved_scene.as_ref() {
                        if let Some(scene_update) =
                            build_scene_update(state, scene, &result.region, &result.pc, &npcs)
                                .await
                        {
                            state.connections.send_to_pc(pc_uuid, scene_update).await;
                        }
                    }

                    send_triggered_events(state, pc_uuid, &result.triggered_events).await;

                    Some(ServerMessage::SceneChanged {
                        pc_id: pc_id.clone(),
                        region: region_to_proto(scene_change.region),
                        npcs_present: scene_change
                            .npcs_present
                            .into_iter()
                            .map(npc_presence_to_proto)
                            .collect(),
                        navigation: navigation_to_proto(scene_change.navigation),
                        region_items: scene_change
                            .region_items
                            .into_iter()
                            .map(region_item_to_proto)
                            .collect(),
                    })
                }
            }
        }
        Err(EnterRegionError::RegionNotFound(id)) => Some(error_response(
            ErrorCode::NotFound,
            &format!("Region not found: {}", id),
        )),
        Err(EnterRegionError::PlayerCharacterNotFound(id)) => Some(error_response(
            ErrorCode::NotFound,
            &format!("Player character not found: {}", id),
        )),
        Err(EnterRegionError::WorldNotFound(id)) => Some(error_response(
            ErrorCode::NotFound,
            &format!("World not found: {}", id),
        )),
        Err(EnterRegionError::RegionNotInCurrentLocation) => Some(error_response(
            ErrorCode::BadRequest,
            "Region not in current location",
        )),
        Err(EnterRegionError::NoPathToRegion) => Some(ServerMessage::MovementBlocked {
            pc_id,
            reason: "No path to region".to_string(),
        }),
        Err(EnterRegionError::MovementBlocked(reason)) => {
            Some(ServerMessage::MovementBlocked { pc_id, reason })
        }
        Err(e) => Some(error_response(
            ErrorCode::InternalError,
            &sanitize_repo_error(&e, "move to region"),
        )),
    }
}

pub(super) async fn handle_exit_to_location(
    state: &WsState,
    connection_id: Uuid,
    pc_id: String,
    location_id: String,
    arrival_region_id: Option<String>,
) -> Option<ServerMessage> {
    // Parse IDs
    let pc_uuid = match parse_pc_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let location_uuid = match parse_location_id(&location_id) {
        Ok(id) => id,
        Err(e) => return Some(e),
    };
    let arrival_region_uuid = match arrival_region_id {
        Some(ref id) => match parse_region_id(id) {
            Ok(r) => Some(r),
            Err(e) => return Some(e),
        },
        None => None,
    };

    // Get connection info to verify authorization
    let conn_info = match state.connections.get(connection_id).await {
        Some(info) => info,
        None => {
            return Some(error_response(
                ErrorCode::BadRequest,
                "Connection not found",
            ))
        }
    };

    // Verify the PC belongs to this connection (or is DM)
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response(
            ErrorCode::Unauthorized,
            "Cannot control this PC",
        ));
    }

    match state
        .app
        .use_cases
        .movement
        .exit_location
        .execute(pc_uuid, location_uuid, arrival_region_uuid)
        .await
    {
        Ok(result) => {
            let world_id = result.pc.world_id();

            match result.staging_status {
                StagingStatus::Pending { previous_staging } => {
                    let pending_time_suggestions = crate::stores::TimeSuggestionStore::new(
                        state.pending_time_suggestions.clone(),
                    );
                    let pending_staging_requests = crate::stores::PendingStagingStore::new(
                        state.pending_staging_requests.clone(),
                    );
                    let ctx = crate::use_cases::staging::StagingApprovalContext {
                        pending_time_suggestions: &pending_time_suggestions,
                        pending_staging_requests: &pending_staging_requests,
                    };
                    let input = crate::use_cases::staging::StagingApprovalInput {
                        world_id,
                        region: result.region.clone(),
                        pc: result.pc.clone(),
                        previous_staging: *previous_staging,
                        time_suggestion: result.time_suggestion.clone(),
                        guidance: None,
                    };

                    match state
                        .app
                        .use_cases
                        .staging
                        .request_approval
                        .execute(&ctx, input)
                        .await
                    {
                        Ok(staging_result) => {
                            // Convert domain types to protocol and notify DMs
                            let approval_msg =
                                staging_approval_to_server_message(&staging_result.approval);
                            state
                                .connections
                                .broadcast_to_dms(world_id, approval_msg)
                                .await;

                            // Send time suggestion to DMs if present
                            if let Some(ref time_suggestion) = staging_result.time_suggestion {
                                let suggestion_msg = ServerMessage::TimeSuggestion {
                                    data: time_suggestion_to_protocol(time_suggestion),
                                };
                                state
                                    .connections
                                    .broadcast_to_dms(world_id, suggestion_msg)
                                    .await;
                            }

                            // Send StagingPending to player
                            let pending_msg = ServerMessage::StagingPending {
                                region_id: staging_result.pending.region_id.to_string(),
                                region_name: staging_result.pending.region_name,
                                timeout_seconds: staging_result.pending.timeout_seconds,
                            };
                            state.connections.send_to_pc(pc_uuid, pending_msg).await;

                            None
                        }
                        Err(e) => Some(error_response(
                            ErrorCode::InternalError,
                            &sanitize_repo_error(&e, "process staging"),
                        )),
                    }
                }
                StagingStatus::Ready => {
                    maybe_broadcast_time_suggestion(state, world_id, &result.time_suggestion).await;

                    let npcs = result.npcs.clone();
                    let scene_change = match state
                        .app
                        .use_cases
                        .scene_change
                        .build_scene_change(&result.region, npcs.clone(), conn_info.is_dm())
                        .await
                    {
                        Ok(sc) => sc,
                        Err(e) => {
                            return Some(error_response(
                                ErrorCode::InternalError,
                                &sanitize_repo_error(&e, "build scene"),
                            ));
                        }
                    };

                    if let Some(scene) = result.resolved_scene.as_ref() {
                        if let Some(scene_update) =
                            build_scene_update(state, scene, &result.region, &result.pc, &npcs)
                                .await
                        {
                            state.connections.send_to_pc(pc_uuid, scene_update).await;
                        }
                    }

                    send_triggered_events(state, pc_uuid, &result.triggered_events).await;

                    Some(ServerMessage::SceneChanged {
                        pc_id: pc_id.clone(),
                        region: region_to_proto(scene_change.region),
                        npcs_present: scene_change
                            .npcs_present
                            .into_iter()
                            .map(npc_presence_to_proto)
                            .collect(),
                        navigation: navigation_to_proto(scene_change.navigation),
                        region_items: scene_change
                            .region_items
                            .into_iter()
                            .map(region_item_to_proto)
                            .collect(),
                    })
                }
            }
        }
        Err(crate::use_cases::movement::ExitLocationError::LocationNotFound) => {
            Some(error_response(ErrorCode::NotFound, "Location not found"))
        }
        Err(crate::use_cases::movement::ExitLocationError::RegionNotFound) => {
            Some(error_response(ErrorCode::NotFound, "Region not found"))
        }
        Err(crate::use_cases::movement::ExitLocationError::PlayerCharacterNotFound) => Some(
            error_response(ErrorCode::NotFound, "Player character not found"),
        ),
        Err(crate::use_cases::movement::ExitLocationError::RegionLocationMismatch) => Some(
            error_response(ErrorCode::BadRequest, "Region is not in target location"),
        ),
        Err(crate::use_cases::movement::ExitLocationError::WorldNotFound) => {
            Some(error_response(ErrorCode::NotFound, "World not found"))
        }
        Err(e) => Some(error_response(
            ErrorCode::InternalError,
            &sanitize_repo_error(&e, "exit to location"),
        )),
    }
}

// TODO: Time suggestions are stored in-memory and never expire.
// Consider adding a cleanup mechanism or TTL for suggestions that are never resolved.
// For now, this is acceptable as suggestions are typically resolved quickly.
// We do clean up stale suggestions for the same PC when a new suggestion is created below.
async fn maybe_broadcast_time_suggestion(
    state: &WsState,
    world_id: WorldId,
    time_suggestion: &Option<crate::use_cases::time::TimeSuggestion>,
) {
    if let Some(suggestion) = time_suggestion {
        let msg = ServerMessage::TimeSuggestion {
            data: time_suggestion_to_protocol(suggestion),
        };
        // Remove any existing suggestion for the same PC to prevent unbounded growth.
        // This handles the case where a player performs multiple actions before
        // the DM resolves the first suggestion.
        state
            .pending_time_suggestions
            .remove_for_pc(suggestion.pc_id)
            .await;
        state
            .pending_time_suggestions
            .insert(suggestion.id, suggestion.clone())
            .await;
        state.connections.broadcast_to_dms(world_id, msg).await;
    }
}

async fn build_scene_update(
    state: &WsState,
    scene: &wrldbldr_domain::Scene,
    region: &wrldbldr_domain::Region,
    pc: &wrldbldr_domain::PlayerCharacter,
    npcs: &[wrldbldr_domain::StagedNpc],
) -> Option<ServerMessage> {
    // Get location via graph edge
    let location_id = state
        .app
        .repositories
        .scene
        .get_location(scene.id())
        .await
        .ok()
        .flatten();

    let (location_id_str, location_name) = match location_id {
        Some(loc_id) => {
            let name = match state.app.repositories.location.get_location(loc_id).await {
                Ok(Some(location)) => location.name().to_string(),
                _ => "Unknown Location".to_string(),
            };
            (loc_id.to_string(), name)
        }
        None => ("".to_string(), "Unknown Location".to_string()),
    };

    let time_context = match scene.time_context() {
        wrldbldr_domain::TimeContext::Unspecified => "unspecified".to_string(),
        wrldbldr_domain::TimeContext::TimeOfDay(time) => time.to_string(),
        wrldbldr_domain::TimeContext::During(label) => format!("during {}", label),
        wrldbldr_domain::TimeContext::Custom(label) => label.clone(),
    };

    let scene_data = SceneData {
        id: scene.id().to_string(),
        name: scene.name().to_string(),
        location_id: location_id_str,
        location_name,
        backdrop_asset: scene
            .backdrop_override()
            .map(|s| s.to_string())
            .or_else(|| region.backdrop_asset().map(|s| s.to_string())),
        time_context,
        directorial_notes: scene.directorial_notes().to_string(),
    };

    let mut characters = Vec::with_capacity(1 + npcs.len());
    characters.push(CharacterData {
        id: pc.id().to_string(),
        name: pc.name().to_string(),
        sprite_asset: pc.sprite_asset().map(|s| s.to_string()),
        portrait_asset: pc.portrait_asset().map(|s| s.to_string()),
        position: CharacterPosition::Center,
        is_speaking: false,
        expression: None,
        mood: None,
    });

    for (index, npc) in npcs.iter().enumerate() {
        let position = match index {
            0 => CharacterPosition::Left,
            1 => CharacterPosition::Right,
            _ => CharacterPosition::OffScreen,
        };

        characters.push(CharacterData {
            id: npc.character_id.to_string(),
            name: npc.name.clone(),
            sprite_asset: npc.sprite_asset.as_ref().map(|a| a.to_string()),
            portrait_asset: npc.portrait_asset.as_ref().map(|a| a.to_string()),
            position,
            is_speaking: false,
            expression: None,
            mood: Some(npc.mood.to_string()),
        });
    }

    let interactions = match state
        .app
        .repositories
        .interaction
        .list_for_scene(scene.id())
        .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(error = %e, scene_id = %scene.id(), "Failed to load scene interactions");
            Vec::new()
        }
    };

    let mut interaction_data = Vec::with_capacity(interactions.len());
    for interaction in interactions {
        let (target_id, target_type, target_name) =
            resolve_interaction_target(state, interaction.target()).await;
        interaction_data.push(InteractionData {
            id: interaction.id().to_string(),
            name: interaction.name().to_string(),
            interaction_type: interaction_type_to_str(interaction.interaction_type()).to_string(),
            target_name,
            target_id,
            target_type,
            is_available: interaction.is_available(),
        });
    }

    Some(ServerMessage::SceneUpdate {
        scene: scene_data,
        characters,
        interactions: interaction_data,
    })
}

async fn send_triggered_events(
    state: &WsState,
    pc_id: PlayerCharacterId,
    events: &[wrldbldr_domain::NarrativeEvent],
) {
    for event in events {
        let outcome_description = resolve_event_outcome_description(event);
        let msg = ServerMessage::NarrativeEventTriggered {
            event_id: event.id().to_string(),
            event_name: event.name().to_string(),
            outcome_description,
            scene_direction: event.scene_direction().to_string(),
        };
        state.connections.send_to_pc(pc_id, msg).await;
    }
}

fn resolve_event_outcome_description(event: &wrldbldr_domain::NarrativeEvent) -> String {
    if let Some(default_name) = event.default_outcome() {
        if let Some(outcome) = event.outcomes().iter().find(|o| o.name == default_name) {
            return outcome.description.clone();
        }
    }

    event
        .outcomes()
        .first()
        .map(|outcome| outcome.description.clone())
        .unwrap_or_default()
}

fn interaction_type_to_str(interaction_type: &wrldbldr_domain::InteractionType) -> &'static str {
    match interaction_type {
        wrldbldr_domain::InteractionType::Dialogue => "dialogue",
        wrldbldr_domain::InteractionType::Examine => "examine",
        wrldbldr_domain::InteractionType::UseItem => "use_item",
        wrldbldr_domain::InteractionType::PickUp => "pick_up",
        wrldbldr_domain::InteractionType::GiveItem => "give_item",
        wrldbldr_domain::InteractionType::Attack => "attack",
        wrldbldr_domain::InteractionType::Travel => "travel",
        wrldbldr_domain::InteractionType::Custom(_) => "custom",
    }
}

async fn resolve_interaction_target(
    state: &WsState,
    target: &wrldbldr_domain::InteractionTarget,
) -> (Option<String>, Option<String>, Option<String>) {
    match target {
        wrldbldr_domain::InteractionTarget::Character(id) => {
            let name = state
                .app
                .repositories
                .character
                .get(*id)
                .await
                .ok()
                .flatten()
                .map(|c| c.name().to_string());
            (Some(id.to_string()), Some("character".to_string()), name)
        }
        wrldbldr_domain::InteractionTarget::Item(id) => {
            let name = state
                .app
                .repositories
                .item
                .get(*id)
                .await
                .ok()
                .flatten()
                .map(|item| item.name().to_string());
            (Some(id.to_string()), Some("item".to_string()), name)
        }
        wrldbldr_domain::InteractionTarget::Environment(label) => {
            (None, Some("environment".to_string()), Some(label.clone()))
        }
        wrldbldr_domain::InteractionTarget::None => (None, None, None),
    }
}
