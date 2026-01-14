//! Neo4j test harness for E2E testing.
//!
//! Provides testcontainer-based Neo4j instance management for integration tests.
//!
//! # Shared Container
//!
//! For E2E tests, use `SharedNeo4jHarness::shared()` to get a shared container that is
//! reused across all tests. Each test should use fresh UUIDs for its entities to
//! ensure isolation without needing cleanup between tests.
//!
//! # Thread Safety
//!
//! The shared container uses `std::sync::OnceLock` which works correctly across
//! multiple tokio runtimes (each `#[tokio::test]` creates its own runtime).

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use neo4rs::{query, Graph};
use testcontainers::{core::WaitFor, runners::AsyncRunner, ContainerAsync, GenericImage};
use tokio::time::sleep;

/// Password used for Neo4j test containers.
pub const TEST_NEO4J_PASSWORD: &str = "testpassword";

/// Shared Neo4j harness for all E2E tests.
///
/// This container is started once and reused across all tests.
/// Tests are isolated by using fresh UUIDs for all entities.
///
/// Uses `OnceLock` instead of `tokio::sync::OnceCell` to work correctly
/// across multiple tokio runtimes (each test gets its own runtime).
static SHARED_HARNESS: OnceLock<Arc<SharedNeo4jHarness>> = OnceLock::new();

/// Mutex to serialize initialization attempts.
/// Prevents race condition where multiple threads start containers simultaneously.
static INIT_LOCK: Mutex<bool> = Mutex::new(false);

/// Shared Neo4j harness that keeps the container alive.
///
/// **Important**: This only holds the container and connection info.
/// Each test should call `create_graph()` to get its own Graph connection,
/// because Graph connections are tied to tokio runtimes.
pub struct SharedNeo4jHarness {
    _container: ContainerAsync<GenericImage>,
    /// The bolt URI for connecting to the container.
    bolt_uri: String,
}

impl SharedNeo4jHarness {
    /// Get or create the shared Neo4j container.
    ///
    /// The first call starts the container; subsequent calls return the same instance.
    /// This dramatically speeds up E2E tests by avoiding container startup overhead
    /// for each test.
    ///
    /// # Thread Safety
    ///
    /// Uses double-checked locking with `OnceLock` + `Mutex` to ensure only one
    /// thread initializes the container while others wait.
    pub async fn shared() -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        // Fast path: already initialized
        if let Some(harness) = SHARED_HARNESS.get() {
            return Ok(Arc::clone(harness));
        }

        // Slow path: serialize initialization attempts
        // Use a mutex to ensure only ONE thread starts the container
        let should_init = {
            let mut claimed = INIT_LOCK.lock().unwrap();
            if *claimed {
                // Another thread is already initializing or has initialized
                false
            } else {
                // We claim the right to initialize
                *claimed = true;
                true
            }
        };

        if should_init {
            // We're the initializer - start the container
            let harness = Self::start_shared().await?;
            // Store it (ignore error - shouldn't happen since we claimed)
            let _ = SHARED_HARNESS.set(harness.clone());
            Ok(harness)
        } else {
            // Another thread is initializing - wait for it to complete
            loop {
                if let Some(harness) = SHARED_HARNESS.get() {
                    return Ok(Arc::clone(harness));
                }
                // Brief sleep to avoid busy-waiting
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    /// Start a new shared container (internal).
    async fn start_shared() -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Starting shared Neo4j container for E2E tests...");
        let container: ContainerAsync<GenericImage> =
            neo4j_image(TEST_NEO4J_PASSWORD).start().await;
        let bolt_port = container.get_host_port_ipv4(7687).await;
        let bolt_uri = format!("bolt://127.0.0.1:{bolt_port}");

        // Verify the container is actually ready by creating a test connection
        let _test_graph = connect_with_retry(&bolt_uri, "neo4j", TEST_NEO4J_PASSWORD).await?;
        tracing::info!("Shared Neo4j container ready at {}", bolt_uri);

        Ok(Arc::new(Self {
            _container: container,
            bolt_uri,
        }))
    }

    /// Create a new Graph connection for the current tokio runtime.
    ///
    /// Each test should call this to get its own Graph connection,
    /// as Graph connections are tied to their creating runtime.
    pub async fn create_graph(&self) -> Result<Graph, Box<dyn std::error::Error + Send + Sync>> {
        connect_with_retry(&self.bolt_uri, "neo4j", TEST_NEO4J_PASSWORD).await
    }

    /// Get the bolt URI for manual connection.
    pub fn bolt_uri(&self) -> &str {
        &self.bolt_uri
    }
}

/// Neo4j test harness managing container lifecycle.
///
/// For E2E tests, prefer using `SharedNeo4jHarness::shared()` instead.
/// This struct is kept for backwards compatibility with tests that need
/// isolated containers.
pub struct Neo4jTestHarness {
    _container: ContainerAsync<GenericImage>,
    graph: Graph,
}

impl Neo4jTestHarness {
    /// Start a new Neo4j container and establish a connection.
    ///
    /// # Note
    ///
    /// For E2E tests, prefer `SharedNeo4jHarness::shared()` to avoid
    /// container startup overhead for each test.
    ///
    /// # Errors
    ///
    /// Returns an error if the container fails to start or connection cannot be established.
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let container: ContainerAsync<GenericImage> =
            neo4j_image(TEST_NEO4J_PASSWORD).start().await;
        let bolt_port = container.get_host_port_ipv4(7687).await;
        let uri = format!("bolt://127.0.0.1:{bolt_port}");

