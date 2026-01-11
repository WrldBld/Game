//! WrldBldr Engine - Main entry point.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::http::header::HeaderName;
use axum::http::{HeaderValue, Method};
use axum::routing::get;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod app;
mod entities;
mod infrastructure;
mod use_cases;

use api::{websocket::WsState, ConnectionManager};
use app::App;
use infrastructure::{
    clock::SystemClock,
    comfyui::ComfyUIClient,
    neo4j::Neo4jRepositories,
    ollama::OllamaClient,
    queue::SqliteQueue,
    resilient_llm::{ResilientLlmClient, RetryConfig},
    settings::SqliteSettingsRepo,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment from repo root (Taskfile runs the engine from `crates/engine`).
    load_dotenv_from_repo_root();

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
    let neo4j_uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".into());
    let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".into());
    let neo4j_pass = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".into());
    let ollama_url = std::env::var("OLLAMA_URL")
        .or_else(|_| std::env::var("OLLAMA_BASE_URL"))
        .unwrap_or_else(|_| "http://localhost:11434".into());
    let ollama_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".into());
    let comfyui_url = std::env::var("COMFYUI_URL")
        .or_else(|_| std::env::var("COMFYUI_BASE_URL"))
        .unwrap_or_else(|_| "http://localhost:8188".into());
    let server_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let server_port: u16 = std::env::var("SERVER_PORT")
        .or_else(|_| std::env::var("PORT"))
        .unwrap_or_else(|_| "3000".into())
        .parse()
        .unwrap_or(3000);
    let fivetools_path = std::env::var("FIVETOOLS_DATA_PATH").ok();

    // Create clock for repositories
    let clock: Arc<dyn infrastructure::ports::ClockPort> = Arc::new(SystemClock);

    // Connect to Neo4j
    tracing::info!("Connecting to Neo4j at {}", neo4j_uri);
    let graph = neo4rs::Graph::new(&neo4j_uri, &neo4j_user, &neo4j_pass).await?;

    // Ensure database schema (constraints and indexes)
    infrastructure::neo4j::ensure_schema(&graph).await?;

    let repos = Neo4jRepositories::new(graph, clock.clone());

    // Create infrastructure clients
    let ollama_client = Arc::new(OllamaClient::new(&ollama_url, &ollama_model));
    let retry_config = RetryConfig::default();
    tracing::info!(
        "LLM client configured with retry: max_retries={}, base_delay_ms={}",
        retry_config.max_retries,
        retry_config.base_delay_ms
    );
    let llm = Arc::new(ResilientLlmClient::new(ollama_client, retry_config));
    let image_gen = Arc::new(ComfyUIClient::new(&comfyui_url));

    // Create queue
    let queue_db = std::env::var("QUEUE_DB").unwrap_or_else(|_| "queues.db".into());
    let queue = Arc::new(SqliteQueue::new(&queue_db, clock.clone()).await?);
    let settings_repo = Arc::new(SqliteSettingsRepo::new(&queue_db, clock.clone()).await?);

    // Configure content service
    let content_config = use_cases::content::ContentServiceConfig {
        fivetools_path: fivetools_path.map(std::path::PathBuf::from),
        preload: false,
    };
    if content_config.fivetools_path.is_some() {
        tracing::info!(
            path = ?content_config.fivetools_path,
            "FIVETOOLS_DATA_PATH configured, will register D&D 5e content provider"
        );
    }

    // Create application
    let app = Arc::new(App::new(
        repos,
        llm,
        image_gen,
        queue,
        settings_repo,
        content_config,
    ));

    // Create connection manager
    let connections = Arc::new(ConnectionManager::new());

    // Create WebSocket state
    let ws_state = Arc::new(WsState {
        app: app.clone(),
        connections,
        pending_time_suggestions: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        pending_staging_requests: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        generation_read_state: tokio::sync::RwLock::new(std::collections::HashMap::new()),
    });

    // Spawn queue processor
    let queue_app = app.clone();
    let queue_connections = ws_state.connections.clone();
    tokio::spawn(async move {
        loop {
            // Process player actions
            if let Err(e) = queue_app
                .use_cases
                .queues
                .process_player_action
                .execute()
                .await
            {
                tracing::warn!(error = %e, "Failed to process player action");
            }

            // Process LLM requests
            // Pass a callback for immediate events (e.g., SuggestionProgress) that need
            // to be broadcast BEFORE the LLM call starts.
            let immediate_connections = queue_connections.clone();
            match queue_app
                .use_cases
                .queues
                .process_llm_request
                .execute(|events| {
                    // Spawn async broadcasts for immediate events
                    for event in events {
                        let connections = immediate_connections.clone();
                        tokio::spawn(async move {
                            match event {
                                use_cases::queues::BroadcastEvent::SuggestionProgress {
                                    world_id,
                                    request_id,
                                } => {
                                    let msg =
                                        wrldbldr_protocol::ServerMessage::SuggestionProgress {
                                            request_id,
                                            status: "processing".to_string(),
                                        };
                                    connections.broadcast_to_world(world_id, msg).await;
                                    tracing::info!(world_id = %world_id, "Broadcast SuggestionProgress (immediate)");
                                }
                                _ => {
                                    // Other events should not be sent as immediate
                                    tracing::warn!("Unexpected immediate event type");
                                }
                            }
                        });
                    }
                })
                .await
            {
                Ok(Some(result)) => {
                    // Handle completion broadcast events
                    for event in result.broadcast_events {
                        match event {
                            use_cases::queues::BroadcastEvent::OutcomeSuggestionReady {
                                world_id,
                                resolution_id,
                                suggestions,
                            } => {
                                let msg =
                                    wrldbldr_protocol::ServerMessage::OutcomeSuggestionReady {
                                        resolution_id: resolution_id.to_string(),
                                        suggestions,
                                    };
                                queue_connections.broadcast_to_dms(world_id, msg).await;
                                tracing::info!(
                                    world_id = %world_id,
                                    resolution_id = %resolution_id,
                                    "Broadcast OutcomeSuggestionReady to DMs"
                                );
                            }

                            use_cases::queues::BroadcastEvent::SuggestionProgress {
                                world_id,
                                request_id,
                            } => {
                                // This shouldn't happen anymore (progress is now immediate)
                                // but handle it for safety
                                let msg = wrldbldr_protocol::ServerMessage::SuggestionProgress {
                                    request_id,
                                    status: "processing".to_string(),
                                };
                                queue_connections.broadcast_to_world(world_id, msg).await;
                                tracing::info!(world_id = %world_id, "Broadcast SuggestionProgress");
                            }

                            use_cases::queues::BroadcastEvent::SuggestionComplete {
                                world_id,
                                request_id,
                                suggestions,
                            } => {
                                let msg = wrldbldr_protocol::ServerMessage::SuggestionComplete {
                                    request_id,
                                    suggestions,
                                };
                                queue_connections.broadcast_to_world(world_id, msg).await;
                                tracing::info!(world_id = %world_id, "Broadcast SuggestionComplete");
                            }
                        }
                    }
                }
                Ok(None) => {} // Queue empty
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to process LLM request");
                }
            }

            // Process DM approval requests - send to DMs for review
            match queue_app
                .queue
                .dequeue_dm_approval()
                .await
            {
                Ok(Some(item)) => {
                    if let infrastructure::ports::QueueItemData::DmApproval(data) = item.data {
                        // Convert domain types to protocol types
                        let proposed_tools: Vec<wrldbldr_protocol::ProposedToolInfo> = data
                            .proposed_tools
                            .into_iter()
                            .map(|t| wrldbldr_protocol::ProposedToolInfo {
                                id: t.id,
                                name: t.name,
                                description: t.description,
                                arguments: t.arguments,
                            })
                            .collect();

                        let challenge_suggestion = data.challenge_suggestion.map(|cs| {
                            wrldbldr_protocol::ChallengeSuggestionInfo {
                                challenge_id: cs.challenge_id,
                                challenge_name: cs.challenge_name,
                                skill_name: cs.skill_name,
                                difficulty_display: cs.difficulty_display,
                                confidence: cs.confidence,
                                reasoning: cs.reasoning,
                                target_pc_id: cs.target_pc_id.map(|id| id.to_string()),
                                outcomes: cs.outcomes.map(|o| {
                                    wrldbldr_protocol::ChallengeSuggestionOutcomes {
                                        success: o.success,
                                        failure: o.failure,
                                        critical_success: o.critical_success,
                                        critical_failure: o.critical_failure,
                                    }
                                }),
                            }
                        });

                        let narrative_event_suggestion = data.narrative_event_suggestion.map(|nes| {
                            wrldbldr_protocol::NarrativeEventSuggestionInfo {
                                event_id: nes.event_id,
                                event_name: nes.event_name,
                                description: nes.description,
                                scene_direction: nes.scene_direction,
                                confidence: nes.confidence,
                                reasoning: nes.reasoning,
                                matched_triggers: nes.matched_triggers,
                                suggested_outcome: nes.suggested_outcome,
                            }
                        });

                        // Build and broadcast ApprovalRequired message to DMs
                        let msg = wrldbldr_protocol::ServerMessage::ApprovalRequired {
                            request_id: item.id.to_string(),
                            npc_name: data.npc_name,
                            proposed_dialogue: data.proposed_dialogue,
                            internal_reasoning: data.internal_reasoning,
                            proposed_tools,
                            challenge_suggestion,
                            narrative_event_suggestion,
                        };

                        queue_connections.broadcast_to_dms(data.world_id, msg).await;
                        tracing::info!(
                            world_id = %data.world_id,
                            request_id = %item.id,
                            "Broadcast ApprovalRequired to DMs"
                        );
                    }
                }
                Ok(None) => {} // Queue empty
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to dequeue DM approval request");
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    // Spawn staging timeout processor
    let staging_ws_state = ws_state.clone();
    tokio::spawn(async move {
        loop {
            // Check for expired staging requests
            let now = chrono::Utc::now();

            // Collect all pending requests with their world IDs
            let pending_requests: Vec<(String, use_cases::staging::PendingStagingRequest)> = {
                let guard = staging_ws_state.pending_staging_requests.read().await;
                guard
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            };

            // Process each pending request, checking world-specific settings
            for (request_id, pending) in pending_requests {
                let world_id = pending.world_id;

                // Fetch world settings to get timeout configuration
                let settings = match staging_ws_state
                    .app
                    .use_cases
                    .settings
                    .get_for_world(world_id)
                    .await
                {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            world_id = %world_id,
                            "Failed to fetch world settings for staging timeout, using defaults"
                        );
                        Default::default()
                    }
                };

                let timeout_seconds = settings.staging_timeout_seconds;

                // Skip if timeout is disabled (0) or not yet expired
                if timeout_seconds == 0 {
                    continue;
                }

                let elapsed = now.signed_duration_since(pending.created_at);
                if elapsed.num_seconds() < timeout_seconds as i64 {
                    continue;
                }

                // Request has expired - atomically remove from pending
                // Only process if we successfully removed it (prevents double processing)
                let was_removed = {
                    let mut guard = staging_ws_state.pending_staging_requests.write().await;
                    guard.remove(&request_id).is_some()
                };

                if !was_removed {
                    // Another task (e.g., manual DM approval) already handled this request
                    tracing::debug!(
                        request_id = %request_id,
                        "Staging request already removed by another handler, skipping timeout processing"
                    );
                    continue;
                }

                // Check if auto-approve is enabled for this world
                if !settings.auto_approve_on_timeout {
                    // Notify all players in the world that staging for this region timed out
                    // Players waiting for this region will see it and can retry
                    let region_id_str = pending.region_id.to_string();
                    tracing::info!(
                        request_id = %request_id,
                        world_id = %world_id,
                        region_id = %region_id_str,
                        "Staging timeout without auto-approve, broadcasting timeout notification"
                    );
                    staging_ws_state
                        .connections
                        .broadcast_to_world(
                            world_id,
                            wrldbldr_protocol::ServerMessage::StagingTimedOut {
                                region_id: region_id_str.clone(),
                                region_name: region_id_str, // Use ID as name (player has region info)
                            },
                        )
                        .await;
                    continue;
                }

                // Auto-approve with rule-based NPCs
                match staging_ws_state
                    .app
                    .use_cases
                    .staging
                    .auto_approve_timeout
                    .execute(request_id.clone(), pending.clone())
                    .await
                {
                    Ok(payload) => {
                        // Broadcast StagingReady to all players in world
                        staging_ws_state
                            .connections
                            .broadcast_to_world(
                                world_id,
                                wrldbldr_protocol::ServerMessage::StagingReady {
                                    region_id: payload.region_id.to_string(),
                                    npcs_present: payload.npcs_present,
                                    visual_state: payload.visual_state,
                                },
                            )
                            .await;
                        tracing::info!(
                            request_id = %request_id,
                            world_id = %world_id,
                            timeout_seconds = %timeout_seconds,
                            "Auto-approved staging on timeout, broadcast StagingReady"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            request_id = %request_id,
                            "Failed to auto-approve staging on timeout"
                        );
                    }
                }
            }

            // Check every 5 seconds
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Build router with separate states for HTTP and WebSocket
    let mut router = api::http::routes()
        .with_state(app)
        .route("/ws", get(api::websocket::ws_handler).with_state(ws_state))
        .layer(TraceLayer::new_for_http());

    if let Some(cors) = build_cors_layer_from_env() {
        router = router.layer(cors);
    }

    // Start server
    let addr: SocketAddr = format!("{server_host}:{server_port}").parse()?;
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

fn load_dotenv_from_repo_root() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");

    // Prefer local overrides.
    for filename in [".env.local", ".env"] {
        let path = repo_root.join(filename);
        if path.exists() {
            let _ = dotenvy::from_path(path);
        }
    }
}

fn build_cors_layer_from_env() -> Option<CorsLayer> {
    let allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let Some(allowed_origins) = allowed_origins else {
        return None;
    };

    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        // The Player sends X-User-Id and JSON content types which trigger CORS preflights.
        .allow_headers([
            HeaderName::from_static("x-user-id"),
            axum::http::header::CONTENT_TYPE,
        ]);

    if allowed_origins == "*" {
        cors = cors.allow_origin(Any);
    } else {
        let origins: Vec<HeaderValue> = allowed_origins
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| HeaderValue::from_str(s).ok())
            .collect();

        if origins.is_empty() {
            return None;
        }

        cors = cors.allow_origin(origins);
    }

    Some(cors)
}
