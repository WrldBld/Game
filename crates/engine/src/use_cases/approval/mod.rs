// Approval use cases - methods for future DM approval workflows
#![allow(dead_code)]

//! DM approval use cases.
//!
//! Handles approval workflows for:
//! - NPC staging (who appears in a region)
//! - LLM suggestions (NPC dialogue, tool calls)
//! - Challenge outcomes

pub mod tool_executor;

use std::sync::Arc;
use wrldbldr_domain::{CharacterId, ConversationId, QueueItemId, RegionId, WorldId};

use crate::queue_types::DmApprovalDecision;
use crate::use_cases::time::SuggestTime;

use crate::infrastructure::ports::{PlayerCharacterRepo, QueueError, QueuePort, RepoError, StagingRepo, WorldRepo};

/// Container for approval use cases.
pub struct ApprovalUseCases {
    pub approve_staging: Arc<ApproveStaging>,
    pub approve_suggestion: Arc<ApproveSuggestion>,
    pub decision_flow: Arc<ApprovalDecisionFlow>,
}

impl ApprovalUseCases {
    pub fn new(
        approve_staging: Arc<ApproveStaging>,
        approve_suggestion: Arc<ApproveSuggestion>,
        decision_flow: Arc<ApprovalDecisionFlow>,
    ) -> Self {
        Self {
            approve_staging,
            approve_suggestion,
            decision_flow,
        }
    }
}

/// Result of staging approval.
#[derive(Debug)]
pub struct StagingApprovalResult {
    /// The region that was staged
    pub region_id: RegionId,
    /// NPCs that are now staged in the region
    pub staged_npcs: Vec<CharacterId>,
}

/// Approve staging use case.
///
/// Handles DM approval of which NPCs appear in a region.
pub struct ApproveStaging {
    staging: Arc<dyn StagingRepo>,
}

impl ApproveStaging {
    pub fn new(staging: Arc<dyn StagingRepo>) -> Self {
        Self { staging }
    }

    /// Approve staging for a region with a specific set of NPCs.
    ///
    /// # Arguments
    /// * `region_id` - The region being staged
    /// * `npc_ids` - The NPCs to stage in the region
    ///
    /// # Returns
    /// * `Ok(StagingApprovalResult)` - Staging was applied
    /// * `Err(ApprovalError)` - Failed to process staging
    pub async fn execute(
        &self,
        region_id: RegionId,
        npc_ids: Vec<CharacterId>,
    ) -> Result<StagingApprovalResult, ApprovalError> {
        // Stage the approved NPCs
        for npc_id in &npc_ids {
            self.staging.stage_npc(region_id, *npc_id).await?;
        }

        Ok(StagingApprovalResult {
            region_id,
            staged_npcs: npc_ids,
        })
    }

    /// Clear staging for a region (remove all NPCs).
    pub async fn clear_staging(&self, region_id: RegionId) -> Result<(), ApprovalError> {
        let current = self.staging.get_staged_npcs(region_id).await?;
        for npc in current {
            self.staging
                .unstage_npc(region_id, npc.character_id)
                .await?;
        }
        Ok(())
    }
}

/// Result of suggestion approval.
#[derive(Debug)]
pub struct SuggestionApprovalResult {
    /// The original suggestion ID
    pub suggestion_id: QueueItemId,
    /// Whether it was approved
    pub approved: bool,
    /// The final dialogue (possibly modified)
    pub final_dialogue: Option<String>,
    /// Tools that were approved
    pub approved_tools: Vec<String>,
    /// NPC ID (speaker)
    pub npc_id: Option<String>,
    /// NPC name (speaker)
    pub npc_name: Option<String>,
    /// Conversation ID (for dialogue tracking)
    pub conversation_id: Option<ConversationId>,
}

/// Approve LLM suggestion use case.
///
/// Handles DM approval of LLM-generated content (dialogue, tool calls).
pub struct ApproveSuggestion {
    queue: Arc<dyn QueuePort>,
}

impl ApproveSuggestion {
    pub fn new(queue: Arc<dyn QueuePort>) -> Self {
        Self { queue }
    }

