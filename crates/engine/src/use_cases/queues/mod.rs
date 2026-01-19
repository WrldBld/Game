// Queue processing - fields for future queue result types
#![allow(dead_code)]

//! Queue processing use cases.
//!
//! These use cases are background workers that poll queues and process items.
//! They form the backbone of the asynchronous processing pipeline:
//!
//! 1. Player Action Queue -> Builds LLM prompt -> LLM Request Queue
//! 2. LLM Request Queue -> Calls LLM -> DM Approval Queue
//! 3. DM Approval Queue -> (handled by DM via WebSocket)

pub mod response_parser;

#[cfg(test)]
mod llm_integration_tests;

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{WantVisibility, WorldId};

use crate::llm_context::{
    ActiveChallengeContext, ActiveNarrativeEventContext, CharacterContext, GamePromptRequest,
    MotivationEntry, MotivationsContext, PlayerActionContext, SceneContext, SecretMotivationEntry,
    TriggerHints,
};
use crate::prompt_templates::defaults::{
    DIALOGUE_CHALLENGE_SUGGESTION_FORMAT, DIALOGUE_NARRATIVE_EVENT_FORMAT, DIALOGUE_RESPONSE_FORMAT,
};
use crate::queue_types::{LlmRequestData, LlmRequestType, PlayerActionData};

use crate::infrastructure::ports::{
    ChallengeRepo, CharacterRepo, LlmPort, LocationRepo, NarrativeRepo, PlayerCharacterRepo,
    QueuePort, RepoError, SceneRepo, StagingRepo, WorldRepo,
};

/// Events that need to be broadcast to clients after queue processing.
///
/// These are returned from queue processors and should be handled by the
/// caller (typically main.rs) which has access to ConnectionManager.
#[derive(Debug)]
pub enum BroadcastEvent {
    /// LLM-generated outcome suggestions ready to send to DMs
    OutcomeSuggestionReady {
        world_id: WorldId,
        resolution_id: uuid::Uuid,
        suggestions: Vec<String>,
    },

    /// LLM suggestion request started processing (Creator UI).
    SuggestionProgress {
        world_id: WorldId,
        request_id: String,
    },

    /// LLM suggestion request completed (Creator UI).
    SuggestionComplete {
        world_id: WorldId,
        request_id: String,
        suggestions: Vec<String>,
    },
}

/// Container for queue use cases.
pub struct QueueUseCases {
    pub process_player_action: Arc<ProcessPlayerAction>,
    pub process_llm_request: Arc<ProcessLlmRequest>,
}

impl QueueUseCases {
    pub fn new(
        process_player_action: Arc<ProcessPlayerAction>,
        process_llm_request: Arc<ProcessLlmRequest>,
    ) -> Self {
        Self {
            process_player_action,
            process_llm_request,
        }
    }
}

/// Result of processing a player action.
#[derive(Debug)]
pub struct PlayerActionProcessed {
    /// The original action ID
    pub action_id: uuid::Uuid,
    /// The LLM request ID that was queued
    pub llm_request_id: uuid::Uuid,
}

/// Process player action from queue.
///
/// Dequeues player actions, builds LLM prompts, and enqueues LLM requests.
#[allow(dead_code)]
pub struct ProcessPlayerAction {
    queue: Arc<dyn QueuePort>,
    character: Arc<dyn CharacterRepo>,
    player_character: Arc<dyn PlayerCharacterRepo>,
    staging: Arc<dyn StagingRepo>,
    scene: Arc<dyn SceneRepo>,
    world: Arc<dyn WorldRepo>,
    narrative: Arc<crate::use_cases::narrative_operations::NarrativeOps>,
    location: Arc<dyn LocationRepo>,
    challenge: Arc<dyn ChallengeRepo>,
}

impl ProcessPlayerAction {
    pub fn new(
        queue: Arc<dyn QueuePort>,
        character: Arc<dyn CharacterRepo>,
        player_character: Arc<dyn PlayerCharacterRepo>,
        staging: Arc<dyn StagingRepo>,
        scene: Arc<dyn SceneRepo>,
        world: Arc<dyn WorldRepo>,
        narrative: Arc<crate::use_cases::narrative_operations::NarrativeOps>,
        location: Arc<dyn LocationRepo>,
        challenge: Arc<dyn ChallengeRepo>,
    ) -> Self {
        Self {
            queue,
            character,
            player_character,
            staging,
            scene,
            world,
            narrative,
            location,
            challenge,
        }
    }

    /// Process the next player action in the queue.
    ///
    /// Returns None if the queue is empty.
    pub async fn execute(&self) -> Result<Option<PlayerActionProcessed>, QueueError> {
        // Dequeue the next player action
        let item = match self.queue.dequeue_player_action().await? {
            Some(item) => item,
            None => return Ok(None),
        };

        // Extract the action data
        let action_data = match item.data {
            crate::infrastructure::ports::QueueItemData::PlayerAction(data) => data,
            _ => {
                // Wrong type - mark as failed and return
                self.queue
                    .mark_failed(item.id, "Invalid queue item type")
                    .await?;
                return Err(QueueError::InvalidItemType);
            }
        };

        // Build the prompt with character context
        let prompt = self
            .build_prompt(&action_data)
            .await
            .unwrap_or_else(|_| self.build_fallback_prompt(&action_data));

        let llm_request = LlmRequestData {
            request_type: LlmRequestType::NpcResponse {
                action_item_id: item.id,
            },
            world_id: action_data.world_id,
            pc_id: action_data.pc_id,
            prompt: Some(prompt),
            suggestion_context: None,
            callback_id: item.id.to_string(),
            conversation_id: action_data.conversation_id,
        };

        // Enqueue the LLM request
        let llm_request_id = self.queue.enqueue_llm_request(&llm_request).await?;

        // Mark the player action as complete
        self.queue.mark_complete(item.id).await?;

        Ok(Some(PlayerActionProcessed {
            action_id: item.id,
            llm_request_id,
        }))
    }

