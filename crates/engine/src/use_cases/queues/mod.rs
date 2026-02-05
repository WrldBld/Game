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
pub mod uuid_aliaser;
pub mod phases;
use uuid_aliaser::UuidAliaser;

#[cfg(test)]
mod llm_integration_tests;
#[cfg(test)]
mod dialogue_persistence_tests;

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{WantVisibility, WorldId};

use crate::llm_context::{
    ActiveChallengeContext, ActiveNarrativeEventContext, CharacterContext, GamePromptRequest,
    MotivationEntry, MotivationsContext, PlayerActionContext, SceneContext, SecretMotivationEntry,
    TriggerHints,
};
use crate::prompt_templates::{
    keys::DIALOGUE_RESPONSE_FORMAT,
};
use crate::queue_types::{LlmRequestData, LlmRequestType, PlayerActionData};

use crate::infrastructure::ports::{
    ChallengeRepo, CharacterRepo, LlmPort, LocationRepo, NarrativeRepo, PlayerCharacterRepo,
    PromptTemplateRepo, QueuePort, RepoError, SceneRepo, SettingsRepo, WorldRepo,
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

/// Dependencies for player action processing phase.
///
/// This struct encapsulates all repositories and services needed to build
/// prompts from player actions. It reduces coupling by only exposing the
/// dependencies required for this specific phase.
#[derive(Clone)]
pub struct PlayerActionContextDeps {
    /// Queue port for dequeuing and marking items
    pub queue: Arc<dyn QueuePort>,
    /// Character repository for NPC data
    pub character: Arc<dyn CharacterRepo>,
    /// Player character repository for PC data
    pub player_character: Arc<dyn PlayerCharacterRepo>,
    /// Scene repository for current scene context
    pub scene: Arc<dyn SceneRepo>,
    /// World repository for game time
    pub world: Arc<dyn WorldRepo>,
    /// Location repository for location names
    pub location: Arc<dyn LocationRepo>,
    /// Challenge repository for active challenges
    pub challenge: Arc<dyn ChallengeRepo>,
    /// Narrative operations for events and conversation history
    pub narrative_ops: Arc<crate::use_cases::narrative_operations::NarrativeOps>,
    /// Settings repository for temperature resolution
    pub settings: Arc<dyn SettingsRepo>,
}

/// Resolve temperature using the standard precedence chain.
///
/// Temperature precedence:
/// 1. Template front matter temperature (if provided)
/// 2. Per-world settings temperature (via getter closure)
/// 3. Global settings temperature (via getter closure)
/// 4. Fallback default value
async fn resolve_temperature(
    template_temperature: Option<f32>,
    world_id: WorldId,
    settings: Arc<dyn SettingsRepo>,
    getter: fn(&crate::infrastructure::app_settings::AppSettings) -> f32,
    default: f32,
) -> f32 {
    if let Some(temp) = template_temperature {
        tracing::debug!(temperature = temp, "Using template front matter temperature");
        return temp;
    }

    match settings.get_for_world(world_id).await {
        Ok(Some(settings)) => return getter(&settings),
        Ok(None) => {
            // No world-specific settings, fall through to global
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                world_id = %world_id,
                operation = "resolve_temperature::get_for_world",
                "SettingsRepo lookup failed, falling back to global settings"
            );
        }
    }

    match settings.get_global().await {
        Ok(Some(settings)) => return getter(&settings),
        Ok(None) => {
            // No global settings, fall back to default
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                world_id = %world_id,
                operation = "resolve_temperature::get_global",
                "SettingsRepo global lookup failed, falling back to default temperature"
            );
        }
    }

    tracing::debug!(
        world_id = %world_id,
        default = default,
        "Using default temperature"
    );
    default
}

impl PlayerActionContextDeps {
    /// Create a new player action context dependencies container.
    pub fn new(
        queue: Arc<dyn QueuePort>,
        character: Arc<dyn CharacterRepo>,
        player_character: Arc<dyn PlayerCharacterRepo>,
        scene: Arc<dyn SceneRepo>,
        world: Arc<dyn WorldRepo>,
        location: Arc<dyn LocationRepo>,
        challenge: Arc<dyn ChallengeRepo>,
        narrative_ops: Arc<crate::use_cases::narrative_operations::NarrativeOps>,
        settings: Arc<dyn SettingsRepo>,
    ) -> Self {
        Self {
            queue,
            character,
            player_character,
            scene,
            world,
            location,
            challenge,
            narrative_ops,
            settings,
        }
    }

    /// Resolve temperature for NPC response generation.
    ///
    /// Temperature precedence:
    /// 1. Template front matter temperature (if provided)
    /// 2. AppSettings per-world temperature
    /// 3. AppSettings global temperature
    /// 4. Default (0.7)
    pub async fn resolve_temperature_npc_response(
        &self,
        world_id: WorldId,
        template_temperature: Option<f32>,
    ) -> f32 {
        resolve_temperature(
            template_temperature,
            world_id,
            Arc::clone(&self.settings),
            crate::infrastructure::app_settings::AppSettings::llm_temperature_npc_response,
            0.7,
        )
        .await
    }

