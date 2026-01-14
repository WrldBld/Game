//! Neo4j graph wrapper with optional test benchmarking.

use std::time::Instant;

use futures_util::{Stream, TryStreamExt};
use neo4rs::{Graph, Query, Row};
use std::pin::Pin;

#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use crate::e2e_tests::benchmark::E2EBenchmark;

/// Concrete wrapper around `neo4rs::Graph`.
///
/// In tests, this can record query timings into `E2EBenchmark`.
#[derive(Clone)]
pub struct Neo4jGraph {
    inner: Graph,
    #[cfg(test)]
    benchmark: Option<Arc<E2EBenchmark>>,
}

pub struct Neo4jRowStream {
    inner: Pin<Box<dyn Stream<Item = Result<Row, neo4rs::Error>> + Send>>,
}

impl Neo4jRowStream {
    fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Row, neo4rs::Error>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }

    pub async fn next(&mut self) -> Result<Option<Row>, neo4rs::Error> {
        let next = futures_util::future::poll_fn(|cx| self.inner.as_mut().poll_next(cx)).await;
        match next {
            Some(row) => row.map(Some),
            None => Ok(None),
        }
    }
}

impl Neo4jGraph {
    pub fn new(graph: Graph) -> Self {
        Self {
            inner: graph,
            #[cfg(test)]
            benchmark: None,
        }
    }

    #[cfg(test)]
    pub fn with_benchmark(graph: Graph, benchmark: Arc<E2EBenchmark>) -> Self {
        Self {
            inner: graph,
            benchmark: Some(benchmark),
        }
    }

    pub fn inner(&self) -> &Graph {
        &self.inner
    }

    pub fn inner_clone(&self) -> Graph {
        self.inner.clone()
    }

    fn record(&self, _operation: &str, _elapsed_ms: u64) {
        #[cfg(test)]
        if let Some(benchmark) = &self.benchmark {
            benchmark.record_neo4j_query(_operation, _elapsed_ms);
        }
    }

    pub async fn run(&self, query: Query) -> Result<(), neo4rs::Error> {
        let start = Instant::now();
        let result = self.inner.run(query).await;
        self.record("run", start.elapsed().as_millis() as u64);
        result
    }

    pub async fn execute(
        &self,
        query: Query,
    ) -> Result<Neo4jRowStream, neo4rs::Error> {
        let start = Instant::now();
        let result = self.inner.execute(query).await;
        self.record("execute", start.elapsed().as_millis() as u64);
        result.map(|stream| {
            let stream = stream.into_stream();
            let stream = TryStreamExt::into_stream(stream);
            Neo4jRowStream::from_stream(stream)
        })
    }
}

impl std::ops::Deref for Neo4jGraph {
    type Target = Graph;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
