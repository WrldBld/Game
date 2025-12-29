use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{routing::get, Router};
use tokio_util::sync::CancellationToken;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use wrldbldr_engine_app::application::services::{
    ChallengeApprovalEventPublisher, GenerationEventPublisher,
};
use wrldbldr_engine_ports::outbound::{ApprovalQueuePort, QueueNotificationPort, QueuePort};

use wrldbldr_engine_adapters::infrastructure;
use wrldbldr_engine_adapters::infrastructure::config::AppConfig;
use wrldbldr_engine_adapters::infrastructure::http;
use super::queue_workers::{
    approval_notification_worker, challenge_outcome_notification_worker, dm_action_worker,
};

use crate::composition::new_app_state;

/// Creates a cancellation token and spawns a task that cancels it on SIGTERM/SIGINT
fn setup_shutdown_signal(cancel_token: CancellationToken) {
    tokio::spawn(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
            }
            _ = terminate => {
                tracing::info!("Received SIGTERM, initiating graceful shutdown...");
            }
        }

        cancel_token.cancel();
    });
}

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

    // Create cancellation token for graceful shutdown
    let cancel_token = CancellationToken::new();
    setup_shutdown_signal(cancel_token.clone());

    // Load configuration
    let config = AppConfig::from_env()?;
    tracing::info!("Configuration loaded");
    tracing::info!("  Neo4j: {}", config.neo4j_uri);
    tracing::info!("  Ollama: {}", config.ollama_base_url);
    tracing::info!("  ComfyUI: {}", config.comfyui_base_url);

    // Initialize application state
    let (state, generation_event_rx, challenge_approval_rx) = new_app_state(config).await?;
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
        let cancel = cancel_token.clone();
        tokio::spawn(async move {
            tracing::info!("Starting LLM queue worker");
            service.run_worker(recovery_interval_clone, cancel).await;
        })
    };

    let asset_worker = {
        let service = state.queues.asset_generation_queue_service.clone();
        let recovery_interval_clone = recovery_interval;
        let cancel = cancel_token.clone();
        tokio::spawn(async move {
            tracing::info!("Starting asset generation queue worker");
            service.run_worker(recovery_interval_clone, cancel).await;
        })
    };

    // Player action queue worker (processes actions and routes to LLM queue)
    let player_action_worker = {
        let service = state.queues.player_action_queue_service.clone();
        let prompt_context_service = state.prompt_context_service.clone();
        let notifier = service.queue().notifier();
        let recovery_interval_clone = recovery_interval;
        let cancel = cancel_token.clone();
        tokio::spawn(async move {
            tracing::info!("Starting player action queue worker");
            loop {
                // Check for cancellation
                if cancel.is_cancelled() {
                    tracing::info!("Player action queue worker shutting down");
                    break;
                }

                let prompt_service = prompt_context_service.clone();
                match service
                    .process_next(|action| {
                        let prompt_service = prompt_service.clone();
                        async move {
                            let world_id = action.world_id;
                            prompt_service
                                .build_prompt_from_action(world_id, &action)
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
                        // Use select to also check for cancellation during wait
                        tokio::select! {
                            _ = cancel.cancelled() => {
                                tracing::info!("Player action queue worker shutting down");
                                break;
                            }
                            _ = notifier.wait_for_work(recovery_interval_clone) => {}
                        }
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
        let cancel = cancel_token.clone();
        tokio::spawn(async move {
            approval_notification_worker(
                service,
                world_connection_manager,
                recovery_interval_clone,
                cancel,
            )
            .await;
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
        let cancel = cancel_token.clone();
        tokio::spawn(async move {
            dm_action_worker(
                service,
                approval_service,
                narrative_event_service,
                scene_service,
                interaction_service,
                world_connection_manager,
                recovery_interval_clone,
                cancel,
            )
            .await;
        })
    };

    // Challenge outcome notification worker (sends pending challenge outcomes to DM)
    let challenge_outcome_worker_task = {
        let challenge_queue = state.queues.challenge_outcome_queue.clone();
        let world_connection_manager = state.world_connection_manager.clone();
        let recovery_interval_clone = recovery_interval;
        let cancel = cancel_token.clone();
        tokio::spawn(async move {
            challenge_outcome_notification_worker(
                challenge_queue,
                world_connection_manager,
                recovery_interval_clone,
                cancel,
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
        let cancel = cancel_token.clone();
        tokio::spawn(async move {
            tracing::info!("Starting queue cleanup worker");
            loop {
                // Check for cancellation
                if cancel.is_cancelled() {
                    tracing::info!("Cleanup worker shutting down");
                    break;
                }

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

                // Run cleanup using configured interval, but check for cancellation
                let sleep_duration =
                    std::time::Duration::from_secs(queue_config_clone.cleanup_interval_seconds);
                tokio::select! {
                    _ = cancel.cancelled() => {
                        tracing::info!("Cleanup worker shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(sleep_duration) => {}
                }
            }
        })
    };

    // Generation event publisher (converts GenerationEvents to AppEvents and publishes to event bus)
    let generation_event_worker = {
        let event_bus = state.events.event_bus.clone();
        let publisher = GenerationEventPublisher::new(event_bus);
        let cancel = cancel_token.clone();
        tokio::spawn(async move {
            tracing::info!("Starting generation event publisher");
            publisher.run(generation_event_rx, cancel).await;
        })
    };

    // Challenge approval event publisher (converts ChallengeApprovalEvents to GameEvents and broadcasts via BroadcastPort)
    let challenge_approval_worker = {
        let broadcast_port = state.use_cases.broadcast.clone();
        let publisher = ChallengeApprovalEventPublisher::new(broadcast_port);
        let cancel = cancel_token.clone();
        tokio::spawn(async move {
            tracing::info!("Starting challenge approval event publisher");
            publisher.run(challenge_approval_rx, cancel).await;
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
        let origins: Vec<_> = state
            .config
            .cors_allowed_origins
            .iter()
            .filter_map(|origin| origin.parse().ok())
            .collect();
        tracing::info!(
            "CORS configured for origins: {:?}",
            state.config.cors_allowed_origins
        );
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

    let server = axum::serve(tokio::net::TcpListener::bind(addr).await?, app)
        .with_graceful_shutdown(async move {
            cancel_token.cancelled().await;
            tracing::info!("HTTP server received shutdown signal");
        });

    // Run server until shutdown, then wait for workers to finish
    if let Err(e) = server.await {
        tracing::error!("Server error: {}", e);
    }

    tracing::info!("Waiting for workers to complete...");

    // Give workers a chance to finish gracefully
    // JoinHandles will complete when workers check cancellation token
    let _ = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let _ = llm_worker.await;
        let _ = asset_worker.await;
        let _ = player_action_worker.await;
        let _ = approval_notification_worker_task.await;
        let _ = dm_action_worker_task.await;
        let _ = challenge_outcome_worker_task.await;
        let _ = cleanup_worker.await;
        let _ = generation_event_worker.await;
        let _ = challenge_approval_worker.await;
    })
    .await;

    tracing::info!("WrldBldr Engine shutdown complete");
    Ok(())
}