    /// Helper to fetch active challenges for a world.
    pub async fn fetch_active_challenges(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<ActiveChallengeContext>, QueueError> {
        let challenges = self
            .challenge
            .list_for_world(world_id)
            .await
            .map_err(QueueError::Repo)?;

        Ok(challenges
            .into_iter()
            .filter(|c| c.active())
            .map(|c| {
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
            .collect())
    }

    /// Helper to fetch active narrative events for a world.
    pub async fn fetch_active_narrative_events(
        &self,
        world_id: WorldId,
        player_dialogue: Option<&str>,
    ) -> Result<Vec<ActiveNarrativeEventContext>, QueueError> {
        let dialogue_lower = player_dialogue.map(|d| d.to_lowercase());

        let events = self
            .narrative_ops
            .list_events(world_id)
            .await
            .map_err(QueueError::Repo)?;

        let mut events_with_scores: Vec<(ActiveNarrativeEventContext, usize, Vec<String>)> = events
            .into_iter()
            .filter(|e| e.is_active() && (!e.is_triggered() || e.is_repeatable()))
            .map(|e| {
                let mut trigger_hints = TriggerHints::default();
                let mut all_keywords: Vec<String> = vec![];

                for tc in e.trigger_conditions() {
                    match &tc.trigger_type {
                        wrldbldr_domain::NarrativeTriggerType::DialogueTopic {
                            keywords, ..
                        } => {
                            trigger_hints.pc_mentions.extend(keywords.clone());
                            all_keywords.extend(keywords.clone());
                        }
                        wrldbldr_domain::NarrativeTriggerType::NpcAction {
                            action_keywords,
                            ..
                        } => {
                            let hint = format!("NPC action: {}", action_keywords.join(", "));
                            trigger_hints.custom.push(hint);
                            all_keywords.extend(action_keywords.clone());
                        }
                        wrldbldr_domain::NarrativeTriggerType::Custom { description, .. } => {
                            trigger_hints.custom.push(description.clone());
                        }
                        wrldbldr_domain::NarrativeTriggerType::HasItem { item_name, .. } => {
                            let hint = format!("PC has item: {}", item_name);
                            trigger_hints.custom.push(hint);
                        }
                        wrldbldr_domain::NarrativeTriggerType::FlagSet { flag_name } => {
                            let hint = format!("flag set: {}", flag_name);
                            trigger_hints.custom.push(hint);
                        }
                        _ => {}
                    }
                }

                let (score, matched_keywords) = dialogue_lower.as_ref().map_or(
                    (0, vec![]),
                    |dialogue| {
                        let mut score = 0usize;
                        let mut matched = vec![];
                        for kw in &all_keywords {
                            let kw_lower = kw.to_lowercase();
                            if dialogue.contains(&kw_lower) {
                                score += 1;
                                if !matched.contains(kw) {
                                    matched.push(kw.clone());
                                }
                            }
                        }
                        (score, matched)
                    },
                );

                let ctx = ActiveNarrativeEventContext {
                    id: e.id().to_string(),
                    name: e.name().to_string(),
                    description: e.description().to_string(),
                    scene_direction: e.scene_direction().to_string(),
                    trigger_hints,
                    featured_npc_names: vec![],
                    priority: e.priority(),
                };

                (ctx, score, matched_keywords)
            })
            .collect();

        events_with_scores.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(events_with_scores
            .into_iter()
            .map(|(mut ctx, _, matched)| {
                if !matched.is_empty() {
                    ctx.trigger_hints
                        .custom
                        .push(format!("[MATCHED_KEYWORDS: {}]", matched.join(", ")));
                }
                ctx
            })
            .collect())
    }
}

/// Dependencies for LLM request processing phase.
///
/// This struct encapsulates all repositories and services needed to execute
/// LLM requests and process responses. It reduces coupling by only exposing
/// the dependencies required for this specific phase.
#[derive(Clone)]
pub struct LlmRequestContextDeps {
    /// Queue port for dequeuing, marking complete, and enqueuing approvals
    pub queue: Arc<dyn QueuePort>,
    /// Challenge repository for enrichment
    pub challenge: Arc<dyn ChallengeRepo>,
    /// Narrative repository for event enrichment
    pub narrative: Arc<dyn NarrativeRepo>,
    /// Settings repository for temperature resolution
    pub settings: Arc<dyn SettingsRepo>,
}

impl LlmRequestContextDeps {
    /// Create a new LLM request context dependencies container.
    pub fn new(
        queue: Arc<dyn QueuePort>,
        challenge: Arc<dyn ChallengeRepo>,
        narrative: Arc<dyn NarrativeRepo>,
        settings: Arc<dyn SettingsRepo>,
    ) -> Self {
        Self {
            queue,
            challenge,
            narrative,
            settings,
        }
    }

    /// Resolve temperature for generic content suggestions.
    pub async fn resolve_temperature_suggestion(&self, world_id: WorldId) -> f32 {
        resolve_temperature(
            None,
            world_id,
            Arc::clone(&self.settings),
            crate::infrastructure::app_settings::AppSettings::llm_temperature_suggestion,
            0.8,
        )
        .await
    }

    /// Resolve temperature for outcome suggestions.
    pub async fn resolve_temperature_outcome_suggestion(&self, world_id: WorldId) -> f32 {
        resolve_temperature(
            None,
            world_id,
            Arc::clone(&self.settings),
            crate::infrastructure::app_settings::AppSettings::llm_temperature_outcome_suggestion,
            0.8,
        )
        .await
    }

    /// Resolve temperature for NPC response generation.
    pub async fn resolve_temperature_npc_response(
        &self,
        world_id: WorldId,
        template_temperature: Option<f32>,
    ) -> f32 {
        resolve_temperature(
            template_temperature,
            world_id,
            Arc::clone(&self.settings),
            crate::infrastructure::app_settings::AppSettings::llm_temperature_npc_response,
            0.7,
        )
        .await
    }
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
    deps: PlayerActionContextDeps,
    prompt_templates: Arc<dyn PromptTemplateRepo>,
}

impl ProcessPlayerAction {
    /// Create a new ProcessPlayerAction with phase-specific dependencies.
    pub fn new(deps: PlayerActionContextDeps, prompt_templates: Arc<dyn PromptTemplateRepo>) -> Self {
        Self { deps, prompt_templates }
    }

    /// Process the next player action in the queue.
    ///
    /// Returns None if the queue is empty.
    pub async fn execute(&self) -> Result<Option<PlayerActionProcessed>, QueueError> {
        // Dequeue the next player action
        let item = match self.deps.queue.dequeue_player_action().await? {
            Some(item) => item,
            None => return Ok(None),
        };

        // Extract the action data
        let action_data = match item.data {
            crate::infrastructure::ports::QueueItemData::PlayerAction(data) => data,
            _ => {
                // Wrong type - mark as failed and return
                self.deps
                    .queue
                    .mark_failed(item.id, "Invalid queue item type")
                    .await?;
                return Err(QueueError::InvalidItemType);
            }
        };

        // Build the prompt with character context
        // Fail-fast: repository errors during prompt building are treated as hard errors
        // because missing context data can materially change LLM decisions
        let prompt = match self.build_prompt(&action_data).await {
            Ok(prompt) => prompt,
            Err(e) => {
                tracing::error!(error = %e, "Failed to build prompt, marking queue item as failed");
                let message = format!("Prompt building failed: {}", e);
                self.deps.queue.mark_failed(item.id, &message).await?;
                return Err(e);
            }
        };

        // Resolve template temperature from front matter (if available)
        let template_temp = self
            .prompt_templates
            .resolve_template_with_metadata(Some(action_data.world_id), DIALOGUE_RESPONSE_FORMAT)
            .await
            .ok()
            .flatten()
            .and_then(|t| t.temperature);

        // Resolve temperature with precedence: template -> settings -> default
        let temperature = self
            .deps
            .resolve_temperature_npc_response(action_data.world_id, template_temp)
            .await;

        // Initialize multi-phase pipeline metadata
        let phase_meta = self.build_phase_metadata(&prompt, action_data.world_id, item.id).await;

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
            temperature: Some(temperature),
            phase_meta,
        };

        // Enqueue the LLM request
        // On failure, mark the player action as failed to prevent it from being stuck in "processing"
        let llm_request_id = match self.deps.queue.enqueue_llm_request(&llm_request).await {
            Ok(id) => id,
            Err(e) => {
                let message = format!("Failed to enqueue LLM request: {}", e);
                tracing::error!(error = %e, item_id = %item.id, "enqueue_llm_request failed, marking player action as failed");
                self.deps.queue.mark_failed(item.id, &message).await?;
                return Err(QueueError::EnqueueFailed { message });
            }
        };

        // Mark the player action as complete
        self.deps.queue.mark_complete(item.id).await?;

        Ok(Some(PlayerActionProcessed {
            action_id: item.id.to_uuid(),
            llm_request_id: llm_request_id.to_uuid(),
        }))
    }

    /// Build a full GamePromptRequest with character context from the database.
    async fn build_prompt(
        &self,
        action_data: &PlayerActionData,
    ) -> Result<GamePromptRequest, QueueError> {
        let pc = if let Some(pc_id) = action_data.pc_id {
            self.deps.player_character.get(pc_id).await?
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
            Some(id) => self.deps.character.get(id).await?.clone(),
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
        let current_scene = self.deps.scene.get_current(action_data.world_id).await?;
        let game_time = self
            .deps
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
            self.deps
                .location
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
            (Some(npc), Some(pc)) => match self.deps.character.get_disposition(npc, pc).await {
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
            Some(npc) => match self.deps.character.get_wants(npc).await {
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
        // Note: Conversation history is treated as best-effort because it is enrichment
        // data that should not block the core player action processing. A warning is logged
        // when the fetch fails so operators can detect data quality issues.
        let conversation_history = match (action_data.pc_id, npc_id) {
            (Some(pc_id), Some(npc_id)) => self
                .deps
                .narrative_ops
                .get_conversation_turns(pc_id, npc_id, 20)
                .await
                .map_err(|e| {
                    tracing::warn!(
                        error = %e,
                        pc_id = %pc_id,
                        npc_id = %npc_id,
                        "Failed to fetch conversation history, proceeding without context"
                    );
                    e
                })
                .unwrap_or_default(),
            _ => vec![],
        };

        // Fetch active challenges for this world
        // Fail-fast: challenge data is critical for LLM decision quality
        let active_challenges = self
            .deps
            .fetch_active_challenges(action_data.world_id)
            .await?;

        // Fetch active narrative events for this world
        // Pass player dialogue for relevance scoring and keyword matching
        // Fail-fast: event data is critical for LLM decision quality
        let player_dialogue = action_data.dialogue.as_deref();
        let active_narrative_events: Vec<ActiveNarrativeEventContext> = self
            .deps
            .fetch_active_narrative_events(action_data.world_id, player_dialogue)
            .await?;

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

    /// Build phase metadata for multi-phase LLM pipeline initialization.
    ///
    /// Creates Phase 0 context digest (deterministic) and sets up the pipeline
    /// to start at Phase 1 (ChallengeTriggers).
    async fn build_phase_metadata(
        &self,
        prompt: &GamePromptRequest,
        world_id: WorldId,
        action_item_id: wrldbldr_domain::QueueItemId,
    ) -> Option<crate::queue_types::PhaseMetadata> {
        use crate::queue_types::{LlmPhase, PhaseOutputs};
        use crate::use_cases::queues::phases::{build_context_digest, generate_phase_key};
        use sha2::{Digest, Sha256};

        // Build Phase 0 context digest (deterministic)
        let phase0_context = build_context_digest(prompt);

        // Build initial PhaseOutputs with just the context digest
        let phase_outputs = PhaseOutputs {
            phase0_context: Some(phase0_context),
            phase1_challenges: None,
            phase2_events: None,
            phase3_tools: None,
            phase4_behavior: None,
            phase5_response: None,
        };

        // Serialize phase outputs to JSON
        let phase_outputs_json = match serde_json::to_string(&phase_outputs) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to serialize phase outputs, falling back to single-phase");
                return None;
            }
        };

        // Build base request for phase key generation
        let base_request = crate::queue_types::LlmRequestData {
            request_type: crate::queue_types::LlmRequestType::NpcResponse { action_item_id },
            world_id,
            pc_id: None,
            prompt: Some(prompt.clone()),
            suggestion_context: None,
            callback_id: action_item_id.to_string(),
            conversation_id: None,
            temperature: None,
            phase_meta: None,
        };

        // Generate phase key for Phase 1
        // Include Phase 0 outputs in the key so retries with different context produce different keys
        let phase_key = generate_phase_key(LlmPhase::ChallengeTriggers, &base_request, prompt, Some(&phase_outputs));

        // Compute template version (short hash of resolved template text)
        let template_version = match self
            .prompt_templates
            .resolve_template(Some(world_id), LlmPhase::ChallengeTriggers.prompt_template_key())
            .await
        {
            Ok(value) => value
                .map(|text| {
                    let mut hasher = Sha256::new();
                    hasher.update(text.as_bytes());
                    format!("{:.8}", hex::encode(hasher.finalize()))
                })
                .unwrap_or_else(|| "default".to_string()),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    world_id = %world_id,
                    template_key = %LlmPhase::ChallengeTriggers.prompt_template_key(),
                    "Failed to resolve Phase 1 template; using default version"
                );
                "default".to_string()
            }
        };

        Some(crate::queue_types::PhaseMetadata {
            current_phase: LlmPhase::ChallengeTriggers,
            phase_key,
            phase_outputs_json: Some(phase_outputs_json),
            phase_version: "1.0".to_string(),
            template_version,
        })
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
    deps: LlmRequestContextDeps,
    llm: Arc<dyn LlmPort>,
    prompt_templates: Arc<dyn PromptTemplateRepo>,
    resolve_model: Arc<crate::use_cases::ai::ResolveLlmModel>,
}

impl ProcessLlmRequest {
    /// Create a new ProcessLlmRequest with phase-specific dependencies.
    pub fn new(
        deps: LlmRequestContextDeps,
        llm: Arc<dyn LlmPort>,
        prompt_templates: Arc<dyn PromptTemplateRepo>,
        resolve_model: Arc<crate::use_cases::ai::ResolveLlmModel>,
    ) -> Self {
        Self {
            deps,
            llm,
            prompt_templates,
            resolve_model,
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
        let item = match self.deps.queue.dequeue_llm_request().await? {
            Some(item) => item,
            None => return Ok(None),
        };

        // Extract the request data
        let request_data = match &item.data {
            crate::infrastructure::ports::QueueItemData::LlmRequest(data) => data.clone(),
            _ => {
                self.deps
                    .queue
                    .mark_failed(item.id, "Invalid queue item type")
                    .await?;
                return Err(QueueError::InvalidItemType);
            }
        };

        // Handle different request types
        let result = match &request_data.request_type {
            LlmRequestType::OutcomeSuggestion {
                resolution_id,
                world_id,
                challenge_name,
                current_description,
                guidance,
            } => {
                // Load system prompt from template repository with metadata (includes temperature)
                let template_result = self
                    .prompt_templates
                    .resolve_template_with_metadata(Some(*world_id), crate::prompt_templates::keys::SYSTEM_OUTCOME_SUGGESTIONS)
                    .await
                    .ok()
                    .flatten();

                let system_prompt = template_result
                    .as_ref()
                    .map(|t| t.text.clone())
                    .unwrap_or_else(|| {
                        // Fallback if template not found
                        "You are a creative TTRPG game master assistant. \
                         Generate 3 alternative narrative descriptions for a challenge outcome. \
                         Each suggestion should be evocative and fit the fantasy setting. \
                         Return each suggestion on a separate line, numbered 1-3.".to_string()
                    });

                let user_message = format!(
                    "Challenge: {}\nCurrent outcome description: \"{}\"\n{}Generate 3 alternative descriptions.",
                    challenge_name,
                    current_description,
                    guidance.as_ref().map(|g| format!("DM guidance: {}\n", g)).unwrap_or_default()
                );

                // Resolve the model for this request
                let resolve_input = crate::use_cases::ai::ResolveModelInput {
                    world_id: Some(*world_id),
                    env_default_override: None,
                };
                let resolved_model = match self.resolve_model.execute(resolve_input).await {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to resolve model, using default");
                        crate::use_cases::ai::ResolvedModel {
                            model_id: "mlx-community/gpt-oss-20b-MXFP4-Q8".to_string(),
                            source: crate::use_cases::ai::ModelSource::EnvDefault,
                            warning: Some(format!("Model resolution failed: {}", e)),
                        }
                    }
                };

                // Use persisted temperature, prefer template temperature over settings
                let temperature = match request_data.temperature {
                    Some(t) => t,
                    None => {
                        // Prefer template front matter temperature, fallback to settings
                        match template_result.as_ref().and_then(|t| t.temperature) {
                            Some(template_temp) => template_temp,
                            None => self.deps.resolve_temperature_outcome_suggestion(*world_id).await,
                        }
                    }
                };

                let llm_request = crate::infrastructure::ports::LlmRequest::new(vec![
                    crate::infrastructure::ports::ChatMessage::user(&user_message),
                ])
                .with_system_prompt(system_prompt.to_string())
                .with_temperature(temperature)
                .with_model(resolved_model.model_id);

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
                    resolution_id: resolution_id.to_uuid(),
                    suggestions: suggestions.clone(),
                };

                // Mark the LLM request as complete
                self.deps.queue.mark_complete(item.id).await?;

                Ok(Some(LlmRequestProcessed {
                    request_id: item.id.to_uuid(),
                    approval_id: resolution_id.to_uuid(), // Convert ApprovalRequestId to Uuid for tracking
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

                // Resolve the model for this request
                let resolve_input = crate::use_cases::ai::ResolveModelInput {
                    world_id: Some(world_id),
                    env_default_override: None,
                };
                let resolved_model = match self.resolve_model.execute(resolve_input).await {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to resolve model, using default");
                        crate::use_cases::ai::ResolvedModel {
                            model_id: "mlx-community/gpt-oss-20b-MXFP4-Q8".to_string(),
                            source: crate::use_cases::ai::ModelSource::EnvDefault,
                            warning: Some(format!("Model resolution failed: {}", e)),
                        }
                    }
                };

                // Use persisted temperature or resolve from settings
                let temperature = match request_data.temperature {
                    Some(t) => t,
                    None => self.deps.resolve_temperature_suggestion(world_id).await,
                };

                // Load system prompt from template repository
                let system_prompt = match self
                    .prompt_templates
                    .resolve_template(
                        Some(world_id),
                        crate::prompt_templates::keys::SYSTEM_GENERIC_SUGGESTION,
                    )
                    .await
                {
                    Ok(value) => value.unwrap_or_else(|| {
                        "You are a helpful worldbuilding assistant. Return only suggestions, one per line.".to_string()
                    }),
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            world_id = %world_id,
                            template_key = %crate::prompt_templates::keys::SYSTEM_GENERIC_SUGGESTION,
                            "Failed to resolve suggestion system prompt; using default"
                        );
                        "You are a helpful worldbuilding assistant. Return only suggestions, one per line.".to_string()
                    }
                };

                let llm_request = crate::infrastructure::ports::LlmRequest::new(vec![
                    crate::infrastructure::ports::ChatMessage::user(&prompt),
                ])
                .with_system_prompt(system_prompt)
                .with_temperature(temperature)
                .with_model(resolved_model.model_id);

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
                    .deps
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
                self.deps.queue.mark_complete(item.id).await?;

                // Return only the completion event (progress was already emitted via callback)
                Ok(Some(LlmRequestProcessed {
                    request_id: item.id.to_uuid(),
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
                // Check if this is a phased pipeline request
                if let Some(ref phase_meta) = request_data.phase_meta {
                    // Multi-phase pipeline execution
                    return self.execute_phased_npc_response(&request_data, phase_meta, item.id).await;
                }

                // Legacy single-shot execution (preserve backward compatibility)
                return self.execute_legacy_npc_response(&request_data, item.id).await;
            }

        };

        if let Err(ref e) = result {
            let message = format!("{}", e);
            if let Err(mark_err) = self.deps.queue.mark_failed(item.id, &message).await {
                tracing::warn!(
                    error = %mark_err,
                    item_id = %item.id,
                    "Failed to mark queue item as failed after error"
                );
            }
        }

        result
    }

    /// Enrich a raw challenge suggestion with challenge metadata from the database.
    ///
    /// If an aliaser is provided, the challenge_id will be dealiased first (e.g., CHAL_0 -> UUID).
    /// This supports both aliased IDs from LLM responses and raw UUIDs for backward compatibility.
    async fn enrich_challenge_suggestion(
        &self,
        raw: &response_parser::RawChallengeSuggestion,
        pc_id: Option<wrldbldr_domain::PlayerCharacterId>,
        aliaser: Option<&UuidAliaser>,
    ) -> Option<crate::queue_types::ChallengeSuggestion> {
        // Dealias the challenge ID if an aliaser is provided (CHAL_0 -> UUID)
        // Falls back to raw ID if not aliased or no aliaser provided
        let challenge_id_str = aliaser
            .and_then(|a| a.dealias(&raw.challenge_id))
            .unwrap_or_else(|| raw.challenge_id.clone());

        // Parse the challenge ID
        let challenge_id: wrldbldr_domain::ChallengeId = try_parse_typed_id(&challenge_id_str)?;

        // Fetch challenge from database
        let challenge = match self.deps.challenge.get(challenge_id).await {
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
            // Store the typed ID, not the string, for type safety
            challenge_id,
            challenge_name: challenge.name().to_string(),
            skill_name: challenge
                .check_stat()
                .map_or(String::new(), |s| s.to_string()),
            difficulty_display: challenge.difficulty().display(),
            confidence: raw.confidence.clone(),
            reasoning: raw.reasoning.clone(),
            target_pc_id: pc_id,
            outcomes: None, // Outcomes can be populated later if needed
        })
    }

    /// Enrich a raw narrative event suggestion with event metadata from the database.
    ///
    /// If an aliaser is provided, the event_id will be dealiased first (e.g., EVT_0 -> UUID).
    /// This supports both aliased IDs from LLM responses and raw UUIDs for backward compatibility.
    async fn enrich_narrative_event_suggestion(
        &self,
        raw: &response_parser::RawNarrativeEventSuggestion,
        aliaser: Option<&UuidAliaser>,
    ) -> Option<crate::queue_types::NarrativeEventSuggestion> {
        // Dealias the event ID if an aliaser is provided (EVT_0 -> UUID)
        // Falls back to raw ID if not aliased or no aliaser provided
        let event_id_str = aliaser
            .and_then(|a| a.dealias(&raw.event_id))
            .unwrap_or_else(|| raw.event_id.clone());

        // Parse the event ID
        let event_id: wrldbldr_domain::NarrativeEventId = try_parse_typed_id(&event_id_str)?;

        // Fetch event from database
        let event = match self.deps.narrative.get_event(event_id).await {
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
            // Store the typed ID, not string, for type safety
            event_id,
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
        let events = match self.deps.narrative.list_events_for_world(world_id).await {
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
            event_id: event.id(),
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
        let challenges = match self.deps.challenge.list_for_world(world_id).await {
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
            challenge_id: challenge.id(),
            challenge_name: challenge.name().to_string(),
            skill_name: challenge
                .check_stat()
                .map_or(String::new(), |s| s.to_string()),
            difficulty_display: challenge.difficulty().display(),
            confidence: "high".to_string(), // Challenge tag means LLM determined it triggered
            reasoning: "Player dialogue matched challenge trigger keywords".to_string(),
            target_pc_id: pc_id,
            outcomes: None,
        })
    }

    /// Execute phased NPC response pipeline.
    ///
    /// Processes the current phase, updates phase outputs, and either:
    /// - Enqueues the next phase if not at FinalResponse
    /// - Creates ApprovalRequestData if at FinalResponse
    async fn execute_phased_npc_response(
        &self,
        request_data: &LlmRequestData,
        phase_meta: &crate::queue_types::PhaseMetadata,
        item_id: wrldbldr_domain::QueueItemId,
    ) -> Result<Option<LlmRequestProcessed>, QueueError> {
        use crate::queue_types::{
            LlmPhase, PhaseOutputs,
        };
        use crate::use_cases::queues::phases::{
            build_phase_prompt, generate_phase_key, parsers,
        };
        use crate::infrastructure::ports::PhaseFingerprintMeta;
        use sha2::{Digest, Sha256};

        let prompt = request_data
            .prompt
            .as_ref()
            .ok_or_else(|| QueueError::Parse(QueueParseError::missing_field("prompt")))?;

        // Deserialize phase outputs
        let mut phase_outputs: PhaseOutputs = match phase_meta.phase_outputs_json {
            Some(ref json) => match serde_json::from_str(json) {
                Ok(parsed) => parsed,
                Err(e) => {
                    let message = format!(
                        "Failed to deserialize phase outputs for {}: {}",
                        phase_meta.phase_key, e
                    );
                    self.deps.queue.mark_failed(item_id, &message).await?;
                    return Err(QueueError::Parse(QueueParseError::deserialize(message)));
                }
            },
            None => PhaseOutputs::default(),
        };

        let current_phase = phase_meta.current_phase;

        tracing::info!(
            phase = ?current_phase,
            phase_key = %phase_meta.phase_key,
            "Executing phased NPC response"
        );

        // Build phase-specific prompt
        let phase_prompt = build_phase_prompt(current_phase, prompt, &phase_outputs);

        // Resolve template (with metadata) for this phase
        let template_key = current_phase.prompt_template_key();
        let resolved_template = match self
            .prompt_templates
            .resolve_template_with_metadata(Some(request_data.world_id), template_key)
            .await
        {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    world_id = %request_data.world_id,
                    template_key,
                    "Failed to resolve phase template; using defaults"
                );
                None
            }
        };
        let template_text = resolved_template.as_ref().map(|t| t.text.clone());
        let template_temperature = resolved_template.as_ref().and_then(|t| t.temperature);

        // Compute template version
        let template_version = template_text
            .as_ref()
            .map(|text| {
                let mut hasher = Sha256::new();
                hasher.update(text.as_bytes());
                format!("{:.8}", hex::encode(hasher.finalize()))
            })
            .unwrap_or_else(|| "default".to_string());

        // Build system prompt from template or use default
        let system_prompt = template_text.unwrap_or_else(|| {
            match current_phase {
                LlmPhase::ChallengeTriggers => "Analyze player dialogue for challenge triggers.".to_string(),
                LlmPhase::EventTriggers => "Analyze player dialogue for narrative event triggers.".to_string(),
                LlmPhase::ToolIntent => "Determine appropriate game tools based on context.".to_string(),
                LlmPhase::BehaviorAnalysis => "Analyze NPC behavioral stance.".to_string(),
                LlmPhase::FinalResponse => "Generate NPC dialogue response.".to_string(),
                LlmPhase::ContextDigest => "Generate context digest.".to_string(),
            }
        });

        // Resolve temperature: use persisted value or resolve from settings
        let temperature = match request_data.temperature {
            Some(t) => t,
            None => {
                self.deps
                    .resolve_temperature_npc_response(request_data.world_id, template_temperature)
                    .await
            }
        };

        // Build LLM request with fingerprint metadata for VCR
        let fingerprint_meta = PhaseFingerprintMeta {
            phase_id: Some(current_phase.short_id().to_string()),
            template_key: Some(template_key.to_string()),
            template_version: Some(template_version.clone()),
        };

        let llm_request = crate::infrastructure::ports::LlmRequest::new(vec![
            crate::infrastructure::ports::ChatMessage::user(&phase_prompt),
        ])
        .with_system_prompt(system_prompt)
        .with_temperature(temperature)
        .with_max_tokens(Some(2048))
        .with_fingerprint_meta(fingerprint_meta);

        // Resolve model
        let resolve_input = crate::use_cases::ai::ResolveModelInput {
            world_id: Some(request_data.world_id),
            env_default_override: None,
        };
        let resolved_model = match self.resolve_model.execute(resolve_input).await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to resolve model, using default");
                crate::use_cases::ai::ResolvedModel {
                    model_id: "mlx-community/gpt-oss-20b-MXFP4-Q8".to_string(),
                    source: crate::use_cases::ai::ModelSource::EnvDefault,
                    warning: Some(format!("Model resolution failed: {}", e)),
                }
            }
        };

        let llm_request = llm_request.with_model(resolved_model.model_id);

        // Call LLM
        let llm_response = self.llm.generate(llm_request).await.map_err(|e| {
            tracing::error!(
                phase = ?current_phase,
                world_id = %request_data.world_id,
                error = %e,
                "Phased LLM generation failed"
            );
            e
        })?;

        // Parse response based on phase
        let raw_response = llm_response.content;

        match current_phase {
            LlmPhase::ChallengeTriggers => {
                let challenge_order: Vec<String> = prompt
                    .active_challenges
                    .iter()
                    .map(|c| c.id.clone())
                    .collect();
                let parsed = match parsers::parse_phase1_challenges(
                    &raw_response,
                    &challenge_order,
                ) {
                    Ok(value) => value,
                    Err(e) => {
                        let err = QueueParseError::phase(1, e);
                        let message = format!("{}", err);
                        self.deps.queue.mark_failed(item_id, &message).await?;
                        return Err(QueueError::Parse(err));
                    }
                };
                phase_outputs.phase1_challenges = Some(parsed);
            }
            LlmPhase::EventTriggers => {
                let event_order: Vec<String> = prompt
                    .active_narrative_events
                    .iter()
                    .map(|e| e.id.clone())
                    .collect();
                let parsed = match parsers::parse_phase2_events(
                    &raw_response,
                    &event_order,
                ) {
                    Ok(value) => value,
                    Err(e) => {
                        let err = QueueParseError::phase(2, e);
                        let message = format!("{}", err);
                        self.deps.queue.mark_failed(item_id, &message).await?;
                        return Err(QueueError::Parse(err));
                    }
                };
                phase_outputs.phase2_events = Some(parsed);
            }
            LlmPhase::ToolIntent => {
                let parsed = match parsers::parse_phase3_tools(&raw_response) {
                    Ok(value) => value,
                    Err(e) => {
                        let err = QueueParseError::phase(3, e);
                        let message = format!("{}", err);
                        self.deps.queue.mark_failed(item_id, &message).await?;
                        return Err(QueueError::Parse(err));
                    }
                };
                phase_outputs.phase3_tools = Some(parsed);
            }
            LlmPhase::BehaviorAnalysis => {
                let parsed = match parsers::parse_phase4_behavior(&raw_response) {
                    Ok(value) => value,
                    Err(e) => {
                        let err = QueueParseError::phase(4, e);
                        let message = format!("{}", err);
                        self.deps.queue.mark_failed(item_id, &message).await?;
                        return Err(QueueError::Parse(err));
                    }
                };
                phase_outputs.phase4_behavior = Some(parsed);
            }
            LlmPhase::FinalResponse => {
                let parsed = match parsers::parse_phase5_response(&raw_response) {
                    Ok(value) => value,
                    Err(e) => {
                        let err = QueueParseError::phase(5, e);
                        let message = format!("{}", err);
                        self.deps.queue.mark_failed(item_id, &message).await?;
                        return Err(QueueError::Parse(err));
                    }
                };
                phase_outputs.phase5_response = Some(parsed);
            }
            LlmPhase::ContextDigest => {
                // Context digest is typically deterministic, but handle LLM-based if needed
                let parsed = parsers::parse_phase0_digest(&raw_response)
                    .unwrap_or_else(|| crate::use_cases::queues::phases::build_context_digest(prompt));
                phase_outputs.phase0_context = Some(parsed);
            }
        }

        // Check if there's a next phase
        if let Some(next_phase) = current_phase.next() {
            // Serialize updated phase outputs
            let phase_outputs_json = match serde_json::to_string(&phase_outputs) {
                Ok(json) => json,
                Err(e) => {
                    let err = QueueParseError::serialize(format!("Failed to serialize phase outputs: {}", e));
                    let message = format!("{}", err);
                    self.deps.queue.mark_failed(item_id, &message).await?;
                    return Err(QueueError::Parse(err));
                }
            };

            // Generate new phase key
            let base_request = crate::queue_types::LlmRequestData {
                request_type: request_data.request_type.clone(),
                world_id: request_data.world_id,
                pc_id: request_data.pc_id,
                prompt: request_data.prompt.clone(),
                suggestion_context: None,
                callback_id: request_data.callback_id.clone(),
                conversation_id: request_data.conversation_id,
                temperature: request_data.temperature,
                phase_meta: None,
            };
            // Generate phase key including the updated phase outputs
            // This ensures that if prior phases produce different outputs,
            // the next phase gets a different key (preventing stale prompts)
            let next_phase_key = generate_phase_key(next_phase, &base_request, prompt, Some(&phase_outputs));

            // Create next phase metadata
            let next_template_version = match self
                .prompt_templates
                .resolve_template_with_metadata(
                    Some(request_data.world_id),
                    next_phase.prompt_template_key(),
                )
                .await
            {
                Ok(value) => value
                    .map(|t| {
                        let mut hasher = Sha256::new();
                        hasher.update(t.text.as_bytes());
                        format!("{:.8}", hex::encode(hasher.finalize()))
                    })
                    .unwrap_or_else(|| "default".to_string()),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        world_id = %request_data.world_id,
                        template_key = %next_phase.prompt_template_key(),
                        "Failed to resolve next phase template; using default version"
                    );
                    "default".to_string()
                }
            };

            let next_phase_meta = crate::queue_types::PhaseMetadata {
                current_phase: next_phase,
                phase_key: next_phase_key.clone(),
                phase_outputs_json: Some(phase_outputs_json),
                phase_version: phase_meta.phase_version.clone(),
                template_version: next_template_version,
            };

            // Enqueue next phase atomically with marking current complete
            // This prevents duplicate phase execution if the operation is retried
            let next_request = crate::queue_types::LlmRequestData {
                request_type: request_data.request_type.clone(),
                world_id: request_data.world_id,
                pc_id: request_data.pc_id,
                prompt: request_data.prompt.clone(),
                suggestion_context: None,
                callback_id: request_data.callback_id.clone(),
                conversation_id: request_data.conversation_id,
                temperature: request_data.temperature,
                phase_meta: Some(next_phase_meta),
            };

            self.deps
                .queue
                .advance_phased_llm_request(item_id, &next_request, &next_phase_key)
                .await?;

            Ok(None) // No final result yet, more phases to process
        } else {
            // Final phase - create approval request using phase outputs
            self.create_approval_from_phase_outputs(
                request_data,
                &phase_outputs,
                item_id,
            ).await
        }
    }

    /// Create approval request from phase outputs.
    async fn create_approval_from_phase_outputs(
        &self,
        request_data: &LlmRequestData,
        phase_outputs: &crate::queue_types::PhaseOutputs,
        item_id: wrldbldr_domain::QueueItemId,
    ) -> Result<Option<LlmRequestProcessed>, QueueError> {
        use crate::queue_types::{ChallengeSuggestion, NarrativeEventSuggestion};

        let prompt = request_data
            .prompt
            .as_ref()
            .ok_or_else(|| QueueError::Parse(QueueParseError::missing_field("prompt")))?;

        // Extract data from phase outputs
        let phase5 = phase_outputs.phase5_response.as_ref().ok_or_else(|| {
            QueueError::Parse(QueueParseError::missing_field("Phase5FinalResponse"))
        })?;

        let phase3 = phase_outputs.phase3_tools.as_ref();
        let phase1 = phase_outputs.phase1_challenges.as_ref();
        let phase2 = phase_outputs.phase2_events.as_ref();

        // Get NPC info from prompt
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

        // Build challenge suggestion from phase1 if any challenge triggered
        let challenge_suggestion = if let Some(p1) = phase1 {
            p1.challenges
                .iter()
                .find(|c| c.should_trigger)
                .and_then(|triggered| {
                    // Find the matching challenge in prompt data
                    prompt
                        .active_challenges
                        .iter()
                        .find(|c| {
                            if !triggered.challenge_id.is_empty() {
                                c.id == triggered.challenge_id
                            } else {
                                c.name == triggered.challenge_name
                            }
                        })
                        .and_then(|challenge_ctx| {
                            try_parse_typed_id::<wrldbldr_domain::ChallengeId>(&challenge_ctx.id)
                                .map(|challenge_id| ChallengeSuggestion {
                                    challenge_id,
                                    challenge_name: challenge_ctx.name.clone(),
                                    skill_name: challenge_ctx.skill_name.clone(),
                                    difficulty_display: challenge_ctx.difficulty_display.clone(),
                                    confidence: triggered.confidence.clone(),
                                    reasoning: triggered.reasoning.clone(),
                                    target_pc_id: request_data.pc_id,
                                    outcomes: None,
                                })
                        })
                })
        } else {
            None
        };

        // Build narrative event suggestion from phase2 if an event was selected
        let narrative_event_suggestion = if let Some(p2) = phase2 {
            p2.events
                .iter()
                .find(|e| e.should_trigger)
                .and_then(|triggered| {
                    // Find the matching event in prompt data
                    prompt
                        .active_narrative_events
                        .iter()
                        .find(|e| {
                            if !triggered.event_id.is_empty() {
                                e.id == triggered.event_id
                            } else {
                                e.name == triggered.event_name
                            }
                        })
                        .and_then(|event_ctx| {
                            try_parse_typed_id::<wrldbldr_domain::NarrativeEventId>(&event_ctx.id)
                                .map(|event_id| NarrativeEventSuggestion {
                                    event_id,
                                    event_name: event_ctx.name.clone(),
                                    description: event_ctx.description.clone(),
                                    scene_direction: event_ctx.scene_direction.clone(),
                                    confidence: "high".to_string(),
                                    reasoning: "Selected by Phase 2 event trigger analysis".to_string(),
                                    matched_triggers: vec![],
                                    suggested_outcome: None,
                                })
                        })
                })
        } else {
            None
        };

        // Build proposed tools from phase3
        let mut proposed_tools = phase3
            .map(|p3| p3.proposed_tools.clone())
            .unwrap_or_default();

        let mut proposed_dialogue = phase5.dialogue.clone();
        let mut internal_reasoning = phase5.reasoning.clone();
        let mut topics = phase5.topics.clone();

        // Enforce deflection for secret motivations (phased path)
        if let Some(ref player_dialogue) = prompt.player_action.dialogue {
            if is_probing_secret_motivations(player_dialogue) {
                if let Some(ref motivations) = prompt.responding_character.motivations {
                    if !motivations.secret.is_empty() {
                        let deflection = select_deflection_behavior(motivations, player_dialogue);
                        if !deflection.is_empty() {
                            tracing::info!(
                                npc_name = %prompt.responding_character.name,
                                "Enforcing deflection behavior for secret motivation probe (phased path)"
                            );
                            proposed_dialogue = deflection;
                            internal_reasoning = "Deflection enforced to protect secret motivations.".to_string();
                            topics = vec!["deflection".to_string()];
                            proposed_tools.clear();
                        }
                    }
                }
            }
        }

        // Create approval request
        let approval_data = crate::queue_types::ApprovalRequestData {
            world_id: request_data.world_id,
            source_action_id: item_id,
            decision_type: crate::queue_types::ApprovalDecisionType::NpcResponse,
            urgency: crate::queue_types::ApprovalUrgency::AwaitingPlayer,
            pc_id: request_data.pc_id,
            npc_id,
            npc_name,
            proposed_dialogue: proposed_dialogue.clone(),
            internal_reasoning,
            proposed_tools,
            retry_count: 0,
            challenge_suggestion,
            narrative_event_suggestion,
            challenge_outcome: None,
            player_dialogue,
            scene_id,
            location_id,
            region_id: None,
            game_time,
            topics,
            conversation_id: request_data.conversation_id,
        };

        // Enqueue for DM approval
        let approval_id = self.deps.queue.enqueue_dm_approval(&approval_data).await?;

        // Mark the LLM request as complete
        self.deps.queue.mark_complete(item_id).await?;

        Ok(Some(LlmRequestProcessed {
            request_id: item_id.to_uuid(),
            approval_id: approval_id.to_uuid(),
            npc_dialogue: proposed_dialogue.clone(),
            broadcast_events: vec![],
        }))
    }

    /// Execute legacy single-shot NPC response (preserved for backward compatibility).
    async fn execute_legacy_npc_response(
        &self,
        request_data: &LlmRequestData,
        item_id: wrldbldr_domain::QueueItemId,
    ) -> Result<Option<LlmRequestProcessed>, QueueError> {
        // This is the original implementation, preserved for backward compatibility
        // Create UUID aliaser for this request
        let mut aliaser = UuidAliaser::new();

        // Extract world_id from request_data
        let world_id = request_data.world_id;

        // Build LLM request from the queued prompt data
        let llm_request = if let Some(ref prompt) = request_data.prompt {
            // Format active challenges
            let challenges_context = if prompt.active_challenges.is_empty() {
                String::new()
            } else {
                let challenge_list: Vec<String> = prompt
                    .active_challenges
                    .iter()
                    .map(|c| {
                        let aliased_id = aliaser.alias_challenge(&c.id);
                        format!(
                            "- {} [{}]: {} - {} {}{}",
                            c.name,
                            aliased_id,
                            c.description,
                            c.skill_name,
                            c.difficulty_display,
                            c.trigger_hints.format_for_prompt()
                        )
                    })
                    .collect();
                format!(
                    "\n\nACTIVE CHALLENGES:\n{}\n",
                    challenge_list.join("\n")
                )
            };

            // Format active narrative events
            let events_context = if prompt.active_narrative_events.is_empty() {
                String::new()
            } else {
                let event_list: Vec<String> = prompt
                    .active_narrative_events
                    .iter()
                    .map(|e| {
                        let aliased_id = aliaser.alias_event(&e.id);
                        format!(
                            "- {} [{}]: {}{}",
                            e.name,
                            aliased_id,
                            e.description,
                            e.trigger_hints.format_for_prompt()
                        )
                    })
                    .collect();
                format!(
                    "\n\nACTIVE NARRATIVE EVENTS:\n{}\n",
                    event_list.join("\n")
                )
            };

            // Resolve prompt templates
            let response_format = self
                .prompt_templates
                .resolve_template(Some(world_id), DIALOGUE_RESPONSE_FORMAT.trim())
                .await
                .map_err(QueueError::Repo)?
                .unwrap_or_default();

            let system_prompt = format!(
                "You are roleplaying as an NPC. {}\n\n\
                Scene: {} at {}\n\
                Present: {}\n\n\
                {}{}\n\n\
                {}",
                prompt.directorial_notes,
                prompt.scene_context.scene_name,
                prompt.scene_context.location_name,
                prompt.scene_context.present_characters.join(", "),
                challenges_context,
                events_context,
                response_format
            );

            let current_message = if let Some(ref dialogue) = prompt.player_action.dialogue {
                format!(
                    "The player says: \"{}\"",
                    dialogue
                )
            } else {
                format!(
                    "The player performs action: {}",
                    prompt.player_action.action_type
                )
            };

            let npc_name = &prompt.responding_character.name;
            let mut messages: Vec<crate::infrastructure::ports::ChatMessage> = prompt
                .conversation_history
                .iter()
                .map(|turn| {
                    if turn.speaker == *npc_name
                        || turn.speaker.to_lowercase() == npc_name.to_lowercase()
                    {
                        crate::infrastructure::ports::ChatMessage::assistant(&turn.text)
                    } else {
                        crate::infrastructure::ports::ChatMessage::user(&turn.text)
                    }
                })
                .collect();

            messages.push(crate::infrastructure::ports::ChatMessage::user(&current_message));

            let temperature = match request_data.temperature {
                Some(t) => t,
                None => self.deps.resolve_temperature_npc_response(world_id, None).await,
            };

            crate::infrastructure::ports::LlmRequest::new(messages)
                .with_system_prompt(system_prompt)
                .with_temperature(temperature)
                .with_max_tokens(Some(2048))
        } else {
            // Fallback
            let temperature = match request_data.temperature {
                Some(t) => t,
                None => self.deps.resolve_temperature_npc_response(world_id, None).await,
            };

            crate::infrastructure::ports::LlmRequest::new(vec![
                crate::infrastructure::ports::ChatMessage::user(
                    "Generate NPC response.",
                ),
            ])
            .with_system_prompt("You are an NPC in a fantasy TTRPG.")
            .with_temperature(temperature)
            .with_max_tokens(Some(512))
        };

        // Resolve model
        let resolve_input = crate::use_cases::ai::ResolveModelInput {
            world_id: Some(world_id),
            env_default_override: None,
        };
        let resolved_model = match self.resolve_model.execute(resolve_input).await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to resolve model, using default");
                crate::use_cases::ai::ResolvedModel {
                    model_id: "mlx-community/gpt-oss-20b-MXFP4-Q8".to_string(),
                    source: crate::use_cases::ai::ModelSource::EnvDefault,
                    warning: Some(format!("Model resolution failed: {}", e)),
                }
            }
        };

        let llm_request = llm_request.with_model(resolved_model.model_id);

        let llm_response = self.llm.generate(llm_request).await.map_err(|e| {
            tracing::error!(
                world_id = %world_id,
                error = %e,
                "Legacy LLM NPC response generation failed"
            );
            e
        })?;

        // Parse response
        // Fail-fast: parse errors are treated as hard errors because an unparseable
        // response cannot be used for DM approval. This is consistent with the
        // phased pipeline's strict parsing semantics.
        let mut parsed = match response_parser::parse_llm_response(&llm_response.content) {
            Ok(parsed) => parsed,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    world_id = %world_id,
                    "Legacy LLM response parsing failed"
                );
                let message = format!("Parse error: {}", e);
                self.deps.queue.mark_failed(item_id, &message).await?;
                return Err(QueueError::Parse(QueueParseError::other(message)));
            }
        };

        // Check deflection (only for legacy path - phased path handles this in Phase 4/5)
        if let Some(ref prompt) = request_data.prompt {
            if let Some(ref dialogue) = prompt.player_action.dialogue {
                if is_probing_secret_motivations(dialogue) {
                    if let Some(ref motivations) = prompt.responding_character.motivations {
                        if !motivations.secret.is_empty() {
                            let deflection = select_deflection_behavior(motivations, dialogue);
                            if !deflection.is_empty() {
                                tracing::info!(
                                    npc_name = %prompt.responding_character.name,
                                    "Enforcing deflection behavior for secret motivation probe (legacy path)"
                                );
                                parsed.dialogue = deflection;
                                parsed.proposed_tools.clear();
                            }
                        }
                    }
                }
            }
        }

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

        // Enrich suggestions
        let challenge_suggestion = if let Some(raw) = parsed.challenge_suggestion {
            self.enrich_challenge_suggestion(&raw, request_data.pc_id, Some(&aliaser))
                .await
        } else if let Some(triggered_name) = parsed.triggered_challenge_names.first() {
            self.create_challenge_suggestion_from_name(
                request_data.world_id,
                triggered_name,
                request_data.pc_id,
            )
            .await
        } else {
            None
        };

        let narrative_event_suggestion =
            if let Some(raw) = parsed.narrative_event_suggestion {
                self.enrich_narrative_event_suggestion(&raw, Some(&aliaser))
                    .await
            } else if let Some(triggered_name) = parsed.triggered_event_names.first() {
                self.create_narrative_event_suggestion_from_name(
                    request_data.world_id,
                    triggered_name,
                )
                .await
            } else {
                None
            };

        let npc_dialogue = parsed.dialogue.clone();

        let approval_data = crate::queue_types::ApprovalRequestData {
            world_id: request_data.world_id,
            source_action_id: item_id,
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
            region_id: None,
            game_time,
            topics: parsed.topics,
            conversation_id: request_data.conversation_id,
        };

        let approval_id = self.deps.queue.enqueue_dm_approval(&approval_data).await?;
        self.deps.queue.mark_complete(item_id).await?;

        Ok(Some(LlmRequestProcessed {
            request_id: item_id.to_uuid(),
            approval_id: approval_id.to_uuid(),
            npc_dialogue,
            broadcast_events: vec![],
        }))
    }
}