    /// Build a full GamePromptRequest with character context from the database.
    async fn build_prompt(
        &self,
        action_data: &PlayerActionData,
    ) -> Result<GamePromptRequest, QueueError> {
        let pc = if let Some(pc_id) = action_data.pc_id {
            self.player_character.get(pc_id).await?
        } else {
            None
        };

        let pc_name = pc
            .as_ref()
            .map(|pc| pc.name().to_string())
            .unwrap_or_else(|| "Unknown Player".to_string());

        let (pc_location_id, _pc_region_id) = pc
            .as_ref()
            .map(|pc| (Some(pc.current_location_id()), pc.current_region_id()))
            .unwrap_or((None, None));

        let npc_id = action_data
            .target
            .as_deref()
            .and_then(try_parse_typed_id::<wrldbldr_domain::CharacterId>);

        // Fetch full NPC entity instead of just name
        let npc_entity = match npc_id {
            Some(id) => self.character.get(id).await?.clone(),
            None => None,
        };

        let target_name = npc_entity
            .as_ref()
            .map(|npc| npc.name().to_string())
            .or_else(|| action_data.target.clone())
            .unwrap_or_else(|| "the NPC".to_string());

        // Get player dialogue
        let dialogue = action_data
            .dialogue
            .clone()
            .unwrap_or_else(|| "[No dialogue]".to_string());

        // Build player action context
        let player_action = PlayerActionContext {
            action_type: action_data.action_type.clone(),
            target: action_data.target.clone(),
            dialogue: action_data.dialogue.clone(),
        };

        // Build scene context with actual location name
        let current_scene = self.scene.get_current(action_data.world_id).await?;
        let game_time = self
            .world
            .get(action_data.world_id)
            .await?
            .map(|world| world.game_time().clone());
        let game_time_display = game_time.as_ref().map(|gt| gt.display_date());
        let time_context = game_time
            .as_ref()
            .map(|gt| gt.time_of_day().display_name().to_string())
            .unwrap_or_else(|| "Present".to_string());

        // Get actual location name from the location entity
        let location_name = if let Some(location_id) = pc_location_id {
            self.location
                .get_location(location_id)
                .await?
                .map(|loc| loc.name().to_string())
                .unwrap_or_else(|| "Unknown Location".to_string())
        } else {
            "Unknown Location".to_string()
        };

        let scene_context = SceneContext {
            scene_name: current_scene
                .as_ref()
                .map(|scene| scene.name().to_string())
                .unwrap_or_else(|| "Current Scene".to_string()),
            location_name,
            time_context,
            present_characters: vec![pc_name.clone(), target_name.clone()],
            region_items: vec![],
        };

        // Fetch NPC disposition toward PC (if both IDs exist) - optional context for prompt
        let disposition = match (npc_id, action_data.pc_id) {
            (Some(npc), Some(pc)) => match self.character.get_disposition(npc, pc).await {
                Ok(d) => d,
                Err(e) => {
                    tracing::debug!(npc_id = %npc, pc_id = %pc, error = %e, "Failed to fetch NPC disposition for prompt context");
                    None
                }
            },
            _ => None,
        };

        // Fetch NPC wants (motivations) - optional context for prompt
        let wants = match npc_id {
            Some(npc) => match self.character.get_wants(npc).await {
                Ok(w) => Some(w),
                Err(e) => {
                    tracing::debug!(npc_id = %npc, error = %e, "Failed to fetch NPC wants for prompt context");
                    None
                }
            },
            None => None,
        };

        // Build responding character context with full NPC data
        let responding_character = if let Some(ref npc) = npc_entity {
            // Build motivations context from wants
            let motivations = wants.map(|want_list| {
                let mut known = Vec::new();
                let mut suspected = Vec::new();
                let mut secret = Vec::new();

                for want_details in want_list {
                    let want = &want_details.want;
                    let priority = want_details.priority;
                    let intensity = match want.intensity() {
                        i if i >= 0.8 => "Obsessive",
                        i if i >= 0.6 => "Strong",
                        i if i >= 0.4 => "Moderate",
                        _ => "Mild",
                    }
                    .to_string();

                    match want.visibility() {
                        WantVisibility::Known => {
                            known.push(MotivationEntry {
                                description: want.description().to_string(),
                                priority,
                                intensity,
                                target: want_details.target.as_ref().map(|t| format!("{:?}", t)),
                                helpers: vec![],
                                opponents: vec![],
                            });
                        }
                        WantVisibility::Suspected => {
                            suspected.push(MotivationEntry {
                                description: want.description().to_string(),
                                priority,
                                intensity,
                                target: want_details.target.as_ref().map(|t| format!("{:?}", t)),
                                helpers: vec![],
                                opponents: vec![],
                            });
                        }
                        WantVisibility::Hidden => {
                            secret.push(SecretMotivationEntry {
                                description: want.description().to_string(),
                                priority,
                                intensity,
                                target: want_details.target.as_ref().map(|t| format!("{:?}", t)),
                                helpers: vec![],
                                opponents: vec![],
                                sender: None,
                                receiver: None,
                                deflection_behavior: want
                                    .deflection_behavior()
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| {
                                        "Change the subject or become evasive".to_string()
                                    }),
                                tells: want.tells().to_vec(),
                            });
                        }
                    }
                }

                MotivationsContext {
                    known,
                    suspected,
                    secret,
                }
            });

            CharacterContext {
                character_id: npc_id.map(|id| id.to_string()),
                name: npc.name().to_string(),
                archetype: npc.current_archetype().to_string(),
                current_mood: Some(npc.default_mood().to_string()),
                disposition_toward_player: disposition
                    .as_ref()
                    .map(|d| d.disposition().to_string())
                    .or_else(|| Some(npc.default_disposition().to_string())),
                motivations,
                social_stance: None, // Could be populated from actantial context
                relationship_to_player: disposition.as_ref().map(|d| d.relationship().to_string()),
                available_expressions: Some(npc.expression_config().expressions().to_vec()),
                available_actions: Some(npc.expression_config().actions().to_vec()),
            }
        } else {
            // Fallback for when NPC entity is not found
            CharacterContext {
                character_id: npc_id.map(|id| id.to_string()),
                name: target_name.clone(),
                archetype: "NPC".to_string(),
                current_mood: None,
                disposition_toward_player: None,
                motivations: None,
                social_stance: None,
                relationship_to_player: None,
                available_expressions: None,
                available_actions: None,
            }
        };

