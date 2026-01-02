//! Generation Queue Projection Service
//!
//! Builds a unified `GenerationQueueSnapshot` for a given (user, world) from:
//! - Active generation batches exposed by `AssetService`
//! - Suggestion-related `DomainEvent`s from the event repository
//! - Per-user/world read markers from `GenerationReadStatePort`
//!
//! This keeps HTTP routes thin and centralizes queue reconstruction logic.

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use crate::application::services::asset_service::{AssetService, AssetServiceImpl};
use wrldbldr_domain::entities::{BatchStatus, GenerationBatch};
use wrldbldr_domain::{DomainEvent, WorldId};
use crate::application::services::internal::{
    GenerationBatchSnapshot as PortGenerationBatchSnapshot, GenerationQueueProjectionUseCasePort,
    GenerationQueueSnapshot as PortGenerationQueueSnapshot,
    SuggestionTaskSnapshot as PortSuggestionTaskSnapshot,
};
use wrldbldr_engine_ports::outbound::{
    DomainEventRepositoryPort, GenerationReadKind, GenerationReadStatePort,
};
/// App-layer DTO for generation batches (avoids protocol dependency)
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationBatchDto {
    pub id: String,
    pub world_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub asset_type: String,
    pub workflow: String,
    pub prompt: String,
    pub count: u8,
    pub status: String,
    pub progress: Option<u8>,
    pub asset_count: usize,
    pub requested_at: String,
    pub completed_at: Option<String>,
}