/// Build the SECRET MOTIVATIONS context block for the NPC system prompt.
///
/// This block instructs the LLM how to behave when players probe secret motivations,
/// including the exact deflection behavior to use and behavioral tells to avoid revealing.
fn build_secret_motivations_context(character: &CharacterContext) -> String {
    let Some(motivations) = &character.motivations else {
        return String::new();
    };

    if motivations.secret.is_empty() {
        return String::new();
    }

    let secrets: Vec<String> = motivations
        .secret
        .iter()
        .map(|s| {
            let tells = if s.tells.is_empty() {
                String::new()
            } else {
                format!("\n  Behavioral tells to AVOID displaying: {}", s.tells.join("; "))
            };
            format!(
                "- Secret: {}\n  Deflection behavior (USE VERBATIM if probed): \"{}\"{}",
                s.description, s.deflection_behavior, tells
            )
        })
        .collect();

    format!(
        "\n\nSECRET MOTIVATIONS (CRITICAL - DO NOT REVEAL):\n\
        The following motivations are hidden from players.\n\n\
        DEFLECTION RULES (MUST FOLLOW - THIS IS MANDATORY):\n\
        - PROBING QUESTIONS include: asking what you're hiding, if you have secrets, why you're acting strange, \"what's wrong\", or direct accusations\n\
        - When probed, your ENTIRE dialogue response MUST be the deflection behavior verbatim - NOTHING else\n\
        - Do NOT add narration, do NOT explain yourself, do NOT answer the question first then deflect\n\
        - NEVER say 'I'm not hiding anything', 'I don't know what you mean', or deny having secrets\n\
        - NEVER acknowledge that you are hiding something\n\
        - Simply redirect to a different topic using the deflection behavior verbatim\n\
        - Do NOT reveal any secret details\n\n{}",
        secrets.join("\n")
    )
}