        // Build directorial notes with rich context
        // Include scene directorial notes if available
        let scene_direction = current_scene
            .as_ref()
            .map(|s| s.directorial_notes())
            .filter(|notes| !notes.is_empty())
            .map(|notes| format!("\n\nSCENE DIRECTION: {}", notes))
            .unwrap_or_default();

        // Build NPC character guidance
        let npc_guidance = if let Some(ref npc) = npc_entity {
            let archetype_info = format!("Archetype: {}", npc.current_archetype());
            let disposition_info = disposition
                .as_ref()
                .map(|d| {
                    format!(
                        "Disposition toward player: {} ({})",
                        d.disposition(),
                        d.relationship()
                    )
                })
                .unwrap_or_else(|| format!("Default disposition: {}", npc.default_disposition()));
            let mood_info = format!("Current mood: {}", npc.default_mood());

            format!(
                "\n\nCHARACTER GUIDANCE:\n- {}\n- {}\n- {}",
                archetype_info, disposition_info, mood_info
            )
        } else {
            String::new()
        };

        let directorial_notes = format!(
            "You are roleplaying as {} in a fantasy TTRPG.{}{}\n\n\
            The player character \"{}\" says: \"{}\"\n\n\
            Keep the response concise (1-3 sentences). Stay in character.",
            target_name, scene_direction, npc_guidance, pc_name, dialogue
        );

        // Fetch conversation history if we have both PC and NPC IDs
        // Default limit is 20 turns (can be made configurable via settings)
        let conversation_history = match (action_data.pc_id, npc_id) {
            (Some(pc_id), Some(npc_id)) => self
                .narrative
                .get_conversation_turns(pc_id, npc_id, 20)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        error = %e,
                        "Failed to fetch conversation history, using empty"
                    );
                    vec![]
                }),
            _ => vec![],
        };

        // Fetch active challenges for this world
        let active_challenges = self
            .challenge
            .list_for_world(action_data.world_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to fetch active challenges");
                vec![]
            })
            .into_iter()
            .filter(|c| c.active())
            .map(|c| {
                // Extract trigger hints from trigger conditions, grouped by type
                let mut trigger_hints = TriggerHints::default();

                for tc in c.trigger_conditions() {
                    match &tc.condition_type {
                        wrldbldr_domain::TriggerType::DialogueTopic { topic_keywords } => {
                            trigger_hints.pc_mentions.extend(topic_keywords.clone());
                        }
                        wrldbldr_domain::TriggerType::ObjectInteraction { keywords } => {
                            trigger_hints.interacts_with.extend(keywords.clone());
                        }
                        wrldbldr_domain::TriggerType::Custom { description } => {
                            trigger_hints.custom.push(description.clone());
                        }
                        _ => {}
                    }
                }

                ActiveChallengeContext {
                    id: c.id().to_string(),
                    name: c.name().to_string(),
                    description: c.description().to_string(),
                    skill_name: c
                        .check_stat()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "General".to_string()),
                    difficulty_display: c.difficulty().display(),
                    trigger_hints,
                }
            })
            .collect();

        // Fetch active narrative events for this world
        let active_narrative_events: Vec<ActiveNarrativeEventContext> = self
            .narrative
            .list_events(action_data.world_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to fetch narrative events");
                vec![]
            })
            .into_iter()
            // Include active events that are either:
            // 1. Not yet triggered, OR
            // 2. Repeatable (can trigger again even if already triggered)
            .filter(|e| e.is_active() && (!e.is_triggered() || e.is_repeatable()))
            .map(|e| {
                // Extract trigger hints from trigger conditions, grouped by type
                let mut trigger_hints = TriggerHints::default();

                for tc in e.trigger_conditions() {
                    match &tc.trigger_type {
                        wrldbldr_domain::NarrativeTriggerType::DialogueTopic {
                            keywords, ..
                        } => {
                            trigger_hints.pc_mentions.extend(keywords.clone());
                        }
                        wrldbldr_domain::NarrativeTriggerType::NpcAction {
                            action_keywords,
                            ..
                        } => {
                            trigger_hints
                                .custom
                                .push(format!("NPC action: {}", action_keywords.join(", ")));
                        }
                        wrldbldr_domain::NarrativeTriggerType::Custom { description, .. } => {
                            trigger_hints.custom.push(description.clone());
                        }
                        wrldbldr_domain::NarrativeTriggerType::HasItem { item_name, .. } => {
                            trigger_hints
                                .custom
                                .push(format!("PC has item: {}", item_name));
                        }
                        wrldbldr_domain::NarrativeTriggerType::FlagSet { flag_name } => {
                            trigger_hints
                                .custom
                                .push(format!("flag set: {}", flag_name));
                        }
                        _ => {}
                    }
                }

                // Get featured NPC names (if any are associated via graph)
                // Note: This would require a separate query; for now, use empty vec
                let featured_npc_names: Vec<String> = vec![];

                ActiveNarrativeEventContext {
                    id: e.id().to_string(),
                    name: e.name().to_string(),
                    description: e.description().to_string(),
                    scene_direction: e.scene_direction().to_string(),
                    trigger_hints,
                    featured_npc_names,
                    priority: e.priority(),
                }
            })
            .collect();

        Ok(GamePromptRequest {
            world_id: Some(action_data.world_id.to_string()),
            player_action,
            scene_context,
            directorial_notes,
            conversation_history,
            responding_character,
            active_challenges,
            active_narrative_events,
            context_budget: None,
            scene_id: current_scene.as_ref().map(|scene| scene.id().to_string()),
            location_id: pc_location_id.map(|id| id.to_string()),
            game_time: game_time_display,
        })
    }

    /// Build a fallback prompt when database lookups fail.
    fn build_fallback_prompt(&self, action_data: &PlayerActionData) -> GamePromptRequest {
        let target_name = action_data
            .target
            .clone()
            .unwrap_or_else(|| "the NPC".to_string());

        let dialogue = action_data
            .dialogue
            .clone()
            .unwrap_or_else(|| "[No dialogue]".to_string());

        let player_action = PlayerActionContext {
            action_type: action_data.action_type.clone(),
            target: action_data.target.clone(),
            dialogue: action_data.dialogue.clone(),
        };

        let scene_context = SceneContext {
            scene_name: "Current Scene".to_string(),
            location_name: "Current Location".to_string(),
            time_context: "Present".to_string(),
            present_characters: vec!["Player".to_string(), target_name.clone()],
            region_items: vec![],
        };

        let responding_character = CharacterContext {
            character_id: None,
            name: target_name.clone(),
            archetype: "NPC".to_string(),
            current_mood: None,
            disposition_toward_player: None,
            motivations: None,
            social_stance: None,
            relationship_to_player: None,
            available_expressions: None,
            available_actions: None,
        };

        let directorial_notes = format!(
            "You are roleplaying as an NPC in a fantasy TTRPG. \
            The player says to {}: \"{}\". \
            Respond in character as {}. Keep the response concise (1-3 sentences).",
            target_name, dialogue, target_name
        );

        GamePromptRequest {
            world_id: Some(action_data.world_id.to_string()),
            player_action,
            scene_context,
            directorial_notes,
            conversation_history: vec![],
            responding_character,
            active_challenges: vec![],
            active_narrative_events: vec![],
            context_budget: None,
            scene_id: None,
            location_id: None,
            game_time: None,
        }
    }
}