    /// Process a DM decision on an LLM suggestion.
    ///
    /// # Arguments
    /// * `approval_queue_id` - The ID of the approval queue item
    /// * `decision` - The DM's decision (accept, modify, reject, takeover)
    ///
    /// # Returns
    /// * `Ok(SuggestionApprovalResult)` - Decision was processed
    /// * `Err(ApprovalError)` - Failed to process decision
    pub async fn execute(
        &self,
        approval_queue_id: QueueItemId,
        decision: DmApprovalDecision,
    ) -> Result<SuggestionApprovalResult, ApprovalError> {
        // Get the queue item first to extract NPC info
        let queue_item: Option<crate::queue_types::ApprovalRequestData> =
            self.queue.get_approval_request(approval_queue_id).await?;

        let (npc_id, npc_name, original_dialogue, conversation_id) = queue_item
            .map(|data| {
                (
                    data.npc_id.map(|id| id.to_string()),
                    Some(data.npc_name),
                    Some(data.proposed_dialogue),
                    data.conversation_id.map(ConversationId::from),
                )
            })
            .unwrap_or((None, None, None, None));

        let (approved, final_dialogue, approved_tools) = match &decision {
            DmApprovalDecision::Accept => (true, original_dialogue, vec![]),
            DmApprovalDecision::AcceptWithRecipients { .. } => {
                // Item distribution handled separately
                (true, original_dialogue, vec![])
            }
            DmApprovalDecision::AcceptWithModification {
                modified_dialogue,
                approved_tools,
                ..
            } => (
                true,
                Some(modified_dialogue.clone()),
                approved_tools.clone(),
            ),
            DmApprovalDecision::Reject { .. } => (false, None, vec![]),
            DmApprovalDecision::TakeOver { dm_response } => {
                (true, Some(dm_response.clone()), vec![])
            }
        };

        // Mark the queue item based on decision
        if approved {
            self.queue.mark_complete(approval_queue_id).await?;
        } else {
            self.queue
                .mark_failed(approval_queue_id, "Rejected by DM")
                .await?;
        }

        Ok(SuggestionApprovalResult {
            suggestion_id: approval_queue_id,
            approved,
            final_dialogue,
            approved_tools,
            npc_id,
            npc_name,
            conversation_id,
        })
    }
}

/// Full approval decision flow (approval + dialogue persistence + tool execution + time suggestion).
pub struct ApprovalDecisionFlow {
    approve_suggestion: Arc<ApproveSuggestion>,
    narrative: Arc<crate::use_cases::narrative_operations::NarrativeOps>,
    queue: Arc<dyn QueuePort>,
    tool_executor: Arc<tool_executor::ToolExecutor>,
    suggest_time: Arc<SuggestTime>,
    world: Arc<dyn WorldRepo>,
    player_character: Arc<dyn PlayerCharacterRepo>,
}

impl ApprovalDecisionFlow {
    pub fn new(
        approve_suggestion: Arc<ApproveSuggestion>,
        narrative: Arc<crate::use_cases::narrative_operations::NarrativeOps>,
        queue: Arc<dyn QueuePort>,
        tool_executor: Arc<tool_executor::ToolExecutor>,
        suggest_time: Arc<SuggestTime>,
        world: Arc<dyn WorldRepo>,
        player_character: Arc<dyn PlayerCharacterRepo>,
    ) -> Self {
        Self {
            approve_suggestion,
            narrative,
            queue,
            tool_executor,
            suggest_time,
            world,
            player_character,
        }
    }