/// Check if the player's dialogue is probing secret motivations.
///
/// This detects direct questions about hiding things, secrets, or suspicious behavior.
fn is_probing_secret_motivations(player_dialogue: &str) -> bool {
    let lower = player_dialogue.to_lowercase();
    let probe_patterns = [
        "what are you hiding",
        "what're you hiding",
        "what do you hide",
        "are you hiding",
        "you seem troubled",
        "you seem worried",
        "you seem nervous",
        "what's wrong",
        "whats wrong",
        "what is wrong",
        "do you have secrets",
        "what secrets",
        "are you keeping secrets",
        "what aren't you telling",
        "what are you not telling",
        "why are you acting",
        "why do you seem",
        "what's bothering you",
        "whats bothering you",
    ];
    probe_patterns.iter().any(|pattern| lower.contains(pattern))
}

/// Select the appropriate deflection behavior based on the player's probe.
///
/// Returns the deflection behavior from the first secret motivation, or empty string if none.
fn select_deflection_behavior(
    motivations: &MotivationsContext,
    _player_dialogue: &str,
) -> String {
    // For now, use the first secret's deflection behavior
    // In the future, could match against specific secrets based on probe content
    motivations
        .secret
        .first()
        .map(|s| s.deflection_behavior.clone())
        .unwrap_or_default()
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
        .map_err(|e| QueueError::Parse(QueueParseError::other(format!("Invalid UUID for {}: {}", field, e))))
}

