//! LLM Queue Service - Concurrency-controlled LLM processing
//!
//! This service manages the LLMReasoningQueue, which processes LLM requests
//! with controlled concurrency using semaphores. It routes responses to the
//! DMApprovalQueue for NPC responses.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::application::services::generation_service::GenerationEvent;
use crate::application::services::llm::LLMService;
use crate::application::services::SuggestionContext;
use wrldbldr_domain::value_objects::{
    ApprovalDecisionType, ApprovalRequestData, ApprovalUrgency, ChallengeSuggestion,
    LlmRequestData, LlmRequestType, NarrativeEventSuggestion, ProposedTool,
};
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, SceneId, WorldId};
use wrldbldr_engine_ports::outbound::{
    ApprovalQueuePort, ChallengeCrudPort, ChallengeSkillPort, LlmPort, LlmQueueItem,
    LlmQueueRequest, LlmQueueResponse, LlmQueueServicePort, LlmRequestType as PortLlmRequestType,
    LlmSuggestionContext as PortSuggestionContext, NarrativeEventCrudPort, ProcessingQueuePort,
    QueueError, QueueItemId, QueueItemStatus, QueueNotificationPort, SkillRepositoryPort,
    PromptTemplateServicePort,
};

/// Priority constant for queue operations
const PRIORITY_NORMAL: u8 = 0;

/// Service for managing the LLM reasoning queue
pub struct LLMQueueService<
    Q: ProcessingQueuePort<LlmRequestData>,
    L: LlmPort + Clone,
    N: QueueNotificationPort,
> {
    pub(crate) queue: Arc<Q>,
    llm_client: Arc<L>, // Keep for SuggestionService
    approval_queue: Arc<dyn ApprovalQueuePort<ApprovalRequestData>>,
    challenge_crud: Arc<dyn ChallengeCrudPort>,
    challenge_skill: Arc<dyn ChallengeSkillPort>,
    skill_repo: Arc<dyn SkillRepositoryPort>,
    narrative_event_repo: Arc<dyn NarrativeEventCrudPort>,
    semaphore: Arc<Semaphore>,
    notifier: N,
    generation_event_tx: tokio::sync::mpsc::Sender<GenerationEvent>,
    prompt_template_service: Arc<dyn PromptTemplateServicePort>,
}

