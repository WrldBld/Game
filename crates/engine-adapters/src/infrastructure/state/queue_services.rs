//! Queue processing services

use std::sync::Arc;

use crate::infrastructure::comfyui::ComfyUIClient;
use crate::infrastructure::ollama::OllamaClient;
use crate::infrastructure::queues::QueueBackendEnum;
use wrldbldr_domain::value_objects::{
    ApprovalRequestData, AssetGenerationData, ChallengeOutcomeData, DmActionData, LlmRequestData, PlayerActionData,
};
use wrldbldr_engine_app::application::services::{
    AssetGenerationQueueService, DMApprovalQueueService, DmActionQueueService, ItemServiceImpl,
    LLMQueueService, PlayerActionQueueService,
};

/// Queue processing services for asynchronous operations
///
/// This struct groups all queue-related services that handle background
/// processing of player actions, DM actions, LLM requests, asset generation,
/// and approval workflows.
pub struct QueueServices {
    pub player_action_queue_service: Arc<
        PlayerActionQueueService<
            QueueBackendEnum<PlayerActionData>,
            QueueBackendEnum<LlmRequestData>,
        >,
    >,
    pub dm_action_queue_service: Arc<DmActionQueueService<QueueBackendEnum<DmActionData>>>,
    pub llm_queue_service: Arc<
        LLMQueueService<
            QueueBackendEnum<LlmRequestData>,
            OllamaClient,
            crate::infrastructure::queues::InProcessNotifier,
        >,
    >,
    pub asset_generation_queue_service: Arc<
        AssetGenerationQueueService<
            QueueBackendEnum<AssetGenerationData>,
            ComfyUIClient,
            crate::infrastructure::queues::InProcessNotifier,
        >,
    >,
    pub dm_approval_queue_service:
        Arc<DMApprovalQueueService<QueueBackendEnum<ApprovalRequestData>, ItemServiceImpl>>,
    /// Queue for challenge outcomes awaiting DM approval
    pub challenge_outcome_queue: Arc<QueueBackendEnum<ChallengeOutcomeData>>,
}

impl QueueServices {
    /// Creates a new QueueServices instance with all queue processing services
    pub fn new(
        player_action_queue_service: Arc<
            PlayerActionQueueService<
                QueueBackendEnum<PlayerActionData>,
                QueueBackendEnum<LlmRequestData>,
            >,
        >,
        dm_action_queue_service: Arc<DmActionQueueService<QueueBackendEnum<DmActionData>>>,
        llm_queue_service: Arc<
            LLMQueueService<
                QueueBackendEnum<LlmRequestData>,
                OllamaClient,
                crate::infrastructure::queues::InProcessNotifier,
            >,
        >,
        asset_generation_queue_service: Arc<
            AssetGenerationQueueService<
                QueueBackendEnum<AssetGenerationData>,
                ComfyUIClient,
                crate::infrastructure::queues::InProcessNotifier,
            >,
        >,
        dm_approval_queue_service: Arc<
            DMApprovalQueueService<QueueBackendEnum<ApprovalRequestData>, ItemServiceImpl>,
        >,
        challenge_outcome_queue: Arc<QueueBackendEnum<ChallengeOutcomeData>>,
    ) -> Self {
        Self {
            player_action_queue_service,
            dm_action_queue_service,
            llm_queue_service,
            asset_generation_queue_service,
            dm_approval_queue_service,
            challenge_outcome_queue,
        }
    }
}