/// Try to parse a typed ID, returning None if the value is empty or invalid.
/// Use this for optional fields where failure means "no value" rather than "bad data".
fn try_parse_typed_id<T: From<Uuid>>(value: &str) -> Option<T> {
    if value.is_empty() {
        return None;
    }
    match Uuid::parse_str(value) {
        Ok(uuid) => Some(T::from(uuid)),
        Err(e) => {
            tracing::warn!(
                error = %e,
                value = %value,
                "Invalid UUID for optional field"
            );
            None
        }
    }
}

/// Errors that can occur during phase output parsing in the queue pipeline.
///
/// This is a wrapper around `PhaseParseError` that adds queue-specific context
/// such as the phase number and item ID where the error occurred.
#[derive(Debug, Clone, thiserror::Error)]
pub enum QueueParseError {
    /// A phase parser returned an error.
    #[error("Phase {phase} parse error: {source}")]
    Phase {
        phase: u8,
        #[source]
        source: crate::use_cases::queues::phases::PhaseParseError,
    },

    /// Failed to deserialize phase outputs from JSON.
    #[error("Failed to deserialize phase outputs: {message}")]
    Deserialize { message: String },

    /// Failed to serialize phase outputs to JSON.
    #[error("Failed to serialize phase outputs: {message}")]
    Serialize { message: String },

