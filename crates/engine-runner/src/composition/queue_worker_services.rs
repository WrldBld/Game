//! Concrete queue services needed by engine-runner background workers.
//!
//! These are intentionally concrete because worker loops need access to
//! implementation-specific helper methods (e.g., `run_worker`, `process_next`).
//! Keeping these out of `composition/factories/**` allows Phase 7 composition
//! factories to stay port-typed while still enabling efficient workers.

use std::sync::Arc;

use wrldbldr_engine_adapters::infrastructure::{
    comfyui::ComfyUIClient,
    ollama::OllamaClient,
    queues::{InProcessNotifier, QueueBackendEnum},
};
use wrldbldr_engine_app::application::services::{
    AssetGenerationQueueService, DMApprovalQueueService, DmActionQueueService, ItemServiceImpl,
    LLMQueueService, PlayerActionQueueService,
};
use wrldbldr_engine_ports::outbound::DmActionProcessorPort;
use wrldbldr_engine_ports::outbound::{
    ApprovalRequestData, AssetGenerationData, ChallengeOutcomeData, DmActionData, LlmRequestData,
    PlayerActionData,
};

/// Concrete queue services used by background workers.
#[derive(Clone)]
pub struct QueueWorkerServices {
    pub llm_queue_service:
        Arc<LLMQueueService<QueueBackendEnum<LlmRequestData>, OllamaClient, InProcessNotifier>>,
    pub asset_generation_queue_service: Arc<
        AssetGenerationQueueService<
            QueueBackendEnum<AssetGenerationData>,
            ComfyUIClient,
            InProcessNotifier,
        >,
    >,
    pub player_action_queue_service: Arc<
        PlayerActionQueueService<
            QueueBackendEnum<PlayerActionData>,
            QueueBackendEnum<LlmRequestData>,
        >,
    >,
    pub dm_action_queue_service: Arc<DmActionQueueService<QueueBackendEnum<DmActionData>>>,
    pub dm_approval_queue_service:
        Arc<DMApprovalQueueService<QueueBackendEnum<ApprovalRequestData>, ItemServiceImpl>>,
    pub challenge_outcome_queue: Arc<QueueBackendEnum<ChallengeOutcomeData>>,
    pub dm_action_processor: Arc<dyn DmActionProcessorPort>,
}