/// Result of processing an LLM request.
#[derive(Debug)]
pub struct LlmRequestProcessed {
    /// The original request ID
    pub request_id: uuid::Uuid,
    /// The approval queue ID
    pub approval_id: uuid::Uuid,
    /// The generated NPC dialogue
    pub npc_dialogue: String,
    /// Events to broadcast to clients
    pub broadcast_events: Vec<BroadcastEvent>,
}

/// Process LLM request from queue.
///
/// Dequeues LLM requests, calls the LLM, and enqueues DM approval requests.
pub struct ProcessLlmRequest {
    queue: Arc<dyn QueuePort>,
    llm: Arc<dyn LlmPort>,
    challenge: Arc<dyn ChallengeRepo>,
    narrative: Arc<dyn NarrativeRepo>,
}

impl ProcessLlmRequest {
    pub fn new(
        queue: Arc<dyn QueuePort>,
        llm: Arc<dyn LlmPort>,
        challenge: Arc<dyn ChallengeRepo>,
        narrative: Arc<dyn NarrativeRepo>,
    ) -> Self {
        Self {
            queue,
            llm,
            challenge,
            narrative,
        }
    }

    /// Process the next LLM request in the queue.
    ///
    /// Returns None if the queue is empty.
    ///
    /// The `on_start` callback is invoked with immediate broadcast events BEFORE
    /// the LLM call starts (e.g., SuggestionProgress). This allows the caller to
    /// broadcast progress events before the potentially slow LLM operation.
    pub async fn execute<F>(&self, on_start: F) -> Result<Option<LlmRequestProcessed>, QueueError>
    where
        F: FnOnce(Vec<BroadcastEvent>),
    {
        // Dequeue the next LLM request
        let item = match self.queue.dequeue_llm_request().await? {
            Some(item) => item,
            None => return Ok(None),
        };

        // Extract the request data
        let request_data = match &item.data {
            crate::infrastructure::ports::QueueItemData::LlmRequest(data) => data.clone(),
            _ => {
                self.queue
                    .mark_failed(item.id, "Invalid queue item type")
                    .await?;
                return Err(QueueError::InvalidItemType);
            }
        };

        // Handle different request types
        match &request_data.request_type {
            LlmRequestType::OutcomeSuggestion {
                resolution_id,
                world_id,
                challenge_name,
                current_description,
                guidance,
            } => {
                // Build prompt for outcome suggestions
                let system_prompt = "You are a creative TTRPG game master assistant. \
                    Generate 3 alternative narrative descriptions for a challenge outcome. \
                    Each suggestion should be evocative and fit the fantasy setting. \
                    Return each suggestion on a separate line, numbered 1-3.";

                let user_message = format!(
                    "Challenge: {}\nCurrent outcome description: \"{}\"\n{}Generate 3 alternative descriptions.",
                    challenge_name,
                    current_description,
                    guidance.as_ref().map(|g| format!("DM guidance: {}\n", g)).unwrap_or_default()
                );

                let llm_request = crate::infrastructure::ports::LlmRequest::new(vec![
                    crate::infrastructure::ports::ChatMessage::user(&user_message),
                ])
                .with_system_prompt(system_prompt.to_string())
                .with_temperature(0.8);

                let llm_response = self.llm.generate(llm_request).await.map_err(|e| {
                    tracing::error!(
                        resolution_id = %resolution_id,
                        world_id = %world_id,
                        challenge = %challenge_name,
                        error = %e,
                        "LLM outcome suggestion generation failed"
                    );
                    e
                })?;

                // Parse suggestions from response (one per line)
                let suggestions: Vec<String> = llm_response
                    .content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| {
                        // Strip numbering if present (e.g., "1. ", "1) ", etc.)
                        let trimmed = line.trim();
                        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
                            rest.trim_start_matches(['.', ')', ':', '-', ' '])
                                .trim()
                                .to_string()
                        } else {
                            trimmed.to_string()
                        }
                    })
                    .take(3)
                    .collect();

                tracing::info!(
                    resolution_id = %resolution_id,
                    world_id = %world_id,
                    suggestion_count = suggestions.len(),
                    "Generated outcome suggestions"
                );

                // Create broadcast event for DMs
                let broadcast_event = BroadcastEvent::OutcomeSuggestionReady {
                    world_id: *world_id,
                    resolution_id: *resolution_id,
                    suggestions: suggestions.clone(),
                };

                // Mark the LLM request as complete
                self.queue.mark_complete(item.id).await?;

                Ok(Some(LlmRequestProcessed {
                    request_id: item.id,
                    approval_id: *resolution_id, // Use resolution_id as the "approval" for tracking
                    npc_dialogue: llm_response.content,
                    broadcast_events: vec![broadcast_event],
                }))
            }
            LlmRequestType::Suggestion {
                field_type,
                entity_id: _,
            } => {
                // Generic content suggestions are returned directly to clients (no DM approval).

                // Capture info for progress event before processing
                let world_id = request_data.world_id;
                let callback_id = request_data.callback_id.clone();

                // Emit progress event BEFORE the LLM call starts
                on_start(vec![BroadcastEvent::SuggestionProgress {
                    world_id,
                    request_id: callback_id.clone(),
                }]);

                let context = request_data.suggestion_context.clone().unwrap_or_default();

                let prompt = build_suggestion_prompt(field_type, &context);

                let llm_request = crate::infrastructure::ports::LlmRequest::new(vec![
                    crate::infrastructure::ports::ChatMessage::user(&prompt),
                ])
                .with_system_prompt(
                    "You are a helpful worldbuilding assistant. Return only suggestions, one per line.".to_string(),
                )
                .with_temperature(0.8);

                let llm_response = self.llm.generate(llm_request).await.map_err(|e| {
                    tracing::error!(
                        world_id = %world_id,
                        field_type = ?field_type,
                        callback_id = %callback_id,
                        error = %e,
                        "LLM suggestion generation failed"
                    );
                    e
                })?;

                let suggestions: Vec<String> = llm_response
                    .content
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(|line| {
                        // Strip simple numbering/bullets
                        line.trim_start_matches(|c: char| {
                            c.is_ascii_digit() || matches!(c, '.' | ')' | '-' | ':' | ' ')
                        })
                        .trim()
                        .to_string()
                    })
                    .filter(|l| !l.is_empty())
                    .take(10)
                    .collect();

                // Persist for hydration.
                let result_json = serde_json::json!({ "suggestions": suggestions });
                if let Err(e) = self
                    .queue
                    .set_result_json(item.id, &result_json.to_string())
                    .await
                {
                    tracing::warn!(
                        item_id = %item.id,
                        world_id = %world_id,
                        callback_id = %callback_id,
                        error = %e,
                        "Failed to persist suggestion results for hydration - suggestions delivered but won't survive reconnection"
                    );
                }

                // Mark the LLM request as complete.
                self.queue.mark_complete(item.id).await?;

                // Return only the completion event (progress was already emitted via callback)
                Ok(Some(LlmRequestProcessed {
                    request_id: item.id,
                    approval_id: uuid::Uuid::nil(),
                    npc_dialogue: llm_response.content,
                    broadcast_events: vec![BroadcastEvent::SuggestionComplete {
                        world_id,
                        request_id: callback_id,
                        suggestions,
                    }],
                }))
            }