    /// Missing required field in phased request.
    #[error("Missing required field in phased request: {field}")]
    MissingField { field: String },

    /// Generic parse error with context.
    #[error("Parse error: {message}")]
    Other { message: String },
}

impl QueueParseError {
    /// Create a phase-specific parse error.
    pub fn phase(phase: u8, source: crate::use_cases::queues::phases::PhaseParseError) -> Self {
        Self::Phase { phase, source }
    }

    /// Create a deserialization error.
    pub fn deserialize(message: impl Into<String>) -> Self {
        Self::Deserialize {
            message: message.into(),
        }
    }

    /// Create a serialization error.
    pub fn serialize(message: impl Into<String>) -> Self {
        Self::Serialize {
            message: message.into(),
        }
    }

    /// Create a missing field error.
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }

    /// Create a generic parse error.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
        }
    }
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
    /// Parse error with structured context.
    #[error("Parse error: {0}")]
    Parse(#[from] QueueParseError),
    /// Failed to enqueue LLM request after player action was dequeued.
    /// The player action queue item has been marked as failed.
    #[error("Failed to enqueue LLM request: {message}")]
    EnqueueFailed { message: String },
}

// =============================================================================
// Legacy Parser Error Propagation Tests
// =============================================================================

#[cfg(test)]
mod legacy_parse_error_tests {
    use super::response_parser::{parse_llm_response, LegacyParseError};
    use super::{QueueError, QueueParseError};

