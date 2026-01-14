//! Generic query helpers to reduce Neo4j repository boilerplate.

use neo4rs::{Query, Row};

use crate::infrastructure::neo4j::Neo4jGraph;
use crate::infrastructure::ports::RepoError;

/// Execute a query and collect results using a converter function.
pub async fn collect_rows<T, F>(
    graph: &Neo4jGraph,
    query: Query,
    converter: F,
) -> Result<Vec<T>, RepoError>
where
    F: Fn(Row) -> Result<T, RepoError>,
{
    let mut result = graph
        .execute(query)
        .await
        .map_err(|e| RepoError::database("execute", e.to_string()))?;

    let mut items = Vec::new();
    while let Some(row) = result
        .next()
        .await
        .map_err(|e| RepoError::database("fetch_row", e.to_string()))?
    {
        items.push(converter(row)?);
    }
    Ok(items)
}

/// Execute a query and return first result using a converter function.
pub async fn get_first_row<T, F>(
    graph: &Neo4jGraph,
    query: Query,
    converter: F,
) -> Result<Option<T>, RepoError>
where
    F: Fn(Row) -> Result<T, RepoError>,
{
    let mut result = graph
        .execute(query)
        .await
        .map_err(|e| RepoError::database("execute", e.to_string()))?;

    if let Some(row) = result
        .next()
        .await
        .map_err(|e| RepoError::database("fetch_row", e.to_string()))?
    {
        Ok(Some(converter(row)?))
    } else {
        Ok(None)
    }
}

/// Execute a write query with no return value.
pub async fn run_query(
    graph: &Neo4jGraph,
    query: Query,
    operation: &'static str,
) -> Result<(), RepoError> {
    graph
        .run(query)
        .await
        .map_err(|e| RepoError::database(operation, e.to_string()))
}