            LlmRequestType::NpcResponse { .. } => {
                // Build LLM request from the queued prompt data
                let llm_request = if let Some(ref prompt) = request_data.prompt {
                    // Use the full GamePromptRequest to build a rich prompt
                    // Include structured output format from prompt_templates

                    // Format active challenges with their trigger hints
                    let challenges_context = if prompt.active_challenges.is_empty() {
                        String::new()
                    } else {
                        let challenge_list: Vec<String> = prompt
                            .active_challenges
                            .iter()
                            .map(|c| {
                                format!(
                                    "- {} [{}]: {} - {} {}{}",
                                    c.name,
                                    c.id,
                                    c.description,
                                    c.skill_name,
                                    c.difficulty_display,
                                    c.trigger_hints.format_for_prompt()
                                )
                            })
                            .collect();
                        format!(
                            "\n\nACTIVE CHALLENGES:\nYou MUST analyze each challenge below and determine if the player's action triggers it.\nFor each challenge, check if the player's words match any keywords in `pc_mentions` or if their action matches `interacts_with`.\n\n{}\n\nYou MUST include a <challenge_analysis> section in your response evaluating EACH challenge.",
                            challenge_list.join("\n")
                        )
                    };

                    // Format active narrative events with their trigger hints
                    let events_context = if prompt.active_narrative_events.is_empty() {
                        String::new()
                    } else {
                        let event_list: Vec<String> = prompt
                            .active_narrative_events
                            .iter()
                            .map(|e| {
                                format!(
                                    "- {} [{}]: {}{}",
                                    e.name,
                                    e.id,
                                    e.description,
                                    e.trigger_hints.format_for_prompt()
                                )
                            })
                            .collect();
                        format!(
                            "\n\nACTIVE NARRATIVE EVENTS:\nYou MUST analyze each event below and determine if the player's action triggers it.\nFor each event, check if the player's words match any keywords in `pc_mentions`.\n\n{}\n\nYou MUST include an <event> block for EACH event listed above.",
                            event_list.join("\n")
                        )
                    };

                    let system_prompt = format!(
                        "You are roleplaying as an NPC in a fantasy TTRPG. {}\n\n\
                        Scene: {} at {}\n\
                        Present characters: {}\n\n\
                        Stay true to the NPC's personality and motivations.{}{}\n\n\
                        {}\n\n\
                        {}\n\n\
                        {}",
                        prompt.directorial_notes,
                        prompt.scene_context.scene_name,
                        prompt.scene_context.location_name,
                        prompt.scene_context.present_characters.join(", "),
                        challenges_context,
                        events_context,
                        DIALOGUE_RESPONSE_FORMAT,
                        DIALOGUE_CHALLENGE_SUGGESTION_FORMAT,
                        DIALOGUE_NARRATIVE_EVENT_FORMAT
                    );

                    let current_message = if let Some(ref dialogue) = prompt.player_action.dialogue
                    {
                        format!(
                            "The player character says to {}: \"{}\"",
                            prompt.player_action.target.as_deref().unwrap_or("you"),
                            dialogue
                        )
                    } else {
                        format!(
                            "The player character performs action '{}' targeting {}",
                            prompt.player_action.action_type,
                            prompt.player_action.target.as_deref().unwrap_or("you")
                        )
                    };

                    // Convert conversation history to chat messages
                    // NPC turns become "assistant" messages, player turns become "user" messages
                    // The NPC's name is in responding_character.name
                    let npc_name = &prompt.responding_character.name;

                    let mut messages: Vec<crate::infrastructure::ports::ChatMessage> = prompt
                        .conversation_history
                        .iter()
                        .map(|turn| {
                            // If the speaker matches the NPC's name, it's an assistant message
                            // Otherwise it's a user message (player/PC)
                            if turn.speaker == *npc_name
                                || turn.speaker.to_lowercase() == npc_name.to_lowercase()
                            {
                                crate::infrastructure::ports::ChatMessage::assistant(&turn.text)
                            } else {
                                // Player/PC dialogue
                                crate::infrastructure::ports::ChatMessage::user(&turn.text)
                            }
                        })
                        .collect();

                    // Add the current message
                    messages.push(crate::infrastructure::ports::ChatMessage::user(
                        &current_message,
                    ));

                    crate::infrastructure::ports::LlmRequest::new(messages)
                        .with_system_prompt(system_prompt)
                        .with_temperature(0.7)
                        .with_max_tokens(Some(2048)) // Allow room for model's internal reasoning + structured output
                } else {
                    // Fallback if no prompt was provided
                    tracing::warn!("LLM request has no prompt data, using fallback");
                    crate::infrastructure::ports::LlmRequest::new(vec![
                        crate::infrastructure::ports::ChatMessage::user(
                            "Generate a brief, in-character NPC response to the player's action.",
                        ),
                    ])
                    .with_system_prompt(
                        "You are an NPC in a fantasy TTRPG. Respond briefly and in character.",
                    )
                    .with_max_tokens(Some(512)) // Simpler fallback response
                };

                // Extract NPC context for error logging
                let npc_name_for_log = request_data
                    .prompt
                    .as_ref()
                    .map(|p| p.responding_character.name.as_str())
                    .unwrap_or("unknown");
                let world_id_for_log = request_data.world_id;

                let llm_response = self.llm.generate(llm_request).await.map_err(|e| {
                    tracing::error!(
                        world_id = %world_id_for_log,
                        npc_name = %npc_name_for_log,
                        request_id = %item.id,
                        error = %e,
                        "LLM NPC response generation failed"
                    );
                    e
                })?;

                // Log raw LLM response for debugging structured output
                tracing::debug!(
                    raw_content = %llm_response.content,
                    content_len = llm_response.content.len(),
                    "Raw LLM response content"
                );

                // Parse structured content from LLM response
                let parsed = response_parser::parse_llm_response(&llm_response.content);

                // Log parsed output for debugging
                tracing::debug!(
                    reasoning_len = parsed.reasoning.len(),
                    dialogue_len = parsed.dialogue.len(),
                    topics_count = parsed.topics.len(),
                    tools_count = parsed.proposed_tools.len(),
                    has_challenge_suggestion = parsed.challenge_suggestion.is_some(),
                    has_narrative_event_suggestion = parsed.narrative_event_suggestion.is_some(),
                    "Parsed LLM response"
                );

                // Extract proposed tools from XML tags in LLM response
                let proposed_tools = parsed.proposed_tools.clone();

                let (npc_id, npc_name, player_dialogue, scene_id, location_id, game_time) =
                    if let Some(ref prompt) = request_data.prompt {
                        let npc_id = prompt
                            .responding_character
                            .character_id
                            .as_deref()
                            .and_then(try_parse_typed_id::<wrldbldr_domain::CharacterId>);
                        let npc_name = prompt.responding_character.name.clone();
                        let player_dialogue = prompt.player_action.dialogue.clone();
                        let scene_id = prompt
                            .scene_id
                            .as_deref()
                            .and_then(try_parse_typed_id::<wrldbldr_domain::SceneId>);
                        let location_id = prompt
                            .location_id
                            .as_deref()
                            .and_then(try_parse_typed_id::<wrldbldr_domain::LocationId>);
                        let game_time = prompt.game_time.clone();

                        (
                            npc_id,
                            npc_name,
                            player_dialogue,
                            scene_id,
                            location_id,
                            game_time,
                        )
                    } else {
                        (None, String::new(), None, None, None, None)
                    };

                // Enrich challenge suggestion with metadata if present
                // First check for explicit JSON suggestion, then fall back to triggered challenge tags
                let challenge_suggestion = if let Some(raw) = parsed.challenge_suggestion {
                    self.enrich_challenge_suggestion(&raw, request_data.pc_id)
                        .await
                } else if let Some(triggered_name) = parsed.triggered_challenge_names.first() {
                    // Convert triggered challenge from <challenge> tag to challenge_suggestion
                    self.create_challenge_suggestion_from_name(
                        request_data.world_id,
                        triggered_name,
                        request_data.pc_id,
                    )
                    .await
                } else {
                    None
                };

                // Enrich narrative event suggestion with metadata if present
                // First check for explicit JSON suggestion, then fall back to triggered event tags
                let narrative_event_suggestion =
                    if let Some(raw) = parsed.narrative_event_suggestion {
                        self.enrich_narrative_event_suggestion(&raw).await
                    } else if let Some(triggered_name) = parsed.triggered_event_names.first() {
                        // Convert triggered event from <event> tag to narrative_event_suggestion
                        self.create_narrative_event_suggestion_from_name(
                            request_data.world_id,
                            triggered_name,
                        )
                        .await
                    } else {
                        None
                    };

                // Clone dialogue for return value before moving into approval data
                let npc_dialogue = parsed.dialogue.clone();

                // Create approval request
                let approval_data = crate::queue_types::ApprovalRequestData {
                    world_id: request_data.world_id,
                    source_action_id: item.id,
                    decision_type: crate::queue_types::ApprovalDecisionType::NpcResponse,
                    urgency: crate::queue_types::ApprovalUrgency::AwaitingPlayer,
                    pc_id: request_data.pc_id,
                    npc_id,
                    npc_name,
                    proposed_dialogue: parsed.dialogue,
                    internal_reasoning: parsed.reasoning,
                    proposed_tools,
                    retry_count: 0,
                    challenge_suggestion,
                    narrative_event_suggestion,
                    challenge_outcome: None,
                    player_dialogue,
                    scene_id,
                    location_id,
                    game_time,
                    topics: parsed.topics,
                    conversation_id: request_data.conversation_id,
                };

                // Enqueue for DM approval
                let approval_id = self.queue.enqueue_dm_approval(&approval_data).await?;

                // Mark the LLM request as complete
                self.queue.mark_complete(item.id).await?;
                Ok(Some(LlmRequestProcessed {
                    request_id: item.id,
                    approval_id,
                    npc_dialogue,
                    broadcast_events: vec![],
                }))
            }
        }
    }

    /// Enrich a raw challenge suggestion with challenge metadata from the database.
    async fn enrich_challenge_suggestion(
        &self,
        raw: &response_parser::RawChallengeSuggestion,
        pc_id: Option<wrldbldr_domain::PlayerCharacterId>,
    ) -> Option<crate::queue_types::ChallengeSuggestion> {
        // Parse the challenge ID
        let challenge_id: wrldbldr_domain::ChallengeId = try_parse_typed_id(&raw.challenge_id)?;

        // Fetch challenge from database
        let challenge = match self.challenge.get(challenge_id).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                tracing::warn!(
                    challenge_id = %raw.challenge_id,
                    "Challenge not found for LLM suggestion"
                );
                return None;
            }
            Err(e) => {
                tracing::error!(
                    challenge_id = %raw.challenge_id,
                    error = %e,
                    "Failed to fetch challenge for LLM suggestion"
                );
                return None;
            }
        };

        Some(crate::queue_types::ChallengeSuggestion {
            challenge_id: raw.challenge_id.clone(),
            challenge_name: challenge.name().to_string(),
            skill_name: challenge.check_stat().unwrap_or("").to_string(),
            difficulty_display: challenge.difficulty().display(),
            confidence: raw.confidence.clone(),
            reasoning: raw.reasoning.clone(),
            target_pc_id: pc_id,
            outcomes: None, // Outcomes can be populated later if needed
        })
    }

    /// Enrich a raw narrative event suggestion with event metadata from the database.
    async fn enrich_narrative_event_suggestion(
        &self,
        raw: &response_parser::RawNarrativeEventSuggestion,
    ) -> Option<crate::queue_types::NarrativeEventSuggestion> {
        // Parse the event ID
        let event_id: wrldbldr_domain::NarrativeEventId = try_parse_typed_id(&raw.event_id)?;

        // Fetch event from database
        let event = match self.narrative.get_event(event_id).await {
            Ok(Some(e)) => e,
            Ok(None) => {
                tracing::warn!(
                    event_id = %raw.event_id,
                    "Narrative event not found for LLM suggestion"
                );
                return None;
            }
            Err(e) => {
                tracing::error!(
                    event_id = %raw.event_id,
                    error = %e,
                    "Failed to fetch narrative event for LLM suggestion"
                );
                return None;
            }
        };

        Some(crate::queue_types::NarrativeEventSuggestion {
            event_id: raw.event_id.clone(),
            event_name: event.name().to_string(),
            description: event.description().to_string(),
            scene_direction: event.scene_direction().to_string(),
            confidence: raw.confidence.clone(),
            reasoning: raw.reasoning.clone(),
            matched_triggers: raw.matched_triggers.clone(),
            suggested_outcome: event.default_outcome().map(|s| s.to_string()),
        })
    }

    /// Create a narrative event suggestion from an event name found in <event> tags.
    ///
    /// This bridges the <event name="..."><trigger>YES</trigger></event> format
    /// to the NarrativeEventSuggestion structure expected by the approval queue.
    async fn create_narrative_event_suggestion_from_name(
        &self,
        world_id: wrldbldr_domain::WorldId,
        event_name: &str,
    ) -> Option<crate::queue_types::NarrativeEventSuggestion> {
        // Find the event by name in this world
        let events = match self.narrative.list_events_for_world(world_id).await {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(
                    world_id = %world_id,
                    event_name = %event_name,
                    error = %e,
                    "Failed to list events for triggered event lookup"
                );
                return None;
            }
        };

        let event = events
            .into_iter()
            .find(|e| e.name().as_str() == event_name)?;

        tracing::info!(
            event_id = %event.id(),
            event_name = %event_name,
            "Created narrative event suggestion from <event> tag"
        );

        Some(crate::queue_types::NarrativeEventSuggestion {
            event_id: event.id().to_string(),
            event_name: event.name().to_string(),
            description: event.description().to_string(),
            scene_direction: event.scene_direction().to_string(),
            confidence: "high".to_string(), // Event tag means LLM determined it triggered
            reasoning: "Player dialogue matched event trigger keywords".to_string(),
            matched_triggers: vec![], // Could be populated from parsed quote if needed
            suggested_outcome: event.default_outcome().map(|s| s.to_string()),
        })
    }

    /// Create a challenge suggestion from a challenge name found in <challenge> tags.
    ///
    /// This bridges the <challenge name="..."><trigger>YES</trigger></challenge> format
    /// to the ChallengeSuggestion structure expected by the approval queue.
    async fn create_challenge_suggestion_from_name(
        &self,
        world_id: wrldbldr_domain::WorldId,
        challenge_name: &str,
        pc_id: Option<wrldbldr_domain::PlayerCharacterId>,
    ) -> Option<crate::queue_types::ChallengeSuggestion> {
        // Find the challenge by name in this world
        let challenges = match self.challenge.list_for_world(world_id).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(
                    world_id = %world_id,
                    challenge_name = %challenge_name,
                    error = %e,
                    "Failed to list challenges for triggered challenge lookup"
                );
                return None;
            }
        };

        let challenge = challenges
            .into_iter()
            .find(|c| c.name().as_str() == challenge_name)?;

        tracing::info!(
            challenge_id = %challenge.id(),
            challenge_name = %challenge_name,
            "Created challenge suggestion from <challenge> tag"
        );

        Some(crate::queue_types::ChallengeSuggestion {
            challenge_id: challenge.id().to_string(),
            challenge_name: challenge.name().to_string(),
            skill_name: challenge.check_stat().unwrap_or("").to_string(),
            difficulty_display: challenge.difficulty().display(),
            confidence: "high".to_string(), // Challenge tag means LLM determined it triggered
            reasoning: "Player dialogue matched challenge trigger keywords".to_string(),
            target_pc_id: pc_id,
            outcomes: None,
        })
    }
}

