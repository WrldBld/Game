//! Neo4j graph wrapper.

use futures_util::{Stream, TryStreamExt};
use neo4rs::{Graph, Query, Row};
use std::pin::Pin;

/// Concrete wrapper around `neo4rs::Graph`.
#[derive(Clone)]
pub struct Neo4jGraph {
    inner: Graph,
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
        Self { inner: graph }
    }

    pub fn inner(&self) -> &Graph {
        &self.inner
    }

    pub fn inner_clone(&self) -> Graph {
        self.inner.clone()
    }

    pub async fn run(&self, query: Query) -> Result<(), neo4rs::Error> {
        self.inner.run(query).await
    }

    pub async fn execute(
        &self,
        query: Query,
    ) -> Result<Neo4jRowStream, neo4rs::Error> {
        self.inner.execute(query).await.map(|stream| {
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
