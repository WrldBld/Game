//! Use Case Adapters - Thin wrappers that adapt internal service ports to inbound use case ports
//!
//! These adapters bridge the gap between:
//! - Internal service ports (in `engine-app/services/internal/`) for app-to-app collaboration
//! - Inbound use case ports (in `engine-ports/inbound/`) for HTTP handler access
//!
//! The adapters simply delegate method calls from use case ports to service ports.
//! This maintains clean hexagonal boundaries while avoiding code duplication.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use wrldbldr_domain::entities::{
    BatchStatus, EntityType, GalleryAsset, GenerationBatch, InputDefault, PromptMapping,
    WorkflowConfiguration, WorkflowSlot,
};
use wrldbldr_domain::value_objects::AppSettings;
use wrldbldr_domain::{AssetId, BatchId, WorkflowConfigId, WorldId};

// Inbound use case ports
use wrldbldr_engine_ports::inbound::{
    ApprovalDecisionType as InboundApprovalDecisionType,
    ApprovalQueueItem as InboundApprovalQueueItem,
    ApprovalRequest as InboundApprovalRequest, ApprovalUrgency as InboundApprovalUrgency,
    AssetGenerationQueueItem as InboundAssetGenerationQueueItem,
    AssetGenerationQueueUseCasePort, AssetGenerationRequest as InboundAssetGenerationRequest,
    AssetUseCasePort, ConfidenceLevel as InboundConfidenceLevel, CreateAssetRequest,
    DmApprovalDecision, DmApprovalQueueUseCasePort,
    GenerationBatchSnapshot as InboundGenerationBatchSnapshot,
    GenerationQueueProjectionUseCasePort, GenerationQueueSnapshot as InboundGenerationQueueSnapshot,
    GenerationRequest, GenerationResult as InboundGenerationResult, GenerationUseCasePort,
    LlmQueueItem as InboundLlmQueueItem, LlmQueueRequest as InboundLlmQueueRequest,
    LlmQueueResponse as InboundLlmQueueResponse, LlmQueueUseCasePort,
    LlmRequestType as InboundLlmRequestType, PlayerAction as InboundPlayerAction,
    PlayerActionQueueItem as InboundPlayerActionQueueItem, PlayerActionQueueUseCasePort,
    PromptTemplateUseCasePort, QueueItemStatus, SettingsUseCasePort,
    SuggestionTaskSnapshot as InboundSuggestionTaskSnapshot, WorkflowUseCasePort, WorldUseCasePort,
};
use wrldbldr_engine_ports::outbound::{
    PromptTemplateError, ResolvedPromptTemplate, SettingsError,
};

// Internal service ports
use wrldbldr_engine_app::application::services::internal::{
    AssetGenerationQueueServicePort, AssetServicePort, DmApprovalQueueServicePort,
    GenerationQueueProjectionServicePort, GenerationServicePort, LlmQueueServicePort,
    PlayerActionQueueServicePort, PromptTemplateServicePort,
    WorkflowServicePort, WorldServicePort,
};

// ============================================================================
// Settings Use Case Adapter
// ============================================================================

/// Adapter that implements SettingsUseCasePort by delegating to SettingsUseCasePort
pub struct SettingsUseCaseAdapter {
    service: Arc<dyn SettingsUseCasePort>,
}

