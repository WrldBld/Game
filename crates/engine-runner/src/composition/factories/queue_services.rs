//! Queue Services Factory
//!
//! Creates queue infrastructure and queue services.
//! This is Level 2b - can run in parallel with event_infra for queue backend creation,
//! but queue SERVICES need Level 3+ dependencies.
//!
//! Provides both port-typed services (for AppState) and concrete-typed services
//! (for WorkerServices that need `run_worker()` method access).

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;

use wrldbldr_engine_ports::outbound::{
    ApprovalRequestData, AssetGenerationData, ChallengeOutcomeData, DmActionData, LlmRequestData,
    PlayerActionData,
};
use wrldbldr_engine_adapters::infrastructure::{
    comfyui::ComfyUIClient,
    config::AppConfig,
    ollama::OllamaClient,
    queues::{QueueBackendEnum, QueueFactory},
    TokioFileStorageAdapter,
};
use wrldbldr_engine_app::application::services::{
    generation_service::GenerationEvent, AssetGenerationQueueService, DMApprovalQueueService,
    DmActionProcessorService, DmActionQueueService, InteractionService, ItemServiceImpl,
    LLMQueueService, NarrativeEventService, PlayerActionQueueService, SceneService,
};
use wrldbldr_engine_ports::outbound::{
    AssetGenerationQueueServicePort, ClockPort, DmActionProcessorPort, DmActionQueueServicePort,
    DmApprovalQueueServicePort, FileStoragePort, LlmQueueServicePort, PlayerActionQueueServicePort,
    QueuePort,
};

use super::repositories::RepositoryPorts;
use crate::composition::queue_worker_services::QueueWorkerServices;

/// Queue backends created from QueueFactory.
///
/// This is created first (Level 2b) and can run in parallel with event_infra.
pub struct QueueBackends {
    pub player_action_queue: Arc<QueueBackendEnum<PlayerActionData>>,
    pub llm_queue: Arc<QueueBackendEnum<LlmRequestData>>,
    pub dm_action_queue: Arc<QueueBackendEnum<DmActionData>>,
    pub asset_generation_queue: Arc<QueueBackendEnum<AssetGenerationData>>,
    pub approval_queue: Arc<QueueBackendEnum<ApprovalRequestData>>,
    pub challenge_outcome_queue: Arc<QueueBackendEnum<ChallengeOutcomeData>>,
    queue_factory: QueueFactory,
}

/// Creates queue backends (can run in parallel with event_infra).
pub async fn create_queue_backends(
    config: &AppConfig,
    clock: Arc<dyn ClockPort>,
) -> Result<QueueBackends> {
    let queue_factory = QueueFactory::new(config.queue.clone(), clock).await?;
    tracing::info!("Queue backend: {}", queue_factory.config().backend);

    let player_action_queue = queue_factory.create_player_action_queue().await?;
    let llm_queue = queue_factory.create_llm_queue().await?;
    let dm_action_queue = queue_factory.create_dm_action_queue().await?;
    let asset_generation_queue = queue_factory.create_asset_generation_queue().await?;
    let approval_queue = queue_factory.create_approval_queue().await?;
    let challenge_outcome_queue = queue_factory.create_challenge_outcome_queue().await?;

    Ok(QueueBackends {
        player_action_queue,
        llm_queue,
        dm_action_queue,
        asset_generation_queue,
        approval_queue,
        challenge_outcome_queue,
        queue_factory,
    })
}

/// Queue service ports for AppState (ports only).
pub struct QueueServicePorts {
    pub player_action_queue_service_port: Arc<dyn PlayerActionQueueServicePort>,
    pub dm_action_queue_service_port: Arc<dyn DmActionQueueServicePort>,
    pub llm_queue_service_port: Arc<dyn LlmQueueServicePort>,
    pub asset_generation_queue_service_port: Arc<dyn AssetGenerationQueueServicePort>,
    pub dm_approval_queue_service_port: Arc<dyn DmApprovalQueueServicePort>,
    pub challenge_outcome_queue_port: Arc<dyn QueuePort<ChallengeOutcomeData>>,
}

/// Dependencies for queue service creation (needs core services).
pub struct QueueServiceDependencies<'a> {
    pub config: &'a AppConfig,
    pub clock: Arc<dyn ClockPort>,
    /// Worker-only concrete LLM client (used by LLMQueueService generics)
    pub(crate) llm_client: OllamaClient,
    /// Worker-only concrete ComfyUI client (used by AssetGenerationQueueService generics)
    pub(crate) comfyui_client: ComfyUIClient,
    pub prompt_template_service:
        Arc<dyn wrldbldr_engine_ports::outbound::PromptTemplateServicePort>,
    pub repos: &'a RepositoryPorts,
    pub queue_backends: &'a QueueBackends,
    /// Dialogue context service for recording dialogue exchanges (ISP-split from StoryEventService)
    pub dialogue_context_service:
        Arc<dyn wrldbldr_engine_ports::outbound::DialogueContextServicePort>,
    pub generation_event_tx: mpsc::Sender<GenerationEvent>,
    // App-layer services needed for DmActionProcessorService
    pub narrative_event_service: Arc<dyn NarrativeEventService>,
    pub scene_service: Arc<dyn SceneService>,
    pub interaction_service: Arc<dyn InteractionService>,
}

