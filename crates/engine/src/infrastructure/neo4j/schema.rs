//! Neo4j schema initialization - constraints and indexes.

use neo4rs::query;

use crate::infrastructure::neo4j::Neo4jGraph;

/// Safely truncate a string to at most `max_chars` characters.
///
/// This function uses character-based truncation, avoiding UTF-8 slicing panics
/// that can occur when truncating in the middle of multi-byte characters.
fn truncate_utf8_safe(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

/// Initialize Neo4j schema with required constraints and indexes.
///
/// This should be called once on startup. Constraints are created with
/// IF NOT EXISTS to be idempotent.
///
/// # Fail-Fast Behavior
/// This function verifies APOC is available before proceeding. If APOC is
/// not installed or not enabled, startup fails fast with a clear error.
pub async fn ensure_schema(graph: &Neo4jGraph) -> Result<(), neo4rs::Error> {
    // Verify APOC is available - fail fast if not
    verify_apoc_available(graph).await?;

    // Check for malformed lore tag JSON - fail fast if found
    verify_lore_tags_json_valid(graph).await?;

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

/// Verify APOC plugin is available in Neo4j.
///
/// APOC is required for tag search functionality (`apoc.convert.fromJsonList`).
/// This check runs on startup and fails fast with a clear error message
/// if APOC is not available, preventing runtime failures during tag searches.
///
/// # Errors
/// Returns `neo4rs::Error` if APOC is unavailable or the version check fails.
async fn verify_apoc_available(graph: &Neo4jGraph) -> Result<(), neo4rs::Error> {
    let q = query("RETURN apoc.version() AS version");

    let mut result = graph.execute(q).await.map_err(|e| {
        tracing::error!(
            error = %e,
            "APOC availability check failed - APOC plugin may not be installed or enabled"
        );
        e
    })?;

    if let Some(row) = result.next().await? {
        let version: String = row.get("version").map_err(|e| {
            tracing::error!(
                error = %e,
                "Failed to retrieve APOC version from query result"
            );
            neo4rs::Error::UnexpectedMessage(format!("Failed to get APOC version: {}", e))
        })?;
        tracing::info!(apoc_version = %version, "APOC plugin verified");
        Ok(())
    } else {
        let err = neo4rs::Error::UnexpectedMessage(
            "APOC version query returned no rows".to_string(),
        );
        tracing::error!(
            error = %err,
            "APOC availability check failed - query returned no results"
        );
        Err(err)
    }
}

/// Verify all Lore node tags contain valid JSON.
///
/// This check runs on startup and fails fast if any Lore nodes have malformed
/// tags JSON. This prevents runtime failures during tag searches which use
/// `apoc.convert.fromJsonList` to parse the tags field.
///
/// The check:
/// - Finds Lore nodes with non-empty tags
/// - Attempts to parse each tag value using `apoc.convert.fromJsonList`
/// - Returns an error listing all malformed entries if any are found
///
/// # Errors
/// Returns `neo4rs::Error` if any malformed JSON is found in Lore tags.
async fn verify_lore_tags_json_valid(graph: &Neo4jGraph) -> Result<(), neo4rs::Error> {
    // Query to find Lore nodes with malformed tags JSON
    // We attempt to parse the tags field using apoc.convert.fromJsonList
    // If parsing fails, the tag value is malformed
    let q = query(
        "MATCH (l:Lore)
         WHERE l.tags IS NOT NULL AND l.tags <> '' AND l.tags <> '[]'
         WITH l, apoc.convert.fromJsonList(coalesce(nullIf(l.tags, ''), '[]')) AS parsed
         WHERE parsed IS NULL
         RETURN l.id AS lore_id, l.title AS title, l.tags AS tags
         LIMIT 10"
    );

    let mut result = match graph.execute(q).await {
        Ok(r) => r,
        Err(e) => {
            // If the query itself fails (e.g., APOC function error), it may indicate
            // malformed JSON that caused APOC to throw
            tracing::error!(
                error = %e,
                "Lore tags JSON validation query failed - possible malformed JSON in database"
            );
            return Err(neo4rs::Error::UnexpectedMessage(format!(
                "Lore tags JSON validation failed: {}. This may indicate malformed JSON in Lore tags. \
                 Run the following query to identify problematic entries: \
                 MATCH (l:Lore) WHERE l.tags IS NOT NULL AND l.tags <> '' AND l.tags <> '[]' RETURN l.id, l.title, l.tags",
                e
            )));
        }
    };

    let mut malformed_entries: Vec<(String, String, String)> = Vec::new();

    while let Some(row) = result.next().await? {
        let lore_id: String = row.get("lore_id").map_err(|e| {
            tracing::error!(
                error = %e,
                "Failed to decode lore_id from query result"
            );
            neo4rs::Error::UnexpectedMessage(format!(
                "Failed to decode lore_id: {}",
                e
            ))
        })?;
        let title: String = row.get("title").map_err(|e| {
            tracing::error!(
                error = %e,
                "Failed to decode title from query result"
            );
            neo4rs::Error::UnexpectedMessage(format!(
                "Failed to decode title: {}",
                e
            ))
        })?;
        let tags: String = row.get("tags").map_err(|e| {
            tracing::error!(
                error = %e,
                "Failed to decode tags from query result"
            );
            neo4rs::Error::UnexpectedMessage(format!(
                "Failed to decode tags: {}",
                e
            ))
        })?;
        malformed_entries.push((lore_id, title, tags));
    }

    if !malformed_entries.is_empty() {
        let entries_desc = malformed_entries
            .iter()
            .map(|(id, title, tags)| {
                format!(
                    "  - Lore ID: {}, Title: '{}', Tags: '{}'",
                    id,
                    title,
                    if tags.len() > 50 {
                        format!("{}...", truncate_utf8_safe(tags, 50))
                    } else {
                        tags.clone()
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let error_message = format!(
            "MALFORMED LORE TAGS JSON DETECTED:\n\
             Found {} Lore entr{} with malformed tags JSON. \
             The tags field must be a valid JSON array string (e.g., '[\"tag1\", \"tag2\"]').\n\
             Affected entries:\n{}\n\
             To fix this issue, either:\n\
             1. Delete and recreate the affected Lore entries with valid tags, or\n\
             2. Update the tags directly in Neo4j Browser with: \n\
                MATCH (l:Lore) WHERE l.id IN [{}] SET l.tags = '[]'\n\
             3. Or set valid tags: \n\
                MATCH (l:Lore) WHERE l.id IN [{}] SET l.tags = '[\"corrected\", \"tags\"]'",
            malformed_entries.len(),
            if malformed_entries.len() == 1 { "y" } else { "ies" },
            entries_desc,
            malformed_entries
                .iter()
                .map(|(id, _, _)| format!("'{}'", id))
                .collect::<Vec<_>>()
                .join(", "),
            malformed_entries
                .iter()
                .map(|(id, _, _)| format!("'{}'", id))
                .collect::<Vec<_>>()
                .join(", ")
        );

        tracing::error!("{}", error_message);

        return Err(neo4rs::Error::UnexpectedMessage(format!(
            "Database integrity check failed: {} Lore entr{} with malformed tags JSON. \
             Check server logs for details.",
            malformed_entries.len(),
            if malformed_entries.len() == 1 { "y" } else { "ies" }
        )));
    }

    tracing::info!("Lore tags JSON validation passed - all tags are valid JSON");
    Ok(())
}
