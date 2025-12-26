use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::get, Router};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use wrldbldr_domain::WorldId;
use wrldbldr_engine_app::application::services::GenerationEventPublisher;
use wrldbldr_engine_ports::outbound::{ApprovalQueuePort, CharacterRepositoryPort, PlayerCharacterRepositoryPort, QueueNotificationPort, QueuePort, RegionRepositoryPort};

use crate::infrastructure;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::http;
use crate::infrastructure::queue_workers::{approval_notification_worker, dm_action_worker};
use crate::infrastructure::state::AppState;
use crate::infrastructure::websocket_helpers::build_prompt_from_action;

pub async fn run() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wrldbldr_engine=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting WrldBldr Engine");

    // Load configuration
    let config = AppConfig::from_env()?;
    tracing::info!("Configuration loaded");
    tracing::info!("  Neo4j: {}", config.neo4j_uri);
    tracing::info!("  Ollama: {}", config.ollama_base_url);
    tracing::info!("  ComfyUI: {}", config.comfyui_base_url);

    // Initialize application state
    let (state, generation_event_rx) = AppState::new(config).await?;
    let state = Arc::new(state);
    tracing::info!("Application state initialized");

    // Clone queue config for workers
    let queue_config = state.config.queue.clone();
    let recovery_interval =
        std::time::Duration::from_secs(queue_config.recovery_poll_interval_seconds);

    // Start background queue workers
    let llm_worker = {
        let service = state.queues.llm_queue_service.clone();
        let recovery_interval_clone = recovery_interval;
        tokio::spawn(async move {
            tracing::info!("Starting LLM queue worker");
            service.run_worker(recovery_interval_clone).await;
        })
    };

    let asset_worker = {
        let service = state.queues.asset_generation_queue_service.clone();
        let recovery_interval_clone = recovery_interval;
        tokio::spawn(async move {
            tracing::info!("Starting asset generation queue worker");
            service.run_worker(recovery_interval_clone).await;
        })
    };

    // Player action queue worker (processes actions and routes to LLM queue)
    let player_action_worker = {
        let service = state.queues.player_action_queue_service.clone();
        let world_service = state.core.world_service.clone();
        let world_state = state.world_state.clone();
        let challenge_service = state.game.challenge_service.clone();
        let skill_service = state.core.skill_service.clone();
        let narrative_event_service = state.game.narrative_event_service.clone();
        let character_repo: Arc<dyn CharacterRepositoryPort> =
            Arc::new(state.repository.characters());
        let pc_repo: Arc<dyn PlayerCharacterRepositoryPort> =
            Arc::new(state.repository.player_characters());
        let region_repo: Arc<dyn RegionRepositoryPort> =
            Arc::new(state.repository.regions());
        let settings_service = state.settings_service.clone();
        let mood_service = state.game.mood_service.clone();
        let actantial_service = state.game.actantial_context_service.clone();
        let notifier = service.queue().notifier();
        let recovery_interval_clone = recovery_interval;
        tokio::spawn(async move {
            tracing::info!("Starting player action queue worker");
            loop {
                let world_service_clone = world_service.clone();
                let world_state_clone = world_state.clone();
                let challenge_service_clone = challenge_service.clone();
                let skill_service_clone = skill_service.clone();
                let narrative_event_service_clone = narrative_event_service.clone();
                let character_repo_clone = character_repo.clone();
                let pc_repo_clone = pc_repo.clone();
                let region_repo_clone = region_repo.clone();
                let settings_service_clone = settings_service.clone();
                let mood_service_clone = mood_service.clone();
                let actantial_service_clone = actantial_service.clone();
                match service
                    .process_next(|action| {
                        let world_service = world_service_clone.clone();
                        let world_state = world_state_clone.clone();
                        let challenge_service = challenge_service_clone.clone();
                        let skill_service = skill_service_clone.clone();
                        let narrative_event_service = narrative_event_service_clone.clone();
                        let character_repo = character_repo_clone.clone();
                        let pc_repo = pc_repo_clone.clone();
                        let region_repo = region_repo_clone.clone();
                        let settings_service = settings_service_clone.clone();
                        let mood_service = mood_service_clone.clone();
                        let actantial_service = actantial_service_clone.clone();
                        async move {
                            let world_id = WorldId::from_uuid(action.world_id);
                            build_prompt_from_action(
                                world_id,
                                &world_service,
                                &world_state,
                                &challenge_service,
                                &skill_service,
                                &narrative_event_service,
                                &character_repo,
                                &pc_repo,
                                &region_repo,
                                &settings_service,
                                &mood_service,
                                &actantial_service,
                                &action,
                            )
                            .await
                        }
                    })
                    .await
                {
                    Ok(Some(action_id)) => {
                        tracing::debug!("Processed player action: {}", action_id);
                    }
                    Ok(None) => {
                        // Queue empty - wait for notification or recovery timeout
                        let _ = notifier.wait_for_work(recovery_interval_clone).await;
                    }
                    Err(e) => {
                        tracing::error!("Error processing player action: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        })
    };

    // Approval notification worker (sends ApprovalRequired messages to DM)
    let approval_notification_worker_task = {
        let service = state.queues.dm_approval_queue_service.clone();
        let world_connection_manager = state.world_connection_manager.clone();
        let recovery_interval_clone = recovery_interval;
        tokio::spawn(async move {
            approval_notification_worker(service, world_connection_manager, recovery_interval_clone).await;
        })
    };

    // DM action queue worker (processes approval decisions and other DM actions)
    let dm_action_worker_task = {
        let service = state.queues.dm_action_queue_service.clone();
        let approval_service = state.queues.dm_approval_queue_service.clone();
        let narrative_event_service = state.game.narrative_event_service.clone();
        let scene_service = state.core.scene_service.clone();
        let interaction_service = state.core.interaction_service.clone();
        let world_connection_manager = state.world_connection_manager.clone();
        let recovery_interval_clone = recovery_interval;
        tokio::spawn(async move {
            dm_action_worker(
                service,
                approval_service,
                narrative_event_service,
                scene_service,
                interaction_service,
                world_connection_manager,
                recovery_interval_clone,
            )
            .await;
        })
    };

    // Cleanup worker (removes old completed/failed queue items)
    let cleanup_worker = {
        let player_action_service = state.queues.player_action_queue_service.clone();
        let llm_service = state.queues.llm_queue_service.clone();
        let approval_service = state.queues.dm_approval_queue_service.clone();
        let asset_service = state.queues.asset_generation_queue_service.clone();
        let queue_config_clone = queue_config.clone();
        tokio::spawn(async move {
            tracing::info!("Starting queue cleanup worker");
            loop {
                let retention = std::time::Duration::from_secs(
                    queue_config_clone.history_retention_hours * 3600,
                );

                // Cleanup all queues
                let _ = player_action_service.queue().cleanup(retention).await;
                let _ = llm_service.queue().cleanup(retention).await;
                let _ = approval_service.queue().cleanup(retention).await;
                let _ = asset_service.queue().cleanup(retention).await;

                // Expire old approvals
                let approval_timeout = std::time::Duration::from_secs(
                    queue_config_clone.approval_timeout_minutes * 60,
                );
                let _ = approval_service.queue().expire_old(approval_timeout).await;

                // Run cleanup using configured interval
                tokio::time::sleep(std::time::Duration::from_secs(
                    queue_config_clone.cleanup_interval_seconds,
                ))
                .await;
            }
        })
    };

    // Generation event publisher (converts GenerationEvents to AppEvents and publishes to event bus)
    let generation_event_worker = {
        let event_bus = state.events.event_bus.clone();
        let publisher = GenerationEventPublisher::new(event_bus);
        tokio::spawn(async move {
            tracing::info!("Starting generation event publisher");
            publisher.run(generation_event_rx).await;
        })
    };

    // Build CORS layer based on configuration
    let cors_layer = if state.config.cors_allowed_origins.len() == 1 
        && state.config.cors_allowed_origins[0] == "*" 
    {
        tracing::warn!("CORS configured to allow ANY origin - this is insecure for production!");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins: Vec<_> = state.config.cors_allowed_origins
            .iter()
            .filter_map(|origin| origin.parse().ok())
            .collect();
        tracing::info!("CORS configured for origins: {:?}", state.config.cors_allowed_origins);
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(Any)
            .allow_headers(Any)
    };

    // Build HTTP router
    let app = Router::new()
        .route("/", get(|| async { "WrldBldr Engine API" }))
        .merge(http::create_routes())
        .route("/ws", get(infrastructure::websocket::ws_handler))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer)
        .with_state(state.clone());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], state.config.server_port));
    tracing::info!("Listening on {}", addr);

    let server = axum::serve(tokio::net::TcpListener::bind(addr).await?, app);

    tokio::select! {
        result = server => {
            result?;
        }
        _ = llm_worker => {}
        _ = asset_worker => {}
        _ = player_action_worker => {}
        _ = approval_notification_worker_task => {}
        _ = dm_action_worker_task => {}
        _ = cleanup_worker => {}
        _ = generation_event_worker => {}
    }

    Ok(())
}
