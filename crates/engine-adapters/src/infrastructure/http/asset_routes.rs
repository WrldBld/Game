//! Asset Gallery and Generation API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use std::str::FromStr;

use wrldbldr_domain::entities::{
    AssetType, BatchStatus, EntityType, GenerationBatch, GenerationRequest,
};
use wrldbldr_domain::WorldId;
use wrldbldr_domain::{AssetId, BatchId};
use wrldbldr_engine_ports::inbound::AppStatePort;
use wrldbldr_engine_ports::outbound::{AssetGenerationRequest, CreateAssetRequest};
use wrldbldr_protocol::{
    GenerateAssetRequestDto, SelectFromBatchRequestDto, UpdateAssetLabelRequestDto,
    UploadAssetRequestDto,
};

// Import conversion functions from adapters
use crate::infrastructure::dto_conversions::{gallery_asset_to_dto, generation_batch_to_dto};

// ==================== Character Gallery Routes ====================

/// List all assets for a character
pub async fn list_character_assets(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(character_id): Path<String>,
) -> Result<Json<Vec<wrldbldr_protocol::GalleryAssetResponseDto>>, (StatusCode, String)> {
    let assets = state
        .asset_service()
        .list_assets(EntityType::Character, &character_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        assets.into_iter().map(gallery_asset_to_dto).collect(),
    ))
}