/// Creates queue services (needs dialogue_context_service from core services).
pub fn create_queue_services(
    deps: QueueServiceDependencies<'_>,
) -> Result<(QueueServicePorts, QueueWorkerServices)> {
    let QueueServiceDependencies {
        config,
        clock,
        llm_client,
        comfyui_client,
        prompt_template_service,
        repos,
        queue_backends,
        dialogue_context_service,
        generation_event_tx,
        narrative_event_service,
        scene_service,
        interaction_service,
    } = deps;

    // =========================================================================
    // Item Service (needed by dm_approval_queue_service)
    // =========================================================================
    let item_service_impl = ItemServiceImpl::new(
        repos.item.clone(),
        repos.player_character.inventory.clone(),
        repos.region.item.clone(),
    );

    // =========================================================================
    // Queue Services
    // =========================================================================
    let player_action_queue_service = Arc::new(PlayerActionQueueService::new(
        queue_backends.player_action_queue.clone(),
        queue_backends.llm_queue.clone(),
        clock.clone(),
    ));

    let dm_action_queue_service = Arc::new(DmActionQueueService::new(
        queue_backends.dm_action_queue.clone(),
        clock.clone(),
    ));

    let llm_client_arc = Arc::new(llm_client);
    let llm_queue_service = Arc::new(LLMQueueService::new(
        queue_backends.llm_queue.clone(),
        llm_client_arc,
        queue_backends.approval_queue.clone(),
        repos.challenge.crud.clone(),
        repos.challenge.skill.clone(),
        repos.skill.clone(),
        repos.narrative_event.crud.clone(),
        queue_backends.queue_factory.config().llm_batch_size,
        queue_backends.queue_factory.llm_notifier(),
        generation_event_tx,
        prompt_template_service,
    ));

    let file_storage: Arc<dyn FileStoragePort> = Arc::new(TokioFileStorageAdapter::new());
    let asset_generation_queue_service = Arc::new(AssetGenerationQueueService::new(
        queue_backends.asset_generation_queue.clone(),
        Arc::new(comfyui_client),
        repos.asset.clone(),
        clock.clone(),
        file_storage,
        config.generated_assets_path.clone(),
        queue_backends.queue_factory.config().asset_batch_size,
        queue_backends.queue_factory.asset_generation_notifier(),
    ));

    let dm_approval_queue_service = Arc::new(DMApprovalQueueService::new(
        queue_backends.approval_queue.clone(),
        dialogue_context_service,
        Arc::new(item_service_impl),
        clock.clone(),
    ));

    // =========================================================================
    // DM Action Processor Service
    // =========================================================================
    // This service handles the business logic for DM actions (approval decisions,
    // direct NPC control, event triggering, scene transitions)
    let dm_action_processor: Arc<dyn DmActionProcessorPort> =
        Arc::new(DmActionProcessorService::new(
            dm_approval_queue_service.clone(),
            narrative_event_service,
            scene_service,
            interaction_service,
            clock,
        ));

    tracing::info!("Initialized queue services");

    let ports = QueueServicePorts {
        player_action_queue_service_port: player_action_queue_service.clone(),
        dm_action_queue_service_port: dm_action_queue_service.clone(),
        llm_queue_service_port: llm_queue_service.clone(),
        asset_generation_queue_service_port: asset_generation_queue_service.clone(),
        dm_approval_queue_service_port: dm_approval_queue_service.clone(),
        challenge_outcome_queue_port: queue_backends.challenge_outcome_queue.clone(),
    };

    let workers = QueueWorkerServices {
        llm_queue_service,
        asset_generation_queue_service,
        player_action_queue_service,
        dm_action_queue_service,
        dm_approval_queue_service,
        challenge_outcome_queue: queue_backends.challenge_outcome_queue.clone(),
        dm_action_processor,
    };

    Ok((ports, workers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_service_ports_types() {
        fn _assert_port_types(ctx: &QueueServicePorts) {
            let _: &Arc<dyn PlayerActionQueueServicePort> = &ctx.player_action_queue_service_port;
            let _: &Arc<dyn DmActionQueueServicePort> = &ctx.dm_action_queue_service_port;
            let _: &Arc<dyn LlmQueueServicePort> = &ctx.llm_queue_service_port;
            let _: &Arc<dyn AssetGenerationQueueServicePort> =
                &ctx.asset_generation_queue_service_port;
            let _: &Arc<dyn DmApprovalQueueServicePort> = &ctx.dm_approval_queue_service_port;
            let _: &Arc<dyn QueuePort<ChallengeOutcomeData>> = &ctx.challenge_outcome_queue_port;
        }
    }
}
