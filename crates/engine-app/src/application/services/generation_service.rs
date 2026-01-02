//! Generation Service - Manages the asset generation queue
//!
//! This service handles:
//! - Queueing generation requests
//! - Processing batches through ComfyUI
//! - Tracking progress and notifying clients via WebSocket

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use wrldbldr_domain::entities::{
    AssetType, BatchStatus, EntityType, GalleryAsset, GenerationBatch, GenerationMetadata,
};
use wrldbldr_domain::value_objects::BatchQueueFailurePolicy;
use wrldbldr_domain::{AssetId, BatchId, WorldId};
use crate::application::services::internal::{
    GenerationRequest, GenerationServicePort, SettingsServicePort,
};
use wrldbldr_engine_ports::outbound::{
    ActiveGenerationBatch, ActiveGenerationBatchesPort, AssetRepositoryPort, ClockPort,
    ComfyUIPort, FileStoragePort,
};

/// Events emitted by the generation service
#[derive(Debug, Clone)]
pub enum GenerationEvent {
    /// A batch has been queued
    BatchQueued {
        batch_id: BatchId,
        world_id: WorldId,
        entity_type: EntityType,
        entity_id: String,
        asset_type: AssetType,
        position: u32,
    },
    /// A batch is generating (progress update)
    BatchProgress {
        batch_id: BatchId,
        world_id: WorldId,
        progress: u8,
    },
    /// A batch has completed
    BatchComplete {
        batch_id: BatchId,
        world_id: WorldId,
        entity_type: EntityType,
        entity_id: String,
        asset_type: AssetType,
        asset_count: u32,
    },
    /// A batch has failed
    BatchFailed {
        batch_id: BatchId,
        world_id: WorldId,
        entity_type: EntityType,
        entity_id: String,
        asset_type: AssetType,
        error: String,
    },
    /// A suggestion request has been queued
    SuggestionQueued {
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
        world_id: Option<WorldId>,
    },
    /// A suggestion request is being processed
    SuggestionProgress {
        request_id: String,
        status: String,
        world_id: Option<WorldId>,
    },
    /// A suggestion request has completed
    SuggestionComplete {
        request_id: String,
        field_type: String,
        suggestions: Vec<String>,
        world_id: Option<WorldId>,
    },
    /// A suggestion request has failed
    SuggestionFailed {
        request_id: String,
        field_type: String,
        error: String,
        world_id: Option<WorldId>,
    },
}

/// Generation service for managing asset generation
pub struct GenerationService {
    /// ComfyUI client for sending generation requests
    comfyui_client: Arc<dyn ComfyUIPort>,
    /// Asset repository for persisting results
    repository: Arc<dyn AssetRepositoryPort>,
    /// Clock for time operations (required for testability)
    clock: Arc<dyn ClockPort>,
    /// File storage for abstracting file system operations
    file_storage: Arc<dyn FileStoragePort>,
    /// Directory to save generated assets
    output_dir: String,
    /// Active batches being processed (adapter-managed concurrency)
    active_batches: Arc<dyn ActiveGenerationBatchesPort>,
    /// Event sender for notifying about generation progress (bounded channel)
    event_sender: mpsc::Sender<GenerationEvent>,
    /// Workflow templates directory
    workflow_dir: String,
    /// Settings service used to resolve per-world generation behavior.
    settings_service: Arc<dyn SettingsServicePort>,
}

impl GenerationService {
    /// Create a new generation service
    ///
    /// # Arguments
    /// * `clock` - Clock for time operations. Use `SystemClock` in production,
    ///             `MockClockPort` in tests for deterministic behavior.
    /// * `file_storage` - File storage port for abstracting file system operations.
    pub fn new(
        comfyui_client: Arc<dyn ComfyUIPort>,
        repository: Arc<dyn AssetRepositoryPort>,
        clock: Arc<dyn ClockPort>,
        file_storage: Arc<dyn FileStoragePort>,
        output_dir: String,
        workflow_dir: String,
        event_sender: mpsc::Sender<GenerationEvent>,
        active_batches: Arc<dyn ActiveGenerationBatchesPort>,
        settings_service: Arc<dyn SettingsServicePort>,
    ) -> Self {
        Self {
            comfyui_client,
            repository,
            clock,
            file_storage,
            output_dir,
            active_batches,
            event_sender,
            workflow_dir,
            settings_service,
        }
    }

