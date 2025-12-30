//! Asset Generation Queue Service - Concurrency-controlled asset generation
//!
//! This service manages the AssetGenerationQueue, which processes ComfyUI
//! requests with controlled concurrency (typically batch_size=1).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use wrldbldr_domain::entities::{AssetType, EntityType, GalleryAsset, GenerationMetadata};
use wrldbldr_domain::value_objects::AssetGenerationData;
use wrldbldr_domain::{AssetId, WorldId};
use wrldbldr_engine_ports::outbound::{
    AssetGenerationQueueItem, AssetGenerationQueueServicePort, AssetGenerationRequest,
    AssetRepositoryPort, ClockPort, ComfyUIPort, FileStoragePort, GenerationResult,
    ProcessingQueuePort, QueueError, QueueItemId, QueueItemStatus, QueueNotificationPort,
};

/// Priority constants for queue operations
const PRIORITY_NORMAL: u8 = 0;

/// Service for managing the asset generation queue
pub struct AssetGenerationQueueService<
    Q: ProcessingQueuePort<AssetGenerationData>,
    C: ComfyUIPort,
    N: QueueNotificationPort,
> {
    pub(crate) queue: Arc<Q>,
    comfyui_client: Arc<C>,
    asset_repository: Arc<dyn AssetRepositoryPort>,
    /// Clock for time operations (required for testability)
    clock: Arc<dyn ClockPort>,
    /// File storage for saving generated assets
    file_storage: Arc<dyn FileStoragePort>,
    /// Output directory for generated assets
    output_dir: String,
    semaphore: Arc<Semaphore>,
    notifier: N,
}