impl<
        Q: ProcessingQueuePort<LlmRequestData> + 'static,
        L: LlmPort + Clone + 'static,
        N: QueueNotificationPort + 'static,
    > LLMQueueService<Q, L, N>
{
    pub fn queue(&self) -> &Arc<Q> {
        &self.queue
    }

    /// Create a new LLM queue service
    ///
    /// # Arguments
    ///
    /// * `queue` - The LLM request queue
    /// * `llm_client` - The LLM client for processing requests
    /// * `approval_queue` - The approval queue for routing NPC responses
    /// * `challenge_crud` - CRUD port for looking up challenge details
    /// * `challenge_skill` - Skill port for getting required skills from challenges
    /// * `skill_repo` - Repository for looking up skill details
    /// * `narrative_event_repo` - Repository for looking up narrative event details
    /// * `batch_size` - Maximum concurrent LLM requests (default: 1)
    /// * `notifier` - The notifier for waking workers
    /// * `generation_event_tx` - Channel for emitting generation events (suggestions)
    pub fn new(
        queue: Arc<Q>,
        llm_client: Arc<L>,
        approval_queue: Arc<dyn ApprovalQueuePort<ApprovalRequestData>>,
        challenge_crud: Arc<dyn ChallengeCrudPort>,
        challenge_skill: Arc<dyn ChallengeSkillPort>,
        skill_repo: Arc<dyn SkillRepositoryPort>,
        narrative_event_repo: Arc<dyn NarrativeEventCrudPort>,
        batch_size: usize,
        notifier: N,
        generation_event_tx: tokio::sync::mpsc::Sender<GenerationEvent>,
        prompt_template_service: Arc<dyn PromptTemplateServicePort>,
    ) -> Self {
        Self {
            queue,
            llm_client,
            approval_queue,
            challenge_crud,
            challenge_skill,
            skill_repo,
            narrative_event_repo,
            semaphore: Arc::new(Semaphore::new(batch_size.max(1))),
            notifier,
            generation_event_tx,
            prompt_template_service,
        }
    }

    /// Enqueue an LLM request
    pub async fn enqueue(&self, request: LlmRequestData) -> Result<QueueItemId, QueueError> {
        self.queue.enqueue(request, PRIORITY_NORMAL).await
    }

    /// Clean up old completed/failed items beyond retention period
    pub async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64> {
        let count = self.queue.cleanup(retention).await?;
        Ok(count as u64)
    }

    /// Cancel a suggestion request by its callback_id (request_id)
    pub async fn cancel_suggestion(&self, request_id: &str) -> Result<bool, QueueError> {
        // Search through pending and processing items
        let pending_items = self.queue.list_by_status(QueueItemStatus::Pending).await?;
        let processing_items = self
            .queue
            .list_by_status(QueueItemStatus::Processing)
            .await?;

        // Find item with matching callback_id
        for item in pending_items.iter().chain(processing_items.iter()) {
            if item.payload.callback_id == request_id {
                // Mark as failed with cancellation message
                self.queue.fail(item.id, "Cancelled by user").await?;

                // Emit cancellation event (non-blocking, logs warning if buffer full)
                if let Err(e) =
                    self.generation_event_tx
                        .try_send(GenerationEvent::SuggestionFailed {
                            request_id: request_id.to_string(),
                            field_type: match &item.payload.request_type {
                                LlmRequestType::Suggestion { field_type, .. } => field_type.clone(),
                                _ => String::new(),
                            },
                            error: "Cancelled by user".to_string(),
                            world_id: Some(item.payload.world_id),
                        })
                {
                    tracing::warn!("Failed to send cancellation event: {}", e);
                }

                return Ok(true);
            }
        }

        Ok(false) // Not found
    }

    /// Background worker that processes LLM requests
    ///
    /// This method runs in a loop, processing items from the queue with
    /// concurrency control via semaphore. Each request is processed in
    /// a spawned task to allow parallel processing up to batch_size.
    ///
    /// # Arguments
    /// * `recovery_interval` - Fallback poll interval for crash recovery
    /// * `cancel_token` - Token to signal graceful shutdown
    pub async fn run_worker(
        self: Arc<Self>,
        recovery_interval: Duration,
        cancel_token: CancellationToken,
    ) {
        loop {
            // Check for cancellation
            if cancel_token.is_cancelled() {
                tracing::info!("LLM queue worker shutting down");
                break;
            }

            // Try to get next item
            let item = match self.queue.dequeue().await {
                Ok(Some(item)) => item,
                Ok(None) => {
                    // Queue empty - wait for notification or recovery timeout
                    // Use select to also check for cancellation during wait
                    tokio::select! {
                        _ = cancel_token.cancelled() => {
                            tracing::info!("LLM queue worker shutting down");
                            break;
                        }
                        _ = self.notifier.wait_for_work(recovery_interval) => {
                            continue;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to dequeue LLM request: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Process in spawned task - acquire permit inside the task for proper lifetime
            // Clone all needed data before spawning to avoid lifetime issues
            let semaphore = self.semaphore.clone();
            let llm_client_clone = self.llm_client.clone();
            let queue_clone = self.queue.clone();
            let approval_queue_clone = self.approval_queue.clone();
            let challenge_crud_clone = self.challenge_crud.clone();
            let challenge_skill_clone = self.challenge_skill.clone();
            let skill_repo_clone = self.skill_repo.clone();
            let narrative_event_repo_clone = self.narrative_event_repo.clone();
            let generation_event_tx_clone = self.generation_event_tx.clone();
            let prompt_template_service_clone = self.prompt_template_service.clone();
            let request = item.payload.clone();
            let item_id = item.id;

            tokio::spawn(async move {
                // Wait for capacity inside the spawned task
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("Semaphore error: {}", e);
                        return;
                    }
                };

                let llm_service = LLMService::new(
                    Arc::clone(&llm_client_clone),
                    prompt_template_service_clone.clone(),
                );

                match &request.request_type {
                    LlmRequestType::NpcResponse { action_item_id } => {
                        // Process NPC response request
                        let Some(prompt) = request.prompt.as_ref() else {
                            let error = "Missing prompt for NPC response".to_string();
                            tracing::error!("{}", error);
                            if let Err(e) = queue_clone.fail(item_id, &error).await {
                                tracing::error!("Failed to mark queue item as failed: {}", e);
                            }
                            return;
                        };
                        match llm_service.generate_npc_response(prompt.clone()).await
                        {
                            Ok(response) => {
                                // Create approval item for DM
                                let world_id = request.world_id;

                                // Extract NPC name and ID from the prompt's responding character
                                let npc_name = prompt.responding_character.name.clone();
                                let npc_id = prompt
                                    .responding_character
                                    .character_id
                                    .as_ref()
                                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                    .map(CharacterId::from_uuid);

                                // Extract challenge suggestion from LLM response
                                let challenge_suggestion = if let Some(cs) =
                                    response.challenge_suggestion
                                {
                                    // Parse the challenge ID from string
                                    let challenge_id_result =
                                        uuid::Uuid::parse_str(&cs.challenge_id)
                                            .map(wrldbldr_domain::ChallengeId::from_uuid);

                                    match challenge_id_result {
                                        Ok(challenge_id) => {
                                            // Look up challenge details
                                            match challenge_crud_clone.get(challenge_id).await {
                                                Ok(Some(challenge)) => {
                                                    // Fetch skill_id from REQUIRES_SKILL edge
                                                    let skill_id = match challenge_skill_clone
                                                        .get_required_skill(challenge_id)
                                                        .await
                                                    {
                                                        Ok(sid) => sid,
                                                        Err(e) => {
                                                            tracing::warn!("Failed to get required skill for challenge {}: {}", challenge_id, e);
                                                            None
                                                        }
                                                    };

                                                    // Look up skill name
                                                    let skill_name = if let Some(sid) = skill_id {
                                                        match skill_repo_clone.get(sid).await {
                                                            Ok(Some(skill)) => skill.name,
                                                            Ok(None) => {
                                                                tracing::warn!("Skill {} not found for challenge {}", sid, cs.challenge_id);
                                                                sid.to_string()
                                                            }
                                                            Err(e) => {
                                                                tracing::error!("Failed to look up skill {}: {}", sid, e);
                                                                sid.to_string()
                                                            }
                                                        }
                                                    } else {
                                                        "Unknown Skill".to_string()
                                                    };

                                                    Some(ChallengeSuggestion {
                                                        challenge_id: cs.challenge_id,
                                                        challenge_name: challenge.name,
                                                        skill_name,
                                                        difficulty_display: challenge
                                                            .difficulty
                                                            .display(),
                                                        confidence: format!("{:?}", cs.confidence),
                                                        reasoning: cs.reasoning,
                                                        target_pc_id: request.pc_id,
                                                        outcomes: None,
                                                    })
                                                }
                                                Ok(None) => {
                                                    tracing::warn!("Challenge {} not found, using minimal info", cs.challenge_id);
                                                    Some(ChallengeSuggestion {
                                                        challenge_id: cs.challenge_id.clone(),
                                                        challenge_name: format!(
                                                            "Challenge {}",
                                                            cs.challenge_id
                                                        ),
                                                        skill_name: String::new(),
                                                        difficulty_display: String::new(),
                                                        confidence: format!("{:?}", cs.confidence),
                                                        reasoning: cs.reasoning.clone(),
                                                        target_pc_id: request.pc_id,
                                                        outcomes: None,
                                                    })
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        "Failed to look up challenge {}: {}",
                                                        cs.challenge_id,
                                                        e
                                                    );
                                                    Some(ChallengeSuggestion {
                                                        challenge_id: cs.challenge_id.clone(),
                                                        challenge_name: format!(
                                                            "Challenge {}",
                                                            cs.challenge_id
                                                        ),
                                                        skill_name: String::new(),
                                                        difficulty_display: String::new(),
                                                        confidence: format!("{:?}", cs.confidence),
                                                        reasoning: cs.reasoning.clone(),
                                                        target_pc_id: request.pc_id,
                                                        outcomes: None,
                                                    })
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to parse challenge ID {}: {}",
                                                cs.challenge_id,
                                                e
                                            );
                                            Some(ChallengeSuggestion {
                                                challenge_id: cs.challenge_id,
                                                challenge_name: format!(
                                                    "Challenge {}",
                                                    "invalid-id"
                                                ),
                                                skill_name: String::new(),
                                                difficulty_display: String::new(),
                                                confidence: format!("{:?}", cs.confidence),
                                                reasoning: cs.reasoning,
                                                target_pc_id: request.pc_id,
                                                outcomes: None,
                                            })
                                        }
                                    }
                                } else {
                                    None
                                };

                                // Extract narrative event suggestion from LLM response
                                let narrative_event_suggestion = if let Some(nes) =
                                    response.narrative_event_suggestion
                                {
                                    // Parse the narrative event ID from string
                                    let event_id_result = uuid::Uuid::parse_str(&nes.event_id)
                                        .map(wrldbldr_domain::NarrativeEventId::from_uuid);

                                    match event_id_result {
                                        Ok(event_id) => {
                                            // Look up narrative event details
                                            match narrative_event_repo_clone.get(event_id).await {
                                                Ok(Some(event)) => Some(NarrativeEventSuggestion {
                                                    event_id: nes.event_id.clone(),
                                                    event_name: event.name,
                                                    description: event.description,
                                                    scene_direction: event.scene_direction,
                                                    confidence: format!("{:?}", nes.confidence),
                                                    reasoning: nes.reasoning.clone(),
                                                    matched_triggers: nes.matched_triggers.clone(),
                                                    suggested_outcome: None,
                                                }),
                                                Ok(None) => {
                                                    tracing::warn!("Narrative event {} not found, using minimal info", nes.event_id);
                                                    Some(NarrativeEventSuggestion {
                                                        event_id: nes.event_id.clone(),
                                                        event_name: format!(
                                                            "Event {}",
                                                            nes.event_id
                                                        ),
                                                        description: String::new(),
                                                        scene_direction: String::new(),
                                                        confidence: format!("{:?}", nes.confidence),
                                                        reasoning: nes.reasoning.clone(),
                                                        matched_triggers: nes
                                                            .matched_triggers
                                                            .clone(),
                                                        suggested_outcome: None,
                                                    })
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        "Failed to look up narrative event {}: {}",
                                                        nes.event_id,
                                                        e
                                                    );
                                                    Some(NarrativeEventSuggestion {
                                                        event_id: nes.event_id.clone(),
                                                        event_name: format!(
                                                            "Event {}",
                                                            nes.event_id
                                                        ),
                                                        description: String::new(),
                                                        scene_direction: String::new(),
                                                        confidence: format!("{:?}", nes.confidence),
                                                        reasoning: nes.reasoning.clone(),
                                                        matched_triggers: nes
                                                            .matched_triggers
                                                            .clone(),
                                                        suggested_outcome: None,
                                                    })
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to parse narrative event ID {}: {}",
                                                nes.event_id,
                                                e
                                            );
                                            Some(NarrativeEventSuggestion {
                                                event_id: nes.event_id,
                                                event_name: format!("Event {}", "invalid-id"),
                                                description: String::new(),
                                                scene_direction: String::new(),
                                                confidence: format!("{:?}", nes.confidence),
                                                reasoning: nes.reasoning,
                                                matched_triggers: nes.matched_triggers,
                                                suggested_outcome: None,
                                            })
                                        }
                                    }
                                } else {
                                    None
                                };

                                let approval = ApprovalRequestData {
                                    world_id,
                                    source_action_id: *action_item_id,
                                    decision_type: ApprovalDecisionType::NpcResponse,
                                    urgency: ApprovalUrgency::AwaitingPlayer,
                                    pc_id: request.pc_id,
                                    npc_id,
                                    npc_name,
                                    proposed_dialogue: response.npc_dialogue.clone(),
                                    internal_reasoning: response.internal_reasoning.clone(),
                                    proposed_tools: response
                                        .proposed_tool_calls
                                        .iter()
                                        .map(|t| ProposedTool {
                                            id: uuid::Uuid::new_v4().to_string(), // Generate ID for tool call
                                            name: t.tool_name.clone(),
                                            description: format!("Tool call: {}", t.tool_name),
                                            arguments: t.arguments.clone(),
                                        })
                                        .collect(),
                                    retry_count: 0,
                                    challenge_suggestion,
                                    narrative_event_suggestion,
                                    // P1.2: Context for dialogue persistence
                                    player_dialogue: prompt.player_action.dialogue.clone(),
                                    scene_id: prompt
                                        .scene_id
                                        .as_ref()
                                        .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                        .map(SceneId::from_uuid),
                                    location_id: prompt
                                        .location_id
                                        .as_ref()
                                        .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                        .map(LocationId::from_uuid),
                                    game_time: prompt.game_time.clone(),
                                    topics: response.topics.clone(),
                                };

                                // Enqueue approval and notify DM
                                match approval_queue_clone
                                    .enqueue(
                                        approval.clone(),
                                        ApprovalUrgency::AwaitingPlayer as u8,
                                    )
                                    .await
                                {
                                    Ok(approval_item_id) => {
                                        // Note: ApprovalRequired message is created and sent by the approval notification worker
                                        // The suggestions are stored in ApprovalItem and will be used by the worker
                                        // No need to create the message here

                                        // Store pending approval and send to DM
                                        // This requires access to world state manager, which we'll handle in a worker
                                        tracing::info!(
                                            "Enqueued approval {} for NPC {} in world {}",
                                            approval_item_id,
                                            approval.npc_name,
                                            approval.world_id
                                        );

                                        if let Err(e) = queue_clone.complete(item_id).await {
                                            tracing::error!(
                                                "Failed to mark queue item as complete: {}",
                                                e
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to enqueue approval: {}", e);
                                        if let Err(e2) =
                                            queue_clone.fail(item_id, &e.to_string()).await
                                        {
                                            tracing::error!(
                                                "Failed to mark queue item as failed: {}",
                                                e2
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("LLM generation failed: {}", e);
                                if let Err(e2) = queue_clone.fail(item_id, &e.to_string()).await {
                                    tracing::error!("Failed to mark queue item as failed: {}", e2);
                                }
                            }
                        }
                    }
                    LlmRequestType::Suggestion {
                        field_type,
                        entity_id,
                    } => {
                        // Process suggestion request
                        let Some(domain_context) = request.suggestion_context.as_ref() else {
                            let error = "Missing suggestion context".to_string();
                            tracing::error!("{}", error);
                            if let Err(e) = queue_clone.fail(item_id, &error).await {
                                tracing::error!("Failed to mark queue item as failed: {}", e);
                            }
                            return;
                        };

                        // Convert domain SuggestionContext to service SuggestionContext
                        let context = SuggestionContext {
                            entity_type: domain_context.entity_type.clone(),
                            entity_name: domain_context.entity_name.clone(),
                            world_setting: domain_context.world_setting.clone(),
                            hints: domain_context.hints.clone(),
                            additional_context: domain_context.additional_context.clone(),
                            world_id: domain_context.world_id.map(|id| id.to_string()),
                        };

                        let request_id = request.callback_id.clone();
                        let field_type_clone = field_type.clone();
                        let entity_id_clone = entity_id.clone();
                        let world_id_clone = request.world_id;

                        // Emit queued event
                        if let Err(e) =
                            generation_event_tx_clone.try_send(GenerationEvent::SuggestionQueued {
                                request_id: request_id.clone(),
                                field_type: field_type_clone.clone(),
                                entity_id: entity_id_clone.clone(),
                                world_id: Some(world_id_clone),
                            })
                        {
                            tracing::warn!("Failed to send SuggestionQueued event: {}", e);
                        }

                        // Create suggestion service
                        use crate::application::services::SuggestionService;
                        let suggestion_service = SuggestionService::new(
                            (*llm_client_clone).clone(),
                            prompt_template_service_clone.clone(),
                        );

                        // Process based on field type
                        let result = match field_type.as_str() {
                            "character_name" => {
                                suggestion_service.suggest_character_names(&context).await
                            }
                            "character_description" => {
                                suggestion_service
                                    .suggest_character_description(&context)
                                    .await
                            }
                            "character_wants" => {
                                suggestion_service.suggest_character_wants(&context).await
                            }
                            "character_fears" => {
                                suggestion_service.suggest_character_fears(&context).await
                            }
                            "character_backstory" => {
                                suggestion_service
                                    .suggest_character_backstory(&context)
                                    .await
                            }
                            "location_name" => {
                                suggestion_service.suggest_location_names(&context).await
                            }
                            "location_description" => {
                                suggestion_service
                                    .suggest_location_description(&context)
                                    .await
                            }
                            "location_atmosphere" => {
                                suggestion_service
                                    .suggest_location_atmosphere(&context)
                                    .await
                            }
                            "location_features" => {
                                suggestion_service.suggest_location_features(&context).await
                            }
                            "location_secrets" => {
                                suggestion_service.suggest_location_secrets(&context).await
                            }
                            // Actantial Model suggestions
                            "deflection_behavior" => {
                                suggestion_service
                                    .suggest_deflection_behavior(&context)
                                    .await
                            }
                            "behavioral_tells" => {
                                suggestion_service.suggest_behavioral_tells(&context).await
                            }
                            "want_description" => {
                                suggestion_service.suggest_want_description(&context).await
                            }
                            "actantial_reason" => {
                                suggestion_service.suggest_actantial_reason(&context).await
                            }
                            _ => {
                                let error =
                                    format!("Unknown suggestion field type: {}", field_type);
                                tracing::error!("{}", error);
                                if let Err(e) = generation_event_tx_clone.try_send(
                                    GenerationEvent::SuggestionFailed {
                                        request_id: request_id.clone(),
                                        field_type: field_type_clone.clone(),
                                        error: error.clone(),
                                        world_id: Some(world_id_clone),
                                    },
                                ) {
                                    tracing::warn!("Failed to send SuggestionFailed event: {}", e);
                                }
                                if let Err(e) = queue_clone.fail(item_id, &error).await {
                                    tracing::error!("Failed to mark queue item as failed: {}", e);
                                }
                                return;
                            }
                        };

                        match result {
                            Ok(suggestions) => {
                                tracing::info!(
                                    "Suggestion request {} completed with {} suggestions",
                                    request_id,
                                    suggestions.len()
                                );
                                if let Err(e) = generation_event_tx_clone.try_send(
                                    GenerationEvent::SuggestionComplete {
                                        request_id: request_id.clone(),
                                        field_type: field_type_clone.clone(),
                                        suggestions,
                                        world_id: Some(world_id_clone),
                                    },
                                ) {
                                    tracing::warn!(
                                        "Failed to send SuggestionComplete event: {}",
                                        e
                                    );
                                }
                                if let Err(e) = queue_clone.complete(item_id).await {
                                    tracing::error!("Failed to mark queue item as complete: {}", e);
                                }
                            }
                            Err(e) => {
                                let error = e.to_string();
                                tracing::error!(
                                    "Suggestion request {} failed: {}",
                                    request_id,
                                    error
                                );
                                if let Err(e) = generation_event_tx_clone.try_send(
                                    GenerationEvent::SuggestionFailed {
                                        request_id: request_id.clone(),
                                        field_type: field_type_clone.clone(),
                                        error: error.clone(),
                                        world_id: Some(world_id_clone),
                                    },
                                ) {
                                    tracing::warn!("Failed to send SuggestionFailed event: {}", e);
                                }
                                if let Err(e) = queue_clone.fail(item_id, &error).await {
                                    tracing::error!("Failed to mark queue item as failed: {}", e);
                                }
                            }
                        }
                    }
                }
            });
        }
    }
}

// ============================================================================
// Port Implementation
// ============================================================================

/// Convert port LlmRequestType to domain LlmRequestType
fn convert_port_request_type(request_type: PortLlmRequestType) -> LlmRequestType {
    match request_type {
        PortLlmRequestType::NpcResponse { action_item_id } => {
            LlmRequestType::NpcResponse { action_item_id }
        }
        PortLlmRequestType::Suggestion {
            field_type,
            entity_id,
        } => LlmRequestType::Suggestion {
            field_type,
            entity_id,
        },
    }
}

/// Convert domain LlmRequestType to port LlmRequestType
fn convert_domain_request_type(request_type: LlmRequestType) -> PortLlmRequestType {
    match request_type {
        LlmRequestType::NpcResponse { action_item_id } => {
            PortLlmRequestType::NpcResponse { action_item_id }
        }
        LlmRequestType::Suggestion {
            field_type,
            entity_id,
        } => PortLlmRequestType::Suggestion {
            field_type,
            entity_id,
        },
    }
}

/// Convert port SuggestionContext to domain SuggestionContext
fn convert_port_suggestion_context(
    ctx: Option<PortSuggestionContext>,
) -> Option<wrldbldr_domain::value_objects::SuggestionContext> {
    ctx.map(|c| wrldbldr_domain::value_objects::SuggestionContext {
        entity_type: c.entity_type,
        entity_name: c.entity_name,
        world_setting: c.world_setting,
        hints: c.hints,
        additional_context: c.additional_context,
        world_id: c
            .world_id
            .and_then(|s| uuid::Uuid::parse_str(&s).ok().map(WorldId::from_uuid)),
    })
}

/// Convert domain SuggestionContext to port SuggestionContext
fn convert_domain_suggestion_context(
    ctx: Option<wrldbldr_domain::value_objects::SuggestionContext>,
) -> Option<PortSuggestionContext> {
    ctx.map(|c| PortSuggestionContext {
        entity_type: c.entity_type,
        entity_name: c.entity_name,
        world_setting: c.world_setting,
        hints: c.hints,
        additional_context: c.additional_context,
        world_id: c.world_id.map(|id| id.to_string()),
    })
}

#[async_trait]
impl<Q, L, N> LlmQueueServicePort for LLMQueueService<Q, L, N>
where
    Q: ProcessingQueuePort<LlmRequestData> + Send + Sync + 'static,
    L: LlmPort + Clone + Send + Sync + 'static,
    N: QueueNotificationPort + Send + Sync + 'static,
{
    async fn enqueue(&self, request: LlmQueueRequest) -> anyhow::Result<uuid::Uuid> {
        let item = LlmRequestData {
            request_type: convert_port_request_type(request.request_type),
            world_id: WorldId::from_uuid(request.world_id),
            pc_id: request.pc_id.map(PlayerCharacterId::from_uuid),
            prompt: request.prompt,
            suggestion_context: convert_port_suggestion_context(request.suggestion_context),
            callback_id: request.callback_id,
        };

        self.queue
            .enqueue(item, PRIORITY_NORMAL)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn dequeue(&self) -> anyhow::Result<Option<LlmQueueItem>> {
        let item = self
            .queue
            .dequeue()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(item.map(|i| LlmQueueItem {
            id: i.id,
            payload: LlmQueueRequest {
                request_type: convert_domain_request_type(i.payload.request_type),
                world_id: i.payload.world_id.to_uuid(),
                pc_id: i.payload.pc_id.map(|id| id.to_uuid()),
                prompt: i.payload.prompt,
                suggestion_context: convert_domain_suggestion_context(i.payload.suggestion_context),
                callback_id: i.payload.callback_id.clone(),
            },
            priority: i.priority,
            callback_id: i.payload.callback_id,
        }))
    }

    async fn complete(&self, id: uuid::Uuid, _result: LlmQueueResponse) -> anyhow::Result<()> {
        // The port receives a LlmQueueResponse but the underlying queue just marks complete
        // The actual response handling is done in run_worker
        self.queue
            .complete(id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn fail(&self, id: uuid::Uuid, error: String) -> anyhow::Result<()> {
        self.queue
            .fail(id, &error)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn cancel_suggestion(&self, callback_id: &str) -> anyhow::Result<bool> {
        self.cancel_suggestion(callback_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn depth(&self) -> anyhow::Result<usize> {
        self.queue
            .depth()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn processing_count(&self) -> anyhow::Result<usize> {
        self.queue
            .processing_count()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn list_by_status(&self, status: QueueItemStatus) -> anyhow::Result<Vec<LlmQueueItem>> {
        let items = self
            .queue
            .list_by_status(status)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(items
            .into_iter()
            .map(|i| LlmQueueItem {
                id: i.id,
                payload: LlmQueueRequest {
                    request_type: convert_domain_request_type(i.payload.request_type),
                    world_id: i.payload.world_id.to_uuid(),
                    pc_id: i.payload.pc_id.map(|id| id.to_uuid()),
                    prompt: i.payload.prompt,
                    suggestion_context: convert_domain_suggestion_context(
                        i.payload.suggestion_context,
                    ),
                    callback_id: i.payload.callback_id.clone(),
                },
                priority: i.priority,
                callback_id: i.payload.callback_id,
            })
            .collect())
    }

    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64> {
        self.cleanup(retention).await
    }
}
