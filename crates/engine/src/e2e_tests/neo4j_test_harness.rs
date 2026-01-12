//! Neo4j test harness for E2E testing.
//!
//! Provides testcontainer-based Neo4j instance management for integration tests.

use std::time::Duration;

use neo4rs::{query, Graph};
use testcontainers::{core::WaitFor, runners::AsyncRunner, ContainerAsync, GenericImage};
use tokio::time::sleep;

/// Password used for Neo4j test containers.
pub const TEST_NEO4J_PASSWORD: &str = "testpassword";

/// Neo4j test harness managing container lifecycle.
pub struct Neo4jTestHarness {
    _container: ContainerAsync<GenericImage>,
    graph: Graph,
}

impl Neo4jTestHarness {
    /// Start a new Neo4j container and establish a connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the container fails to start or connection cannot be established.
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let container: ContainerAsync<GenericImage> = neo4j_image(TEST_NEO4J_PASSWORD).start().await;
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
pub fn neo4j_image(password: &str) -> GenericImage {
    GenericImage::new("neo4j", "5")
        .with_env_var("NEO4J_AUTH", format!("neo4j/{password}"))
        .with_env_var(
            "NEO4J_dbms_connector_bolt_advertised__address",
            "localhost:7687",
        )
        .with_exposed_port(7687)
        .with_wait_for(WaitFor::message_on_stdout("Started."))
}

/// Connect to Neo4j with retry logic.
///
/// Retries connection up to 60 times with 250ms delay between attempts.
pub async fn connect_with_retry(
    uri: &str,
    user: &str,
    pass: &str,
) -> Result<Graph, Box<dyn std::error::Error + Send + Sync>> {
    let mut last_err: Option<String> = None;
    for _ in 0..60 {
        match Graph::new(uri, user, pass).await {
            Ok(graph) => return Ok(graph),
            Err(e) => {
                last_err = Some(e.to_string());
                sleep(Duration::from_millis(250)).await;
            }
        }
    }

    Err(format!(
        "Failed to connect to Neo4j at {uri} after retries: {:?}",
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
