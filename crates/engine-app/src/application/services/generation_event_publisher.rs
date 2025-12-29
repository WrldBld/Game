//! Generation Event Publisher - Maps GenerationEvents to DomainEvents
//!
//! This service listens to the GenerationEvent channel and publishes
//! corresponding DomainEvents through the event bus.

use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;

use wrldbldr_domain::DomainEvent;
use wrldbldr_engine_ports::outbound::EventBusPort;
use crate::application::services::generation_service::GenerationEvent;

/// Publisher that converts GenerationEvents to DomainEvents
pub struct GenerationEventPublisher {
    event_bus: Arc<dyn EventBusPort>,
}

impl GenerationEventPublisher {
    /// Create a new publisher
    pub fn new(event_bus: Arc<dyn EventBusPort>) -> Self {
        Self { event_bus }
    }

    /// Run the publisher, consuming generation events and publishing domain events
    ///
    /// This should be spawned as a background task
    ///
    /// # Arguments
    /// * `generation_event_rx` - Channel receiver for generation events
    /// * `cancel_token` - Token to signal graceful shutdown
    pub async fn run(self, mut generation_event_rx: Receiver<GenerationEvent>, cancel_token: CancellationToken) {
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    tracing::info!("Generation event publisher shutting down (cancelled)");
                    break;
                }
                event = generation_event_rx.recv() => {
                    match event {
                        Some(event) => {
                            let domain_event = self.map_to_domain_event(event);
                            if let Some(domain_event) = domain_event {
                                if let Err(e) = self.event_bus.publish(domain_event).await {
                                    tracing::error!("Failed to publish generation domain event: {}", e);
                                }
                            }
                        }
                        None => {
                            tracing::info!("Generation event publisher shutting down (channel closed)");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Map a GenerationEvent to a DomainEvent.
    ///
    /// For now, generation events are not session-scoped and are broadcast
    /// based on world context in the subscriber. When generation is invoked
    /// from a specific live session in the future, this mapping can be
    /// extended to populate `session_id`.
    fn map_to_domain_event(&self, event: GenerationEvent) -> Option<DomainEvent> {
        match event {
            GenerationEvent::BatchQueued {
                batch_id,
                entity_type,
                entity_id,
                asset_type,
                position,
            } => Some(DomainEvent::GenerationBatchQueued {
                batch_id: batch_id.to_string(),
                entity_type: entity_type.to_string(),
                entity_id,
                asset_type: asset_type.to_string(),
                position,
                session_id: None,
            }),
            GenerationEvent::BatchProgress {
                batch_id,
                progress,
            } => Some(DomainEvent::GenerationBatchProgress {
                batch_id: batch_id.to_string(),
                // GenerationEvent uses u8 (0-100), DomainEvent uses f32 (0.0-1.0)
                progress: progress as f32 / 100.0,
                session_id: None,
            }),
            GenerationEvent::BatchComplete {
                batch_id,
                entity_type,
                entity_id,
                asset_type,
                asset_count,
            } => Some(DomainEvent::GenerationBatchCompleted {
                batch_id: batch_id.to_string(),
                entity_type: entity_type.to_string(),
                entity_id,
                asset_type: asset_type.to_string(),
                asset_count,
                session_id: None,
            }),
            GenerationEvent::BatchFailed {
                batch_id,
                entity_type,
                entity_id,
                asset_type,
                error,
            } => Some(DomainEvent::GenerationBatchFailed {
                batch_id: batch_id.to_string(),
                entity_type: entity_type.to_string(),
                entity_id,
                asset_type: asset_type.to_string(),
                error,
                session_id: None,
            }),
            GenerationEvent::SuggestionQueued {
                request_id,
                field_type,
                entity_id,
                world_id,
            } => Some(DomainEvent::SuggestionQueued {
                request_id,
                field_type,
                entity_id,
                world_id,
            }),
            GenerationEvent::SuggestionProgress { request_id, status, world_id } => {
                Some(DomainEvent::SuggestionProgress {
                    request_id,
                    status,
                    world_id,
                })
            }
            GenerationEvent::SuggestionComplete {
                request_id,
                field_type,
                suggestions,
                world_id,
            } => Some(DomainEvent::SuggestionCompleted {
                request_id,
                field_type,
                suggestions,
                world_id,
            }),
            GenerationEvent::SuggestionFailed {
                request_id,
                field_type,
                error,
                world_id,
            } => Some(DomainEvent::SuggestionFailed {
                request_id,
                field_type,
                error,
                world_id,
            }),
        }
    }
}