    /// Queue a new generation batch
    pub async fn queue_generation(&self, request: GenerationRequest) -> Result<BatchId> {
        // Create a new batch
        let batch_id = BatchId::new();
        let workflow = self.get_workflow_name(&request.asset_type);

        let batch = GenerationBatch {
            id: batch_id,
            world_id: request.world_id,
            entity_type: request.entity_type,
            entity_id: request.entity_id.clone(),
            asset_type: request.asset_type,
            workflow: workflow.clone(),
            prompt: request.prompt.clone(),
            negative_prompt: request.negative_prompt.clone(),
            count: request.count,
            status: BatchStatus::Queued,
            assets: vec![],
            style_reference_id: request.style_reference_id,
            requested_at: self.clock.now(),
            completed_at: None,
        };

        // Persist the batch
        self.repository.create_batch(&batch).await?;

        // Get queue position
        let position = self.active_batches.len().await as u32 + 1;

        // Send queued event (non-blocking, logs warning if buffer full)
        if let Err(e) = self.event_sender.try_send(GenerationEvent::BatchQueued {
            batch_id,
            world_id: batch.world_id,
            entity_type: batch.entity_type,
            entity_id: batch.entity_id.clone(),
            asset_type: batch.asset_type,
            position,
        }) {
            tracing::warn!("Failed to send BatchQueued event: {}", e);
        }

        // Start processing (this would normally be done by a background worker)
        self.start_batch_processing(batch).await?;

        Ok(batch_id)
    }

    /// Start processing a batch
    pub async fn start_batch_processing(&self, batch: GenerationBatch) -> Result<()> {
        let batch_id = batch.id;

        let batch_queue_failure_policy = self
            .settings_service
            .get_for_world(batch.world_id)
            .await
            .batch_queue_failure_policy;

        // Update status to generating
        self.repository
            .update_batch_status(batch_id, &BatchStatus::Generating { progress: 0 })
            .await?;

        self.active_batches
            .insert(
                batch_id,
                ActiveGenerationBatch {
                    batch: batch.clone(),
                    prompt_ids: vec![],
                },
            )
            .await;

        // Load the workflow template
        let workflow_template = self.load_workflow_template(&batch.workflow).await?;

        // Queue each generation
        let mut prompt_ids = Vec::new();
        for i in 0..batch.count {
            // Modify the workflow with our parameters
            let workflow = self
                .prepare_workflow(
                    workflow_template.clone(),
                    &batch.prompt,
                    batch.negative_prompt.as_deref(),
                    i as i64, // Use index as seed variation
                    &batch.asset_type,
                    batch.style_reference_id,
                )
                .await?;

            // Queue with ComfyUI
            match self.comfyui_client.queue_prompt(workflow).await {
                Ok(response) => {
                    prompt_ids.push(response.prompt_id);
                }
                Err(e) => {
                    let error =
                        format!("Failed to queue prompt {} for batch {}: {}", i, batch_id, e);
                    tracing::error!("{}", error);

                    match batch_queue_failure_policy {
                        BatchQueueFailurePolicy::AllOrNothing => {
                            let status = BatchStatus::Failed {
                                error: error.clone(),
                            };
                            self.repository
                                .update_batch_status(batch_id, &status)
                                .await?;
                            self.active_batches.remove(batch_id).await;

                            if let Err(send_err) =
                                self.event_sender.try_send(GenerationEvent::BatchFailed {
                                    batch_id,
                                    world_id: batch.world_id,
                                    entity_type: batch.entity_type,
                                    entity_id: batch.entity_id.clone(),
                                    asset_type: batch.asset_type,
                                    error,
                                })
                            {
                                tracing::warn!("Failed to send BatchFailed event: {}", send_err);
                            }

                            return Ok(());
                        }
                        BatchQueueFailurePolicy::BestEffort => {
                            // Continue trying to queue remaining prompts.
                            continue;
                        }
                        BatchQueueFailurePolicy::Unknown => {
                            // Treat unknown as the safe default.
                            let status = BatchStatus::Failed {
                                error: error.clone(),
                            };
                            self.repository
                                .update_batch_status(batch_id, &status)
                                .await?;
                            self.active_batches.remove(batch_id).await;

                            if let Err(send_err) =
                                self.event_sender.try_send(GenerationEvent::BatchFailed {
                                    batch_id,
                                    world_id: batch.world_id,
                                    entity_type: batch.entity_type,
                                    entity_id: batch.entity_id.clone(),
                                    asset_type: batch.asset_type,
                                    error,
                                })
                            {
                                tracing::warn!("Failed to send BatchFailed event: {}", send_err);
                            }

                            return Ok(());
                        }
                    }
                }
            }
        }

        if prompt_ids.is_empty() {
            let error = format!("Failed to queue any prompts for batch {}", batch_id);
            let status = BatchStatus::Failed {
                error: error.clone(),
            };
            self.repository
                .update_batch_status(batch_id, &status)
                .await?;
            self.active_batches.remove(batch_id).await;

            if let Err(send_err) = self.event_sender.try_send(GenerationEvent::BatchFailed {
                batch_id,
                world_id: batch.world_id,
                entity_type: batch.entity_type,
                entity_id: batch.entity_id.clone(),
                asset_type: batch.asset_type,
                error,
            }) {
                tracing::warn!("Failed to send BatchFailed event: {}", send_err);
            }

            return Ok(());
        }

        // Update tracker with prompt IDs
        self.active_batches
            .update_prompt_ids(batch_id, prompt_ids)
            .await;

        Ok(())
    }