/// Convert GenerationBatch to GenerationBatchDto
fn generation_batch_to_dto(b: GenerationBatch) -> GenerationBatchDto {
    let (status, progress) = match &b.status {
        BatchStatus::Queued => ("Queued".to_string(), None),
        BatchStatus::Generating { progress } => ("Generating".to_string(), Some(*progress)),
        BatchStatus::ReadyForSelection => ("ReadyForSelection".to_string(), Some(100)),
        BatchStatus::Completed => ("Completed".to_string(), Some(100)),
        BatchStatus::Failed { error } => (format!("Failed: {}", error), None),
    };

    GenerationBatchDto {
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

/// Snapshot DTO for suggestion tasks, mirrored from `infrastructure::http::queue_routes`.
#[derive(Debug, serde::Serialize)]
pub struct SuggestionTaskSnapshot {
    pub request_id: String,
    pub field_type: String,
    pub entity_id: Option<String>,
    pub status: String,
    pub suggestions: Option<Vec<String>>,
    pub error: Option<String>,
    pub is_read: bool,
}

/// Unified generation queue snapshot (batches + suggestions).
///
/// This is intentionally kept in the application layer so HTTP routes and
/// WebSocket projections can share the same reconstruction logic.
#[derive(Debug, serde::Serialize)]
pub struct GenerationQueueSnapshot {
    pub batches: Vec<GenerationBatchResponseDtoWithRead>,
    pub suggestions: Vec<SuggestionTaskSnapshot>,
}

#[derive(Debug, serde::Serialize)]
pub struct GenerationBatchResponseDtoWithRead {
    #[serde(flatten)]
    pub batch: GenerationBatchDto,
    pub is_read: bool,
}

/// Service responsible for projecting the current generation queue state.
pub struct GenerationQueueProjectionService {
    asset_service: AssetServiceImpl,
    domain_event_repository: std::sync::Arc<dyn DomainEventRepositoryPort>,
    read_state: std::sync::Arc<dyn GenerationReadStatePort>,
}

impl GenerationQueueProjectionService {
    pub fn new(
        asset_service: AssetServiceImpl,
        domain_event_repository: std::sync::Arc<dyn DomainEventRepositoryPort>,
        read_state: std::sync::Arc<dyn GenerationReadStatePort>,
    ) -> Self {
        Self {
            asset_service,
            domain_event_repository,
            read_state,
        }
    }

    /// Build a `GenerationQueueSnapshot` for the given user + world.
    ///
    /// When `user_id` is `None`, the snapshot will treat all items as unread.
    pub async fn project_queue(
        &self,
        user_id: Option<&str>,
        world_id: WorldId,
    ) -> anyhow::Result<GenerationQueueSnapshot> {
        let world_key = world_id.to_string();

        // 1. Compute read markers for this user/world
        let mut read_batches: HashSet<String> = HashSet::new();
        let mut read_suggestions: HashSet<String> = HashSet::new();

        if let Some(uid) = user_id {
            if let Ok(markers) = self
                .read_state
                .list_read_for_user_world(uid, &world_key)
                .await
            {
                for (item_id, kind) in markers {
                    match kind {
                        GenerationReadKind::Batch => {
                            read_batches.insert(item_id);
                        }
                        GenerationReadKind::Suggestion => {
                            read_suggestions.insert(item_id);
                        }
                    }
                }
            }
        }

        // 2. Image batches from AssetService - filtered by world_id
        let batches = AssetService::list_active_batches_by_world(&self.asset_service, world_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|b| {
                let dto = generation_batch_to_dto(b);
                let is_read = read_batches.contains(&dto.id);
                GenerationBatchResponseDtoWithRead {
                    batch: dto,
                    is_read,
                }
            })
            .collect();

        // 3. Suggestion tasks reconstructed from recent DomainEvents
        let mut suggestions_map: HashMap<String, SuggestionTaskSnapshot> = HashMap::new();

        if let Ok(events) = self.domain_event_repository.fetch_since(0, 500).await {
            for (_id, event, _ts) in events {
                match event {
                    DomainEvent::SuggestionQueued {
                        request_id,
                        field_type,
                        entity_id,
                        ..
                    } => {
                        let entry = suggestions_map.entry(request_id.clone()).or_insert(
                            SuggestionTaskSnapshot {
                                request_id,
                                field_type,
                                entity_id,
                                status: "queued".to_string(),
                                suggestions: None,
                                error: None,
                                is_read: false,
                            },
                        );
                        entry.status = "queued".to_string();
                    }
                    DomainEvent::SuggestionProgress { request_id, .. } => {
                        let entry = suggestions_map.entry(request_id.clone()).or_insert(
                            SuggestionTaskSnapshot {
                                request_id,
                                field_type: String::new(),
                                entity_id: None,
                                status: "processing".to_string(),
                                suggestions: None,
                                error: None,
                                is_read: false,
                            },
                        );
                        entry.status = "processing".to_string();
                    }
                    DomainEvent::SuggestionCompleted {
                        request_id,
                        field_type,
                        suggestions,
                        ..
                    } => {
                        let entry = suggestions_map.entry(request_id.clone()).or_insert(
                            SuggestionTaskSnapshot {
                                request_id,
                                field_type: field_type.clone(),
                                entity_id: None,
                                status: "ready".to_string(),
                                suggestions: Some(suggestions.clone()),
                                error: None,
                                is_read: false,
                            },
                        );
                        entry.field_type = field_type;
                        entry.status = "ready".to_string();
                        entry.suggestions = Some(suggestions);
                        entry.error = None;
                    }
                    DomainEvent::SuggestionFailed {
                        request_id,
                        field_type,
                        error,
                        ..
                    } => {
                        let entry = suggestions_map.entry(request_id.clone()).or_insert(
                            SuggestionTaskSnapshot {
                                request_id,
                                field_type: field_type.clone(),
                                entity_id: None,
                                status: "failed".to_string(),
                                suggestions: None,
                                error: Some(error.clone()),
                                is_read: false,
                            },
                        );
                        entry.field_type = field_type;
                        entry.status = "failed".to_string();
                        entry.error = Some(error);
                    }
                    _ => {}
                }
            }
        }

        let mut suggestions: Vec<SuggestionTaskSnapshot> = suggestions_map.into_values().collect();
        for s in &mut suggestions {
            if read_suggestions.contains(&s.request_id) {
                s.is_read = true;
            }
        }

        Ok(GenerationQueueSnapshot {
            batches,
            suggestions,
        })
    }
}

// Implementation of the port trait for hexagonal architecture compliance
#[async_trait]
impl GenerationQueueProjectionUseCasePort for GenerationQueueProjectionService {
    async fn project_queue(
        &self,
        user_id: Option<String>,
        world_id: WorldId,
    ) -> anyhow::Result<PortGenerationQueueSnapshot> {
        // Delegate to the internal method
        let snapshot = self.project_queue(user_id.as_deref(), world_id).await?;

        // Convert internal types to port types
        // The DTO has: id, world_id, entity_type, entity_id, asset_type, workflow, prompt,
        //              count, status, progress, asset_count, requested_at, completed_at
        // The port expects: id, world_id, entity_type, entity_id, status, item_count, completed_count, is_read
        let batches = snapshot
            .batches
            .into_iter()
            .map(|b| {
                // Calculate completed_count from progress (0-100%)
                let item_count = b.batch.count as usize;
                let completed_count = b
                    .batch
                    .progress
                    .map_or(0, |p| ((p as usize) * item_count / 100).min(item_count));
                PortGenerationBatchSnapshot {
                    id: b.batch.id,
                    world_id: b.batch.world_id,
                    entity_type: b.batch.entity_type,
                    entity_id: Some(b.batch.entity_id),
                    status: b.batch.status,
                    item_count,
                    completed_count,
                    is_read: b.is_read,
                }
            })
            .collect();

        let suggestions = snapshot
            .suggestions
            .into_iter()
            .map(|s| PortSuggestionTaskSnapshot {
                request_id: s.request_id,
                field_type: s.field_type,
                entity_id: s.entity_id,
                status: s.status,
                suggestions: s.suggestions,
                error: s.error,
                is_read: s.is_read,
            })
            .collect();

        Ok(PortGenerationQueueSnapshot {
            batches,
            suggestions,
        })
    }
}
