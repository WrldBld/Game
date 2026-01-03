//! WrldBldr Engine - Main entry point.

use std::net::SocketAddr;
use std::sync::Arc;


use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod app;
mod entities;
mod infrastructure;
mod use_cases;

use app::App;
use infrastructure::{
    clock::SystemClock,
    comfyui::ComfyUIClient,
    neo4j::Neo4jRepositories,
    ollama::OllamaClient,
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
    let ollama_url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let comfyui_url = std::env::var("COMFYUI_URL").unwrap_or_else(|_| "http://localhost:8188".into());
    let server_port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap_or(8080);

    // Create clock for repositories
    let clock = Arc::new(SystemClock);

    // Connect to Neo4j
    tracing::info!("Connecting to Neo4j at {}", neo4j_uri);
    let graph = neo4rs::Graph::new(&neo4j_uri, &neo4j_user, &neo4j_pass).await?;
    let repos = Neo4jRepositories::new(graph, clock);

    // Create infrastructure clients
    let llm = Arc::new(OllamaClient::new(ollama_url));
    let image_gen = Arc::new(ComfyUIClient::new(comfyui_url));
    let queue = Arc::new(SqliteQueue::new("queues.db".into()));

    // Create application
    let app = Arc::new(App::new(repos, llm, image_gen, queue));

    // Build router
    let router = api::http::routes()
        .route("/ws", axum::routing::get(api::websocket::ws_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(app);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], server_port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
