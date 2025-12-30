//! Asset-related DTO conversions

use wrldbldr_domain::entities::{GalleryAsset, GenerationBatch};
use wrldbldr_domain_types::BatchStatus;
use wrldbldr_protocol::{GalleryAssetResponseDto, GenerationBatchResponseDto};

/// Convert GalleryAsset to GalleryAssetResponseDto
pub fn gallery_asset_to_dto(a: GalleryAsset) -> GalleryAssetResponseDto {
    let is_generated = a.is_generated();
    GalleryAssetResponseDto {
        id: a.id.to_string(),
        entity_type: a.entity_type.to_string(),
        entity_id: a.entity_id,
        asset_type: a.asset_type.to_string(),
        file_path: a.file_path,
        is_active: a.is_active,
        label: a.label,
        is_generated,
        style_reference_id: None,
        created_at: a.created_at.to_rfc3339(),
    }
}

/// Convert GalleryAsset reference to GalleryAssetResponseDto
pub fn gallery_asset_ref_to_dto(a: &GalleryAsset) -> GalleryAssetResponseDto {
    let is_generated = a.is_generated();
    GalleryAssetResponseDto {
        id: a.id.to_string(),
        entity_type: a.entity_type.to_string(),
        entity_id: a.entity_id.clone(),
        asset_type: a.asset_type.to_string(),
        file_path: a.file_path.clone(),
        is_active: a.is_active,
        label: a.label.clone(),
        is_generated,
        style_reference_id: None,
        created_at: a.created_at.to_rfc3339(),
    }
}

/// Convert GenerationBatch to GenerationBatchResponseDto
pub fn generation_batch_to_dto(b: GenerationBatch) -> GenerationBatchResponseDto {
    let (status, progress) = match &b.status {
        BatchStatus::Queued => ("Queued".to_string(), None),
        BatchStatus::Generating { progress } => ("Generating".to_string(), Some(*progress)),
        BatchStatus::ReadyForSelection => ("ReadyForSelection".to_string(), Some(100)),
        BatchStatus::Completed => ("Completed".to_string(), Some(100)),
        BatchStatus::Failed { error } => (format!("Failed: {}", error), None),
    };

    GenerationBatchResponseDto {
        id: b.id.to_string(),
        world_id: b.world_id.to_string(),
        entity_type: b.entity_type.to_string(),
        entity_id: b.entity_id,
        asset_type: b.asset_type.to_string(),
        workflow: b.workflow,
        prompt: b.prompt,
        count: b.count,
        status,
        progress,
        asset_count: b.assets.len(),
        requested_at: b.requested_at.to_rfc3339(),
        completed_at: b.completed_at.map(|t| t.to_rfc3339()),
    }
}

/// Convert GenerationBatch reference to GenerationBatchResponseDto
pub fn generation_batch_ref_to_dto(b: &GenerationBatch) -> GenerationBatchResponseDto {
    let (status, progress) = match &b.status {
        BatchStatus::Queued => ("Queued".to_string(), None),
        BatchStatus::Generating { progress } => ("Generating".to_string(), Some(*progress)),
        BatchStatus::ReadyForSelection => ("ReadyForSelection".to_string(), Some(100)),
        BatchStatus::Completed => ("Completed".to_string(), Some(100)),
        BatchStatus::Failed { error } => (format!("Failed: {}", error), None),
    };

    GenerationBatchResponseDto {
        id: b.id.to_string(),
        world_id: b.world_id.to_string(),
        entity_type: b.entity_type.to_string(),
        entity_id: b.entity_id.clone(),
        asset_type: b.asset_type.to_string(),
        workflow: b.workflow.clone(),
        prompt: b.prompt.clone(),
        count: b.count,
        status,
        progress,
        asset_count: b.assets.len(),
        requested_at: b.requested_at.to_rfc3339(),
        completed_at: b.completed_at.map(|t| t.to_rfc3339()),
    }
}