    /// Test that parse errors can be converted to QueueError::Parse
    #[test]
    fn test_legacy_parse_error_to_queue_error() {
        let legacy_err = LegacyParseError::MissingDialogue;
        let queue_parse_err = QueueParseError::other(legacy_err.to_string());

        match &queue_parse_err {
            QueueParseError::Other { message } => {
                assert!(message.contains("dialogue"), "Error message should mention dialogue");
            }
            _ => panic!("Expected QueueParseError::Other, got: {:?}", queue_parse_err),
        }
    }

    /// Test that missing dialogue produces a hard parse error
    #[test]
    fn test_missing_dialogue_is_hard_error() {
        let raw = r#"
<reasoning>
Some internal thoughts without dialogue.
</reasoning>
"#;

        let result = parse_llm_response(raw);
        assert!(result.is_err(), "Missing dialogue should produce an error");

        let err = result.expect_err("should fail");
        assert!(
            matches!(err, LegacyParseError::MissingDialogue),
            "Expected MissingDialogue error, got: {:?}",
            err
        );
    }

    /// Test that valid responses with dialogue succeed
    #[test]
    fn test_valid_response_succeeds() {
        let raw = r#"
<reasoning>
The player is asking about the quest.
</reasoning>

<dialogue>
"I know nothing about that."
</dialogue>
"#;

        let result = parse_llm_response(raw);
        assert!(result.is_ok(), "Valid response should parse successfully");

        let parsed = result.expect("result should be Ok");
        assert!(parsed.dialogue.contains("nothing"));
        assert!(parsed.reasoning.contains("quest"));
    }

