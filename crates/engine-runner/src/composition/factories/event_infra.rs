//! Event Infrastructure Factory
//!
//! Creates event system components including domain event repository,
//! event bus, event notifier, and event channels.
//!
//! This is Level 2a in the composition hierarchy - can run in parallel with queue_services.

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;

use wrldbldr_engine_adapters::infrastructure::config::AppConfig;
use wrldbldr_engine_adapters::infrastructure::event_bus::{InProcessEventNotifier, SqliteEventBus};
use wrldbldr_engine_adapters::infrastructure::repositories::{
    SqliteDomainEventRepository, SqliteGenerationReadStateRepository,
};
use wrldbldr_engine_app::application::services::{
    generation_service::GenerationEvent, ChallengeApprovalEvent,
};
use wrldbldr_engine_ports::outbound::{
    DomainEventRepositoryPort, EventBusPort, GenerationReadStatePort,
};

/// Buffer size for event channels (provides backpressure)
const EVENT_CHANNEL_BUFFER: usize = 256;

/// Event infrastructure components.
///
/// Contains the event bus, repositories, and channels for event-driven communication.
pub struct EventInfrastructure {
    /// Domain event bus for publish-subscribe
    pub event_bus: Arc<dyn EventBusPort>,

    /// Concrete event notifier (needed by services that require Clone)
    pub event_notifier_concrete: InProcessEventNotifier,

    /// Domain event repository
    pub domain_event_repository: Arc<dyn DomainEventRepositoryPort>,

    /// Generation read state repository
    pub generation_read_state_repository: Arc<dyn GenerationReadStatePort>,

    /// Receiver for generation events (for GenerationEventPublisher)
    pub generation_event_rx: mpsc::Receiver<GenerationEvent>,

    /// Sender for generation events (passed to services)
    pub generation_event_tx: mpsc::Sender<GenerationEvent>,

    /// Receiver for challenge approval events
    pub challenge_approval_rx: mpsc::Receiver<ChallengeApprovalEvent>,

    /// Sender for challenge approval events
    pub challenge_approval_tx: mpsc::Sender<ChallengeApprovalEvent>,
}

/// Creates event infrastructure components.
///
/// This factory initializes:
/// - SQLite event database
/// - Domain event repository
/// - Generation read state repository
/// - Event notifier and event bus
/// - Event channels for generation and challenge approval
///
/// # Arguments
/// * `config` - Application configuration (for SQLite path)
///
/// # Returns
/// * `EventInfrastructure` with all event components
pub async fn create_event_infrastructure(config: &AppConfig) -> Result<EventInfrastructure> {
    // =========================================================================
    // SQLite event database
    // =========================================================================
    let event_db_path = config.queue.sqlite_path.replace(".db", "_events.db");
    if let Some(parent) = std::path::Path::new(&event_db_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("Failed to create event database directory: {}", e))?;
    }
    let event_pool = sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", event_db_path))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to event database: {}", e))?;
    tracing::info!("Connected to event database: {}", event_db_path);

    // =========================================================================
    // Domain event repository
    // =========================================================================
    let domain_event_repository_impl = SqliteDomainEventRepository::new(event_pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize domain event repository: {}", e))?;

    // Generation read state repository shares the same pool
    let generation_read_state_repository_impl =
        SqliteGenerationReadStateRepository::new(domain_event_repository_impl.pool().clone());
    generation_read_state_repository_impl.init_schema().await?;
    let generation_read_state_repository: Arc<dyn GenerationReadStatePort> =
        Arc::new(generation_read_state_repository_impl);

    let domain_event_repository: Arc<dyn DomainEventRepositoryPort> =
        Arc::new(domain_event_repository_impl);

    // =========================================================================
    // Event notifier and bus
    // =========================================================================
    let event_notifier_concrete = InProcessEventNotifier::new();
    let event_bus: Arc<dyn EventBusPort> = Arc::new(SqliteEventBus::new(
        domain_event_repository.clone(),
        event_notifier_concrete.clone(),
    ));
    tracing::info!("Initialized event bus and notifier");

    // =========================================================================
    // Event channels
    // =========================================================================
    let (generation_event_tx, generation_event_rx) = mpsc::channel(EVENT_CHANNEL_BUFFER);
    let (challenge_approval_tx, challenge_approval_rx) = mpsc::channel(EVENT_CHANNEL_BUFFER);
    tracing::info!(
        "Created event channels with {} buffer size",
        EVENT_CHANNEL_BUFFER
    );

    Ok(EventInfrastructure {
        event_bus,
        event_notifier_concrete,
        domain_event_repository,
        generation_read_state_repository,
        generation_event_rx,
        generation_event_tx,
        challenge_approval_rx,
        challenge_approval_tx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_channel_buffer_size() {
        assert_eq!(EVENT_CHANNEL_BUFFER, 256);
    }

    #[test]
    fn test_event_infrastructure_types() {
        // Verify the struct has expected fields with correct types
        fn _verify_types(infra: &EventInfrastructure) {
            let _: &Arc<dyn EventBusPort> = &infra.event_bus;
            let _: &InProcessEventNotifier = &infra.event_notifier_concrete;
            let _: &Arc<dyn DomainEventRepositoryPort> = &infra.domain_event_repository;
            let _: &Arc<dyn GenerationReadStatePort> = &infra.generation_read_state_repository;
        }
    }
}
