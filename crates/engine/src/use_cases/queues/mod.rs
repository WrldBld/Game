//! Queue processing use cases.
//!
//! These use cases are background workers that poll queues and process items.
//! They form the backbone of the asynchronous processing pipeline:
//!
//! 1. Player Action Queue -> Builds LLM prompt -> LLM Request Queue
//! 2. LLM Request Queue -> Calls LLM -> DM Approval Queue
//! 3. DM Approval Queue -> (handled by DM via WebSocket)

use std::sync::Arc;
use wrldbldr_domain::{
    CharacterContext, GamePromptRequest, LlmRequestData, LlmRequestType, PlayerActionContext,
    PlayerActionData, SceneContext, WorldId,
};

use crate::infrastructure::ports::{LlmPort, QueuePort, RepoError};

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
pub struct ProcessPlayerAction {
    queue: Arc<dyn QueuePort>,
    character: Arc<crate::entities::Character>,
    player_character: Arc<crate::entities::PlayerCharacter>,
    staging: Arc<crate::entities::Staging>,
}

impl ProcessPlayerAction {
    pub fn new(
        queue: Arc<dyn QueuePort>,
        character: Arc<crate::entities::Character>,
        player_character: Arc<crate::entities::PlayerCharacter>,
        staging: Arc<crate::entities::Staging>,
    ) -> Self {
        Self {
            queue,
            character,
            player_character,
            staging,
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
    async fn build_prompt(&self, action_data: &PlayerActionData) -> Result<GamePromptRequest, QueueError> {
        // Get PC name if pc_id is provided
        let pc_name = if let Some(pc_id) = action_data.pc_id {
            self.player_character
                .get(pc_id)
                .await?
                .map(|pc| pc.name)
                .unwrap_or_else(|| "Unknown Player".to_string())
        } else {
            "Unknown Player".to_string()
        };

        // Get target NPC name (use provided target or default)
        let target_name = action_data
            .target
            .clone()
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

        // Build minimal scene context
        // In a full implementation, we would load scene details from the database
        let scene_context = SceneContext {
            scene_name: "Current Scene".to_string(),
            location_name: "Current Location".to_string(),
            time_context: "Present".to_string(),
            present_characters: vec![pc_name.clone(), target_name.clone()],
            region_items: vec![],
        };

        // Build responding character context
        // In a full implementation, we would load the NPC's full context
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

        // Build directorial notes with the prompt
        let directorial_notes = format!(
            "You are roleplaying as an NPC in a fantasy TTRPG. \
            The player character \"{}\" says to {}: \"{}\". \
            Respond in character as {}. Keep the response concise (1-3 sentences).",
            pc_name, target_name, dialogue, target_name
        );

        Ok(GamePromptRequest {
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
}

impl ProcessLlmRequest {
    pub fn new(queue: Arc<dyn QueuePort>, llm: Arc<dyn LlmPort>) -> Self {
        Self { queue, llm }
    }

    /// Process the next LLM request in the queue.
    ///
    /// Returns None if the queue is empty.
    pub async fn execute(&self) -> Result<Option<LlmRequestProcessed>, QueueError> {
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
                guidance 
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

                let llm_response = self
                    .llm
                    .generate(llm_request)
                    .await
                    .map_err(|e| QueueError::LlmError(e.to_string()))?;

                // Parse suggestions from response (one per line)
                let suggestions: Vec<String> = llm_response.content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| {
                        // Strip numbering if present (e.g., "1. ", "1) ", etc.)
                        let trimmed = line.trim();
                        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
                            rest.trim_start_matches(['.', ')', ':', '-', ' ']).trim().to_string()
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
            LlmRequestType::NpcResponse { .. } | LlmRequestType::Suggestion { .. } => {
                // Build LLM request from the queued prompt data
                let llm_request = if let Some(ref prompt) = request_data.prompt {
                    // Use the full GamePromptRequest to build a rich prompt
                    let system_prompt = format!(
                        "You are roleplaying as an NPC in a fantasy TTRPG. {}\n\n\
                        Scene: {} at {}\n\
                        Present characters: {}\n\n\
                        Respond in character. Keep responses concise (1-3 sentences). \
                        Stay true to the NPC's personality and motivations.",
                        prompt.directorial_notes,
                        prompt.scene_context.scene_name,
                        prompt.scene_context.location_name,
                        prompt.scene_context.present_characters.join(", ")
                    );

                    let user_message = if let Some(ref dialogue) = prompt.player_action.dialogue {
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

                    crate::infrastructure::ports::LlmRequest::new(vec![
                        crate::infrastructure::ports::ChatMessage::user(&user_message),
                    ])
                    .with_system_prompt(system_prompt)
                    .with_temperature(0.7)
                } else {
                    // Fallback if no prompt was provided
                    tracing::warn!("LLM request has no prompt data, using fallback");
                    crate::infrastructure::ports::LlmRequest::new(vec![
                        crate::infrastructure::ports::ChatMessage::user(
                            "Generate a brief, in-character NPC response to the player's action.",
                        ),
                    ])
                    .with_system_prompt("You are an NPC in a fantasy TTRPG. Respond briefly and in character.")
                };

                let llm_response = self
                    .llm
                    .generate(llm_request)
                    .await
                    .map_err(|e| QueueError::LlmError(e.to_string()))?;

                // Create approval request
                let approval_data = wrldbldr_domain::ApprovalRequestData {
                    world_id: request_data.world_id,
                    source_action_id: item.id,
                    decision_type: wrldbldr_domain::ApprovalDecisionType::NpcResponse,
                    urgency: wrldbldr_domain::ApprovalUrgency::AwaitingPlayer,
                    pc_id: request_data.pc_id,
                    npc_id: None,
                    npc_name: String::new(),
                    proposed_dialogue: llm_response.content.clone(),
                    internal_reasoning: String::new(),
                    proposed_tools: vec![],
                    retry_count: 0,
                    challenge_suggestion: None,
                    narrative_event_suggestion: None,
                    challenge_outcome: None,
                    player_dialogue: None,
                    scene_id: None,
                    location_id: None,
                    game_time: None,
                    topics: vec![],
                };

                // Enqueue for DM approval
                let approval_id = self.queue.enqueue_dm_approval(&approval_data).await?;

                // Mark the LLM request as complete
                self.queue.mark_complete(item.id).await?;

                Ok(Some(LlmRequestProcessed {
                    request_id: item.id,
                    approval_id,
                    npc_dialogue: llm_response.content,
                    broadcast_events: vec![],
                }))
            }
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
    LlmError(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