fn build_suggestion_prompt(
    field_type: &str,
    context: &crate::queue_types::SuggestionContext,
) -> String {
    let entity_type = context.entity_type.as_deref().unwrap_or("entity");
    let entity_name = context.entity_name.as_deref().unwrap_or("(unnamed)");
    let world_setting = context.world_setting.as_deref().unwrap_or("fantasy");
    let hints = context.hints.as_deref().unwrap_or("");
    let extra = context.additional_context.as_deref().unwrap_or("");

    match field_type {
        "character_name" => format!(
            "Generate 5 unique character names for a {} in a {} setting. Hints: {}. Return one per line.",
            entity_type, world_setting, hints
        ),
        "location_name" => format!(
            "Generate 5 evocative names for a {} called '{}' in a {} setting. Hints: {}. Return one per line.",
            entity_type, entity_name, world_setting, hints
        ),
        "character_description" => format!(
            "Generate 3 different physical descriptions for '{}' (a {}). Setting: {}. Hints: {}. Return each description on its own line.",
            entity_name, entity_type, world_setting, hints
        ),
        "location_description" => format!(
            "Generate 3 different descriptions for '{}' (a {}). Setting: {}. Hints: {}. Return each description on its own line.",
            entity_name, entity_type, world_setting, hints
        ),
        "deflection_behavior" => format!(
            "Generate 3 different deflection behaviors for {entity_name} when trying to hide their desire for: {hints}.\nSetting: {world_setting}.\nCharacter context: {extra}.\n\nA deflection behavior is how a character acts to conceal their true want - nervous habits, diversionary topics, or defensive responses.\nEach suggestion should be 1-2 sentences describing the specific behavior.\nReturn each suggestion on its own line.",
            entity_name = entity_name,
            hints = hints,
            world_setting = world_setting,
            extra = extra
        ),
        "behavioral_tells" => format!(
            "Generate 3 different behavioral tells for {entity_name} that reveal their hidden desire for: {hints}.\nSetting: {world_setting}.\nCharacter context: {extra}.\n\nA behavioral tell is a subtle sign that betrays the character's true motivation - a glance, a pause, an involuntary reaction.\nThese are clues perceptive players might notice.\nEach suggestion should be 1-2 sentences describing the specific tell.\nReturn each suggestion on its own line.",
            entity_name = entity_name,
            hints = hints,
            world_setting = world_setting,
            extra = extra
        ),
        "want_description" => format!(
            "Generate 3 different want descriptions for {entity_name} in a {world_setting} setting.\nCharacter archetype: {hints}.\nAdditional context: {extra}.\n\nEach want should be phrased as a specific desire or goal, not a personality trait.\nFocus on what the character actively pursues or needs.\nEach description should be a single compelling sentence.\nReturn each want on its own line.",
            entity_name = entity_name,
            world_setting = world_setting,
            hints = hints,
            extra = extra
        ),
        "actantial_reason" => format!(
            "Generate 3 different reasons why {entity_name} views {hints} as {extra} regarding their current goal.\nSetting: {world_setting}.\n\nProvide narrative justifications for this actantial relationship that could drive interesting roleplay.\nEach reason should explain the history, incident, or belief that created this dynamic.\nEach suggestion should be 1-2 sentences.\nReturn each reason on its own line.",
            entity_name = entity_name,
            hints = hints,
            extra = extra,
            world_setting = world_setting
        ),
        other => format!(
            "Generate 4 suggestions for {} for '{}' ({}). Setting: {}. Hints: {}. Context: {}. Return one per line.",
            other, entity_name, entity_type, world_setting, hints, extra
        ),
    }
}

/// Parse a string value into a typed ID.
/// Currently unused but kept for future queue processing needs.
#[allow(dead_code)]
fn parse_typed_id<T: From<Uuid>>(value: &str, field: &str) -> Result<T, QueueError> {
    Uuid::parse_str(value)
        .map(T::from)
        .map_err(|e| QueueError::ParseError(format!("Invalid UUID for {}: {}", field, e)))
}

/// Try to parse a typed ID, returning None if the value is empty or invalid.
/// Use this for optional fields where failure means "no value" rather than "bad data".
fn try_parse_typed_id<T: From<Uuid>>(value: &str) -> Option<T> {
    if value.is_empty() {
        return None;
    }
    Uuid::parse_str(value).ok().map(T::from)
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue error: {0}")]
    Queue(#[from] crate::infrastructure::ports::QueueError),
    #[error("Invalid queue item type")]
    InvalidItemType,
    #[error("LLM error: {0}")]
    Llm(#[from] crate::infrastructure::ports::LlmError),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Parse error: {0}")]
    ParseError(String),
}