    pub async fn execute(
        &self,
        approval_id: QueueItemId,
        decision: DmApprovalDecision,
    ) -> Result<ApprovalDecisionOutcome, ApprovalDecisionError> {
        let approval_data: crate::queue_types::ApprovalRequestData = self
            .queue
            .get_approval_request(approval_id)
            .await?
            .ok_or(ApprovalDecisionError::ApprovalNotFound(approval_id))?;

        let result = self
            .approve_suggestion
            .execute(approval_id, decision)
            .await
            .map_err(ApprovalDecisionError::Approval)?;

        let mut time_suggestion: Option<crate::infrastructure::ports::TimeSuggestion> = None;

        if result.approved {
            // Record dialogue exchange
            // Note: Dialogue recording is non-critical - if it fails, the approval still completes
            // Tools are executed first, and dialogue is for narrative history only
            let dialogue = result.final_dialogue.clone().unwrap_or_default();
            if !dialogue.is_empty() {
                if let (Some(pc_id), Some(npc_id)) = (approval_data.pc_id, approval_data.npc_id) {
                    let player_dialogue = approval_data.player_dialogue.clone().unwrap_or_default();
                    if let Err(e) = self
                        .narrative
                        .record_dialogue_exchange(
                            approval_data.world_id,
                            pc_id,
                            npc_id,
                            approval_data.npc_name.clone(),
                            player_dialogue,
                            dialogue,
                            approval_data.topics.clone(),
                            approval_data.scene_id,
                            approval_data.location_id,
                            approval_data.game_time.clone(),
                        )
                        .await
                    {
                        // Log but don't fail - dialogue recording is for history, not core functionality
                        tracing::error!(
                            error = %e,
                            npc_id = %npc_id,
                            scene_id = ?approval_data.scene_id,
                            "Failed to record dialogue exchange (non-critical, approval completed successfully)"
                        );
                    }

                    // Emit time suggestion for dialogue approval (US-TIME-013)
                    // Only for dialogue approvals (when npc_id is present)
                    if let Some(pc_id) = approval_data.pc_id {
                        // Get player character name from repository
                        let pc = self
                            .player_character
                            .get(pc_id)
                            .await?
                            .ok_or(ApprovalDecisionError::PlayerCharacterNotFound(
                                pc_id,
                            ))?;
                        let pc_name = pc.name().as_str().to_string();

                        let action_description =
                            format!("Conversation with {}", approval_data.npc_name);

                        match self
                            .suggest_time
                            .execute(
                                approval_data.world_id,
                                pc_id,
                                pc_name,
                                "conversation",
                                action_description,
                            )
                            .await?
                        {
                            crate::use_cases::time::SuggestTimeResult::SuggestionCreated(
                                suggestion,
                            ) => {
                                time_suggestion = Some(suggestion);
                            }
                            crate::use_cases::time::SuggestTimeResult::NoCost => {
                                // No cost configured for conversation - do nothing
                            }
                            crate::use_cases::time::SuggestTimeResult::ManualMode => {
                                // Manual mode - no suggestions emitted
                            }
                        }
                    }
                }
            }

            // Execute approved tools
            if !result.approved_tools.is_empty() {
                let tool_results = self
                    .tool_executor
                    .execute_approved(
                        &result.approved_tools,
                        &approval_data.proposed_tools,
                        approval_data.pc_id,
                        approval_data.npc_id,
                    )
                    .await;

                if !tool_results.is_empty() {
                    tracing::info!(
                        approval_id = %approval_id,
                        tools_executed = tool_results.len(),
                        "Executed approved tools"
                    );
                }
            }
        }

        Ok(ApprovalDecisionOutcome {
            world_id: approval_data.world_id,
            approved: result.approved,
            final_dialogue: result.final_dialogue,
            approved_tools: result.approved_tools,
            npc_id: result.npc_id,
            npc_name: result.npc_name,
            conversation_id: result.conversation_id,
            time_suggestion,
        })
    }
}