    /// Test that partial content (missing optional fields) still succeeds
    #[test]
    fn test_partial_content_succeeds() {
        let raw = r#"
<dialogue>
"Just dialogue, no other content."
</dialogue>
"#;

        let result = parse_llm_response(raw);
        assert!(result.is_ok(), "Partial content should still parse");

        let parsed = result.expect("result should be Ok");
        assert_eq!(parsed.dialogue, "\"Just dialogue, no other content.\"");
        assert!(parsed.reasoning.is_empty());
        assert!(parsed.topics.is_empty());
        assert!(parsed.proposed_tools.is_empty());
    }

    /// Test error message formatting for queue failure logging
    #[test]
    fn test_error_message_format_for_queue_failure() {
        let test_cases = vec![
            (
                LegacyParseError::MissingDialogue,
                "Missing or empty <dialogue>",
            ),
            (
                LegacyParseError::unrecognized_format("test format"),
                "Unrecognized response format",
            ),
            (
                LegacyParseError::malformed_tag("dialogue", "unclosed"),
                "Malformed tag",
            ),
            (LegacyParseError::other("custom error"), "custom error"),
        ];

        for (err, expected_substring) in test_cases {
            let message = err.to_string();
            assert!(
                message.contains(expected_substring),
                "Error message '{}' should contain '{}'",
                message,
                expected_substring
            );
        }
    }

    /// Test that empty dialogue after tag stripping fails
    #[test]
    fn test_empty_dialogue_after_tag_stripping_fails() {
        // Response that only contains tags, no actual dialogue
        let raw = r#"
<reasoning>
Thinking about this.
</reasoning>

<topics>
topic1
topic2
</topics>

<challenge_suggestion>
{"challenge_id": "test"}
</challenge_suggestion>
"#;

        let result = parse_llm_response(raw);
        assert!(
            result.is_err(),
            "Response with only tags and no dialogue should fail"
        );
    }

    /// Test that whitespace-only dialogue fails
    #[test]
    fn test_whitespace_only_dialogue_fails() {
        let raw = r#"
<dialogue>

   

</dialogue>
"#;

        let result = parse_llm_response(raw);
        assert!(
            result.is_err(),
            "Whitespace-only dialogue should produce an error"
        );
    }

    /// Test consistency between legacy and phased error handling
    /// Both pipelines should treat missing critical content as a hard error
    #[test]
    fn test_legacy_phased_error_consistency() {
        // Legacy parser: missing dialogue = error
        let legacy_result = parse_llm_response("<reasoning>thoughts</reasoning>");
        assert!(
            legacy_result.is_err(),
            "Legacy parser should error on missing dialogue"
        );

        // The phased pipeline uses PhaseParseError::MissingElement for similar cases
        // This test documents that both pipelines have consistent fail-fast semantics
        let legacy_err = legacy_result.expect_err("should fail");
        let err_message = legacy_err.to_string();
        assert!(
            err_message.to_lowercase().contains("dialogue"),
            "Error should mention dialogue: {}",
            err_message
        );
    }

    /// Test that QueueError::EnqueueFailed can be constructed and contains the expected message.
    /// This verifies the error variant exists for the fail-fast handling in ProcessPlayerAction.
    #[test]
    fn test_enqueue_failed_error_variant() {
        let error_message = "duplicate callback_id constraint".to_string();
        let err = QueueError::EnqueueFailed {
            message: error_message.clone(),
        };

        // Verify the error message is included in the display
        let display = err.to_string();
        assert!(
            display.contains("Failed to enqueue LLM request"),
            "Error display should indicate LLM enqueue failure: {}",
            display
        );
        assert!(
            display.contains(&error_message),
            "Error display should contain the original error message: {}",
            display
        );
    }

    /// Test that resolve_temperature falls back to default when SettingsRepo fails.
    /// This verifies the warn-level logging path is exercised (logs are captured by test harness).
    #[tokio::test]
    async fn test_resolve_temperature_logs_on_settings_repo_failure() {
        use std::sync::Arc;
        use crate::infrastructure::ports::{MockSettingsRepo, RepoError, SettingsRepo};
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let mut mock_settings = MockSettingsRepo::new();

        // Simulate a database error for both world-specific and global settings
        mock_settings
            .expect_get_for_world()
            .with(mockall::predicate::eq(world_id))
            .returning(|_| Err(RepoError::database("get_for_world", "connection timeout")));

        mock_settings
            .expect_get_global()
            .returning(|| Err(RepoError::database("get_global", "connection timeout")));

        let settings: Arc<dyn SettingsRepo> = Arc::new(mock_settings);

        // The function should log warnings and fall back to the default
        let result = super::resolve_temperature(
            None, // No template temperature
            world_id,
            settings,
            crate::infrastructure::app_settings::AppSettings::llm_temperature_npc_response,
            0.75, // Expected default
        )
        .await;

        // Should return the default when both lookups fail
        assert!(
            (result - 0.75).abs() < f32::EPSILON,
            "Should fall back to default when SettingsRepo fails, got {}",
            result
        );
    }

    /// Test that resolve_temperature uses template temperature when provided.
    #[tokio::test]
    async fn test_resolve_temperature_uses_template_temperature() {
        use std::sync::Arc;
        use crate::infrastructure::ports::{MockSettingsRepo, SettingsRepo};
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let mock_settings = MockSettingsRepo::new();

        // No expectations needed - template temperature should be used directly
        let settings: Arc<dyn SettingsRepo> = Arc::new(mock_settings);

        let result = super::resolve_temperature(
            Some(0.95), // Template temperature
            world_id,
            settings,
            crate::infrastructure::app_settings::AppSettings::llm_temperature_npc_response,
            0.7, // Default (should not be used)
        )
        .await;

        assert!(
            (result - 0.95).abs() < f32::EPSILON,
            "Should use template temperature when provided, got {}",
            result
        );
    }

    /// Test that resolve_temperature falls back to global settings when world-specific is None.
    #[tokio::test]
    async fn test_resolve_temperature_falls_back_to_global_settings() {
        use std::sync::Arc;
        use crate::infrastructure::ports::{MockSettingsRepo, SettingsRepo};
        use wrldbldr_domain::WorldId;

        let world_id = WorldId::new();
        let mut mock_settings = MockSettingsRepo::new();

        // World-specific settings returns None (not an error)
        mock_settings
            .expect_get_for_world()
            .with(mockall::predicate::eq(world_id))
            .returning(|_| Ok(None));

        // Global settings returns a value
        let global_settings = crate::infrastructure::app_settings::AppSettings::default();
        mock_settings
            .expect_get_global()
            .returning(move || Ok(Some(global_settings.clone())));

        let settings: Arc<dyn SettingsRepo> = Arc::new(mock_settings);

        let result = super::resolve_temperature(
            None,
            world_id,
            settings,
            crate::infrastructure::app_settings::AppSettings::llm_temperature_npc_response,
            0.5, // Default (should not be used)
        )
        .await;

        // Should use the global settings value (default is 0.7 in AppSettings)
        let expected = crate::infrastructure::app_settings::AppSettings::default()
            .llm_temperature_npc_response();
        assert!(
            (result - expected).abs() < 0.01,
            "Should fall back to global settings value ({}), got {}",
            expected, result
        );
    }
}
