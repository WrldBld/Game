//! Neo4j graph wrapper with test-only benchmarking.

use std::sync::Arc;
use std::time::Instant;

use neo4rs::{Graph, Query};

use crate::e2e_tests::E2EBenchmark;
use crate::infrastructure::neo4j::graph::Neo4jGraph as InnerGraph;
use crate::infrastructure::neo4j::Neo4jRowStream;

/// Test-only wrapper around the production graph that records benchmark timing.
#[derive(Clone)]
pub struct Neo4jGraph {
    inner: InnerGraph,
    benchmark: Option<Arc<E2EBenchmark>>,
}

impl Neo4jGraph {
    pub fn new(graph: Graph) -> Self {
        Self {
            inner: InnerGraph::new(graph),
            benchmark: None,
        }
    }

    pub fn with_benchmark(graph: Graph, benchmark: Arc<E2EBenchmark>) -> Self {
        Self {
            inner: InnerGraph::new(graph),
            benchmark: Some(benchmark),
        }
    }

    pub fn inner(&self) -> &Graph {
        self.inner.inner()
    }

    pub fn inner_clone(&self) -> Graph {
        self.inner.inner_clone()
    }

    pub async fn run(&self, query: Query) -> Result<(), neo4rs::Error> {
        let start = Instant::now();
        let result = self.inner.run(query).await;
        if let Some(benchmark) = &self.benchmark {
            benchmark.record_neo4j_query("run", start.elapsed().as_millis() as u64);
        }
        result
    }

    pub async fn execute(&self, query: Query) -> Result<Neo4jRowStream, neo4rs::Error> {
        let start = Instant::now();
        let result = self.inner.execute(query).await;
        if let Some(benchmark) = &self.benchmark {
            benchmark.record_neo4j_query("execute", start.elapsed().as_millis() as u64);
        }
        result
    }
}

impl std::ops::Deref for Neo4jGraph {
    type Target = InnerGraph;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