        let graph = connect_with_retry(&uri, "neo4j", TEST_NEO4J_PASSWORD).await?;

        Ok(Self {
            _container: container,
            graph,
        })
    }

    /// Get reference to the Neo4j graph connection.
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Clone the graph connection for use in other components.
    pub fn graph_clone(&self) -> Graph {
        self.graph.clone()
    }

    /// Clean all data from the database.
    pub async fn clean(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        clean_db(&self.graph).await
    }
}

/// Create a Neo4j container image with the given password.
///
/// Configuration for reliability:
/// - Pinned version for consistency across runs
/// - Memory limits to prevent JVM crashes
/// - No stdout wait (avoids race conditions with log streaming)
/// - Connection readiness is verified by connect_with_retry with exponential backoff
///
/// # Startup Timeout
///
/// To increase container startup timeout, set environment variable:
/// ```bash
/// TESTCONTAINERS_STARTUP_TIMEOUT=120 cargo test ...
/// ```
pub fn neo4j_image(password: &str) -> GenericImage {
    GenericImage::new("neo4j", "5.26.0-community")
        .with_env_var("NEO4J_AUTH", format!("neo4j/{password}"))
        .with_env_var(
            "NEO4J_dbms_connector_bolt_advertised__address",
            "localhost:7687",
        )
        // Memory limits to prevent JVM crashes under resource pressure
        .with_env_var("NEO4J_server_memory_heap_initial__size", "256m")
        .with_env_var("NEO4J_server_memory_heap_max__size", "512m")
        .with_env_var("NEO4J_server_memory_pagecache_size", "128m")
        // Faster checkpoint for test workloads
        .with_env_var("NEO4J_db_checkpoint_iops_limit", "500")
        .with_exposed_port(7687)
        .with_exposed_port(7474) // HTTP port for health checks
        // Wait a brief period for initial container setup, then rely on
        // connect_with_retry for actual readiness (more reliable than stdout parsing)
        .with_wait_for(WaitFor::seconds(5))
}

/// Connect to Neo4j with retry logic using exponential backoff.
///
/// Features:
/// - Exponential backoff: 500ms → 1s → 2s → 4s → 5s (capped)
/// - Connection verification with actual query
/// - Up to 30 attempts (~45 seconds max wait)
pub async fn connect_with_retry(
    uri: &str,
    user: &str,
    pass: &str,
) -> Result<Graph, Box<dyn std::error::Error + Send + Sync>> {
    let max_attempts = 30;
    let initial_delay = Duration::from_millis(500);
    let max_delay = Duration::from_secs(5);

    let mut attempt = 0;
    let mut delay = initial_delay;
    let mut last_err: Option<String> = None;

    while attempt < max_attempts {
        attempt += 1;

        match Graph::new(uri, user, pass).await {
            Ok(graph) => {
                // Verify connection with a simple query before returning
                match graph.run(query("RETURN 1")).await {
                    Ok(_) => {
                        tracing::info!(
                            attempt = attempt,
                            uri = uri,
                            "Neo4j connection established and verified"
                        );
                        return Ok(graph);
                    }
                    Err(e) => {
                        last_err = Some(format!("Connection test query failed: {e}"));
                    }
                }
            }
            Err(e) => {
                last_err = Some(e.to_string());
            }
        }

        tracing::debug!(
            attempt = attempt,
            delay_ms = delay.as_millis(),
            error = last_err.as_deref().unwrap_or("unknown"),
            "Retrying Neo4j connection"
        );

        sleep(delay).await;
        // Exponential backoff with cap
        delay = std::cmp::min(delay.saturating_mul(2), max_delay);
    }

    Err(format!(
        "Failed to connect to Neo4j at {uri} after {max_attempts} attempts: {:?}",
        last_err
    )
    .into())
}

/// Clean all data from a Neo4j database.
pub async fn clean_db(graph: &Graph) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    graph
        .run(query("MATCH (n) DETACH DELETE n"))
        .await
        .map_err(|e| format!("Failed to clean database: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires docker (testcontainers)"]
    async fn test_neo4j_harness_starts_and_connects() {
        let harness = Neo4jTestHarness::start()
            .await
            .expect("Failed to start Neo4j harness");

        // Verify we can query the database
        let mut result = harness
            .graph()
            .execute(query("RETURN 1 as n"))
            .await
            .expect("Query failed");

        let row = result.next().await.expect("No result").expect("Row error");
        let n: i64 = row.get("n").expect("Column not found");
        assert_eq!(n, 1);
    }

    #[tokio::test]
    #[ignore = "requires docker (testcontainers)"]
    async fn test_neo4j_harness_clean_removes_all_data() {
        let harness = Neo4jTestHarness::start()
            .await
            .expect("Failed to start Neo4j harness");

        // Create some test data
        harness
            .graph()
            .run(query("CREATE (:TestNode {name: 'test'})"))
            .await
            .expect("Create failed");

        // Verify data exists
        let mut result = harness
            .graph()
            .execute(query("MATCH (n:TestNode) RETURN count(n) as count"))
            .await
            .expect("Count query failed");

        let row = result.next().await.expect("No result").expect("Row error");
        let count: i64 = row.get("count").expect("Column not found");
        assert_eq!(count, 1);

        // Clean the database
        harness.clean().await.expect("Clean failed");

        // Verify data is gone
        let mut result = harness
            .graph()
            .execute(query("MATCH (n) RETURN count(n) as count"))
            .await
            .expect("Count query failed");

        let row = result.next().await.expect("No result").expect("Row error");
        let count: i64 = row.get("count").expect("Column not found");
        assert_eq!(count, 0);
    }
}