impl<
        Q: ProcessingQueuePort<AssetGenerationData> + 'static,
        C: ComfyUIPort + 'static,
        N: QueueNotificationPort + 'static,
    > AssetGenerationQueueService<Q, C, N>
{
    pub fn queue(&self) -> &Arc<Q> {
        &self.queue
    }

    /// Create a new asset generation queue service
    ///
    /// # Arguments
    ///
    /// * `queue` - The asset generation queue
    /// * `comfyui_client` - The ComfyUI client for processing requests
    /// * `asset_repository` - The asset repository for persisting results
    /// * `clock` - Clock for time operations
    /// * `file_storage` - File storage port for saving generated assets
    /// * `output_dir` - Directory path for storing generated assets
    /// * `batch_size` - Maximum concurrent ComfyUI requests (typically 1)
    /// * `notifier` - The notifier for waking workers
    pub fn new(
        queue: Arc<Q>,
        comfyui_client: Arc<C>,
        asset_repository: Arc<dyn AssetRepositoryPort>,
        clock: Arc<dyn ClockPort>,
        file_storage: Arc<dyn FileStoragePort>,
        output_dir: String,
        batch_size: usize,
        notifier: N,
    ) -> Self {
        Self {
            queue,
            comfyui_client,
            asset_repository,
            clock,
            file_storage,
            output_dir,
            semaphore: Arc::new(Semaphore::new(batch_size.max(1))),
            notifier,
        }
    }

    /// Enqueue an asset generation request
    pub async fn enqueue(&self, request: AssetGenerationData) -> Result<QueueItemId, QueueError> {
        self.queue.enqueue(request, PRIORITY_NORMAL).await
    }

    /// Background worker that processes asset generation requests
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
                tracing::info!("Asset generation queue worker shutting down");
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
                            tracing::info!("Asset generation queue worker shutting down");
                            break;
                        }
                        _ = self.notifier.wait_for_work(recovery_interval) => {
                            continue;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to dequeue asset generation request: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Process in spawned task - acquire permit inside the task for proper lifetime
            // Clone all needed data before spawning to avoid lifetime issues
            let semaphore = self.semaphore.clone();
            let client = self.comfyui_client.clone();
            let asset_repo = self.asset_repository.clone();
            let clock = self.clock.clone();
            let file_storage = self.file_storage.clone();
            let output_dir = self.output_dir.clone();
            let queue_clone = self.queue.clone();
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

                tracing::info!(
                    "Processing asset generation: entity_type={}, entity_id={}, workflow_id={}",
                    request.entity_type,
                    request.entity_id,
                    request.workflow_id
                );

                // Load workflow template (simplified - would load from file system)
                // For now, create a basic workflow JSON
                let workflow = serde_json::json!({
                    "prompt": request.prompt,
                    "workflow_id": request.workflow_id,
                });

                // Submit to ComfyUI
                let prompt_response = match client.queue_prompt(workflow).await {
                    Ok(response) => {
                        tracing::info!(
                            "Queued ComfyUI prompt {} for asset generation {}",
                            response.prompt_id,
                            item_id
                        );
                        response
                    }
                    Err(e) => {
                        tracing::error!("Failed to queue ComfyUI prompt: {}", e);
                        if let Err(e2) = queue_clone
                            .fail(item_id, &format!("ComfyUI queue error: {}", e))
                            .await
                        {
                            tracing::error!("Failed to mark queue item as failed: {}", e2);
                        }
                        return;
                    }
                };

                // Poll for completion with timeout
                let prompt_id = prompt_response.prompt_id.clone();
                let max_wait = Duration::from_secs(300); // 5 minutes timeout
                let poll_interval = Duration::from_secs(2);
                let start_time = clock.instant_now();

                let history_result = loop {
                    if start_time.elapsed() > max_wait {
                        tracing::error!("ComfyUI generation timed out for prompt {}", prompt_id);
                        if let Err(e) = queue_clone
                            .fail(item_id, "Generation timed out after 5 minutes")
                            .await
                        {
                            tracing::error!("Failed to mark queue item as failed: {}", e);
                        }
                        return;
                    }

                    match client.get_history(&prompt_id).await {
                        Ok(history) => {
                            if let Some(prompt_history) = history.prompts.get(&prompt_id) {
                                if prompt_history.status.completed {
                                    tracing::info!(
                                        "ComfyUI generation completed for prompt {}",
                                        prompt_id
                                    );
                                    break prompt_history.clone();
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Error polling ComfyUI history (will retry): {}", e);
                        }
                    }

                    tokio::time::sleep(poll_interval).await;
                };

                // Extract and download images from outputs
                let mut downloaded_images = Vec::new();
                for (node_id, output) in &history_result.outputs {
                    if let Some(images) = &output.images {
                        for img in images {
                            match client
                                .get_image(&img.filename, &img.subfolder, &img.r#type)
                                .await
                            {
                                Ok(bytes) => {
                                    tracing::info!(
                                        "Downloaded image {} ({} bytes) from node {}",
                                        img.filename,
                                        bytes.len(),
                                        node_id
                                    );
                                    downloaded_images.push((img.filename.clone(), bytes));
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to download image {}: {}",
                                        img.filename,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }

                if downloaded_images.is_empty() {
                    tracing::error!("No images were generated for prompt {}", prompt_id);
                    if let Err(e) = queue_clone.fail(item_id, "No images generated").await {
                        tracing::error!("Failed to mark queue item as failed: {}", e);
                    }
                    return;
                }

                // Save images and create asset records
                let entity_type = match request.entity_type.to_lowercase().as_str() {
                    "character" => EntityType::Character,
                    "location" => EntityType::Location,
                    "item" => EntityType::Item,
                    _ => EntityType::Character, // Default fallback
                };

                // Derive asset type from workflow_id
                let asset_type = if request.workflow_id.contains("portrait") {
                    AssetType::Portrait
                } else if request.workflow_id.contains("sprite") {
                    AssetType::Sprite
                } else if request.workflow_id.contains("backdrop") {
                    AssetType::Backdrop
                } else if request.workflow_id.contains("item")
                    || request.workflow_id.contains("icon")
                {
                    AssetType::ItemIcon
                } else {
                    AssetType::Portrait // Default
                };

                // Create assets directory if needed
                let assets_dir = output_dir;
                if let Err(e) = file_storage.create_dir_all(Path::new(&assets_dir)).await {
                    tracing::error!("Failed to create assets directory: {}", e);
                    if let Err(e2) = queue_clone
                        .fail(
                            item_id,
                            &format!("Failed to create assets directory: {}", e),
                        )
                        .await
                    {
                        tracing::error!("Failed to mark queue item as failed: {}", e2);
                    }
                    return;
                }

                let mut created_assets = 0;
                for (original_filename, bytes) in downloaded_images {
                    let asset_id = AssetId::new();
                    let extension = original_filename.split('.').next_back().unwrap_or("png");
                    let file_name = format!("{}_{}.{}", request.entity_id, asset_id, extension);
                    let file_path = format!("{}/{}", assets_dir, file_name);

                    // Save image to disk
                    if let Err(e) = file_storage.write(Path::new(&file_path), &bytes).await {
                        tracing::error!("Failed to save image to disk: {}", e);
                        continue;
                    }

                    // Create asset record
                    let metadata = GenerationMetadata {
                        workflow: request.workflow_id.clone(),
                        prompt: request.prompt.clone(),
                        negative_prompt: None, // Not in AssetGenerationData currently
                        seed: 0,               // ComfyUI doesn't always return seed in history
                        style_reference_id: None,
                        batch_id: wrldbldr_domain::BatchId::from_uuid(Uuid::new_v4()),
                    };

                    let asset = GalleryAsset {
                        id: asset_id,
                        entity_type: entity_type,
                        entity_id: request.entity_id.clone(),
                        asset_type: asset_type,
                        file_path,
                        is_active: created_assets == 0, // First asset is active
                        label: None,
                        generation_metadata: Some(metadata),
                        created_at: clock.now(),
                    };

                    if let Err(e) = asset_repo.create(&asset).await {
                        tracing::error!("Failed to create asset record: {}", e);
                    } else {
                        created_assets += 1;
                        tracing::info!(
                            "Created asset record {} for {}",
                            asset_id,
                            request.entity_id
                        );
                    }
                }

                // Mark as complete
                if created_assets > 0 {
                    if let Err(e) = queue_clone.complete(item_id).await {
                        tracing::error!("Failed to mark queue item as complete: {}", e);
                    } else {
                        tracing::info!(
                            "Asset generation completed: {} assets created for {}",
                            created_assets,
                            request.entity_id
                        );
                    }
                } else if let Err(e) = queue_clone
                    .fail(item_id, "Failed to create any asset records")
                    .await
                {
                    tracing::error!("Failed to mark queue item as failed: {}", e);
                }
            });
        }
    }

    /// Get queue depth (number of pending requests)
    pub async fn depth(&self) -> Result<usize, QueueError> {
        self.queue.depth().await
    }

    /// Get number of items currently processing
    pub async fn processing_count(&self) -> Result<usize, QueueError> {
        self.queue.processing_count().await
    }

    /// Clean up old completed/failed items beyond retention period
    pub async fn cleanup(&self, retention: Duration) -> anyhow::Result<u64> {
        Ok(self.queue.cleanup(retention).await? as u64)
    }

    /// Get a specific item by ID
    pub async fn get(
        &self,
        id: QueueItemId,
    ) -> Result<Option<wrldbldr_engine_ports::outbound::QueueItem<AssetGenerationData>>, QueueError>
    {
        self.queue.get(id).await
    }

    /// List items by status
    pub async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> Result<Vec<wrldbldr_engine_ports::outbound::QueueItem<AssetGenerationData>>, QueueError>
    {
        self.queue.list_by_status(status).await
    }

    /// Check if queue has capacity
    pub async fn has_capacity(&self) -> Result<bool, QueueError> {
        // Check if we have permits available
        Ok(self.semaphore.available_permits() > 0)
    }
}

// ============================================================================
// Port Implementation
// ============================================================================

#[async_trait]
impl<Q, C, N> AssetGenerationQueueServicePort for AssetGenerationQueueService<Q, C, N>
where
    Q: ProcessingQueuePort<AssetGenerationData> + Send + Sync + 'static,
    C: ComfyUIPort + Send + Sync + 'static,
    N: QueueNotificationPort + Send + Sync + 'static,
{
    async fn enqueue(&self, request: AssetGenerationRequest) -> anyhow::Result<uuid::Uuid> {
        let item = AssetGenerationData {
            world_id: request.world_id.map(WorldId::from_uuid),
            entity_type: request.entity_type,
            entity_id: request.entity_id,
            workflow_id: request.workflow_id,
            prompt: request.prompt,
            count: request.count,
            // Note: negative_prompt and style_reference_id from port are not stored in domain
            // These could be added to domain AssetGenerationData if needed
        };

        self.queue
            .enqueue(item, PRIORITY_NORMAL)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn dequeue(&self) -> anyhow::Result<Option<AssetGenerationQueueItem>> {
        let item = self
            .queue
            .dequeue()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(item.map(|i| AssetGenerationQueueItem {
            id: i.id,
            payload: AssetGenerationRequest {
                world_id: i.payload.world_id.map(|id| id.to_uuid()),
                entity_type: i.payload.entity_type,
                entity_id: i.payload.entity_id,
                workflow_id: i.payload.workflow_id,
                prompt: i.payload.prompt,
                count: i.payload.count,
                negative_prompt: None, // Not stored in domain
                style_reference_id: None, // Not stored in domain
            },
            priority: i.priority,
            enqueued_at: i.created_at,
        }))
    }

    async fn complete(&self, id: uuid::Uuid, _result: GenerationResult) -> anyhow::Result<()> {
        // The port receives a GenerationResult but the underlying queue just marks complete
        // The actual result handling is done in run_worker
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

    async fn has_capacity(&self) -> anyhow::Result<bool> {
        self.has_capacity()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn get(&self, id: uuid::Uuid) -> anyhow::Result<Option<AssetGenerationQueueItem>> {
        let item = self
            .queue
            .get(id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(item.map(|i| AssetGenerationQueueItem {
            id: i.id,
            payload: AssetGenerationRequest {
                world_id: i.payload.world_id.map(|id| id.to_uuid()),
                entity_type: i.payload.entity_type,
                entity_id: i.payload.entity_id,
                workflow_id: i.payload.workflow_id,
                prompt: i.payload.prompt,
                count: i.payload.count,
                negative_prompt: None, // Not stored in domain
                style_reference_id: None, // Not stored in domain
            },
            priority: i.priority,
            enqueued_at: i.created_at,
        }))
    }

    async fn list_by_status(
        &self,
        status: QueueItemStatus,
    ) -> anyhow::Result<Vec<AssetGenerationQueueItem>> {
        let items = self
            .queue
            .list_by_status(status)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(items
            .into_iter()
            .map(|i| AssetGenerationQueueItem {
                id: i.id,
                payload: AssetGenerationRequest {
                    world_id: i.payload.world_id.map(|id| id.to_uuid()),
                    entity_type: i.payload.entity_type,
                    entity_id: i.payload.entity_id,
                    workflow_id: i.payload.workflow_id,
                    prompt: i.payload.prompt,
                    count: i.payload.count,
                    negative_prompt: None, // Not stored in domain
                    style_reference_id: None, // Not stored in domain
                },
                priority: i.priority,
                enqueued_at: i.created_at,
            })
            .collect())
    }

    async fn cleanup(&self, retention: std::time::Duration) -> anyhow::Result<u64> {
        self.cleanup(retention).await
    }
}
