//! Neo4j test harness for E2E testing.
//!
//! Provides testcontainer-based Neo4j instance management for integration tests.
//!
//! # Shared Container with Fixed Ports
//!
//! Tests use a container named `wrldbldr-test-neo4j` on fixed ports:
//! - Bolt: 17687
//! - HTTP: 17474
//!
//! This allows:
//! - Detecting if a test container is already running
//! - Reusing existing containers across test runs
//! - Easier debugging (consistent port numbers)
//!
//! # Thread Safety
//!
//! The shared container uses `std::sync::OnceLock` which works correctly across
//! multiple tokio runtimes (each `#[tokio::test]` creates its own runtime).
//!
//! # Container Lifecycle
//!
//! 1. Check if `wrldbldr-test-neo4j` container is running
//! 2. If running and healthy, reuse it
//! 3. If not running, start a new container with fixed ports
//! 4. Old exited containers are cleaned up automatically

use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use neo4rs::{query, Graph};
use testcontainers::{core::WaitFor, runners::AsyncRunner, ContainerAsync, GenericImage};
use tokio::time::sleep;

/// The Neo4j image used for tests.
const NEO4J_TEST_IMAGE: &str = "neo4j:5.26.0-community";

/// Fixed container name for test Neo4j instance.
const TEST_CONTAINER_NAME: &str = "wrldbldr-test-neo4j";

/// Fixed bolt port for test Neo4j instance.
const TEST_BOLT_PORT: u16 = 17687;

/// Fixed HTTP port for test Neo4j instance.
const TEST_HTTP_PORT: u16 = 17474;

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
    /// Container handle (None if reusing an existing external container)
    _container: Option<ContainerAsync<GenericImage>>,
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

    /// Start or connect to the shared Neo4j container.
    ///
    /// First checks if a container named `wrldbldr-test-neo4j` is already running.
    /// If so, connects to it. Otherwise, starts a new container with fixed ports.
    async fn start_shared() -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let bolt_uri = format!("bolt://127.0.0.1:{}", TEST_BOLT_PORT);

        // Check if our named container is already running
        if is_test_container_running() {
            tracing::info!(
                container = TEST_CONTAINER_NAME,
                bolt_port = TEST_BOLT_PORT,
                "Found existing test container, attempting to connect..."
            );

            // Try to connect to the existing container
            match connect_with_retry(&bolt_uri, "neo4j", TEST_NEO4J_PASSWORD).await {
                Ok(_) => {
                    tracing::info!(
                        bolt_uri = %bolt_uri,
                        "Successfully connected to existing Neo4j test container"
                    );
                    return Ok(Arc::new(Self {
                        _container: None, // No container handle - it's external
                        bolt_uri,
                    }));
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Existing container not responding, will start fresh"
                    );
                    // Fall through to start a new container
                }
            }
        }

        // Clean up any old containers before starting a new one
        cleanup_old_containers();

        tracing::info!(
            container = TEST_CONTAINER_NAME,
            bolt_port = TEST_BOLT_PORT,
            http_port = TEST_HTTP_PORT,
            "Starting new Neo4j test container with fixed ports..."
        );

        // Start container using docker directly for fixed ports and naming
        start_named_container()?;

        // Wait for container to be ready
        let _test_graph = connect_with_retry(&bolt_uri, "neo4j", TEST_NEO4J_PASSWORD).await?;
        tracing::info!(bolt_uri = %bolt_uri, "Neo4j test container ready");

        Ok(Arc::new(Self {
            _container: None, // We manage via docker commands, not testcontainers handle
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

/// Check if the test container is already running.
fn is_test_container_running() -> bool {
    let output = Command::new("docker")
        .args(["ps", "-q", "-f", &format!("name={}", TEST_CONTAINER_NAME)])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            !stdout.trim().is_empty()
        }
        _ => false,
    }
}

/// Start a new named container with fixed ports using docker directly.
fn start_named_container() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("docker")
        .args([
            "run",
            "-d", // Detached
            "--name",
            TEST_CONTAINER_NAME,
            "-p",
            &format!("{}:7687", TEST_BOLT_PORT),
            "-p",
            &format!("{}:7474", TEST_HTTP_PORT),
            "-e",
            &format!("NEO4J_AUTH=neo4j/{}", TEST_NEO4J_PASSWORD),
            "-e",
            "NEO4J_server_memory_heap_initial__size=256m",
            "-e",
            "NEO4J_server_memory_heap_max__size=512m",
            "-e",
            "NEO4J_server_memory_pagecache_size=128m",
            "-e",
            "NEO4J_db_checkpoint_iops_limit=500",
            NEO4J_TEST_IMAGE,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to start Neo4j container: {}", stderr).into());
    }

    tracing::info!(container = TEST_CONTAINER_NAME, "Started Neo4j container");
    Ok(())
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

/// Clean up old Neo4j test containers.
///
/// This runs before starting a new container to ensure clean state.
/// It removes:
/// 1. Our named test container if it exists (stopped or running)
/// 2. Any other Neo4j test containers from testcontainers
fn cleanup_old_containers() {
    // First, remove our named container if it exists
    let _ = Command::new("docker")
        .args(["rm", "-f", TEST_CONTAINER_NAME])
        .output();

    // Then, remove any other Neo4j test containers (from testcontainers)
    let find_result = Command::new("docker")
        .args([
            "ps",
            "-aq",
            "--filter",
            &format!("ancestor={}", NEO4J_TEST_IMAGE),
        ])
        .output();

    let container_ids = match find_result {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect::<Vec<_>>(),
        Ok(output) => {
            tracing::debug!(
                stderr = %String::from_utf8_lossy(&output.stderr),
                "docker ps command failed, skipping cleanup"
            );
            return;
        }
        Err(e) => {
            tracing::debug!(error = %e, "Failed to run docker command, skipping cleanup");
            return;
        }
    };

    if container_ids.is_empty() {
        tracing::debug!("No old Neo4j containers to clean up");
        return;
    }

    tracing::info!(
        count = container_ids.len(),
        "Removing old Neo4j test containers"
    );

    // Remove the containers
    let remove_result = Command::new("docker")
        .arg("rm")
        .arg("-f") // Force remove even if running (belt and suspenders)
        .args(&container_ids)
        .output();

    match remove_result {
        Ok(output) if output.status.success() => {
            tracing::info!(
                count = container_ids.len(),
                "Successfully removed old Neo4j test containers"
            );
        }
        Ok(output) => {
            tracing::warn!(
                stderr = %String::from_utf8_lossy(&output.stderr),
                "Failed to remove some containers"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to run docker rm command");
        }
    }
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