impl SettingsUseCaseAdapter {
    pub fn new(service: Arc<dyn SettingsUseCasePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SettingsUseCasePort for SettingsUseCaseAdapter {
    async fn get(&self) -> AppSettings {
        self.service.get().await
    }

    async fn update(&self, settings: AppSettings) -> Result<(), SettingsError> {
        self.service.update(settings).await
    }

    async fn reset(&self) -> Result<AppSettings, SettingsError> {
        self.service.reset().await
    }

    async fn get_for_world(&self, world_id: WorldId) -> AppSettings {
        self.service.get_for_world(world_id).await
    }

    async fn update_for_world(
        &self,
        world_id: WorldId,
        settings: AppSettings,
    ) -> Result<(), SettingsError> {
        self.service.update_for_world(world_id, settings).await
    }

    async fn reset_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
        self.service.reset_for_world(world_id).await
    }

    async fn delete_for_world(&self, world_id: WorldId) -> Result<(), SettingsError> {
        self.service.delete_for_world(world_id).await
    }

    async fn get_llm_config(
        &self,
        world_id: WorldId,
    ) -> Result<wrldbldr_engine_ports::inbound::LlmConfig, SettingsError> {
        let config = self.service.get_llm_config(world_id).await?;
        Ok(wrldbldr_engine_ports::inbound::LlmConfig {
            api_base_url: config.api_base_url,
            model: config.model,
            api_key: config.api_key,
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        })
    }
}

// ============================================================================
// Prompt Template Use Case Adapter
// ============================================================================

/// Adapter that implements PromptTemplateUseCasePort by delegating to PromptTemplateServicePort
pub struct PromptTemplateUseCaseAdapter {
    service: Arc<dyn PromptTemplateServicePort>,
}

impl PromptTemplateUseCaseAdapter {
    pub fn new(service: Arc<dyn PromptTemplateServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl PromptTemplateUseCasePort for PromptTemplateUseCaseAdapter {
    async fn get_all(&self) -> Vec<ResolvedPromptTemplate> {
        self.service.get_all().await
    }

    async fn set_global(&self, key: &str, value: &str) -> Result<(), PromptTemplateError> {
        self.service.set_global(key, value).await
    }

    async fn delete_global(&self, key: &str) -> Result<(), PromptTemplateError> {
        self.service.delete_global(key).await
    }

    async fn reset_global(&self) -> Result<(), PromptTemplateError> {
        self.service.reset_global().await
    }

    async fn get_all_for_world(&self, world_id: WorldId) -> Vec<ResolvedPromptTemplate> {
        self.service.get_all_for_world(world_id).await
    }

    async fn set_for_world(
        &self,
        world_id: WorldId,
        key: &str,
        value: &str,
    ) -> Result<(), PromptTemplateError> {
        self.service.set_for_world(world_id, key, value).await
    }

    async fn delete_for_world(
        &self,
        world_id: WorldId,
        key: &str,
    ) -> Result<(), PromptTemplateError> {
        self.service.delete_for_world(world_id, key).await
    }

    async fn reset_for_world(&self, world_id: WorldId) -> Result<(), PromptTemplateError> {
        self.service.reset_for_world(world_id).await
    }

    async fn resolve_with_source(&self, key: &str) -> ResolvedPromptTemplate {
        self.service.resolve_with_source(key).await
    }

    async fn resolve_for_world_with_source(
        &self,
        world_id: WorldId,
        key: &str,
    ) -> ResolvedPromptTemplate {
        self.service.resolve_for_world_with_source(world_id, key).await
    }

    fn get_metadata(&self) -> Vec<wrldbldr_domain::value_objects::PromptTemplateMetadata> {
        self.service.get_metadata()
    }
}

// ============================================================================
// Asset Use Case Adapter
// ============================================================================

/// Adapter that implements AssetUseCasePort by delegating to AssetServicePort
pub struct AssetUseCaseAdapter {
    service: Arc<dyn AssetServicePort>,
}

impl AssetUseCaseAdapter {
    pub fn new(service: Arc<dyn AssetServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl AssetUseCasePort for AssetUseCaseAdapter {
    async fn get_asset(&self, asset_id: AssetId) -> Result<Option<GalleryAsset>> {
        self.service.get_asset(asset_id).await
    }

    async fn list_assets(
        &self,
        entity_type: EntityType,
        entity_id: &str,
    ) -> Result<Vec<GalleryAsset>> {
        self.service.list_assets(entity_type, entity_id).await
    }

    async fn create_asset(&self, request: CreateAssetRequest) -> Result<GalleryAsset> {
        let internal_request =
            wrldbldr_engine_app::application::services::internal::CreateAssetRequest {
                entity_type: request.entity_type,
                entity_id: request.entity_id,
                asset_type: request.asset_type,
                file_path: request.file_path,
                label: request.label,
            };
        self.service.create_asset(internal_request).await
    }

    async fn update_asset_label(&self, asset_id: AssetId, label: Option<String>) -> Result<()> {
        self.service.update_asset_label(asset_id, label).await
    }

    async fn delete_asset(&self, asset_id: AssetId) -> Result<()> {
        self.service.delete_asset(asset_id).await
    }

    async fn activate_asset(&self, asset_id: AssetId) -> Result<()> {
        self.service.activate_asset(asset_id).await
    }

    async fn create_batch(&self, batch: GenerationBatch) -> Result<GenerationBatch> {
        self.service.create_batch(batch).await
    }

    async fn get_batch(&self, batch_id: BatchId) -> Result<Option<GenerationBatch>> {
        self.service.get_batch(batch_id).await
    }

    async fn list_active_batches_by_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<GenerationBatch>> {
        self.service.list_active_batches_by_world(world_id).await
    }

    async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>> {
        self.service.list_ready_batches().await
    }

    async fn update_batch_status(&self, batch_id: BatchId, status: BatchStatus) -> Result<()> {
        self.service.update_batch_status(batch_id, status).await
    }

    async fn update_batch_assets(&self, batch_id: BatchId, assets: Vec<AssetId>) -> Result<()> {
        self.service.update_batch_assets(batch_id, assets).await
    }

    async fn delete_batch(&self, batch_id: BatchId) -> Result<()> {
        self.service.delete_batch(batch_id).await
    }
}

// ============================================================================
// Generation Use Case Adapter
// ============================================================================

/// Adapter that implements GenerationUseCasePort by delegating to GenerationServicePort
pub struct GenerationUseCaseAdapter {
    service: Arc<dyn GenerationServicePort>,
}

impl GenerationUseCaseAdapter {
    pub fn new(service: Arc<dyn GenerationServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl GenerationUseCasePort for GenerationUseCaseAdapter {
    async fn generate_asset(&self, request: GenerationRequest) -> Result<GenerationBatch> {
        let internal_request =
            wrldbldr_engine_app::application::services::internal::GenerationRequest {
                world_id: request.world_id,
                entity_type: request.entity_type,
                entity_id: request.entity_id,
                asset_type: request.asset_type,
                prompt: request.prompt,
                negative_prompt: request.negative_prompt,
                count: request.count,
                style_reference_id: request.style_reference_id,
            };
        self.service.generate_asset(internal_request).await
    }

    async fn get_batch(&self, id: BatchId) -> Result<Option<GenerationBatch>> {
        self.service.get_batch(id).await
    }

    async fn select_from_batch(
        &self,
        batch_id: BatchId,
        asset_index: usize,
    ) -> Result<GalleryAsset> {
        self.service.select_from_batch(batch_id, asset_index).await
    }

    async fn start_batch_processing(&self, batch: GenerationBatch) -> Result<()> {
        self.service.start_batch_processing(batch).await
    }
}

// ============================================================================
// Workflow Use Case Adapter
// ============================================================================

/// Adapter that implements WorkflowUseCasePort by delegating to WorkflowServicePort
pub struct WorkflowUseCaseAdapter {
    service: Arc<dyn WorkflowServicePort>,
}

impl WorkflowUseCaseAdapter {
    pub fn new(service: Arc<dyn WorkflowServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl WorkflowUseCasePort for WorkflowUseCaseAdapter {
    async fn get_workflow(&self, id: WorkflowConfigId) -> Result<Option<WorkflowConfiguration>> {
        self.service.get_workflow(id).await
    }

    async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>> {
        self.service.list_all().await
    }

    async fn list_by_slot(&self, slot: WorkflowSlot) -> Result<Vec<WorkflowConfiguration>> {
        self.service.list_by_slot(slot).await
    }

    async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>> {
        self.service.get_by_slot(slot).await
    }

    async fn save(&self, config: &WorkflowConfiguration) -> Result<()> {
        self.service.save(config).await
    }

    async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool> {
        self.service.delete_by_slot(slot).await
    }

    async fn get_active_for_slot(
        &self,
        world_id: WorldId,
        slot: WorkflowSlot,
    ) -> Result<Option<WorkflowConfiguration>> {
        self.service.get_active_for_slot(world_id, slot).await
    }

    async fn create_or_update(
        &self,
        slot: WorkflowSlot,
        name: String,
        workflow_json: serde_json::Value,
        prompt_mappings: Vec<PromptMapping>,
        input_defaults: Vec<InputDefault>,
        locked_inputs: Vec<String>,
    ) -> Result<(WorkflowConfiguration, bool)> {
        self.service
            .create_or_update(slot, name, workflow_json, prompt_mappings, input_defaults, locked_inputs)
            .await
    }

    async fn update_defaults(
        &self,
        slot: WorkflowSlot,
        input_defaults: Vec<InputDefault>,
        locked_inputs: Option<Vec<String>>,
    ) -> Result<WorkflowConfiguration> {
        self.service.update_defaults(slot, input_defaults, locked_inputs).await
    }

    async fn import_configs(
        &self,
        configs: Vec<WorkflowConfiguration>,
        replace_existing: bool,
    ) -> Result<(usize, usize)> {
        self.service.import_configs(configs, replace_existing).await
    }
}

// ============================================================================
// Queue Use Case Adapters
// ============================================================================

// Import internal service port types for conversion
use wrldbldr_engine_app::application::services::internal::{
    ApprovalDecisionType as ServiceApprovalDecisionType,
    ApprovalQueueItem as ServiceApprovalQueueItem,
    ApprovalRequest as ServiceApprovalRequest, ApprovalUrgency as ServiceApprovalUrgency,
    AssetGenerationQueueItem as ServiceAssetGenerationQueueItem,
    AssetGenerationRequest as ServiceAssetGenerationRequest,
    ChallengeSuggestion as ServiceChallengeSuggestion,
    ConfidenceLevel as ServiceConfidenceLevel,
    AssetGenerationMetadata as ServiceGenerationMetadata,
    GenerationQueueSnapshot as ServiceGenerationQueueSnapshot,
    GenerationResult as ServiceGenerationResult, LlmQueueItem as ServiceLlmQueueItem,
    LlmQueueRequest as ServiceLlmQueueRequest, LlmQueueResponse as ServiceLlmQueueResponse,
    LlmRequestType as ServiceLlmRequestType,
    NarrativeEventSuggestion as ServiceNarrativeEventSuggestion,
    PlayerAction as ServicePlayerAction, PlayerActionQueueItem as ServicePlayerActionQueueItem,
    ProposedToolCall as ServiceProposedToolCall,
};

// Conversion helper functions
fn convert_queue_status_to_service(
    status: QueueItemStatus,
) -> wrldbldr_engine_ports::outbound::QueueItemStatus {
    // QueueItemStatus is from engine-ports::outbound, used by both
    status
}

fn convert_asset_gen_request_to_service(
    req: InboundAssetGenerationRequest,
) -> ServiceAssetGenerationRequest {
    ServiceAssetGenerationRequest {
        world_id: req.world_id,
        entity_type: req.entity_type,
        entity_id: req.entity_id,
        workflow_id: req.workflow_id,
        prompt: req.prompt,
        count: req.count,
        negative_prompt: req.negative_prompt,
        style_reference_id: req.style_reference_id,
    }
}

fn convert_asset_gen_item_from_service(
    item: ServiceAssetGenerationQueueItem,
) -> InboundAssetGenerationQueueItem {
    InboundAssetGenerationQueueItem {
        id: item.id,
        payload: InboundAssetGenerationRequest {
            world_id: item.payload.world_id,
            entity_type: item.payload.entity_type,
            entity_id: item.payload.entity_id,
            workflow_id: item.payload.workflow_id,
            prompt: item.payload.prompt,
            count: item.payload.count,
            negative_prompt: item.payload.negative_prompt,
            style_reference_id: item.payload.style_reference_id,
        },
        priority: item.priority,
        enqueued_at: item.enqueued_at,
    }
}

fn convert_generation_result_to_service(result: InboundGenerationResult) -> ServiceGenerationResult {
    ServiceGenerationResult {
        asset_ids: result.asset_ids,
        file_paths: result.file_paths,
        metadata: ServiceGenerationMetadata {
            workflow: result.metadata.workflow,
            prompt: result.metadata.prompt,
            negative_prompt: result.metadata.negative_prompt,
            seed: result.metadata.seed,
            duration_ms: result.metadata.duration_ms,
        },
    }
}

/// Adapter for AssetGenerationQueueUseCasePort
pub struct AssetGenerationQueueUseCaseAdapter {
    service: Arc<dyn AssetGenerationQueueServicePort>,
}

impl AssetGenerationQueueUseCaseAdapter {
    pub fn new(service: Arc<dyn AssetGenerationQueueServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl AssetGenerationQueueUseCasePort for AssetGenerationQueueUseCaseAdapter {
    async fn enqueue(&self, request: InboundAssetGenerationRequest) -> anyhow::Result<uuid::Uuid> {
        self.service
            .enqueue(convert_asset_gen_request_to_service(request))
            .await
    }

    async fn dequeue(&self) -> anyhow::Result<Option<InboundAssetGenerationQueueItem>> {
        let result = self.service.dequeue().await?;
        Ok(result.map(convert_asset_gen_item_from_service))
    }

    async fn complete(&self, id: uuid::Uuid, result: InboundGenerationResult) -> anyhow::Result<()> {
        self.service
            .complete(id, convert_generation_result_to_service(result))
            .await
    }

    async fn fail(&self, id: uuid::Uuid, error: String) -> anyhow::Result<()> {
        self.service.fail(id, error).await
    }

    async fn depth(&self) -> anyhow::Result<usize> {
        self.service.depth().await
    }

    async fn processing_count(&self) -> anyhow::Result<usize> {
        self.service.processing_count().await
    }

    async fn has_capacity(&self) -> anyhow::Result<bool> {
        self.service.has_capacity().await
    }

    async fn get(&self, id: uuid::Uuid) -> anyhow::Result<Option<InboundAssetGenerationQueueItem>> {
        let result = self.service.get(id).await?;
        Ok(result.map(convert_asset_gen_item_from_service))
    }

    async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> anyhow::Result<Vec<InboundAssetGenerationQueueItem>> {
        let items = self
            .service
            .list_by_status(convert_queue_status_to_service(status))
            .await?;
        Ok(items.into_iter().map(convert_asset_gen_item_from_service).collect())
    }

    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64> {
        self.service.cleanup(retention).await
    }
}

fn convert_generation_queue_snapshot_from_service(
    snapshot: ServiceGenerationQueueSnapshot,
) -> InboundGenerationQueueSnapshot {
    InboundGenerationQueueSnapshot {
        batches: snapshot
            .batches
            .into_iter()
            .map(|b| InboundGenerationBatchSnapshot {
                id: b.id,
                world_id: b.world_id,
                entity_type: b.entity_type,
                entity_id: b.entity_id,
                status: b.status,
                item_count: b.item_count,
                completed_count: b.completed_count,
                is_read: b.is_read,
            })
            .collect(),
        suggestions: snapshot
            .suggestions
            .into_iter()
            .map(|s| InboundSuggestionTaskSnapshot {
                request_id: s.request_id,
                field_type: s.field_type,
                entity_id: s.entity_id,
                status: s.status,
                suggestions: s.suggestions,
                error: s.error,
                is_read: s.is_read,
            })
            .collect(),
    }
}

/// Adapter for GenerationQueueProjectionUseCasePort
pub struct GenerationQueueProjectionUseCaseAdapter {
    service: Arc<dyn GenerationQueueProjectionServicePort>,
}

impl GenerationQueueProjectionUseCaseAdapter {
    pub fn new(service: Arc<dyn GenerationQueueProjectionServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl GenerationQueueProjectionUseCasePort for GenerationQueueProjectionUseCaseAdapter {
    async fn project_queue(
        &self,
        user_id: Option<String>,
        world_id: WorldId,
    ) -> anyhow::Result<InboundGenerationQueueSnapshot> {
        let snapshot = self.service.project_queue(user_id, world_id).await?;
        Ok(convert_generation_queue_snapshot_from_service(snapshot))
    }
}

fn convert_player_action_to_service(action: InboundPlayerAction) -> ServicePlayerAction {
    ServicePlayerAction {
        world_id: action.world_id,
        player_id: action.player_id,
        pc_id: action.pc_id,
        action_type: action.action_type,
        target: action.target,
        dialogue: action.dialogue,
        timestamp: action.timestamp,
    }
}

fn convert_player_action_item_from_service(
    item: ServicePlayerActionQueueItem,
) -> InboundPlayerActionQueueItem {
    InboundPlayerActionQueueItem {
        id: item.id,
        payload: InboundPlayerAction {
            world_id: item.payload.world_id,
            player_id: item.payload.player_id,
            pc_id: item.payload.pc_id,
            action_type: item.payload.action_type,
            target: item.payload.target,
            dialogue: item.payload.dialogue,
            timestamp: item.payload.timestamp,
        },
        priority: item.priority,
        enqueued_at: item.enqueued_at,
    }
}

/// Adapter for PlayerActionQueueUseCasePort
pub struct PlayerActionQueueUseCaseAdapter {
    service: Arc<dyn PlayerActionQueueServicePort>,
}

impl PlayerActionQueueUseCaseAdapter {
    pub fn new(service: Arc<dyn PlayerActionQueueServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl PlayerActionQueueUseCasePort for PlayerActionQueueUseCaseAdapter {
    async fn enqueue(&self, action: InboundPlayerAction) -> anyhow::Result<uuid::Uuid> {
        self.service
            .enqueue(convert_player_action_to_service(action))
            .await
    }

    async fn dequeue(&self) -> anyhow::Result<Option<InboundPlayerActionQueueItem>> {
        let result = self.service.dequeue().await?;
        Ok(result.map(convert_player_action_item_from_service))
    }

    async fn complete(&self, id: uuid::Uuid) -> anyhow::Result<()> {
        self.service.complete(id).await
    }

    async fn depth(&self) -> anyhow::Result<usize> {
        self.service.depth().await
    }

    async fn get(&self, id: uuid::Uuid) -> anyhow::Result<Option<InboundPlayerActionQueueItem>> {
        let result = self.service.get(id).await?;
        Ok(result.map(convert_player_action_item_from_service))
    }

    async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> anyhow::Result<Vec<InboundPlayerActionQueueItem>> {
        let items = self
            .service
            .list_by_status(convert_queue_status_to_service(status))
            .await?;
        Ok(items
            .into_iter()
            .map(convert_player_action_item_from_service)
            .collect())
    }

    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64> {
        self.service.cleanup(retention).await
    }
}

fn convert_llm_request_type_to_service(req_type: InboundLlmRequestType) -> ServiceLlmRequestType {
    match req_type {
        InboundLlmRequestType::NpcResponse { action_item_id } => {
            ServiceLlmRequestType::NpcResponse { action_item_id }
        }
        InboundLlmRequestType::Suggestion {
            field_type,
            entity_id,
        } => ServiceLlmRequestType::Suggestion {
            field_type,
            entity_id,
        },
    }
}

fn convert_llm_request_type_from_service(req_type: ServiceLlmRequestType) -> InboundLlmRequestType {
    match req_type {
        ServiceLlmRequestType::NpcResponse { action_item_id } => {
            InboundLlmRequestType::NpcResponse { action_item_id }
        }
        ServiceLlmRequestType::Suggestion {
            field_type,
            entity_id,
        } => InboundLlmRequestType::Suggestion {
            field_type,
            entity_id,
        },
    }
}

fn convert_llm_request_to_service(req: InboundLlmQueueRequest) -> ServiceLlmQueueRequest {
    ServiceLlmQueueRequest {
        request_type: convert_llm_request_type_to_service(req.request_type),
        world_id: req.world_id,
        pc_id: req.pc_id,
        prompt: req.prompt,
        suggestion_context: req.suggestion_context,
        callback_id: req.callback_id,
    }
}

fn convert_llm_item_from_service(item: ServiceLlmQueueItem) -> InboundLlmQueueItem {
    InboundLlmQueueItem {
        id: item.id,
        payload: InboundLlmQueueRequest {
            request_type: convert_llm_request_type_from_service(item.payload.request_type),
            world_id: item.payload.world_id,
            pc_id: item.payload.pc_id,
            prompt: item.payload.prompt,
            suggestion_context: item.payload.suggestion_context,
            callback_id: item.payload.callback_id,
        },
        priority: item.priority,
        callback_id: item.callback_id,
    }
}

fn convert_confidence_level_to_service(level: InboundConfidenceLevel) -> ServiceConfidenceLevel {
    match level {
        InboundConfidenceLevel::Low => ServiceConfidenceLevel::Low,
        InboundConfidenceLevel::Medium => ServiceConfidenceLevel::Medium,
        InboundConfidenceLevel::High => ServiceConfidenceLevel::High,
    }
}

fn convert_llm_response_to_service(resp: InboundLlmQueueResponse) -> ServiceLlmQueueResponse {
    ServiceLlmQueueResponse {
        npc_dialogue: resp.npc_dialogue,
        internal_reasoning: resp.internal_reasoning,
        proposed_tool_calls: resp
            .proposed_tool_calls
            .into_iter()
            .map(|t| ServiceProposedToolCall {
                tool_name: t.tool_name,
                arguments: t.arguments,
            })
            .collect(),
        challenge_suggestion: resp.challenge_suggestion.map(|c| ServiceChallengeSuggestion {
            challenge_id: c.challenge_id,
            confidence: convert_confidence_level_to_service(c.confidence),
            reasoning: c.reasoning,
        }),
        narrative_event_suggestion: resp
            .narrative_event_suggestion
            .map(|n| ServiceNarrativeEventSuggestion {
                event_id: n.event_id,
                confidence: convert_confidence_level_to_service(n.confidence),
                reasoning: n.reasoning,
                matched_triggers: n.matched_triggers,
            }),
        topics: resp.topics,
    }
}

/// Adapter for LlmQueueUseCasePort
pub struct LlmQueueUseCaseAdapter {
    service: Arc<dyn LlmQueueServicePort>,
}

impl LlmQueueUseCaseAdapter {
    pub fn new(service: Arc<dyn LlmQueueServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl LlmQueueUseCasePort for LlmQueueUseCaseAdapter {
    async fn enqueue(&self, request: InboundLlmQueueRequest) -> anyhow::Result<uuid::Uuid> {
        self.service
            .enqueue(convert_llm_request_to_service(request))
            .await
    }

    async fn dequeue(&self) -> anyhow::Result<Option<InboundLlmQueueItem>> {
        let result = self.service.dequeue().await?;
        Ok(result.map(convert_llm_item_from_service))
    }

    async fn complete(&self, id: uuid::Uuid, result: InboundLlmQueueResponse) -> anyhow::Result<()> {
        self.service
            .complete(id, convert_llm_response_to_service(result))
            .await
    }

    async fn fail(&self, id: uuid::Uuid, error: String) -> anyhow::Result<()> {
        self.service.fail(id, error).await
    }

    async fn cancel_suggestion(&self, callback_id: &str) -> anyhow::Result<bool> {
        self.service.cancel_suggestion(callback_id).await
    }

    async fn depth(&self) -> anyhow::Result<usize> {
        self.service.depth().await
    }

    async fn processing_count(&self) -> anyhow::Result<usize> {
        self.service.processing_count().await
    }

    async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> anyhow::Result<Vec<InboundLlmQueueItem>> {
        let items = self
            .service
            .list_by_status(convert_queue_status_to_service(status))
            .await?;
        Ok(items.into_iter().map(convert_llm_item_from_service).collect())
    }

    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64> {
        self.service.cleanup(retention).await
    }
}

fn convert_approval_decision_type_to_service(
    dt: InboundApprovalDecisionType,
) -> ServiceApprovalDecisionType {
    match dt {
        InboundApprovalDecisionType::NpcResponse => ServiceApprovalDecisionType::NpcResponse,
        InboundApprovalDecisionType::ToolUsage => ServiceApprovalDecisionType::ToolUsage,
        InboundApprovalDecisionType::ChallengeSuggestion => {
            ServiceApprovalDecisionType::ChallengeSuggestion
        }
        InboundApprovalDecisionType::SceneTransition => ServiceApprovalDecisionType::SceneTransition,
        InboundApprovalDecisionType::ChallengeOutcome => {
            ServiceApprovalDecisionType::ChallengeOutcome
        }
    }
}

fn convert_approval_decision_type_from_service(
    dt: ServiceApprovalDecisionType,
) -> InboundApprovalDecisionType {
    match dt {
        ServiceApprovalDecisionType::NpcResponse => InboundApprovalDecisionType::NpcResponse,
        ServiceApprovalDecisionType::ToolUsage => InboundApprovalDecisionType::ToolUsage,
        ServiceApprovalDecisionType::ChallengeSuggestion => {
            InboundApprovalDecisionType::ChallengeSuggestion
        }
        ServiceApprovalDecisionType::SceneTransition => InboundApprovalDecisionType::SceneTransition,
        ServiceApprovalDecisionType::ChallengeOutcome => {
            InboundApprovalDecisionType::ChallengeOutcome
        }
    }
}

fn convert_approval_urgency_to_service(u: InboundApprovalUrgency) -> ServiceApprovalUrgency {
    match u {
        InboundApprovalUrgency::Normal => ServiceApprovalUrgency::Normal,
        InboundApprovalUrgency::AwaitingPlayer => ServiceApprovalUrgency::AwaitingPlayer,
        InboundApprovalUrgency::SceneCritical => ServiceApprovalUrgency::SceneCritical,
    }
}

fn convert_approval_urgency_from_service(u: ServiceApprovalUrgency) -> InboundApprovalUrgency {
    match u {
        ServiceApprovalUrgency::Normal => InboundApprovalUrgency::Normal,
        ServiceApprovalUrgency::AwaitingPlayer => InboundApprovalUrgency::AwaitingPlayer,
        ServiceApprovalUrgency::SceneCritical => InboundApprovalUrgency::SceneCritical,
    }
}

fn convert_approval_request_to_service(req: InboundApprovalRequest) -> ServiceApprovalRequest {
    ServiceApprovalRequest {
        world_id: req.world_id,
        source_action_id: req.source_action_id,
        decision_type: convert_approval_decision_type_to_service(req.decision_type),
        urgency: convert_approval_urgency_to_service(req.urgency),
        pc_id: req.pc_id,
        npc_id: req.npc_id,
        npc_name: req.npc_name,
        proposed_dialogue: req.proposed_dialogue,
        internal_reasoning: req.internal_reasoning,
        proposed_tools: req.proposed_tools,
        retry_count: req.retry_count,
        challenge_suggestion: req.challenge_suggestion,
        narrative_event_suggestion: req.narrative_event_suggestion,
        player_dialogue: req.player_dialogue,
        scene_id: req.scene_id,
        location_id: req.location_id,
        game_time: req.game_time,
        topics: req.topics,
    }
}

fn convert_approval_item_from_service(item: ServiceApprovalQueueItem) -> InboundApprovalQueueItem {
    InboundApprovalQueueItem {
        id: item.id,
        payload: InboundApprovalRequest {
            world_id: item.payload.world_id,
            source_action_id: item.payload.source_action_id,
            decision_type: convert_approval_decision_type_from_service(item.payload.decision_type),
            urgency: convert_approval_urgency_from_service(item.payload.urgency),
            pc_id: item.payload.pc_id,
            npc_id: item.payload.npc_id,
            npc_name: item.payload.npc_name,
            proposed_dialogue: item.payload.proposed_dialogue,
            internal_reasoning: item.payload.internal_reasoning,
            proposed_tools: item.payload.proposed_tools,
            retry_count: item.payload.retry_count,
            challenge_suggestion: item.payload.challenge_suggestion,
            narrative_event_suggestion: item.payload.narrative_event_suggestion,
            player_dialogue: item.payload.player_dialogue,
            scene_id: item.payload.scene_id,
            location_id: item.payload.location_id,
            game_time: item.payload.game_time,
            topics: item.payload.topics,
        },
        priority: item.priority,
        enqueued_at: item.enqueued_at,
        updated_at: item.updated_at,
    }
}

/// Adapter for DmApprovalQueueUseCasePort
pub struct DmApprovalQueueUseCaseAdapter {
    service: Arc<dyn DmApprovalQueueServicePort>,
}

impl DmApprovalQueueUseCaseAdapter {
    pub fn new(service: Arc<dyn DmApprovalQueueServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl DmApprovalQueueUseCasePort for DmApprovalQueueUseCaseAdapter {
    async fn enqueue(&self, approval: InboundApprovalRequest) -> anyhow::Result<uuid::Uuid> {
        self.service
            .enqueue(convert_approval_request_to_service(approval))
            .await
    }

    async fn dequeue(&self) -> anyhow::Result<Option<InboundApprovalQueueItem>> {
        let result = self.service.dequeue().await?;
        Ok(result.map(convert_approval_item_from_service))
    }

    async fn complete(&self, id: uuid::Uuid, decision: DmApprovalDecision) -> anyhow::Result<()> {
        self.service.complete(id, decision).await
    }

    async fn get_pending(&self, world_id: WorldId) -> anyhow::Result<Vec<InboundApprovalQueueItem>> {
        let items = self.service.get_pending(world_id).await?;
        Ok(items
            .into_iter()
            .map(convert_approval_item_from_service)
            .collect())
    }

    async fn get(&self, id: uuid::Uuid) -> anyhow::Result<Option<InboundApprovalQueueItem>> {
        let result = self.service.get(id).await?;
        Ok(result.map(convert_approval_item_from_service))
    }

    async fn get_history(
        &self,
        world_id: WorldId,
        limit: usize,
    ) -> anyhow::Result<Vec<InboundApprovalQueueItem>> {
        let items = self.service.get_history(world_id, limit).await?;
        Ok(items
            .into_iter()
            .map(convert_approval_item_from_service)
            .collect())
    }

    async fn delay(
        &self,
        id: uuid::Uuid,
        until: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<()> {
        self.service.delay(id, until).await
    }

    async fn discard_challenge(&self, request_id: &str) -> anyhow::Result<()> {
        self.service.discard_challenge(request_id).await
    }

    async fn depth(&self) -> anyhow::Result<usize> {
        self.service.depth().await
    }

    async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> anyhow::Result<Vec<InboundApprovalQueueItem>> {
        let items = self
            .service
            .list_by_status(convert_queue_status_to_service(status))
            .await?;
        Ok(items
            .into_iter()
            .map(convert_approval_item_from_service)
            .collect())
    }

    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64> {
        self.service.cleanup(retention).await
    }

    async fn expire_old(&self, timeout: std::time::Duration) -> anyhow::Result<u64> {
        self.service.expire_old(timeout).await
    }
}

// ============================================================================
// World Use Case Adapter
// ============================================================================

/// Adapter that implements WorldUseCasePort by delegating to WorldServicePort
pub struct WorldUseCaseAdapter {
    service: Arc<dyn WorldServicePort>,
}

impl WorldUseCaseAdapter {
    pub fn new(service: Arc<dyn WorldServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl WorldUseCasePort for WorldUseCaseAdapter {
    async fn get_world(
        &self,
        id: WorldId,
    ) -> Result<Option<wrldbldr_domain::entities::World>> {
        self.service.get_world(id).await
    }

    async fn list_worlds(&self) -> Result<Vec<wrldbldr_domain::entities::World>> {
        self.service.list_worlds().await
    }

    async fn get_current_location(
        &self,
        world_id: WorldId,
    ) -> Result<Option<wrldbldr_domain::entities::Location>> {
        self.service.get_current_location(world_id).await
    }

    async fn export_world_snapshot(
        &self,
        world_id: WorldId,
    ) -> Result<wrldbldr_engine_ports::outbound::PlayerWorldSnapshot> {
        self.service.export_world_snapshot(world_id).await
    }
}
