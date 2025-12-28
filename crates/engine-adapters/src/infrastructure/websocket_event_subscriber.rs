//! WebSocket Event Subscriber - Maps DomainEvents to ServerMessages
//!
//! This subscriber polls the event repository and broadcasts relevant events
//! to WebSocket clients via the WorldConnectionManager.
//!
//! Uses DomainEventRepositoryPort and converts domain events to protocol
//! ServerMessage internally (adapters layer handles wire format conversion).

use std::sync::Arc;
use std::time::Duration;

use wrldbldr_domain::DomainEvent;
use wrldbldr_engine_ports::outbound::DomainEventRepositoryPort;
use crate::infrastructure::event_bus::InProcessEventNotifier;
use crate::infrastructure::world_connection_manager::SharedWorldConnectionManager;
use wrldbldr_protocol::ServerMessage;

/// WebSocket event subscriber
pub struct WebSocketEventSubscriber {
    repository: Arc<dyn DomainEventRepositoryPort>,
    notifier: InProcessEventNotifier,
    world_connection_manager: SharedWorldConnectionManager,
    poll_interval: Duration,
}

impl WebSocketEventSubscriber {
    /// Create a new WebSocket event subscriber
    pub fn new(
        repository: Arc<dyn DomainEventRepositoryPort>,
        notifier: InProcessEventNotifier,
        world_connection_manager: SharedWorldConnectionManager,
        poll_interval_seconds: u64,
    ) -> Self {
        Self {
            repository,
            notifier,
            world_connection_manager,
            poll_interval: Duration::from_secs(poll_interval_seconds),
        }
    }

    /// Run the subscriber (spawned as background task)
    pub async fn run(self) {
        let mut last_event_id: i64 = 0;
        tracing::info!("Starting WebSocket event subscriber");

        loop {
            tokio::select! {
                _ = self.notifier.wait() => {
                    tracing::debug!("Event notifier triggered, fetching new events");
                    if let Err(e) = self.process_new_events(&mut last_event_id).await {
                        tracing::error!("Failed to process new events: {}", e);
                    }
                }
                _ = tokio::time::sleep(self.poll_interval) => {
                    tracing::debug!("Polling for new events (last_id: {})", last_event_id);
                    if let Err(e) = self.process_new_events(&mut last_event_id).await {
                        tracing::error!("Failed to process new events (poll): {}", e);
                    }
                }
            }
        }
    }

    /// Fetch and process new events since last_event_id
    async fn process_new_events(&self, last_event_id: &mut i64) -> anyhow::Result<()> {
        const BATCH_SIZE: u32 = 100;

        // Fetch events since last_event_id
        let events = self
            .repository
            .fetch_since(*last_event_id, BATCH_SIZE)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch events: {}", e))?;

        if events.is_empty() {
            return Ok(());
        }

        tracing::debug!("Processing {} new events", events.len());

        for (event_id, event, _timestamp) in events {
            // Map DomainEvent to ServerMessage and determine target world
            if let Some(message) = self.map_to_server_message(&event) {
                let target_world = event.world_id();

                if let Some(world_uuid) = target_world {
                    // Route to specific world
                    self.world_connection_manager
                        .broadcast_to_world(world_uuid, message)
                        .await;
                } else {
                    // No world_id: broadcast to all connected worlds
                    let world_ids = self.world_connection_manager.get_all_world_ids().await;
                    for world_id in world_ids {
                        self.world_connection_manager
                            .broadcast_to_world(world_id, message.clone())
                            .await;
                    }
                }
            }

            // Update last processed event ID
            *last_event_id = event_id;
        }

        tracing::debug!("Processed events up to ID {}", *last_event_id);
        Ok(())
    }

    /// Map a DomainEvent to a ServerMessage
    ///
    /// Returns None if the event is not relevant to WebSocket clients.
    /// This is where adapters layer converts domain events to wire format.
    fn map_to_server_message(&self, event: &DomainEvent) -> Option<ServerMessage> {
        match event {
            DomainEvent::GenerationBatchQueued {
                batch_id,
                entity_type,
                entity_id,
                asset_type,
                position,
                ..
            } => Some(ServerMessage::GenerationQueued {
                batch_id: batch_id.to_string(),
                entity_type: entity_type.clone(),
                entity_id: entity_id.to_string(),
                asset_type: asset_type.clone(),
                position: *position,
            }),
            DomainEvent::GenerationBatchProgress { batch_id, progress, .. } => {
                Some(ServerMessage::GenerationProgress {
                    batch_id: batch_id.to_string(),
                    progress: *progress,
                })
            }
            DomainEvent::GenerationBatchCompleted {
                batch_id,
                asset_count,
                ..
            } => Some(ServerMessage::GenerationComplete {
                batch_id: batch_id.to_string(),
                asset_count: *asset_count,
            }),
            DomainEvent::GenerationBatchFailed { batch_id, error, .. } => {
                Some(ServerMessage::GenerationFailed {
                    batch_id: batch_id.to_string(),
                    error: error.clone(),
                })
            }
            DomainEvent::SuggestionQueued {
                request_id,
                field_type,
                entity_id,
                ..
            } => Some(ServerMessage::SuggestionQueued {
                request_id: request_id.clone(),
                field_type: field_type.clone(),
                entity_id: entity_id.to_string(),
            }),
            DomainEvent::SuggestionProgress { request_id, status, .. } => {
                Some(ServerMessage::SuggestionProgress {
                    request_id: request_id.clone(),
                    status: status.clone(),
                })
            }
            DomainEvent::SuggestionCompleted {
                request_id,
                suggestions,
                ..
            } => Some(ServerMessage::SuggestionComplete {
                request_id: request_id.clone(),
                suggestions: suggestions.clone(),
            }),
            DomainEvent::SuggestionFailed {
                request_id,
                error,
                ..
            } => Some(ServerMessage::SuggestionFailed {
                request_id: request_id.clone(),
                error: error.clone(),
            }),
            // Story events, narrative events, and challenges are not yet broadcasted via WebSocket
            // These could be added in the future if needed
            DomainEvent::StoryEventCreated { .. }
            | DomainEvent::NarrativeEventTriggered { .. }
            | DomainEvent::ChallengeResolved { .. } => None,
        }
    }
}

