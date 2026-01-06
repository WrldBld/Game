//! WrldBldr Engine - Main entry point.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::routing::get;
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
    clock::SystemClock, comfyui::ComfyUIClient, neo4j::Neo4jRepositories, ollama::OllamaClient,
    queue::SqliteQueue,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment
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
    let neo4j_uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".into());
    let neo4j_user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".into());
    let neo4j_pass = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".into());
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let ollama_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".into());
    let comfyui_url =
        std::env::var("COMFYUI_URL").unwrap_or_else(|_| "http://localhost:8188".into());
    let server_port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap_or(8080);

    // Create clock for repositories
    let clock: Arc<dyn infrastructure::ports::ClockPort> = Arc::new(SystemClock);

    // Connect to Neo4j
    tracing::info!("Connecting to Neo4j at {}", neo4j_uri);
    let graph = neo4rs::Graph::new(&neo4j_uri, &neo4j_user, &neo4j_pass).await?;
    let repos = Neo4jRepositories::new(graph, clock.clone());

    // Create infrastructure clients
    let llm = Arc::new(OllamaClient::new(&ollama_url, &ollama_model));
    let image_gen = Arc::new(ComfyUIClient::new(&comfyui_url));

    // Create queue
    let queue_db = std::env::var("QUEUE_DB").unwrap_or_else(|_| "queues.db".into());
    let queue = Arc::new(SqliteQueue::new(&queue_db, clock).await?);

    // Create application
    let app = Arc::new(App::new(repos, llm, image_gen, queue));

    // Create connection manager
    let connections = Arc::new(ConnectionManager::new());

    // Create WebSocket state
    let ws_state = Arc::new(WsState {
        app: app.clone(),
        connections,
        pending_time_suggestions: tokio::sync::RwLock::new(std::collections::HashMap::new()),
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
            match queue_app
                .use_cases
                .queues
                .process_llm_request
                .execute()
                .await
            {
                Ok(Some(result)) => {
                    // Handle broadcast events
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
                        }
                    }
                }
                Ok(None) => {} // Queue empty
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to process LLM request");
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    // Build router with separate states for HTTP and WebSocket
    let router = api::http::routes()
        .with_state(app)
        .route("/ws", get(api::websocket::ws_handler).with_state(ws_state))
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], server_port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