pub struct ApprovalDecisionOutcome {
    pub world_id: WorldId,
    pub approved: bool,
    pub final_dialogue: Option<String>,
    pub approved_tools: Vec<String>,
    pub npc_id: Option<String>,
    pub npc_name: Option<String>,
    pub conversation_id: Option<ConversationId>,
    /// Time suggestion generated for dialogue approval (US-TIME-013)
    pub time_suggestion: Option<crate::infrastructure::ports::TimeSuggestion>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApprovalError {
    #[error("Item not found")]
    NotFound,
    #[error("Already processed")]
    AlreadyProcessed,
    #[error("Staging was rejected")]
    Rejected,
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[derive(Debug, thiserror::Error)]
pub enum ApprovalDecisionError {
    #[error("Approval request not found: {0}")]
    ApprovalNotFound(QueueItemId),
    #[error("World not found: {0}")]
    WorldNotFound(WorldId),
    #[error("Player character not found: {0}")]
    PlayerCharacterNotFound(wrldbldr_domain::PlayerCharacterId),
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Approval error: {0}")]
    Approval(#[from] ApprovalError),
    #[error("Suggest time error: {0}")]
    SuggestTime(#[from] crate::use_cases::time::SuggestTimeError),
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use chrono::Utc;
    use uuid::Uuid;
    use wrldbldr_domain::{CharacterId, CharacterName, GameTime, GameTimeConfig, TimeMode, TimeCostConfig};
    use crate::infrastructure::ports::{
        ClockPort, MockPlayerCharacterRepo, MockWorldRepo, QueuePort,
    };

    struct FixedClock(chrono::DateTime<chrono::Utc>);

    impl ClockPort for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
    }

    fn build_clock(now: chrono::DateTime<chrono::Utc>) -> Arc<dyn ClockPort> {
        Arc::new(FixedClock(now))
    }

    // Simple queue mock for tests
    struct SimpleQueue {
        approvals: std::sync::Mutex<std::collections::HashMap<Uuid, crate::queue_types::ApprovalRequestData>>,
    }

    impl SimpleQueue {
        fn new() -> Self {
            Self {
                approvals: std::sync::Mutex::new(std::collections::HashMap::new()),
            }
        }

        fn insert_approval(&self, id: Uuid, data: crate::queue_types::ApprovalRequestData) {
            self.approvals.lock().unwrap().insert(id, data);
        }
    }

    #[async_trait::async_trait]
    impl QueuePort for SimpleQueue {
        async fn enqueue_player_action(
            &self,
            _data: &crate::queue_types::PlayerActionData,
        ) -> Result<wrldbldr_domain::QueueItemId, crate::infrastructure::ports::QueueError> {
            Err(crate::infrastructure::ports::QueueError::Error("not implemented".to_string()))
        }

        async fn dequeue_player_action(&self) -> Result<Option<crate::queue_types::QueueItem>, crate::infrastructure::ports::QueueError> {
            Ok(None)
        }

        async fn enqueue_llm_request(&self, _data: &crate::queue_types::LlmRequestData) -> Result<wrldbldr_domain::QueueItemId, crate::infrastructure::ports::QueueError> {
            Err(crate::infrastructure::ports::QueueError::Error("not implemented".to_string()))
        }

        async fn dequeue_llm_request(&self) -> Result<Option<crate::queue_types::QueueItem>, crate::infrastructure::ports::QueueError> {
            Ok(None)
        }

        async fn enqueue_dm_approval(
            &self,
            _data: &crate::queue_types::ApprovalRequestData,
        ) -> Result<wrldbldr_domain::QueueItemId, crate::infrastructure::ports::QueueError> {
            Ok(wrldbldr_domain::QueueItemId::new())
        }

        async fn dequeue_dm_approval(&self) -> Result<Option<crate::queue_types::QueueItem>, crate::infrastructure::ports::QueueError> {
            Ok(None)
        }

        async fn enqueue_asset_generation(
            &self,
            _data: &crate::queue_types::AssetGenerationData,
        ) -> Result<wrldbldr_domain::QueueItemId, crate::infrastructure::ports::QueueError> {
            Err(crate::infrastructure::ports::QueueError::Error("not implemented".to_string()))
        }

        async fn dequeue_asset_generation(&self) -> Result<Option<crate::queue_types::QueueItem>, crate::infrastructure::ports::QueueError> {
            Ok(None)
        }

        async fn mark_complete(&self, _id: wrldbldr_domain::QueueItemId) -> Result<(), crate::infrastructure::ports::QueueError> {
            Ok(())
        }

        async fn mark_failed(&self, _id: wrldbldr_domain::QueueItemId, _error: &str) -> Result<(), crate::infrastructure::ports::QueueError> {
            Ok(())
        }

        async fn get_pending_count(&self, _queue_type: &str) -> Result<usize, crate::infrastructure::ports::QueueError> {
            Ok(0)
        }

        async fn list_by_type(
            &self,
            _queue_type: &str,
            _limit: usize,
        ) -> Result<Vec<crate::queue_types::QueueItem>, crate::infrastructure::ports::QueueError> {
            Ok(vec![])
        }

        async fn set_result_json(
            &self,
            _id: wrldbldr_domain::QueueItemId,
            _result_json: &str,
        ) -> Result<(), crate::infrastructure::ports::QueueError> {
            Ok(())
        }

        async fn cancel_pending_llm_request_by_callback_id(
            &self,
            _callback_id: &str,
        ) -> Result<bool, crate::infrastructure::ports::QueueError> {
            Ok(false)
        }

        async fn get_approval_request(
            &self,
            id: wrldbldr_domain::QueueItemId,
        ) -> Result<Option<crate::queue_types::ApprovalRequestData>, crate::infrastructure::ports::QueueError> {
            Ok(self.approvals.lock().unwrap().get(&id.to_uuid()).cloned())
        }

        async fn get_generation_read_state(
            &self,
            _user_id: &str,
            _world_id: wrldbldr_domain::WorldId,
        ) -> Result<Option<(Vec<String>, Vec<String>)>, crate::infrastructure::ports::QueueError> {
            Ok(None)
        }

        async fn upsert_generation_read_state(
            &self,
            _user_id: &str,
            _world_id: wrldbldr_domain::WorldId,
            _read_batches: &[String],
            _read_suggestions: &[String],
        ) -> Result<(), crate::infrastructure::ports::QueueError> {
            Ok(())
        }

        async fn delete_by_callback_id(&self, _callback_id: &str) -> Result<bool, crate::infrastructure::ports::QueueError> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn test_dialogue_approval_emits_time_suggestion_when_mode_is_suggested() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let user_id = wrldbldr_domain::UserId::new();
        let pc_id = wrldbldr_domain::PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let approval_id = QueueItemId::new();
        let conversation_id = Uuid::new_v4();
        let location_id = wrldbldr_domain::LocationId::new();

        // Set up world with Suggested time mode and conversation cost
        let time_config = GameTimeConfig::new();
        time_config.set_mode(TimeMode::Suggested);
        time_config.set_time_costs(TimeCostConfig {
            conversation: 300, // 5 minutes
            ..Default::default()
        });

        let world_name = wrldbldr_domain::value_objects::WorldName::new("TestWorld").unwrap();
        let world = wrldbldr_domain::aggregates::World::new(world_name, now)
            .with_id(world_id)
            .with_time_config(time_config)
            .with_game_time(GameTime::at_epoch());

        let mut world_repo = MockWorldRepo::new();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world.clone())));

        // Set up player character
        let pc_name = CharacterName::new("TestPC").unwrap();
        let pc = wrldbldr_domain::aggregates::PlayerCharacter::new(
            user_id,
            world_id,
            pc_name.clone(),
            location_id,
            now,
        );
        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc.clone())));

        // Set up queue
        let approval_data = crate::queue_types::ApprovalRequestData {
            world_id,
            source_action_id: Uuid::new_v4(),
            decision_type: crate::queue_types::ApprovalDecisionType::NpcResponse,
            urgency: crate::queue_types::ApprovalUrgency::Normal,
            pc_id: Some(pc_id),
            npc_id: Some(npc_id),
            npc_name: "TestNPC".to_string(),
            proposed_dialogue: "Hello there!".to_string(),
            internal_reasoning: "NPC greeting".to_string(),
            proposed_tools: vec![],
            retry_count: 0,
            challenge_suggestion: None,
            narrative_event_suggestion: None,
            challenge_outcome: None,
            player_dialogue: Some("Hi!".to_string()),
            scene_id: None,
            location_id: Some(location_id),
            game_time: None,
            topics: vec![],
            conversation_id: Some(conversation_id),
        };

        let queue = Arc::new(SimpleQueue::new());
        queue.insert_approval(approval_id.to_uuid(), approval_data);

        // Set up narrative repos and real NarrativeOps
        let narrative = Arc::new(crate::use_cases::narrative_operations::NarrativeOps::new(
            Arc::new(crate::infrastructure::ports::MockNarrativeRepo::new()),
            Arc::new(crate::infrastructure::ports::MockLocationRepo::new()),
            Arc::new(world_repo.clone()),
            Arc::new(pc_repo.clone()),
            Arc::new(crate::infrastructure::ports::MockCharacterRepo::new()),
            Arc::new(crate::infrastructure::ports::MockObservationRepo::new()),
            Arc::new(crate::infrastructure::ports::MockChallengeRepo::new()),
            Arc::new(crate::infrastructure::ports::MockFlagRepo::new()),
            Arc::new(crate::infrastructure::ports::MockSceneRepo::new()),
            build_clock(now),
        ));

        let tool_executor = Arc::new(tool_executor::ToolExecutor::new(
            Arc::new(crate::infrastructure::ports::MockItemRepo::new()),
            Arc::new(pc_repo.clone()),
            Arc::new(crate::infrastructure::ports::MockCharacterRepo::new()),
        ));

        // Set up real SuggestTime with mocked repos
        let suggest_time = Arc::new(crate::use_cases::time::SuggestTime::new(
            Arc::new(world_repo.clone()),
            build_clock(now),
        ));

        let world_name = wrldbldr_domain::value_objects::WorldName::new("TestWorld").unwrap();
        let world = wrldbldr_domain::aggregates::World::new(world_name, now)
            .with_id(world_id)
            .with_time_config(time_config)
            .with_game_time(GameTime::at_epoch());

        let mut world_repo = MockWorldRepo::new();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world.clone())));

        // Set up player character
        let pc_name = CharacterName::new("TestPC").unwrap();
        let pc = wrldbldr_domain::aggregates::PlayerCharacter::new(
            world_id,
            pc_name.clone(),
            "Fighter".to_string(),
        );
        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc.clone())));

        // Set up queue
        let approval_data = crate::queue_types::ApprovalRequestData {
            world_id,
            source_action_id: Uuid::new_v4(),
            decision_type: crate::queue_types::ApprovalDecisionType::NpcResponse,
            urgency: crate::queue_types::ApprovalUrgency::Normal,
            pc_id: Some(pc_id),
            npc_id: Some(npc_id),
            npc_name: "TestNPC".to_string(),
            proposed_dialogue: "Hello there!".to_string(),
            internal_reasoning: "NPC greeting".to_string(),
            proposed_tools: vec![],
            retry_count: 0,
            challenge_suggestion: None,
            narrative_event_suggestion: None,
            challenge_outcome: None,
            player_dialogue: Some("Hi!".to_string()),
            scene_id: None,
            location_id: None,
            game_time: None,
            topics: vec![],
            conversation_id: Some(conversation_id),
        };

        let mut queue = MockQueuePort::new();
        queue
            .expect_get_approval_request()
            .withf(move |id| *id == approval_id)
            .returning(move |_| Ok(Some(approval_data.clone())));
        queue
            .expect_mark_complete()
            .returning(|_| Ok(()));

        // Set up mock narrative and tool executor
        let mut narrative = crate::use_cases::narrative_operations::MockNarrativeOps::new();
        narrative.expect_record_dialogue_exchange().returning(|_, _, _, _, _, _, _, _, _, _| Ok(()));

        let tool_executor = Arc::new(tool_executor::ToolExecutor::new(
            Arc::new(crate::infrastructure::ports::MockItemRepo::new()),
            Arc::new(MockPlayerCharacterRepo::new()),
            Arc::new(crate::infrastructure::ports::MockCharacterRepo::new()),
        ));

        // Set up SuggestTime mock
        let mut suggest_time = crate::use_cases::time::MockSuggestTime::new();
        let expected_description = "Conversation with TestNPC".to_string();
        suggest_time
            .expect_execute()
            .withf(move |wid, pc, _, action_type, desc| {
                *wid == world_id && *pc == pc_id && action_type == "conversation" && *desc == expected_description
            })
            .returning(move |_, _, _, _, _| {
                Ok(crate::use_cases::time::SuggestTimeResult::SuggestionCreated(
                    crate::infrastructure::ports::TimeSuggestion {
                        id: wrldbldr_domain::TimeSuggestionId::new(),
                        world_id,
                        pc_id,
                        pc_name: pc_name.as_str().to_string(),
                        action_type: "conversation".to_string(),
                        action_description: "Conversation with TestNPC".to_string(),
                        suggested_seconds: 300,
                        current_time: GameTime::at_epoch(),
                        resulting_time: GameTime::at_epoch(),
                        period_change: None,
                    },
                ))
            });

        let clock = build_clock(now);

        let approve_suggestion = Arc::new(ApproveSuggestion::new(queue.clone()));
        let flow = ApprovalDecisionFlow::new(
            approve_suggestion,
            narrative,
            queue.clone(),
            tool_executor,
            suggest_time,
            Arc::new(world_repo),
            Arc::new(pc_repo),
        );

        let result = flow
            .execute(approval_id, crate::queue_types::DmApprovalDecision::Accept)
            .await
            .expect("Approval should succeed");

        assert!(result.approved, "Approval should be accepted");
        assert_eq!(result.final_dialogue, Some("Hello there!".to_string()));
        assert!(result.time_suggestion.is_some(), "Time suggestion should be emitted");
        let suggestion = result.time_suggestion.unwrap();
        assert_eq!(suggestion.suggested_seconds, 300);
    }

    #[tokio::test]
    async fn test_dialogue_approval_no_suggestion_when_mode_is_manual() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let user_id = wrldbldr_domain::UserId::new();
        let location_id = wrldbldr_domain::LocationId::new();
        let pc_id = wrldbldr_domain::PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let approval_id = QueueItemId::new();
        let conversation_id = Uuid::new_v4();

        // Set up world with Manual time mode
        let time_config = GameTimeConfig::new();
        time_config.set_mode(TimeMode::Manual);
        time_config.set_time_costs(TimeCostConfig {
            conversation: 300,
            ..Default::default()
        });

        let world_name = wrldbldr_domain::value_objects::WorldName::new("TestWorld").unwrap();
        let world = wrldbldr_domain::aggregates::World::new(world_name, now)
            .with_id(world_id)
            .with_time_config(time_config);

        let mut world_repo = MockWorldRepo::new();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world.clone())));

        let pc_name = CharacterName::new("TestPC").unwrap();
        let pc = wrldbldr_domain::aggregates::PlayerCharacter::new(
            user_id,
            world_id,
            pc_name,
            location_id,
            now,
        );
        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc.clone())));

        let approval_data = crate::queue_types::ApprovalRequestData {
            world_id,
            source_action_id: Uuid::new_v4(),
            decision_type: crate::queue_types::ApprovalDecisionType::NpcResponse,
            urgency: crate::queue_types::ApprovalUrgency::Normal,
            pc_id: Some(pc_id),
            npc_id: Some(npc_id),
            npc_name: "TestNPC".to_string(),
            proposed_dialogue: "Hello there!".to_string(),
            internal_reasoning: "NPC greeting".to_string(),
            proposed_tools: vec![],
            retry_count: 0,
            challenge_suggestion: None,
            narrative_event_suggestion: None,
            challenge_outcome: None,
            player_dialogue: Some("Hi!".to_string()),
            scene_id: None,
            location_id: None,
            game_time: None,
            topics: vec![],
            conversation_id: Some(conversation_id),
        };

        let queue = Arc::new(SimpleQueue::new());
        queue.insert_approval(approval_id.to_uuid(), approval_data);

        let narrative = Arc::new(crate::use_cases::narrative_operations::NarrativeOps::new(
            Arc::new(crate::infrastructure::ports::MockNarrativeRepo::new()),
            Arc::new(crate::infrastructure::ports::MockLocationRepo::new()),
            Arc::new(world_repo.clone()),
            Arc::new(pc_repo.clone()),
            Arc::new(crate::infrastructure::ports::MockCharacterRepo::new()),
            Arc::new(crate::infrastructure::ports::MockObservationRepo::new()),
            Arc::new(crate::infrastructure::ports::MockChallengeRepo::new()),
            Arc::new(crate::infrastructure::ports::MockFlagRepo::new()),
            Arc::new(crate::infrastructure::ports::MockSceneRepo::new()),
            build_clock(now),
        ));

        let tool_executor = Arc::new(tool_executor::ToolExecutor::new(
            Arc::new(crate::infrastructure::ports::MockItemRepo::new()),
            Arc::new(pc_repo.clone()),
            Arc::new(crate::infrastructure::ports::MockCharacterRepo::new()),
        ));

        // In Manual mode, suggest_time returns ManualMode
        let suggest_time = Arc::new(crate::use_cases::time::SuggestTime::new(
            Arc::new(world_repo.clone()),
            build_clock(now),
        ));

        let clock = build_clock(now);

        let approve_suggestion = Arc::new(ApproveSuggestion::new(Arc::new(queue.clone())));
        let flow = ApprovalDecisionFlow::new(
            approve_suggestion,
            Arc::new(narrative),
            Arc::new(queue.clone()),
            tool_executor,
            Arc::new(suggest_time),
            Arc::new(world_repo),
            Arc::new(pc_repo),
        );

        let result = flow
            .execute(approval_id, crate::queue_types::DmApprovalDecision::Accept)
            .await
            .expect("Approval should succeed");

        assert!(result.approved);
        assert!(result.time_suggestion.is_none(), "No time suggestion in Manual mode");
    }

    #[tokio::test]
    async fn test_dialogue_approval_no_suggestion_when_cost_is_zero() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let pc_id = wrldbldr_domain::PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let approval_id = QueueItemId::new();
        let conversation_id = Uuid::new_v4();

        // Set up world with Suggested mode but zero conversation cost
        let mut time_config = GameTimeConfig::new();
        time_config.set_mode(TimeMode::Suggested);
        time_config.set_time_costs(TimeCostConfig {
            conversation: 0, // No cost
            ..Default::default()
        });

        let world_name = wrldbldr_domain::value_objects::WorldName::new("TestWorld").unwrap();
        let world = wrldbldr_domain::aggregates::World::new(world_name, now)
            .with_id(world_id)
            .with_time_config(time_config);

        let mut world_repo = MockWorldRepo::new();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(move |_| Ok(Some(world.clone())));

        let pc_name = CharacterName::new("TestPC").unwrap();
        let pc = wrldbldr_domain::aggregates::PlayerCharacter::new(                        user_id,                    location_id,now);
        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc.clone())));

        let approval_data = crate::queue_types::ApprovalRequestData {
            world_id,
            source_action_id: Uuid::new_v4(),
            decision_type: crate::queue_types::ApprovalDecisionType::NpcResponse,
            urgency: crate::queue_types::ApprovalUrgency::Normal,
            pc_id: Some(pc_id),
            npc_id: Some(npc_id),
            npc_name: "TestNPC".to_string(),
            proposed_dialogue: "Hello there!".to_string(),
            internal_reasoning: "NPC greeting".to_string(),
            proposed_tools: vec![],
            retry_count: 0,
            challenge_suggestion: None,
            narrative_event_suggestion: None,
            challenge_outcome: None,
            player_dialogue: Some("Hi!".to_string()),
            scene_id: None,
            location_id: None,
            game_time: None,
            topics: vec![],
            conversation_id: Some(conversation_id),
        };

        let mut queue = MockQueuePort::new();
        queue
            .expect_get_approval_request()
            .withf(move |id| *id == approval_id)
            .returning(move |_| Ok(Some(approval_data.clone())));
        queue
            .expect_mark_complete()
            .returning(|_| Ok(()));

        let mut narrative = crate::use_cases::narrative_operations::MockNarrativeOps::new();
        narrative.expect_record_dialogue_exchange().returning(|_, _, _, _, _, _, _, _, _, _| Ok(()));

        let tool_executor = Arc::new(tool_executor::ToolExecutor::new(
            Arc::new(crate::infrastructure::ports::MockItemRepo::new()),
            Arc::new(MockPlayerCharacterRepo::new()),
            Arc::new(crate::infrastructure::ports::MockCharacterRepo::new()),
        ));

        let mut suggest_time = crate::use_cases::time::MockSuggestTime::new();
        // When cost is zero, should return NoCost
        suggest_time
            .expect_execute()
            .returning(|_, _, _, _, _| {
                Ok(crate::use_cases::time::SuggestTimeResult::NoCost)
            });

        let clock = build_clock(now);

        let approve_suggestion = Arc::new(ApproveSuggestion::new(Arc::new(queue.clone())));
        let flow = ApprovalDecisionFlow::new(
            approve_suggestion,
            Arc::new(narrative),
            Arc::new(queue.clone()),
            tool_executor,
            Arc::new(suggest_time),
            Arc::new(world_repo),
            Arc::new(pc_repo),
        );

        let result = flow
            .execute(approval_id, crate::queue_types::DmApprovalDecision::Accept)
            .await
            .expect("Approval should succeed");

        assert!(result.approved);
        assert!(result.time_suggestion.is_none(), "No time suggestion when cost is zero");
    }

    #[tokio::test]
    async fn test_dialogue_approval_no_suggestion_on_reject() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let pc_id = wrldbldr_domain::PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let approval_id = QueueItemId::new();
        let conversation_id = Uuid::new_v4();

        let time_config = GameTimeConfig::new();
        let world_name = wrldbldr_domain::value_objects::WorldName::new("TestWorld").unwrap();
        let world = wrldbldr_domain::aggregates::World::new(world_name, now)
            .with_id(world_id)
            .with_time_config(time_config);

        let mut world_repo = MockWorldRepo::new();
        world_repo
            .expect_get()
            .returning(move |_| Ok(Some(world.clone())));

        let pc_name = CharacterName::new("TestPC").unwrap();
        let pc = wrldbldr_domain::aggregates::PlayerCharacter::new(                        user_id,                    location_id,now);
        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc.clone())));

        let approval_data = crate::queue_types::ApprovalRequestData {
            world_id,
            source_action_id: Uuid::new_v4(),
            decision_type: crate::queue_types::ApprovalDecisionType::NpcResponse,
            urgency: crate::queue_types::ApprovalUrgency::Normal,
            pc_id: Some(pc_id),
            npc_id: Some(npc_id),
            npc_name: "TestNPC".to_string(),
            proposed_dialogue: "Hello there!".to_string(),
            internal_reasoning: "NPC greeting".to_string(),
            proposed_tools: vec![],
            retry_count: 0,
            challenge_suggestion: None,
            narrative_event_suggestion: None,
            challenge_outcome: None,
            player_dialogue: Some("Hi!".to_string()),
            scene_id: None,
            location_id: None,
            game_time: None,
            topics: vec![],
            conversation_id: Some(conversation_id),
        };

        let mut queue = MockQueuePort::new();
        queue
            .expect_get_approval_request()
            .withf(move |id| *id == approval_id)
            .returning(move |_| Ok(Some(approval_data.clone())));
        queue
            .expect_mark_failed()
            .returning(|_, _| Ok(()));

        let narrative = crate::use_cases::narrative_operations::MockNarrativeOps::new();
        let tool_executor = Arc::new(tool_executor::ToolExecutor::new(
            Arc::new(crate::infrastructure::ports::MockItemRepo::new()),
            Arc::new(MockPlayerCharacterRepo::new()),
            Arc::new(crate::infrastructure::ports::MockCharacterRepo::new()),
        ));

        // SuggestTime should NOT be called when approval is rejected
        let suggest_time = crate::use_cases::time::MockSuggestTime::new();

        let clock = build_clock(now);

        let approve_suggestion = Arc::new(ApproveSuggestion::new(Arc::new(queue.clone())));
        let flow = ApprovalDecisionFlow::new(
            approve_suggestion,
            Arc::new(narrative),
            Arc::new(queue.clone()),
            tool_executor,
            Arc::new(suggest_time),
            Arc::new(world_repo),
            Arc::new(pc_repo),
        );

        let result = flow
            .execute(
                approval_id,
                crate::queue_types::DmApprovalDecision::Reject {
                    feedback: "No thanks".to_string(),
                },
            )
            .await
            .expect("Approval should succeed");

        assert!(!result.approved, "Approval should be rejected");
        assert!(result.time_suggestion.is_none(), "No time suggestion on reject");
    }
}

