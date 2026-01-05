//! Presentation-layer handler for `PlayerEvent` from the session.
//!
//! This is the canonical place to translate incoming player events into
//! presentation state mutations. PlayerEvent is the application-layer
//! representation of server messages, already translated from wire format.

use crate::presentation::state::{
    approval_state::PendingChallengeOutcome,
    challenge_state::{ChallengePromptData, ChallengeResultData},
    game_state::RegionStagingStatus,
    DialogueState, GameState, GenerationState, LoreState, PendingApproval, SessionState,
};
use dioxus::prelude::{ReadableExt, WritableExt};
use wrldbldr_player_app::application::dto::SessionWorldSnapshot;
use wrldbldr_player_ports::outbound::player_events::{
    CharacterData, CharacterPosition, ConnectedUser, NpcPresenceData, PlayerEvent, SceneData,
};
use wrldbldr_player_ports::outbound::PlatformPort;

/// Handle an incoming `PlayerEvent` and update presentation state.
pub fn handle_server_message(
    message: PlayerEvent,
    session_state: &mut SessionState,
    game_state: &mut GameState,
    dialogue_state: &mut DialogueState,
    generation_state: &mut GenerationState,
    lore_state: &mut LoreState,
    platform: &dyn PlatformPort,
) {
    match message {
        PlayerEvent::ActionReceived {
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

        PlayerEvent::SceneUpdate {
            scene,
            characters,
            interactions,
        } => {
            tracing::info!("SceneUpdate: {}", scene.name);
            // PlayerEvent already contains application-layer types
            game_state.apply_scene_update(scene, characters, interactions);
        }

        PlayerEvent::DialogueResponse {
            speaker_id,
            speaker_name,
            text,
            choices,
        } => {
            // Add to conversation log for DM view
            session_state.add_log_entry(speaker_name.clone(), text.clone(), false, platform);
            // PlayerEvent already contains application-layer types
            dialogue_state.apply_dialogue(speaker_id, speaker_name, text, choices);
        }

        PlayerEvent::LLMProcessing { action_id } => {
            dialogue_state.is_llm_processing.set(true);
            session_state.add_log_entry(
                "System".to_string(),
                format!("Processing action: {}", action_id),
                true,
                platform,
            );
        }

        PlayerEvent::ApprovalRequired {
            request_id,
            npc_name,
            proposed_dialogue,
            internal_reasoning,
            proposed_tools,
            challenge_suggestion,
            narrative_event_suggestion,
        } => {
            // PlayerEvent already contains application-layer types
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

        PlayerEvent::ResponseApproved {
            npc_dialogue: _,
            executed_tools,
        } => {
            tracing::info!("ResponseApproved: executed {} tools", executed_tools.len());
            // Clear processing state - the response has been approved and delivered
            dialogue_state.is_llm_processing.set(false);
        }

        PlayerEvent::Error { code, message } => {
            let error_msg = format!("Server error [{}]: {}", code, message);
            tracing::error!("{}", error_msg);
            session_state.error_message().set(Some(error_msg));
        }

        PlayerEvent::Pong => {}

        // Generation events (Creator Mode)
        PlayerEvent::GenerationQueued {
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
            generation_state.batch_queued(batch_id, entity_type, entity_id, asset_type, position);
        }

        PlayerEvent::GenerationProgress { batch_id, progress } => {
            tracing::info!("Generation progress: {} at {}%", batch_id, progress);
            generation_state.batch_progress(&batch_id, progress);
        }

        PlayerEvent::GenerationComplete {
            batch_id,
            asset_count,
        } => {
            tracing::info!("Generation complete: {} ({} assets)", batch_id, asset_count);
            generation_state.batch_complete(&batch_id, asset_count);
        }

        PlayerEvent::GenerationFailed { batch_id, error } => {
            tracing::error!("Generation failed: {} - {}", batch_id, error);
            generation_state.batch_failed(&batch_id, error);
        }

        PlayerEvent::SuggestionQueued {
            request_id,
            field_type,
            entity_id,
        } => {
            tracing::info!("Suggestion queued: {} ({})", request_id, field_type);
            generation_state.suggestion_queued(request_id, field_type, entity_id);
        }

        PlayerEvent::SuggestionProgress { request_id, status } => {
            tracing::info!("Suggestion progress: {} - {}", request_id, status);
            generation_state.suggestion_progress(&request_id, &status);
        }

        PlayerEvent::SuggestionComplete {
            request_id,
            suggestions,
        } => {
            tracing::info!(
                "Suggestion complete: {} ({} suggestions)",
                request_id,
                suggestions.len()
            );
            generation_state.suggestion_complete(&request_id, suggestions);
        }

        PlayerEvent::SuggestionFailed { request_id, error } => {
            tracing::error!("Suggestion failed: {} - {}", request_id, error);
            generation_state.suggestion_failed(&request_id, error);
        }

        PlayerEvent::ComfyUIStateChanged {
            state,
            message,
            retry_in_seconds,
        } => {
            tracing::info!("ComfyUI state changed: {} - {:?}", state, message);
            session_state.comfyui_state().set(state);
            session_state.comfyui_message().set(message);
            session_state
                .comfyui_retry_in_seconds()
                .set(retry_in_seconds);
        }

        PlayerEvent::ChallengePrompt {
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

        PlayerEvent::ChallengeResolved {
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

        PlayerEvent::NarrativeEventTriggered {
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

        PlayerEvent::SplitPartyNotification {
            location_count,
            locations,
        } => {
            tracing::info!("Party is split across {} locations", location_count);
            // Update UI to show split party warning banner
            if location_count > 1 {
                // PlayerEvent already contains application-layer types
                game_state.set_split_party_locations(locations);
            } else {
                // Party is together (or only one location)
                game_state.clear_split_party();
            }
        }

        PlayerEvent::OutcomeRegenerated {
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
                            // Store the flavor_text as the outcome description
                            let outcome_text = new_outcome.flavor_text.clone();
                            match outcome_type.as_str() {
                                "success" => outcomes.success = Some(outcome_text),
                                "failure" => outcomes.failure = Some(outcome_text),
                                "critical_success" => {
                                    outcomes.critical_success = Some(outcome_text)
                                }
                                "critical_failure" => {
                                    outcomes.critical_failure = Some(outcome_text)
                                }
                                // "all" or unknown: update success/failure as a minimal default
                                _ => {
                                    outcomes.success = Some(outcome_text.clone());
                                    outcomes.failure = Some(outcome_text);
                                }
                            }
                        }
                    }
                }
                session_state.pending_approvals().set(approvals);
            }
        }

        PlayerEvent::ChallengeDiscarded { request_id } => {
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

        PlayerEvent::AdHocChallengeCreated {
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
        PlayerEvent::ChallengeRollSubmitted {
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
        PlayerEvent::ChallengeOutcomePending {
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

            // PlayerEvent already contains application-layer types
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
        PlayerEvent::OutcomeSuggestionReady {
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
        PlayerEvent::OutcomeBranchesReady {
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
            // PlayerEvent already contains application-layer types
            session_state.update_challenge_branches(&resolution_id, outcome_type, branches);
        }

        // =========================================================================
        // Phase 23E: DM Event System
        // =========================================================================
        PlayerEvent::ApproachEvent {
            npc_id,
            npc_name,
            npc_sprite,
            description,
            reveal: _,
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
            game_state.set_approach_event(npc_id, npc_name, npc_sprite, description);
        }

        PlayerEvent::LocationEvent {
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

        PlayerEvent::NpcLocationShared {
            npc_id: _npc_id,
            npc_name,
            region_name,
            notes,
        } => {
            tracing::info!("NPC location shared: {} at {}", npc_name, region_name);
            let msg = if let Some(note) = notes {
                format!(
                    "You heard that {} is at {}. {}",
                    npc_name, region_name, note
                )
            } else {
                format!("You heard that {} is at {}.", npc_name, region_name)
            };
            session_state.add_log_entry("System".to_string(), msg, true, platform);

            // Trigger observations refresh so UI can reload the updated observation list
            game_state.trigger_observations_refresh();
        }

        // =========================================================================
        // Phase 23C: Navigation & Scene Updates
        // =========================================================================
        PlayerEvent::PcSelected {
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

            // Update selected PC in game state
            game_state.selected_pc_id.set(Some(pc_id.clone()));

            session_state.add_log_entry(
                "System".to_string(),
                format!("Now playing as {}", pc_name),
                true,
                platform,
            );
        }

        PlayerEvent::SceneChanged {
            pc_id,
            region,
            npcs_present,
            navigation,
            region_items,
        } => {
            tracing::info!(
                "Scene changed for PC {}: {} in {} ({} NPCs, {} regions, {} exits, {} items)",
                pc_id,
                region.name,
                region.location_name,
                npcs_present.len(),
                navigation.connected_regions.len(),
                navigation.exits.len(),
                region_items.len()
            );

            // PlayerEvent already contains application-layer types
            // Update game state with navigation data and region items
            game_state.apply_scene_changed(
                pc_id.clone(),
                region.clone(),
                npcs_present,
                navigation,
                region_items,
            );

            session_state.add_log_entry(
                "System".to_string(),
                format!("Entered {} ({})", region.name, region.location_name),
                true,
                platform,
            );
        }

        PlayerEvent::MovementBlocked { pc_id, reason } => {
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
        PlayerEvent::GameTimeUpdated { game_time } => {
            // PlayerEvent already contains application-layer types
            let time_display = crate::presentation::game_time_format::display_date(game_time);
            let time_of_day = crate::presentation::game_time_format::time_of_day(game_time);

            tracing::info!(
                "Game time updated: {} ({}, paused: {})",
                time_display,
                time_of_day,
                game_time.is_paused
            );

            game_state.apply_game_time_update(game_time);

            session_state.add_log_entry(
                "System".to_string(),
                format!("Time is now: {}", time_display),
                true,
                platform,
            );
        }

        PlayerEvent::GameTimeAdvanced {
            new_time,
            reason,
            period_changed,
            new_period,
            ..
        } => {
            let time_display = crate::presentation::game_time_format::display_date(new_time);

            tracing::info!(
                "Game time advanced: {} (reason: {}, period_changed: {})",
                time_display,
                reason,
                period_changed
            );

            game_state.apply_game_time_update(new_time);

            let message = if period_changed {
                if let Some(ref period) = new_period {
                    format!("{} - It is now {}.", reason, period)
                } else {
                    format!("{} - Time is now: {}", reason, time_display)
                }
            } else {
                format!("{} - Time is now: {}", reason, time_display)
            };

            session_state.add_log_entry("System".to_string(), message, true, platform);
        }

        PlayerEvent::TimeSuggestion {
            suggestion_id,
            pc_id,
            pc_name,
            action_type,
            action_description,
            suggested_minutes,
            current_time,
            resulting_time,
            period_change,
        } => {
            // DM-only: show time suggestion for approval
            let current_display = crate::presentation::game_time_format::display_date(current_time);
            let resulting_display =
                crate::presentation::game_time_format::display_date(resulting_time);

            tracing::info!(
                "Time suggestion: {} - {} ({} -> {}, +{} min)",
                suggestion_id,
                action_description,
                current_display,
                resulting_display,
                suggested_minutes
            );

            // Add to DM time suggestions queue
            game_state.add_time_suggestion(crate::presentation::state::TimeSuggestionData {
                suggestion_id: suggestion_id.clone(),
                pc_id,
                pc_name: pc_name.clone(),
                action_type,
                action_description: action_description.clone(),
                suggested_minutes,
                current_time,
                resulting_time,
                period_change,
            });

            session_state.add_log_entry(
                "Time".to_string(),
                format!(
                    "{}: {} suggests +{} min ({} -> {})",
                    pc_name,
                    action_description,
                    suggested_minutes,
                    current_display,
                    resulting_display
                ),
                true,
                platform,
            );
        }

        PlayerEvent::TimeModeChanged { world_id, mode } => {
            tracing::info!("Time mode changed for world {}: {}", world_id, mode);
            game_state.set_time_mode(crate::presentation::state::TimeMode::from_str(&mode));
            session_state.add_log_entry(
                "System".to_string(),
                format!("Time mode changed to: {}", mode),
                true,
                platform,
            );
        }

        PlayerEvent::GameTimePaused { paused, .. } => {
            tracing::info!("Game time paused: {}", paused);
            game_state.set_time_paused(paused);
            session_state.add_log_entry(
                "System".to_string(),
                if paused {
                    "Time has been paused".to_string()
                } else {
                    "Time has resumed".to_string()
                },
                true,
                platform,
            );
        }

        PlayerEvent::TimeConfigUpdated { mode, .. } => {
            tracing::info!("Time config updated: mode={}", mode);
            // DM-only notification
        }

        // =========================================================================
        // Queue Status (DM-only, can be ignored by Player view)
        // =========================================================================
        PlayerEvent::ActionQueued {
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

        PlayerEvent::QueueStatus {
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

        // =========================================================================
        // Staging System (NPC Presence Approval)
        // =========================================================================
        PlayerEvent::StagingApprovalRequired {
            request_id,
            region_id,
            region_name,
            location_id,
            location_name,
            game_time,
            previous_staging,
            rule_based_npcs,
            llm_based_npcs,
            default_ttl_hours,
            waiting_pcs,
            .. // Visual state fields - TODO: Handle in UI when implemented
        } => {
            tracing::info!(
                "Staging approval required for region {} ({}): {} rule-based, {} LLM-based NPCs",
                region_name,
                region_id,
                rule_based_npcs.len(),
                llm_based_npcs.len()
            );

            // Convert protocol types to presentation types
            use crate::presentation::state::game_state::{
                PreviousStagingData, StagedNpcData, StagingApprovalData, WaitingPcData,
            };

            let previous = previous_staging.map(|p| PreviousStagingData {
                staging_id: p.staging_id,
                approved_at: p.approved_at,
                npcs: p
                    .npcs
                    .into_iter()
                    .map(|n| StagedNpcData {
                        character_id: n.character_id,
                        name: n.name,
                        sprite_asset: n.sprite_asset,
                        portrait_asset: n.portrait_asset,
                        is_present: n.is_present,
                        reasoning: n.reasoning,
                        is_hidden_from_players: n.is_hidden_from_players,
                    })
                    .collect(),
            });

            let rule_npcs: Vec<StagedNpcData> = rule_based_npcs
                .into_iter()
                .map(|n| StagedNpcData {
                    character_id: n.character_id,
                    name: n.name,
                    sprite_asset: n.sprite_asset,
                    portrait_asset: n.portrait_asset,
                    is_present: n.is_present,
                    reasoning: n.reasoning,
                    is_hidden_from_players: n.is_hidden_from_players,
                })
                .collect();

            let llm_npcs: Vec<StagedNpcData> = llm_based_npcs
                .into_iter()
                .map(|n| StagedNpcData {
                    character_id: n.character_id,
                    name: n.name,
                    sprite_asset: n.sprite_asset,
                    portrait_asset: n.portrait_asset,
                    is_present: n.is_present,
                    reasoning: n.reasoning,
                    is_hidden_from_players: n.is_hidden_from_players,
                })
                .collect();

            let waiting: Vec<WaitingPcData> = waiting_pcs
                .into_iter()
                .map(|p| WaitingPcData {
                    pc_id: p.pc_id,
                    pc_name: p.pc_name,
                    player_id: p.player_id,
                })
                .collect();

            // PlayerEvent already contains application-layer types
            // Update region staging status to Pending
            game_state.set_region_staging_status(region_id.clone(), RegionStagingStatus::Pending);

            game_state.set_pending_staging_approval(StagingApprovalData {
                request_id,
                region_id,
                region_name: region_name.clone(),
                location_id,
                location_name: location_name.clone(),
                game_time,
                previous_staging: previous,
                rule_based_npcs: rule_npcs,
                llm_based_npcs: llm_npcs,
                default_ttl_hours,
                waiting_pcs: waiting,
            });

            session_state.add_log_entry(
                "System".to_string(),
                format!(
                    "Staging approval needed for {} ({})",
                    region_name, location_name
                ),
                true,
                platform,
            );
        }

        PlayerEvent::StagingPending {
            region_id,
            region_name,
        } => {
            tracing::info!("Staging pending for region {} ({})", region_name, region_id);

            // Update region staging status to Pending
            game_state.set_region_staging_status(region_id.clone(), RegionStagingStatus::Pending);

            game_state.set_staging_pending(region_id, region_name.clone());
            session_state.add_log_entry(
                "System".to_string(),
                format!("Setting the scene in {}...", region_name),
                true,
                platform,
            );
        }

        PlayerEvent::StagingReady {
            region_id,
            npcs_present,
            .. // visual_state - TODO: Handle in UI when implemented
        } => {
            tracing::info!(
                "Staging ready for region {}: {} NPCs present",
                region_id,
                npcs_present.len()
            );

            // Clear the pending staging overlay (for players)
            game_state.clear_staging_pending();
            // Clear the DM approval popup (staging has been approved)
            game_state.clear_pending_staging_approval();

            // Update region staging status to Active with NPC names
            let npc_names: Vec<String> = npcs_present.iter().map(|n| n.name.clone()).collect();
            game_state.set_region_staging_status(
                region_id.clone(),
                RegionStagingStatus::Active {
                    staging_id: String::new(), // Not provided in this message
                    npc_names,
                },
            );

            // Update NPCs present (the SceneChanged message will follow with full data)
            let npcs: Vec<NpcPresenceData> = npcs_present
                .into_iter()
                .map(|n| NpcPresenceData {
                    character_id: n.character_id,
                    name: n.name,
                    sprite_asset: n.sprite_asset,
                    portrait_asset: n.portrait_asset,
                })
                .collect();
            game_state.npcs_present.set(npcs);
        }

        PlayerEvent::StagingRegenerated {
            request_id,
            llm_based_npcs,
        } => {
            tracing::info!(
                "Staging regenerated for request {}: {} LLM-based NPCs",
                request_id,
                llm_based_npcs.len()
            );

            // Update the LLM suggestions in the pending staging approval
            use crate::presentation::state::game_state::StagedNpcData;
            let llm_npcs: Vec<StagedNpcData> = llm_based_npcs
                .into_iter()
                .map(|n| StagedNpcData {
                    character_id: n.character_id,
                    name: n.name,
                    sprite_asset: n.sprite_asset,
                    portrait_asset: n.portrait_asset,
                    is_present: n.is_present,
                    reasoning: n.reasoning,
                    is_hidden_from_players: n.is_hidden_from_players,
                })
                .collect();

            game_state.update_staging_llm_suggestions(llm_npcs);
        }

        // =========================================================================
        // Inventory Updates
        // =========================================================================
        PlayerEvent::ItemEquipped {
            pc_id,
            item_id: _,
            item_name,
        } => {
            tracing::info!("Item equipped for PC {}: {}", pc_id, item_name);
            session_state.add_log_entry(
                "System".to_string(),
                format!("Equipped {}", item_name),
                true,
                platform,
            );
            // Trigger inventory refresh - the UI will re-fetch on next render
            game_state.trigger_inventory_refresh();
        }

        PlayerEvent::ItemUnequipped {
            pc_id,
            item_id: _,
            item_name,
        } => {
            tracing::info!("Item unequipped for PC {}: {}", pc_id, item_name);
            session_state.add_log_entry(
                "System".to_string(),
                format!("Unequipped {}", item_name),
                true,
                platform,
            );
            game_state.trigger_inventory_refresh();
        }

        PlayerEvent::ItemDropped {
            pc_id,
            item_id: _,
            item_name,
            quantity,
        } => {
            tracing::info!("Item dropped for PC {}: {} x{}", pc_id, item_name, quantity);
            let msg = if quantity > 1 {
                format!("Dropped {} x{}", item_name, quantity)
            } else {
                format!("Dropped {}", item_name)
            };
            session_state.add_log_entry("System".to_string(), msg, true, platform);
            game_state.trigger_inventory_refresh();
        }

        PlayerEvent::ItemPickedUp {
            pc_id,
            item_id,
            item_name,
        } => {
            tracing::info!("Item picked up for PC {}: {}", pc_id, item_name);
            let msg = format!("Picked up {}", item_name);
            session_state.add_log_entry("System".to_string(), msg, true, platform);
            game_state.trigger_inventory_refresh();
            // Remove the item from visible region items
            game_state.remove_region_item(&item_id);
        }

        PlayerEvent::InventoryUpdated { pc_id } => {
            tracing::info!("Inventory updated for PC {}", pc_id);
            game_state.trigger_inventory_refresh();
        }

        // =========================================================================
        // Character Stat Updates
        // =========================================================================
        PlayerEvent::CharacterStatUpdated {
            character_id,
            character_name,
            stat_name,
            old_value,
            new_value,
            delta,
            source,
        } => {
            let change_str = if delta >= 0 {
                format!("+{}", delta)
            } else {
                format!("{}", delta)
            };

            tracing::info!(
                character_id = %character_id,
                character_name = %character_name,
                stat_name = %stat_name,
                old_value = old_value,
                new_value = new_value,
                delta = delta,
                source = %source,
                "Character stat updated"
            );

            // Add log entry so both DM and player see the stat change
            session_state.add_log_entry(
                "System".to_string(),
                format!(
                    "{}'s {} changed: {} -> {} ({})",
                    character_name, stat_name, old_value, new_value, change_str
                ),
                true,
                platform,
            );

            // Trigger inventory refresh which also covers character sheet stats
            game_state.trigger_inventory_refresh();
        }

        // NPC Disposition messages (P1.4) - Update DM panel state
        PlayerEvent::NpcDispositionChanged {
            npc_id,
            npc_name: _,
            pc_id,
            disposition,
            relationship,
            reason,
        } => {
            tracing::info!(
                npc_id = %npc_id,
                pc_id = %pc_id,
                disposition = %disposition,
                relationship = %relationship,
                reason = ?reason,
                "NPC disposition changed"
            );
            // Update specific NPC disposition in game state
            game_state.update_npc_disposition(&npc_id, disposition, relationship, reason);
        }

        PlayerEvent::NpcDispositionsResponse {
            pc_id,
            dispositions,
        } => {
            tracing::info!(
                pc_id = %pc_id,
                disposition_count = dispositions.len(),
                "Received NPC dispositions for PC"
            );
            // PlayerEvent already contains application-layer types
            game_state.set_npc_dispositions(dispositions);
        }

        PlayerEvent::NpcMoodChanged {
            npc_id,
            npc_name,
            old_mood,
            new_mood,
            reason,
            region_id: _,
        } => {
            tracing::info!(
                npc_id = %npc_id,
                npc_name = %npc_name,
                old_mood = %old_mood,
                new_mood = %new_mood,
                reason = ?reason,
                "NPC mood changed (Tier 2 emotional model)"
            );
            // Update NPC mood in game state - this enables UI to display correct expression/sprite
            game_state.update_npc_mood(npc_id.clone(), new_mood.clone());
        }

        // =========================================================================
        // Actantial Model / Motivations (P1.5)
        // TODO: Implement in Step 8 of Phase 4
        // =========================================================================
        PlayerEvent::NpcWantCreated { npc_id, want } => {
            tracing::info!(
                npc_id = %npc_id,
                want_id = %want.id,
                "NPC want created"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::NpcWantUpdated { npc_id, want } => {
            tracing::info!(
                npc_id = %npc_id,
                want_id = %want.id,
                "NPC want updated"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::NpcWantDeleted { npc_id, want_id } => {
            tracing::info!(
                npc_id = %npc_id,
                want_id = %want_id,
                "NPC want deleted"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::WantTargetSet { want_id, target } => {
            tracing::info!(
                want_id = %want_id,
                target_id = %target.id,
                "Want target set"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::WantTargetRemoved { want_id } => {
            tracing::info!(want_id = %want_id, "Want target removed");
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::ActantialViewAdded { npc_id, view } => {
            tracing::info!(
                npc_id = %npc_id,
                want_id = %view.want_id,
                target_id = %view.target_id,
                role = ?view.role,
                "Actantial view added"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::ActantialViewRemoved {
            npc_id,
            want_id,
            target_id,
            role,
        } => {
            tracing::info!(
                npc_id = %npc_id,
                want_id = %want_id,
                target_id = %target_id,
                role = ?role,
                "Actantial view removed"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::NpcActantialContextResponse { npc_id, context } => {
            let want_count = context
                .get("wants")
                .and_then(|w| w.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            tracing::info!(
                npc_id = %npc_id,
                want_count = want_count,
                "Received NPC actantial context"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::WorldGoalsResponse { world_id, goals } => {
            tracing::info!(
                world_id = %world_id,
                goal_count = goals.len(),
                "Received world goals"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::GoalCreated { world_id, goal } => {
            tracing::info!(
                world_id = %world_id,
                goal_id = %goal.id,
                "Goal created"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::GoalUpdated { goal } => {
            tracing::info!(goal_id = %goal.id, "Goal updated");
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::GoalDeleted { goal_id } => {
            tracing::info!(goal_id = %goal_id, "Goal deleted");
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::DeflectionSuggestions {
            npc_id,
            want_id,
            suggestions,
        } => {
            tracing::info!(
                npc_id = %npc_id,
                want_id = %want_id,
                suggestion_count = suggestions.len(),
                "Received deflection suggestions"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::TellsSuggestions {
            npc_id,
            want_id,
            suggestions,
        } => {
            tracing::info!(
                npc_id = %npc_id,
                want_id = %want_id,
                suggestion_count = suggestions.len(),
                "Received tells suggestions"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::WantDescriptionSuggestions {
            npc_id,
            suggestions,
        } => {
            tracing::info!(
                npc_id = %npc_id,
                suggestion_count = suggestions.len(),
                "Received want description suggestions"
            );
            game_state.trigger_actantial_refresh();
        }

        PlayerEvent::ActantialReasonSuggestions {
            npc_id,
            want_id,
            target_id,
            role,
            suggestions,
        } => {
            tracing::info!(
                npc_id = %npc_id,
                want_id = %want_id,
                target_id = %target_id,
                role = ?role,
                suggestion_count = suggestions.len(),
                "Received actantial reason suggestions"
            );
            game_state.trigger_actantial_refresh();
        }

        // =========================================================================
        // WebSocket-First Protocol Messages (World-scoped connections)
        // =========================================================================
        PlayerEvent::WorldJoined {
            world_id,
            snapshot,
            connected_users,
            your_role,
            your_pc,
        } => {
            tracing::info!(
                world_id = %world_id,
                user_count = connected_users.len(),
                role = ?your_role,
                "Joined world via WebSocket-first protocol"
            );

            // PlayerEvent already contains application-layer types
            // Update connection state with world info
            session_state.set_world_joined(world_id, your_role, connected_users);

            // Parse and load the world snapshot
            match serde_json::from_value::<SessionWorldSnapshot>(snapshot) {
                Ok(world_snapshot) => {
                    // Try to build an initial scene from the world snapshot
                    if let Some(first_scene) = world_snapshot.scenes.first() {
                        let location_name = world_snapshot
                            .locations
                            .iter()
                            .find(|l| l.id == first_scene.location_id)
                            .map(|l| l.name.clone())
                            .unwrap_or_else(|| "Unknown".to_string());

                        let backdrop_asset = first_scene.backdrop_override.clone().or_else(|| {
                            world_snapshot
                                .locations
                                .iter()
                                .find(|l| l.id == first_scene.location_id)
                                .and_then(|l| l.backdrop_asset.clone())
                        });

                        // Construct app DTO directly
                        let initial_scene = SceneData {
                            id: first_scene.id.clone(),
                            name: first_scene.name.clone(),
                            location_id: first_scene.location_id.clone(),
                            location_name,
                            backdrop_asset,
                            time_context: first_scene.time_context.clone(),
                            directorial_notes: first_scene.directorial_notes.clone(),
                        };

                        // Construct app DTOs directly (CharacterData imported from player-ports)
                        let scene_characters: Vec<CharacterData> = first_scene
                            .featured_characters
                            .iter()
                            .filter_map(|char_id| {
                                world_snapshot
                                    .characters
                                    .iter()
                                    .find(|c| &c.id == char_id)
                                    .map(|c| CharacterData {
                                        id: c.id.clone(),
                                        name: c.name.clone(),
                                        sprite_asset: c.sprite_asset.clone(),
                                        portrait_asset: c.portrait_asset.clone(),
                                        position: CharacterPosition::Center,
                                        is_speaking: false,
                                        expression: None,
                                        mood: None,
                                    })
                            })
                            .collect();

                        game_state.apply_scene_update(initial_scene, scene_characters, Vec::new());
                        tracing::info!(
                            "Applied initial scene from world snapshot: {}",
                            first_scene.name
                        );
                    }

                    game_state.load_world(world_snapshot);
                    session_state.add_log_entry(
                        "System".to_string(),
                        format!("Joined world: {}", world_id),
                        true,
                        platform,
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to parse world snapshot: {}", e);
                    session_state.add_log_entry(
                        "System".to_string(),
                        format!("Warning: Could not load world data: {}", e),
                        true,
                        platform,
                    );
                }
            }

            // Handle PC data if present (for Player role)
            if let Some(pc_data) = your_pc {
                tracing::info!("Received PC data with WorldJoined");
                // PC data handling can be expanded here
                let _ = pc_data; // Acknowledge we received it
            }
        }

        PlayerEvent::WorldJoinFailed { world_id, error } => {
            tracing::error!(
                world_id = %world_id,
                error = ?error,
                "Failed to join world"
            );
            let error_msg = format!("Failed to join world: {:?}", error);
            session_state.set_failed(error_msg.clone());
            session_state.add_log_entry("System".to_string(), error_msg, true, platform);
        }

        PlayerEvent::UserJoined {
            user_id,
            username,
            role,
            pc,
        } => {
            tracing::info!(
                user_id = %user_id,
                username = ?username,
                role = ?role,
                "User joined world"
            );

            // PlayerEvent WorldRole is already a String wrapper
            // Add to connected users list
            let new_user = ConnectedUser {
                user_id: user_id.clone(),
                username: username.clone(),
                role: role.0.clone(),
                pc_id: pc
                    .as_ref()
                    .and_then(|p| p.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())),
                connection_count: 1,
            };
            session_state.add_connected_user(new_user);

            session_state.add_log_entry(
                "System".to_string(),
                format!(
                    "{} joined as {:?}",
                    username.unwrap_or_else(|| user_id.clone()),
                    role
                ),
                true,
                platform,
            );
        }

        PlayerEvent::UserLeft { user_id } => {
            tracing::info!(user_id = %user_id, "User left world");
            session_state.remove_connected_user(&user_id);
            session_state.add_log_entry(
                "System".to_string(),
                format!("User {} left", user_id),
                true,
                platform,
            );
        }

        PlayerEvent::Response { request_id, result } => {
            tracing::debug!(
                request_id = %request_id,
                success = result.success,
                "Received response to request"
            );
            // Request/response correlation is handled by the player-app RequestManager
            // The response will be routed to the appropriate pending request handler
        }

        PlayerEvent::EntityChanged(entity_changed) => {
            tracing::debug!(
                entity_type = ?entity_changed.entity_type,
                entity_id = %entity_changed.entity_id,
                change_type = ?entity_changed.change_type,
                "Entity changed broadcast received"
            );
            // PlayerEvent already contains application-layer types
            // Trigger a refresh of relevant UI state based on entity type
            // This enables cache invalidation and reactive updates
            game_state.trigger_entity_refresh(&entity_changed);
        }

        PlayerEvent::SpectateTargetChanged { pc_id, pc_name } => {
            tracing::info!(
                pc_id = %pc_id,
                pc_name = %pc_name,
                "Spectate target changed"
            );
            session_state.add_log_entry(
                "System".to_string(),
                format!("Now spectating: {}", pc_name),
                true,
                platform,
            );
            // The spectate target change should trigger scene updates via SceneChanged messages
        }

        // =========================================================================
        // Lore Events
        // =========================================================================
        PlayerEvent::LoreDiscovered {
            character_id,
            lore,
            discovered_chunk_ids,
            discovery_source,
        } => {
            tracing::info!(
                "Character {} discovered lore: {} ({} chunks via {:?})",
                character_id,
                lore.title,
                discovered_chunk_ids.len(),
                discovery_source
            );
            // Create knowledge metadata for the discovery
            let knowledge = wrldbldr_protocol::types::LoreKnowledgeData {
                lore_id: lore.id.clone(),
                character_id: character_id.clone(),
                known_chunk_ids: discovered_chunk_ids,
                discovery_source,
                discovered_at: chrono::Utc::now().to_rfc3339(),
                notes: None,
            };
            lore_state.add_lore(lore, knowledge);
        }

        PlayerEvent::LoreRevoked {
            character_id,
            lore_id,
        } => {
            tracing::info!(
                "Lore {} revoked from character {}",
                lore_id,
                character_id
            );
            lore_state.remove_lore(&lore_id);
        }

        PlayerEvent::LoreUpdated { lore } => {
            tracing::info!("Lore updated: {}", lore.title);
            lore_state.update_lore(lore);
        }

        PlayerEvent::CharacterLoreResponse {
            character_id,
            known_lore,
        } => {
            // CharacterLoreResponse provides summaries only, not full lore data.
            // This is used for list views; full lore is fetched on demand.
            tracing::info!(
                "Character {} knows {} lore entries (summaries)",
                character_id,
                known_lore.len()
            );
            // TODO: Store summaries in a separate signal for lore list views
            let _ = known_lore;
        }

        // Catch-all for unhandled or future event types
        PlayerEvent::Raw {
            message_type,
            payload,
        } => {
            tracing::debug!(
                message_type = %message_type,
                "Received unhandled raw event type"
            );
            let _ = payload; // Silence unused warning
        }
    }
}
