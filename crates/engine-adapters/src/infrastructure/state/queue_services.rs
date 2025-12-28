//! Queue processing services

use std::sync::Arc;

use wrldbldr_engine_app::application::dto::{
    ApprovalItem, AssetGenerationItem, ChallengeOutcomeApprovalItem, DMActionItem, LLMRequestItem,
    PlayerActionItem,
};
use wrldbldr_engine_app::application::services::{
    AssetGenerationQueueService, DMActionQueueService, DMApprovalQueueService, ItemServiceImpl, LLMQueueService,
    PlayerActionQueueService,
};
use crate::infrastructure::comfyui::ComfyUIClient;
use crate::infrastructure::ollama::OllamaClient;
use crate::infrastructure::queues::QueueBackendEnum;

/// Queue processing services for asynchronous operations
///
/// This struct groups all queue-related services that handle background
/// processing of player actions, DM actions, LLM requests, asset generation,
/// and approval workflows.
pub struct QueueServices {
    pub player_action_queue_service: Arc<
        PlayerActionQueueService<
            QueueBackendEnum<PlayerActionItem>,
            QueueBackendEnum<LLMRequestItem>,
        >,
    >,
    pub dm_action_queue_service: Arc<DMActionQueueService<QueueBackendEnum<DMActionItem>>>,
    pub llm_queue_service: Arc<
        LLMQueueService<
            QueueBackendEnum<LLMRequestItem>,
            OllamaClient,
            crate::infrastructure::queues::InProcessNotifier,
        >,
    >,
    pub asset_generation_queue_service: Arc<
        AssetGenerationQueueService<
            QueueBackendEnum<AssetGenerationItem>,
            ComfyUIClient,
            crate::infrastructure::queues::InProcessNotifier,
        >,
    >,
    pub dm_approval_queue_service: Arc<DMApprovalQueueService<QueueBackendEnum<ApprovalItem>, ItemServiceImpl>>,
    /// Queue for challenge outcomes awaiting DM approval
    pub challenge_outcome_queue: Arc<QueueBackendEnum<ChallengeOutcomeApprovalItem>>,
}

impl QueueServices {
    /// Creates a new QueueServices instance with all queue processing services
    pub fn new(
        player_action_queue_service: Arc<
            PlayerActionQueueService<
                QueueBackendEnum<PlayerActionItem>,
                QueueBackendEnum<LLMRequestItem>,
            >,
        >,
        dm_action_queue_service: Arc<DMActionQueueService<QueueBackendEnum<DMActionItem>>>,
        llm_queue_service: Arc<
            LLMQueueService<
                QueueBackendEnum<LLMRequestItem>,
                OllamaClient,
                crate::infrastructure::queues::InProcessNotifier,
            >,
        >,
        asset_generation_queue_service: Arc<
            AssetGenerationQueueService<
                QueueBackendEnum<AssetGenerationItem>,
                ComfyUIClient,
                crate::infrastructure::queues::InProcessNotifier,
            >,
        >,
        dm_approval_queue_service: Arc<DMApprovalQueueService<QueueBackendEnum<ApprovalItem>, ItemServiceImpl>>,
        challenge_outcome_queue: Arc<QueueBackendEnum<ChallengeOutcomeApprovalItem>>,
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
