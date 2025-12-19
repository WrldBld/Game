//! Presentation-layer handler for Engine WebSocket `ServerMessage`.
//!
//! This is the canonical place to translate incoming server messages into
//! presentation state mutations. Keeping this here avoids application→presentation
//! dependencies and keeps the WebSocket transport parsing separate from UI state.

use crate::application::ports::outbound::Platform;
use crate::application::dto::{ProposedTool, ServerMessage, SessionWorldSnapshot};
use dioxus::prelude::{ReadableExt, WritableExt};
use crate::presentation::state::{
    DialogueState, GameState, GenerationState, PendingApproval, SessionState,
    session_state::{ChallengePromptData, ChallengeResultData},
    approval_state::PendingChallengeOutcome,
};

/// Handle an incoming `ServerMessage` and update presentation state.
pub fn handle_server_message(
    message: ServerMessage,
    session_state: &mut SessionState,
    game_state: &mut GameState,
    dialogue_state: &mut DialogueState,
    generation_state: &mut GenerationState,
    platform: &Platform,
) {
    match message {
        ServerMessage::SessionJoined {
            session_id,
            role,
            participants: _,
            world_snapshot,
        } => {
            tracing::info!("SessionJoined received");

            session_state.set_session_joined(session_id.clone());
            session_state.add_log_entry(
                "System".to_string(),
                format!("Joined session: {}", session_id),
                true,
                platform,
            );

            match serde_json::from_value::<SessionWorldSnapshot>(world_snapshot) {
                Ok(snapshot) => {
                    // Try to build an initial scene from the world snapshot
                    // This provides a default view until a proper SceneUpdate is received
                    if let Some(first_scene) = snapshot.scenes.first() {
                        let location_name = snapshot.locations.iter()
                            .find(|l| l.id == first_scene.location_id)
                            .map(|l| l.name.clone())
                            .unwrap_or_else(|| "Unknown".to_string());
                        
                        let backdrop_asset = first_scene.backdrop_override.clone()
                            .or_else(|| snapshot.locations.iter()
                                .find(|l| l.id == first_scene.location_id)
                                .and_then(|l| l.backdrop_asset.clone()));

                        // Build scene data
                        let initial_scene = crate::application::dto::websocket_messages::SceneSnapshot {
                            id: first_scene.id.clone(),
                            name: first_scene.name.clone(),
                            location_id: first_scene.location_id.clone(),
                            location_name,
                            backdrop_asset,
                            time_context: first_scene.time_context.clone(),
                            directorial_notes: first_scene.directorial_notes.clone(),
                        };

                        // Get characters featured in the scene
                        let scene_characters: Vec<crate::application::dto::websocket_messages::SceneCharacterState> = first_scene.featured_characters.iter()
                            .filter_map(|char_id| {
                                snapshot.characters.iter().find(|c| &c.id == char_id).map(|c| {
                                    crate::application::dto::websocket_messages::SceneCharacterState {
                                        id: c.id.clone(),
                                        name: c.name.clone(),
                                        sprite_asset: c.sprite_asset.clone(),
                                        portrait_asset: c.portrait_asset.clone(),
                                        position: crate::application::dto::websocket_messages::CharacterPosition::Center,
                                        is_speaking: false,
                                        emotion: None,
                                    }
                                })
                            })
                            .collect();

                        // Apply the initial scene
                        game_state.apply_scene_update(initial_scene, scene_characters, Vec::new());
                        tracing::info!("Applied initial scene from world snapshot: {}", first_scene.name);
                    }

                    game_state.load_world(snapshot);
                    session_state.add_log_entry(
                        "System".to_string(),
                        "World data loaded".to_string(),
                        true,
                        platform,
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to parse world snapshot: {}", e);
                }
            }
        }

        ServerMessage::PlayerJoined {
            user_id,
            role,
            character_name,
        } => {
            tracing::info!("Player joined: {} as {:?}", user_id, role);
            session_state.add_log_entry(
                "System".to_string(),
                format!(
                    "Player {} joined as {:?}{}",
                    user_id,
                    role,
                    character_name
                        .map(|n| format!(" ({})", n))
                        .unwrap_or_default()
                ),
                true,
                platform,
            );
        }

        ServerMessage::PlayerLeft { user_id } => {
            tracing::info!("Player left: {}", user_id);
            session_state.add_log_entry(
                "System".to_string(),
                format!("Player {} left", user_id),
                true,
                platform,
            );
        }

        ServerMessage::ActionReceived {
            action_id,
            player_id,
            action_type,
        } => {
            tracing::info!("Action received: {} -> {}", action_type, player_id);
            session_state.add_log_entry(
                "System".to_string(),
                format!("Action {} received: {}", action_id, action_type),
                true,
                platform,
            );
        }

        ServerMessage::SceneUpdate {
            scene,
            characters,
            interactions,
        } => {
            tracing::info!("SceneUpdate: {}", scene.name);
            game_state.apply_scene_update(scene, characters, interactions);
        }

        ServerMessage::DialogueResponse {
            speaker_id,
            speaker_name,
            text,
            choices,
        } => {
            // Add to conversation log for DM view
            session_state.add_log_entry(speaker_name.clone(), text.clone(), false, platform);
            dialogue_state.apply_dialogue(speaker_id, speaker_name, text, choices);
        }

        ServerMessage::LLMProcessing { action_id } => {
            dialogue_state.is_llm_processing.set(true);
            session_state.add_log_entry(
                "System".to_string(),
                format!("Processing action: {}", action_id),
                true,
                platform,
            );
        }

        ServerMessage::ApprovalRequired {
            request_id,
            npc_name,
            proposed_dialogue,
            internal_reasoning,
            proposed_tools,
            challenge_suggestion,
            narrative_event_suggestion,
        } => {
            session_state.add_pending_approval(PendingApproval {
                request_id,
                npc_name,
                proposed_dialogue,
                internal_reasoning,
                proposed_tools,
                challenge_suggestion,
                narrative_event_suggestion,
            });
        }

        ServerMessage::ResponseApproved {
            npc_dialogue: _,
            executed_tools,
        } => {
            tracing::info!("ResponseApproved: executed {} tools", executed_tools.len());
        }

        ServerMessage::Error { code, message } => {
            let error_msg = format!("Server error [{}]: {}", code, message);
            tracing::error!("{}", error_msg);
            session_state.error_message().set(Some(error_msg));
        }

        ServerMessage::Pong => {}

        // Generation events (Creator Mode)
        ServerMessage::GenerationQueued {
            batch_id,
            entity_type,
            entity_id,
            asset_type,
            position,
        } => {
            tracing::info!(
                "Generation queued: {} {} ({}) at position {}",
                entity_type,
                entity_id,
                asset_type,
                position
            );
            generation_state.batch_queued(
                batch_id,
                entity_type,
                entity_id,
                asset_type,
                position,
            );
        }

        ServerMessage::GenerationProgress { batch_id, progress } => {
            tracing::info!("Generation progress: {} at {}%", batch_id, progress);
            generation_state.batch_progress(&batch_id, progress);
        }

        ServerMessage::GenerationComplete { batch_id, asset_count } => {
            tracing::info!("Generation complete: {} ({} assets)", batch_id, asset_count);
            generation_state.batch_complete(&batch_id, asset_count);
        }

        ServerMessage::GenerationFailed { batch_id, error } => {
            tracing::error!("Generation failed: {} - {}", batch_id, error);
            generation_state.batch_failed(&batch_id, error);
        }

        ServerMessage::SuggestionQueued {
            request_id,
            field_type,
            entity_id,
        } => {
            tracing::info!("Suggestion queued: {} ({})", request_id, field_type);
            generation_state.suggestion_queued(request_id, field_type, entity_id);
        }

        ServerMessage::SuggestionProgress { request_id, status } => {
            tracing::info!("Suggestion progress: {} - {}", request_id, status);
            generation_state.suggestion_progress(&request_id, &status);
        }

        ServerMessage::SuggestionComplete {
            request_id,
            suggestions,
        } => {
            tracing::info!("Suggestion complete: {} ({} suggestions)", request_id, suggestions.len());
            generation_state.suggestion_complete(&request_id, suggestions);
        }

        ServerMessage::SuggestionFailed { request_id, error } => {
            tracing::error!("Suggestion failed: {} - {}", request_id, error);
            generation_state.suggestion_failed(&request_id, error);
        }

        ServerMessage::ComfyUIStateChanged {
            state,
            message,
            retry_in_seconds,
        } => {
            tracing::info!("ComfyUI state changed: {} - {:?}", state, message);
            session_state.comfyui_state().set(state);
            session_state.comfyui_message().set(message);
            session_state.comfyui_retry_in_seconds().set(retry_in_seconds);
        }

        ServerMessage::ChallengePrompt {
            challenge_id,
            challenge_name,
            skill_name,
            difficulty_display,
            description,
            character_modifier,
            suggested_dice,
            rule_system_hint,
        } => {
            let challenge = ChallengePromptData {
                challenge_id,
                challenge_name,
                skill_name,
                difficulty_display,
                description,
                character_modifier,
                suggested_dice,
                rule_system_hint,
            };
            session_state.set_active_challenge(challenge);
        }

        ServerMessage::ChallengeResolved {
            challenge_id,
            challenge_name,
            character_name,
            roll,
            modifier,
            total,
            outcome,
            outcome_description,
            roll_breakdown,
            individual_rolls,
        } => {
            // Clear active challenge if it matches
            let active = { session_state.active_challenge().read().clone() };
            if let Some(active_challenge) = active {
                if active_challenge.challenge_id == challenge_id {
                    session_state.clear_active_challenge();
                }
            }

            let timestamp = platform.now_unix_secs();
            let result = ChallengeResultData {
                challenge_name: challenge_name.clone(),
                character_name: character_name.clone(),
                roll,
                modifier,
                total,
                outcome: outcome.clone(),
                outcome_description: outcome_description.clone(),
                timestamp,
                roll_breakdown: roll_breakdown.clone(),
                individual_rolls: individual_rolls.clone(),
            };
            
            // Add to history
            session_state.add_challenge_result(result.clone());
            
            // Trigger popup display (Phase D)
            session_state.set_result_ready(result);
        }

        ServerMessage::NarrativeEventTriggered {
            event_id: _,
            event_name,
            outcome_description,
            scene_direction,
        } => {
            // Log the narrative event trigger for DMs
            tracing::info!(
                "Narrative event '{}' triggered: {} ({})",
                event_name,
                outcome_description,
                scene_direction
            );
            // TODO (Phase 17 Story Arc UI): Update Story Arc timeline when the tab is implemented
            // For now, this is logged to console for DM awareness
        }

        ServerMessage::SplitPartyNotification {
            location_count,
            locations,
        } => {
            tracing::info!(
                "Party is split across {} locations",
                location_count
            );
            // TODO (Phase 23 UX Polish): Update UI to show split party warning with location information
            // For now, this is logged to console for DM awareness
            for loc in locations {
                tracing::debug!(
                    "Location: {} - {} PC(s)",
                    loc.location_name,
                    loc.pc_count
                );
            }
        }

        ServerMessage::OutcomeRegenerated {
            request_id,
            outcome_type,
            new_outcome,
        } => {
            tracing::info!(
                "Outcome '{}' regenerated for request {}: {}",
                outcome_type,
                request_id,
                new_outcome.flavor_text
            );

            // Update the matching pending approval's challenge outcomes in-place
            // Find the index first and drop the read borrow
            let idx = {
                session_state
                    .pending_approvals()
                    .read()
                    .iter()
                    .position(|a| a.request_id == request_id)
            };
            if let Some(idx) = idx {
                let mut approvals = session_state.pending_approvals().read().clone();
                if let Some(approval) = approvals.get_mut(idx) {
                    if let Some(challenge) = &mut approval.challenge_suggestion {
                        if let Some(ref mut outcomes) = challenge.outcomes {
                            // Map outcome_type string to the appropriate field
                            match outcome_type.as_str() {
                                "success" => outcomes.success = Some(new_outcome.clone()),
                                "failure" => outcomes.failure = Some(new_outcome.clone()),
                                "critical_success" => outcomes.critical_success = Some(new_outcome.clone()),
                                "critical_failure" => outcomes.critical_failure = Some(new_outcome.clone()),
                                // "all" or unknown: update success/failure as a minimal default
                                _ => {
                                    outcomes.success = Some(new_outcome.clone());
                                    outcomes.failure = Some(new_outcome.clone());
                                }
                            }
                        }
                    }
                }
                session_state.pending_approvals().set(approvals);
            }
        }

        ServerMessage::ChallengeDiscarded { request_id } => {
            tracing::info!("Challenge discarded for request {}", request_id);

            // Remove the challenge suggestion/outcomes from the approval item
            let mut approvals = session_state.pending_approvals().read().clone();
            for approval in approvals.iter_mut() {
                if approval.request_id == request_id {
                    approval.challenge_suggestion = None;
                    if let Some(ref mut nes) = approval.narrative_event_suggestion {
                        // Leave narrative suggestion intact; only clear challenge-specific state
                        nes.suggested_outcome = None;
                    }
                }
            }
            session_state.pending_approvals().set(approvals);
        }

        ServerMessage::AdHocChallengeCreated {
            challenge_id,
            challenge_name,
            target_pc_id,
        } => {
            tracing::info!(
                "Ad-hoc challenge '{}' (ID: {}) created for PC {}",
                challenge_name,
                challenge_id,
                target_pc_id
            );

            // Log a DM-facing system message so the DM sees confirmation in context
            let msg = format!(
                "[AD-HOC CHALLENGE] '{}' created for PC {} (ID: {})",
                challenge_name, target_pc_id, challenge_id
            );
            session_state.conversation_log().write().push(
                crate::presentation::state::ConversationLogEntry {
                    speaker: "System".to_string(),
                    text: msg,
                    is_system: true,
                    timestamp: platform.now_unix_secs(),
                },
            );
        }

        // P3.3/P3.4: Player's roll is awaiting DM approval
        ServerMessage::ChallengeRollSubmitted {
            challenge_id: _,
            challenge_name: _,
            roll,
            modifier,
            total,
            outcome_type,
            status: _,
        } => {
            tracing::info!(
                "Roll submitted: {} + {} = {} ({}), awaiting approval",
                roll,
                modifier,
                total,
                outcome_type
            );
            session_state.set_awaiting_approval(roll, modifier, total, outcome_type);
        }

        // P3.3/P3.4: Challenge outcome pending DM approval (DM only)
        ServerMessage::ChallengeOutcomePending {
            resolution_id,
            challenge_id: _,
            challenge_name,
            character_id,
            character_name,
            roll,
            modifier,
            total,
            outcome_type,
            outcome_description,
            outcome_triggers,
            roll_breakdown,
        } => {
            tracing::info!(
                "Challenge outcome pending: {} for {} ({} + {} = {})",
                challenge_name,
                character_name,
                roll,
                modifier,
                total
            );

            let timestamp = platform.now_unix_secs();
            let pending = PendingChallengeOutcome {
                resolution_id,
                challenge_name,
                character_id,
                character_name,
                roll,
                modifier,
                total,
                outcome_type,
                outcome_description,
                outcome_triggers,
                roll_breakdown,
                suggestions: None,
                branches: None,
                is_generating_suggestions: false,
                timestamp,
            };
            session_state.add_pending_challenge_outcome(pending);
        }

        // P3.3/P3.4: LLM suggestions ready for challenge outcome (DM only)
        ServerMessage::OutcomeSuggestionReady {
            resolution_id,
            suggestions,
        } => {
            tracing::info!(
                "Outcome suggestions ready for {}: {} suggestions",
                resolution_id,
                suggestions.len()
            );
            session_state.update_challenge_suggestions(&resolution_id, suggestions);
        }

        // Phase 22C: Outcome branches ready for DM selection
        ServerMessage::OutcomeBranchesReady {
            resolution_id,
            outcome_type,
            branches,
        } => {
            tracing::info!(
                "Outcome branches ready for {} ({}): {} branches",
                resolution_id,
                outcome_type,
                branches.len()
            );
            session_state.update_challenge_branches(&resolution_id, outcome_type, branches);
        }

        // =========================================================================
        // Phase 23E: DM Event System
        // =========================================================================

        ServerMessage::ApproachEvent {
            npc_id,
            npc_name,
            npc_sprite,
            description,
        } => {
            tracing::info!("NPC approach event: {} ({})", npc_name, npc_id);
            
            // Add to log
            session_state.add_log_entry(
                npc_name.clone(),
                format!("[APPROACH] {}", description),
                false,
                platform,
            );
            
            // Set the approach event for visual overlay
            game_state.set_approach_event(
                npc_id,
                npc_name,
                npc_sprite,
                description,
            );
        }

        ServerMessage::LocationEvent {
            region_id,
            description,
        } => {
            tracing::info!("Location event in region {}: {}", region_id, description);
            
            // Add to log
            session_state.add_log_entry(
                "Narrator".to_string(),
                format!("[EVENT] {}", description),
                true,
                platform,
            );
            
            // Set the location event for visual banner
            game_state.set_location_event(region_id, description);
        }

        ServerMessage::NpcLocationShared {
            npc_id,
            npc_name,
            region_name,
            notes,
        } => {
            tracing::info!("NPC location shared: {} at {}", npc_name, region_name);
            let msg = if let Some(note) = notes {
                format!("You heard that {} is at {}. {}", npc_name, region_name, note)
            } else {
                format!("You heard that {} is at {}.", npc_name, region_name)
            };
            session_state.add_log_entry("System".to_string(), msg, true, platform);
            // TODO (Phase 23 Player UI): Update observation/map state
        }

        // =========================================================================
        // Phase 23C: Navigation & Scene Updates
        // =========================================================================

        ServerMessage::PcSelected {
            pc_id,
            pc_name,
            location_id,
            region_id,
        } => {
            tracing::info!(
                "PC selected: {} ({}) at location {} region {:?}",
                pc_name,
                pc_id,
                location_id,
                region_id
            );
            session_state.add_log_entry(
                "System".to_string(),
                format!("Now playing as {}", pc_name),
                true,
                platform,
            );
            // TODO (Phase 23 Player UI): Update selected PC state
        }

        ServerMessage::SceneChanged {
            pc_id,
            region,
            npcs_present,
            navigation,
        } => {
            tracing::info!(
                "Scene changed for PC {}: {} in {} ({} NPCs, {} regions, {} exits)",
                pc_id,
                region.name,
                region.location_name,
                npcs_present.len(),
                navigation.connected_regions.len(),
                navigation.exits.len()
            );
            
            // Update game state with navigation data
            game_state.apply_scene_changed(
                pc_id.clone(),
                region.clone(),
                npcs_present,
                navigation,
            );
            
            session_state.add_log_entry(
                "System".to_string(),
                format!("Entered {} ({})", region.name, region.location_name),
                true,
                platform,
            );
        }

        ServerMessage::MovementBlocked { pc_id, reason } => {
            tracing::info!("Movement blocked for PC {}: {}", pc_id, reason);
            session_state.add_log_entry(
                "System".to_string(),
                format!("Cannot proceed: {}", reason),
                true,
                platform,
            );
        }

        // =========================================================================
        // Phase 23F: Game Time Control
        // =========================================================================

        ServerMessage::GameTimeUpdated {
            display: time_display,
            time_of_day,
            is_paused,
        } => {
            tracing::info!(
                "Game time updated: {} ({}, paused: {})",
                time_display,
                time_of_day,
                is_paused
            );
            
            // Update game state with time data
            game_state.apply_game_time_update(
                time_display.clone(),
                time_of_day,
                is_paused,
            );
            
            session_state.add_log_entry(
                "System".to_string(),
                format!("Time is now: {}", time_display),
                true,
                platform,
            );
        }

        // =========================================================================
        // Queue Status (DM-only, can be ignored by Player view)
        // =========================================================================

        ServerMessage::ActionQueued {
            action_id,
            player_name,
            action_type,
            queue_depth,
        } => {
            tracing::debug!(
                "Action queued: {} by {} (type: {}, depth: {})",
                action_id,
                player_name,
                action_type,
                queue_depth
            );
            // DM-only notification, no UI update needed for Player view
        }

        ServerMessage::QueueStatus {
            player_actions_pending,
            llm_requests_pending,
            llm_requests_processing,
            approvals_pending,
        } => {
            tracing::debug!(
                "Queue status: actions={}, llm_pending={}, llm_processing={}, approvals={}",
                player_actions_pending,
                llm_requests_pending,
                llm_requests_processing,
                approvals_pending
            );
            // DM-only status update, no UI update needed for Player view
        }
    }
}

