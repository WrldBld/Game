//! Asset Generation Queue Service - Concurrency-controlled asset generation
//!
//! This service manages the AssetGenerationQueue, which processes ComfyUI
//! requests with controlled concurrency (typically batch_size=1).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use wrldbldr_domain::entities::{AssetType, EntityType, GalleryAsset, GenerationMetadata};
use wrldbldr_domain::value_objects::AssetGenerationData;
use wrldbldr_domain::AssetId;
use wrldbldr_engine_ports::outbound::{
    AssetRepositoryPort, ClockPort, ComfyUIPort, ProcessingQueuePort, QueueError, QueueItemId,
    QueueNotificationPort,
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
    /// * `batch_size` - Maximum concurrent ComfyUI requests (typically 1)
    /// * `notifier` - The notifier for waking workers
    pub fn new(
        queue: Arc<Q>,
        comfyui_client: Arc<C>,
        asset_repository: Arc<dyn AssetRepositoryPort>,
        clock: Arc<dyn ClockPort>,
        batch_size: usize,
        notifier: N,
    ) -> Self {
        Self {
            queue,
            comfyui_client,
            asset_repository,
            clock,
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
                let assets_dir = PathBuf::from("data/generated_assets");
                if let Err(e) = tokio::fs::create_dir_all(&assets_dir).await {
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
                    let extension = original_filename.split('.').last().unwrap_or("png");
                    let file_name = format!("{}_{}.{}", request.entity_id, asset_id, extension);
                    let file_path = assets_dir.join(&file_name);

                    // Save image to disk
                    if let Err(e) = tokio::fs::write(&file_path, &bytes).await {
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
                        entity_type: entity_type.clone(),
                        entity_id: request.entity_id.clone(),
                        asset_type: asset_type.clone(),
                        file_path: file_path.to_string_lossy().to_string(),
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
                } else {
                    if let Err(e) = queue_clone
                        .fail(item_id, "Failed to create any asset records")
                        .await
                    {
                        tracing::error!("Failed to mark queue item as failed: {}", e);
                    }
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
}
