use super::*;
use crate::use_cases::movement::{EnterRegionError, StagingStatus};
use wrldbldr_protocol::{CharacterData, CharacterPosition, InteractionData, SceneData};

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
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Verify the PC belongs to this connection (or is DM)
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response("UNAUTHORIZED", "Cannot control this PC"));
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
            let world_id = result.pc.world_id;

            match result.staging_status {
                StagingStatus::Pending { previous_staging } => {
                    let ctx = crate::use_cases::staging::StagingApprovalContext {
                        connections: &state.connections,
                        pending_time_suggestions: &state.pending_time_suggestions,
                        pending_staging_requests: &state.pending_staging_requests,
                    };
                    let input = crate::use_cases::staging::StagingApprovalInput {
                        world_id,
                        region: result.region.clone(),
                        pc: result.pc.clone(),
                        previous_staging,
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
                        Ok(msg) => {
                            state.connections.broadcast_to_dms(world_id, msg).await;

                            maybe_broadcast_time_suggestion(state, world_id, &result.time_suggestion)
                                .await;

                            let pending = ServerMessage::StagingPending {
                                region_id: region_id.clone(),
                                region_name: result.region.name.clone(),
                            };
                            state.connections.send_to_pc(pc_uuid, pending).await;

                            None
                        }
                        Err(e) => Some(error_response("STAGING_ERROR", &e.to_string())),
                    }
                }
                StagingStatus::Ready => {
                    maybe_broadcast_time_suggestion(state, world_id, &result.time_suggestion).await;

                    let npcs = result.npcs.clone();
                    let scene_change = state
                        .app
                        .use_cases
                        .scene_change
                        .build_scene_change(&result.region, npcs.clone(), conn_info.is_dm())
                        .await;

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
                        region: scene_change.region,
                        npcs_present: scene_change.npcs_present,
                        navigation: scene_change.navigation,
                        region_items: scene_change.region_items,
                    })
                }
            }
        }
        Err(EnterRegionError::RegionNotFound) => Some(error_response("NOT_FOUND", "Region not found")),
        Err(EnterRegionError::PlayerCharacterNotFound) => {
            Some(error_response("NOT_FOUND", "Player character not found"))
        }
        Err(EnterRegionError::WorldNotFound) => Some(error_response("NOT_FOUND", "World not found")),
        Err(EnterRegionError::RegionNotInCurrentLocation) => {
            Some(error_response("INVALID_MOVE", "Region not in current location"))
        }
        Err(EnterRegionError::NoPathToRegion) => Some(ServerMessage::MovementBlocked {
            pc_id,
            reason: "No path to region".to_string(),
        }),
        Err(EnterRegionError::MovementBlocked(reason)) => {
            Some(ServerMessage::MovementBlocked { pc_id, reason })
        }
        Err(e) => Some(error_response("MOVE_ERROR", &e.to_string())),
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
        None => return Some(error_response("NOT_CONNECTED", "Connection not found")),
    };

    // Verify the PC belongs to this connection (or is DM)
    if !conn_info.is_dm() && conn_info.pc_id != Some(pc_uuid) {
        return Some(error_response("UNAUTHORIZED", "Cannot control this PC"));
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
            let world_id = result.pc.world_id;

            match result.staging_status {
                StagingStatus::Pending { previous_staging } => {
                    let ctx = crate::use_cases::staging::StagingApprovalContext {
                        connections: &state.connections,
                        pending_time_suggestions: &state.pending_time_suggestions,
                        pending_staging_requests: &state.pending_staging_requests,
                    };
                    let input = crate::use_cases::staging::StagingApprovalInput {
                        world_id,
                        region: result.region.clone(),
                        pc: result.pc.clone(),
                        previous_staging,
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
                        Ok(msg) => {
                            state.connections.broadcast_to_dms(world_id, msg).await;

                            maybe_broadcast_time_suggestion(state, world_id, &result.time_suggestion)
                                .await;

                            let pending = ServerMessage::StagingPending {
                                region_id: result.region.id.to_string(),
                                region_name: result.region.name.clone(),
                            };
                            state.connections.send_to_pc(pc_uuid, pending).await;

                            None
                        }
                        Err(e) => Some(error_response("STAGING_ERROR", &e.to_string())),
                    }
                }
                StagingStatus::Ready => {
                    maybe_broadcast_time_suggestion(state, world_id, &result.time_suggestion).await;

                    let npcs = result.npcs.clone();
                    let scene_change = state
                        .app
                        .use_cases
                        .scene_change
                        .build_scene_change(&result.region, npcs.clone(), conn_info.is_dm())
                        .await;

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
                        region: scene_change.region,
                        npcs_present: scene_change.npcs_present,
                        navigation: scene_change.navigation,
                        region_items: scene_change.region_items,
                    })
                }
            }
        }
        Err(crate::use_cases::movement::ExitLocationError::LocationNotFound) => {
            Some(error_response("NOT_FOUND", "Location not found"))
        }
        Err(crate::use_cases::movement::ExitLocationError::RegionNotFound) => {
            Some(error_response("NOT_FOUND", "Region not found"))
        }
        Err(crate::use_cases::movement::ExitLocationError::PlayerCharacterNotFound) => {
            Some(error_response("NOT_FOUND", "Player character not found"))
        }
        Err(crate::use_cases::movement::ExitLocationError::RegionLocationMismatch) => {
            Some(error_response("INVALID_MOVE", "Region is not in target location"))
        }
        Err(crate::use_cases::movement::ExitLocationError::WorldNotFound) => {
            Some(error_response("NOT_FOUND", "World not found"))
        }
        Err(e) => Some(error_response("MOVE_ERROR", &e.to_string())),
    }
}

