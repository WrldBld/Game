//! Neo4j schema initialization - constraints and indexes.

use neo4rs::query;

use crate::infrastructure::neo4j::Neo4jGraph;

/// Initialize Neo4j schema with required constraints and indexes.
///
/// This should be called once on startup. Constraints are created with
/// IF NOT EXISTS to be idempotent.
pub async fn ensure_schema(graph: &Neo4jGraph) -> Result<(), neo4rs::Error> {
    // Unique constraint on LoreChunk composite key (lore_id + order).
    // This prevents duplicate orders within the same lore entry at the database level.
    graph
        .run(query(
            "CREATE CONSTRAINT lore_chunk_order_unique IF NOT EXISTS
             FOR (c:LoreChunk) REQUIRE c.lore_order_key IS UNIQUE",
        ))
        .await?;

    // Index on LoreChunk.lore_id for efficient chunk lookups by lore.
    graph
        .run(query(
            "CREATE INDEX lore_chunk_lore_id IF NOT EXISTS
             FOR (c:LoreChunk) ON (c.lore_id)",
        ))
        .await?;

    tracing::info!("Neo4j schema initialized (constraints and indexes ensured)");
    Ok(())
}