/// Upload an asset to a character's gallery
pub async fn upload_character_asset(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(character_id): Path<String>,
    Json(req): Json<UploadAssetRequestDto>,
) -> Result<(StatusCode, Json<wrldbldr_protocol::GalleryAssetResponseDto>), (StatusCode, String)> {
    let asset_type =
        AssetType::from_str(&req.asset_type).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let create_request = CreateAssetRequest {
        entity_type: EntityType::Character,
        entity_id: character_id,
        asset_type,
        file_path: req.file_path,
        label: req.label,
    };

    let mut asset = state
        .asset_service()
        .create_asset(create_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if req.set_active {
        state
            .asset_service()
            .activate_asset(asset.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        asset.is_active = true;
    }

    Ok((
        StatusCode::CREATED,
        Json(gallery_asset_to_dto(asset)),
    ))
}

/// Activate an asset in a character's gallery
pub async fn activate_character_asset(
    State(state): State<Arc<dyn AppStatePort>>,
    Path((_character_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .asset_service()
        .activate_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Update an asset's label
pub async fn update_character_asset_label(
    State(state): State<Arc<dyn AppStatePort>>,
    Path((_character_id, asset_id)): Path<(String, String)>,
    Json(req): Json<UpdateAssetLabelRequestDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .asset_service()
        .update_asset_label(AssetId::from_uuid(uuid), req.label)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Delete an asset from a character's gallery
pub async fn delete_character_asset(
    State(state): State<Arc<dyn AppStatePort>>,
    Path((_character_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .asset_service()
        .delete_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Location Gallery Routes ====================

/// List all assets for a location
pub async fn list_location_assets(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(location_id): Path<String>,
) -> Result<Json<Vec<wrldbldr_protocol::GalleryAssetResponseDto>>, (StatusCode, String)> {
    let assets = state
        .asset_service()
        .list_assets(EntityType::Location, &location_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        assets
            .into_iter()
            .map(gallery_asset_to_dto)
            .collect(),
    ))
}

/// Upload an asset to a location's gallery
pub async fn upload_location_asset(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(location_id): Path<String>,
    Json(req): Json<UploadAssetRequestDto>,
) -> Result<(StatusCode, Json<wrldbldr_protocol::GalleryAssetResponseDto>), (StatusCode, String)> {
    let asset_type =
        AssetType::from_str(&req.asset_type).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let create_request = CreateAssetRequest {
        entity_type: EntityType::Location,
        entity_id: location_id,
        asset_type,
        file_path: req.file_path,
        label: req.label,
    };

    let mut asset = state
        .asset_service()
        .create_asset(create_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if req.set_active {
        state
            .asset_service()
            .activate_asset(asset.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        asset.is_active = true;
    }

    Ok((
        StatusCode::CREATED,
        Json(gallery_asset_to_dto(asset)),
    ))
}

/// Activate an asset in a location's gallery
pub async fn activate_location_asset(
    State(state): State<Arc<dyn AppStatePort>>,
    Path((_location_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .asset_service()
        .activate_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Delete an asset from a location's gallery
pub async fn delete_location_asset(
    State(state): State<Arc<dyn AppStatePort>>,
    Path((_location_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .asset_service()
        .delete_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Item Gallery Routes ====================

/// List all assets for an item
pub async fn list_item_assets(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(item_id): Path<String>,
) -> Result<Json<Vec<wrldbldr_protocol::GalleryAssetResponseDto>>, (StatusCode, String)> {
    let assets = state
        .asset_service()
        .list_assets(EntityType::Item, &item_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        assets
            .into_iter()
            .map(gallery_asset_to_dto)
            .collect(),
    ))
}

/// Upload an asset to an item's gallery
pub async fn upload_item_asset(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(item_id): Path<String>,
    Json(req): Json<UploadAssetRequestDto>,
) -> Result<(StatusCode, Json<wrldbldr_protocol::GalleryAssetResponseDto>), (StatusCode, String)> {
    let asset_type =
        AssetType::from_str(&req.asset_type).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let create_request = CreateAssetRequest {
        entity_type: EntityType::Item,
        entity_id: item_id,
        asset_type,
        file_path: req.file_path,
        label: req.label,
    };

    let mut asset = state
        .asset_service()
        .create_asset(create_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if req.set_active {
        state
            .asset_service()
            .activate_asset(asset.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        asset.is_active = true;
    }

    Ok((
        StatusCode::CREATED,
        Json(gallery_asset_to_dto(asset)),
    ))
}

/// Activate an asset in an item's gallery
pub async fn activate_item_asset(
    State(state): State<Arc<dyn AppStatePort>>,
    Path((_item_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .asset_service()
        .activate_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Delete an asset from an item's gallery
pub async fn delete_item_asset(
    State(state): State<Arc<dyn AppStatePort>>,
    Path((_item_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .asset_service()
        .delete_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Generation Queue Routes ====================

/// Queue a new asset generation request
pub async fn queue_generation(
    State(state): State<Arc<dyn AppStatePort>>,
    Json(req): Json<GenerateAssetRequestDto>,
) -> Result<(StatusCode, Json<wrldbldr_protocol::GenerationBatchResponseDto>), (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&req.world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world_id".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);

    let entity_type =
        EntityType::from_str(&req.entity_type).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let asset_type =
        AssetType::from_str(&req.asset_type).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let mut batch = GenerationBatch::new(
        world_id,
        entity_type,
        &req.entity_id,
        asset_type,
        &req.workflow,
        &req.prompt,
        req.count,
        state.clock().now(),
    );

    if let Some(neg) = req.negative_prompt {
        batch = batch.with_negative_prompt(neg);
    }

    if let Some(ref_id) = req.style_reference_id {
        let uuid = Uuid::parse_str(&ref_id).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid style reference ID".to_string(),
            )
        })?;
        batch = batch.with_style_reference(AssetId::from_uuid(uuid));
    }

    let batch = state
        .asset_service()
        .create_batch(batch)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Queue generation jobs for each image in the batch
    for i in 0..batch.count {
        let generation_item = AssetGenerationRequest {
            world_id: None, // Generation requests don't require world context
            entity_type: format!("{:?}", batch.entity_type),
            entity_id: batch.entity_id.clone(),
            workflow_id: batch.workflow.clone(),
            prompt: batch.prompt.clone(),
            count: 1, // Each item generates one image
            negative_prompt: None,
            style_reference_id: None,
        };

        match state
            .asset_generation_queue_service()
            .enqueue(generation_item)
            .await
        {
            Ok(item_id) => {
                tracing::debug!(
                    "Queued generation item {} for batch {} (image {}/{})",
                    item_id,
                    batch.id,
                    i + 1,
                    batch.count
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to queue generation job for batch {}: {}",
                    batch.id,
                    e
                );
                // Continue queuing other items even if one fails
            }
        }
    }

    tracing::info!(
        "Queued generation batch: {} for {} {} ({} images)",
        batch.id,
        entity_type,
        req.entity_id,
        batch.count
    );

    Ok((
        StatusCode::CREATED,
        Json(generation_batch_to_dto(batch)),
    ))
}

/// List the generation queue for a specific world
pub async fn list_queue(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<wrldbldr_protocol::GenerationBatchResponseDto>>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world_id".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);

    let batches = state
        .asset_service()
        .list_active_batches_by_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        batches
            .into_iter()
            .map(generation_batch_to_dto)
            .collect(),
    ))
}

/// List batches ready for selection
pub async fn list_ready_batches(
    State(state): State<Arc<dyn AppStatePort>>,
) -> Result<Json<Vec<wrldbldr_protocol::GenerationBatchResponseDto>>, (StatusCode, String)> {
    let batches = state
        .asset_service()
        .list_ready_batches()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        batches
            .into_iter()
            .map(generation_batch_to_dto)
            .collect(),
    ))
}

/// Get a batch by ID
pub async fn get_batch(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(batch_id): Path<String>,
) -> Result<Json<wrldbldr_protocol::GenerationBatchResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&batch_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid batch ID".to_string()))?;

    let batch = state
        .asset_service()
        .get_batch(BatchId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Batch not found".to_string()))?;

    Ok(Json(generation_batch_to_dto(batch)))
}

/// Get assets from a completed batch
pub async fn get_batch_assets(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(batch_id): Path<String>,
) -> Result<Json<Vec<wrldbldr_protocol::GalleryAssetResponseDto>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&batch_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid batch ID".to_string()))?;

    let batch = state
        .asset_service()
        .get_batch(BatchId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Batch not found".to_string()))?;

    let mut assets = Vec::new();
    for asset_id in batch.assets {
        if let Some(asset) = state
            .asset_service()
            .get_asset(asset_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        {
            assets.push(gallery_asset_to_dto(asset));
        }
    }

    Ok(Json(assets))
}

/// Select assets from a completed batch
pub async fn select_from_batch(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(batch_id): Path<String>,
    Json(req): Json<SelectFromBatchRequestDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&batch_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid batch ID".to_string()))?;

    let batch = state
        .asset_service()
        .get_batch(BatchId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Batch not found".to_string()))?;

    // Mark batch as completed
    state
        .asset_service()
        .update_batch_status(batch.id, BatchStatus::Completed)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Apply labels to selected assets
    for (i, asset_id_str) in req.selected_assets.iter().enumerate() {
        let asset_uuid = Uuid::parse_str(asset_id_str).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid asset ID in selection".to_string(),
            )
        })?;

        let label = req.labels.get(i).cloned().flatten();
        if label.is_some() {
            state
                .asset_service()
                .update_asset_label(AssetId::from_uuid(asset_uuid), label)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    // Delete unselected assets if requested
    if req.discard_others {
        let selected_set: std::collections::HashSet<_> = req.selected_assets.iter().collect();
        for asset_id in &batch.assets {
            let asset_id_str = asset_id.to_string();
            if !selected_set.contains(&asset_id_str) {
                state
                    .asset_service()
                    .delete_asset(*asset_id)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }
    }

    Ok(StatusCode::OK)
}

/// Cancel a queued batch
pub async fn cancel_batch(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(batch_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&batch_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid batch ID".to_string()))?;

    let batch = state
        .asset_service()
        .get_batch(BatchId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Batch not found".to_string()))?;

    // Can only cancel queued batches
    if !batch.status.is_queued() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Can only cancel queued batches".to_string(),
        ));
    }

    state
        .asset_service()
        .delete_batch(batch.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Retry a failed batch by creating a new batch with the same parameters
pub async fn retry_batch(
    State(state): State<Arc<dyn AppStatePort>>,
    Path(batch_id): Path<String>,
) -> Result<Json<wrldbldr_protocol::GenerationBatchResponseDto>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&batch_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid batch ID".to_string()))?;

    // Get the original batch
    let original_batch = state
        .asset_service()
        .get_batch(BatchId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Batch not found".to_string()))?;

    // Can only retry failed batches
    if !matches!(original_batch.status, BatchStatus::Failed { .. }) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Can only retry failed batches".to_string(),
        ));
    }

    // Create new batch with same parameters
    let retry_request = GenerationRequest {
        world_id: original_batch.world_id,
        entity_type: original_batch.entity_type,
        entity_id: original_batch.entity_id,
        asset_type: original_batch.asset_type,
        workflow: original_batch.workflow,
        prompt: original_batch.prompt,
        negative_prompt: original_batch.negative_prompt,
        count: original_batch.count,
        style_reference_id: original_batch.style_reference_id,
    };

    let new_batch = retry_request.into_batch(state.clock().now());

    // Create the new batch
    let created_batch = state
        .asset_service()
        .create_batch(new_batch.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Start processing the new batch
    state
        .generation_service()
        .start_batch_processing(created_batch.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(generation_batch_to_dto(created_batch)))
}