    /// Poll for batch completion (call periodically)
    pub async fn poll_batch_progress(&self, batch_id: BatchId) -> Result<Option<BatchStatus>> {
        let Some(tracker) = self.active_batches.get(batch_id).await else {
            return Ok(None);
        };

        let prompt_ids = tracker.prompt_ids;
        let batch = tracker.batch;

        let mut completed_count = 0u8;
        let mut generated_assets = Vec::new();

        for prompt_id in &prompt_ids {
            match self.comfyui_client.get_history(prompt_id).await {
                Ok(history) => {
                    if let Some(prompt_history) = history.prompts.get(prompt_id) {
                        if prompt_history.status.completed {
                            completed_count += 1;

                            // Extract generated images
                            for output in prompt_history.outputs.values() {
                                if let Some(images) = &output.images {
                                    for image in images {
                                        // Download and save the image
                                        match self
                                            .download_and_save_asset(
                                                &batch,
                                                &image.filename,
                                                &image.subfolder,
                                                &image.r#type,
                                            )
                                            .await
                                        {
                                            Ok(asset) => generated_assets.push(asset),
                                            Err(e) => {
                                                tracing::error!("Failed to download asset: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get history for prompt {}: {}", prompt_id, e);
                }
            }
        }

        // Calculate progress
        let progress = if prompt_ids.is_empty() {
            0
        } else {
            ((completed_count as f32 / prompt_ids.len() as f32) * 100.0) as u8
        };

        // Update progress (non-blocking, logs warning if buffer full)
        if let Err(e) = self.event_sender.try_send(GenerationEvent::BatchProgress {
            batch_id,
            world_id: batch.world_id,
            progress,
        }) {
            tracing::warn!("Failed to send BatchProgress event: {}", e);
        }

        // Check if all completed
        if completed_count as usize >= prompt_ids.len() {
            // Batch is complete
            let asset_ids: Vec<AssetId> = generated_assets.iter().map(|a| a.id).collect();

            self.repository
                .update_batch_status(batch_id, &BatchStatus::ReadyForSelection)
                .await?;
            self.repository
                .update_batch_assets(batch_id, &asset_ids)
                .await?;

            if let Err(e) = self.event_sender.try_send(GenerationEvent::BatchComplete {
                batch_id,
                world_id: batch.world_id,
                entity_type: batch.entity_type,
                entity_id: batch.entity_id.clone(),
                asset_type: batch.asset_type,
                asset_count: generated_assets.len() as u32,
            }) {
                tracing::warn!("Failed to send BatchComplete event: {}", e);
            }

            // Remove from active batches
            self.active_batches.remove(batch_id).await;

            return Ok(Some(BatchStatus::ReadyForSelection));
        }

        Ok(Some(BatchStatus::Generating { progress }))
    }

    /// Download an asset from ComfyUI and save it locally
    async fn download_and_save_asset(
        &self,
        batch: &GenerationBatch,
        filename: &str,
        subfolder: &str,
        folder_type: &str,
    ) -> Result<GalleryAsset> {
        // Download the image
        let image_data = self
            .comfyui_client
            .get_image(filename, subfolder, folder_type)
            .await?;

        // Create output directory structure
        let entity_dir = format!(
            "{}/{}/{}/{}",
            self.output_dir,
            batch.entity_type.as_str(),
            batch.entity_id,
            batch.asset_type.as_str()
        );
        self.file_storage
            .create_dir_all(Path::new(&entity_dir))
            .await?;

        // Generate unique filename
        let asset_id = AssetId::new();
        let extension = filename.rsplit('.').next().unwrap_or("png");
        let output_filename = format!("{}.{}", asset_id, extension);
        let output_path = format!("{}/{}", entity_dir, output_filename);

        // Save the file
        self.file_storage
            .write(Path::new(&output_path), &image_data)
            .await?;

        // Create generation metadata
        let mut metadata = GenerationMetadata::new(
            batch.workflow.clone(),
            batch.prompt.clone(),
            0, // seed - will be set when we have access to actual seed from ComfyUI
            batch.id,
        );
        if let Some(ref neg) = batch.negative_prompt {
            metadata = metadata.with_negative_prompt(neg.clone());
        }
        if let Some(style_ref) = batch.style_reference_id {
            metadata = metadata.with_style_reference(style_ref);
        }

        // Create asset entity
        let mut asset = GalleryAsset::new_generated(
            batch.entity_type,
            batch.entity_id.clone(),
            batch.asset_type,
            output_path,
            metadata,
            self.clock.now(),
        );
        // Override the auto-generated ID with our pre-generated one for the filename
        asset.id = asset_id;

        // Persist the asset
        self.repository.create(&asset).await?;

        Ok(asset)
    }

    /// Load a workflow template from file
    async fn load_workflow_template(&self, workflow_name: &str) -> Result<serde_json::Value> {
        let path = format!("{}/{}.json", self.workflow_dir, workflow_name);

        if self.file_storage.exists(Path::new(&path)).await? {
            let content = self.file_storage.read_to_string(Path::new(&path)).await?;
            let workflow: serde_json::Value = serde_json::from_str(&content)?;
            Ok(workflow)
        } else {
            // Return a basic placeholder workflow
            tracing::warn!(
                "Workflow template not found: {}, using placeholder",
                workflow_name
            );
            Ok(self.get_placeholder_workflow())
        }
    }

    /// Prepare a workflow with generation parameters
    async fn prepare_workflow(
        &self,
        mut workflow: serde_json::Value,
        prompt: &str,
        negative_prompt: Option<&str>,
        seed_offset: i64,
        asset_type: &AssetType,
        style_reference_id: Option<AssetId>,
    ) -> Result<serde_json::Value> {
        // Get dimensions for this asset type
        let (width, height) = asset_type.default_dimensions();

        // This is a simplified workflow preparation
        // In practice, you'd need to navigate the workflow JSON structure
        // and update specific nodes

        if let Some(obj) = workflow.as_object_mut() {
            // Try to find and update common node types
            for (_node_id, node) in obj.iter_mut() {
                if let Some(node_obj) = node.as_object_mut() {
                    if let Some(inputs) = node_obj.get_mut("inputs") {
                        if let Some(inputs_obj) = inputs.as_object_mut() {
                            // Update text prompts
                            if inputs_obj.contains_key("text") {
                                inputs_obj.insert(
                                    "text".to_string(),
                                    serde_json::Value::String(prompt.to_string()),
                                );
                            }

                            // Update positive prompt
                            if inputs_obj.contains_key("positive") {
                                if let Some(s) = inputs_obj.get("positive") {
                                    if s.is_string() {
                                        inputs_obj.insert(
                                            "positive".to_string(),
                                            serde_json::Value::String(prompt.to_string()),
                                        );
                                    }
                                }
                            }

                            // Update negative prompt
                            if let Some(neg) = negative_prompt {
                                if inputs_obj.contains_key("negative") {
                                    if let Some(s) = inputs_obj.get("negative") {
                                        if s.is_string() {
                                            inputs_obj.insert(
                                                "negative".to_string(),
                                                serde_json::Value::String(neg.to_string()),
                                            );
                                        }
                                    }
                                }
                            }

                            // Update seed with offset
                            if inputs_obj.contains_key("seed") {
                                let base_seed =
                                    inputs_obj.get("seed").and_then(|s| s.as_i64()).unwrap_or(0);
                                inputs_obj.insert(
                                    "seed".to_string(),
                                    serde_json::Value::Number((base_seed + seed_offset).into()),
                                );
                            }

                            // Update dimensions
                            if inputs_obj.contains_key("width") {
                                inputs_obj.insert(
                                    "width".to_string(),
                                    serde_json::Value::Number(width.into()),
                                );
                            }
                            if inputs_obj.contains_key("height") {
                                inputs_obj.insert(
                                    "height".to_string(),
                                    serde_json::Value::Number(height.into()),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Inject style reference if provided
        if let Some(ref_id) = style_reference_id {
            // Try to load the style reference asset
            if let Ok(Some(ref_asset)) = self.repository.get(ref_id).await {
                // Try to find IPAdapter node and inject image
                if let Some(obj) = workflow.as_object_mut() {
                    let mut ipadapter_found = false;

                    // First pass: find IPAdapter nodes
                    for (_node_id, node) in obj.iter() {
                        if let Some(node_obj) = node.as_object() {
                            if let Some(class_type) =
                                node_obj.get("class_type").and_then(|c| c.as_str())
                            {
                                if class_type.contains("IPAdapter") {
                                    ipadapter_found = true;
                                    break;
                                }
                            }
                        }
                    }

                    // If IPAdapter found, inject image path
                    if ipadapter_found {
                        for (_node_id, node) in obj.iter_mut() {
                            if let Some(node_obj) = node.as_object_mut() {
                                if let Some(class_type) =
                                    node_obj.get("class_type").and_then(|c| c.as_str())
                                {
                                    if class_type.contains("IPAdapter") {
                                        if let Some(inputs) = node_obj.get_mut("inputs") {
                                            if let Some(inputs_obj) = inputs.as_object_mut() {
                                                // Inject image path - ComfyUI expects format: "filename" or full path
                                                // For now, use the file_path from the asset
                                                inputs_obj.insert(
                                                    "image".to_string(),
                                                    serde_json::Value::String(
                                                        ref_asset.file_path.clone(),
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // No IPAdapter found - inject style keywords into prompt
                        // This is a simplified approach - in production, you'd extract style keywords
                        // from the reference asset's generation metadata or analyze the image
                        let enhanced_prompt =
                            format!("{}, in the style of: {}", prompt, "reference style");

                        // Update prompt in workflow
                        for (_node_id, node) in obj.iter_mut() {
                            if let Some(node_obj) = node.as_object_mut() {
                                if let Some(inputs) = node_obj.get_mut("inputs") {
                                    if let Some(inputs_obj) = inputs.as_object_mut() {
                                        if inputs_obj.contains_key("text") {
                                            inputs_obj.insert(
                                                "text".to_string(),
                                                serde_json::Value::String(enhanced_prompt.clone()),
                                            );
                                        }
                                        if inputs_obj.contains_key("positive") {
                                            if let Some(s) = inputs_obj.get("positive") {
                                                if s.is_string() {
                                                    inputs_obj.insert(
                                                        "positive".to_string(),
                                                        serde_json::Value::String(
                                                            enhanced_prompt.clone(),
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(workflow)
    }

    /// Get the workflow name for an asset type
    fn get_workflow_name(&self, asset_type: &AssetType) -> String {
        match asset_type {
            AssetType::Portrait => "character-portrait-generation".to_string(),
            AssetType::Sprite => "character-sprite-generation".to_string(),
            AssetType::Backdrop => "backdrop-generation".to_string(),
            AssetType::Tilesheet => "tilesheet-generation".to_string(),
            AssetType::ItemIcon => "item-icon-generation".to_string(),
            AssetType::EmotionSheet => "character-emotion-sheet".to_string(),
            AssetType::RegionBackdrop => "map-region-backdrop".to_string(),
        }
    }

    /// Get a placeholder workflow for testing
    fn get_placeholder_workflow(&self) -> serde_json::Value {
        serde_json::json!({
            "3": {
                "class_type": "KSampler",
                "inputs": {
                    "seed": 0,
                    "steps": 20,
                    "cfg": 7.0,
                    "sampler_name": "euler",
                    "scheduler": "normal",
                    "denoise": 1.0
                }
            },
            "6": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "text": "placeholder prompt"
                }
            }
        })
    }

    /// Get the status of a batch
    pub async fn get_batch_status(&self, batch_id: BatchId) -> Result<Option<GenerationBatch>> {
        self.repository.get_batch(batch_id).await
    }

    /// List all active (queued or generating) batches for a specific world
    pub async fn list_active_batches_by_world(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<GenerationBatch>> {
        self.repository.list_active_batches_by_world(world_id).await
    }

    /// List batches ready for selection
    pub async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>> {
        self.repository.list_ready_batches().await
    }

    /// Mark batch as completed after user selection
    pub async fn complete_batch(&self, batch_id: BatchId) -> Result<()> {
        self.repository
            .update_batch_status(batch_id, &BatchStatus::Completed)
            .await
    }

    /// Cancel a batch
    pub async fn cancel_batch(&self, batch_id: BatchId) -> Result<()> {
        self.active_batches.remove(batch_id).await;
        self.repository.delete_batch(batch_id).await
    }
}

// Implementation of the port trait for hexagonal architecture compliance
#[async_trait]
impl GenerationServicePort for GenerationService {
    async fn generate_asset(&self, request: GenerationRequest) -> Result<GenerationBatch> {
        // Queue and return the batch
        let batch_id = self.queue_generation(request).await?;
        self.get_batch_status(batch_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created batch"))
    }

    async fn get_batch(&self, id: BatchId) -> Result<Option<GenerationBatch>> {
        self.get_batch_status(id).await
    }

    async fn select_from_batch(
        &self,
        batch_id: BatchId,
        asset_index: usize,
    ) -> Result<GalleryAsset> {
        // Get the batch to access its assets
        let batch = self
            .get_batch_status(batch_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Batch not found: {}", batch_id))?;

        // Validate the batch is ready for selection
        if !matches!(batch.status, BatchStatus::ReadyForSelection) {
            anyhow::bail!("Batch is not ready for selection");
        }

        // Validate the asset index
        if asset_index >= batch.assets.len() {
            anyhow::bail!(
                "Asset index {} out of bounds (batch has {} assets)",
                asset_index,
                batch.assets.len()
            );
        }

        let asset_id = batch.assets[asset_index];

        // Get the asset
        let asset = self
            .repository
            .get(asset_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", asset_id))?;

        // Mark the batch as completed
        self.complete_batch(batch_id).await?;

        // Activate the selected asset
        self.repository.activate(asset_id).await?;

        Ok(asset)
    }

    async fn start_batch_processing(&self, batch: GenerationBatch) -> Result<()> {
        GenerationService::start_batch_processing(self, batch).await
    }
}