async fn maybe_broadcast_time_suggestion(
    state: &WsState,
    world_id: WorldId,
    time_suggestion: &Option<crate::use_cases::time::TimeSuggestion>,
) {
    if let Some(suggestion) = time_suggestion {
        let msg = ServerMessage::TimeSuggestion {
            data: suggestion.to_protocol(),
        };
        {
            let mut guard = state.pending_time_suggestions.write().await;
            guard.insert(suggestion.id, suggestion.clone());
        }
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
    let location_name = match state.app.entities.location.get(scene.location_id).await {
        Ok(Some(location)) => location.name,
        _ => "Unknown Location".to_string(),
    };

    let time_context = match &scene.time_context {
        wrldbldr_domain::TimeContext::Unspecified => "unspecified".to_string(),
        wrldbldr_domain::TimeContext::TimeOfDay(time) => time.to_string(),
        wrldbldr_domain::TimeContext::During(label) => format!("during {}", label),
        wrldbldr_domain::TimeContext::Custom(label) => label.clone(),
    };

    let scene_data = SceneData {
        id: scene.id.to_string(),
        name: scene.name.clone(),
        location_id: scene.location_id.to_string(),
        location_name,
        backdrop_asset: scene
            .backdrop_override
            .clone()
            .or_else(|| region.backdrop_asset.clone()),
        time_context,
        directorial_notes: scene.directorial_notes.clone(),
    };

    let mut characters = Vec::with_capacity(1 + npcs.len());
    characters.push(CharacterData {
        id: pc.id.to_string(),
        name: pc.name.clone(),
        sprite_asset: pc.sprite_asset.clone(),
        portrait_asset: pc.portrait_asset.clone(),
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
            sprite_asset: npc.sprite_asset.clone(),
            portrait_asset: npc.portrait_asset.clone(),
            position,
            is_speaking: false,
            expression: None,
            mood: Some(npc.mood.to_string()),
        });
    }

    let interactions = match state.app.entities.interaction.list_for_scene(scene.id).await {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(error = %e, scene_id = %scene.id, "Failed to load scene interactions");
            Vec::new()
        }
    };

    let mut interaction_data = Vec::with_capacity(interactions.len());
    for interaction in interactions {
        let (target_id, target_type, target_name) =
            resolve_interaction_target(state, &interaction.target).await;
        interaction_data.push(InteractionData {
            id: interaction.id.to_string(),
            name: interaction.name,
            interaction_type: interaction_type_to_str(&interaction.interaction_type).to_string(),
            target_name,
            target_id,
            target_type,
            is_available: interaction.is_available,
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
            event_id: event.id.to_string(),
            event_name: event.name.clone(),
            outcome_description,
            scene_direction: event.scene_direction.clone(),
        };
        state.connections.send_to_pc(pc_id, msg).await;
    }
}

fn resolve_event_outcome_description(event: &wrldbldr_domain::NarrativeEvent) -> String {
    if let Some(ref default_name) = event.default_outcome {
        if let Some(outcome) = event.outcomes.iter().find(|o| &o.name == default_name) {
            return outcome.description.clone();
        }
    }

    event
        .outcomes
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
                .entities
                .character
                .get(*id)
                .await
                .ok()
                .flatten()
                .map(|c| c.name);
            (Some(id.to_string()), Some("character".to_string()), name)
        }
        wrldbldr_domain::InteractionTarget::Item(id) => {
            let name = state
                .app
                .entities
                .inventory
                .get(*id)
                .await
                .ok()
                .flatten()
                .map(|item| item.name);
            (Some(id.to_string()), Some("item".to_string()), name)
        }
        wrldbldr_domain::InteractionTarget::Environment(label) => (
            None,
            Some("environment".to_string()),
            Some(label.clone()),
        ),
        wrldbldr_domain::InteractionTarget::None => (None, None, None),
    }
}
