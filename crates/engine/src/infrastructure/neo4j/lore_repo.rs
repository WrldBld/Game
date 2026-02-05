//! Neo4j lore repository implementation.
//!
//! Lore chunks are stored as separate `:LoreChunk` nodes with a relationship
//! `(Lore)-[:HAS_CHUNK]->(LoreChunk)`. A unique constraint on `lore_order_key`
//! (composite of lore_id + order) enforces order uniqueness at the database level.

use std::sync::Arc;

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Row};
use wrldbldr_domain::*;

use super::helpers::{
    parse_typed_id, RowExt, NodeExt, match_lore_by_id, match_lore_by_world,
    match_character_by_param, return_knowledge_fields,
};
use crate::infrastructure::ports::{ClockPort, LoreRepo, RepoError};
use crate::infrastructure::ports::types::{LoreJournalEntry, LoreDetailRepo, LoreChunkRepo, LoreKnowledgeRepo, LoreDiscoverySourceRepo};

/// Repository for Lore operations.
pub struct Neo4jLoreRepo {
    graph: Neo4jGraph,
    clock: Arc<dyn ClockPort>,
}

impl Neo4jLoreRepo {
    pub fn new(graph: Neo4jGraph, clock: Arc<dyn ClockPort>) -> Self {
        Self { graph, clock }
    }

    /// Fetch chunks for a lore entry from the database.
    async fn fetch_chunks(&self, lore_id: LoreId) -> Result<Vec<LoreChunk>, RepoError> {
        let q = query(
            "MATCH (l:Lore {id: $lore_id})-[:HAS_CHUNK]->(c:LoreChunk)
             RETURN c ORDER BY c.order",
        )
        .param("lore_id", lore_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut chunks = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            chunks.push(self.row_to_chunk(row)?);
        }

        Ok(chunks)
    }

    fn row_to_chunk(&self, row: Row) -> Result<LoreChunk, RepoError> {
        let node: neo4rs::Node = row
            .get("c")
            .map_err(|e| RepoError::database("query", format!("Failed to get 'c' node: {}", e)))?;

        let id: LoreChunkId = parse_typed_id(&node, "id").map_err(|e| {
            RepoError::database("query", format!("Failed to parse LoreChunkId: {}", e))
        })?;
        let order: i64 = node.get("order").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get 'order' for LoreChunk {}: {}", id, e),
            )
        })?;
        let content: String = node.get("content").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get 'content' for LoreChunk {}: {}", id, e),
            )
        })?;
        let title: Option<String> = node.get_optional_string("title");
        let discovery_hint: Option<String> = node.get_optional_string("discovery_hint");

        let mut chunk = LoreChunk::new(content).with_id(id).with_order(order as u32);
        if let Some(t) = title {
            chunk = chunk.with_title(t);
        }
        if let Some(hint) = discovery_hint {
            chunk = chunk.with_discovery_hint(hint);
        }
        Ok(chunk)
    }

    fn row_to_lore_without_chunks(&self, row: Row) -> Result<Lore, RepoError> {
        let node: neo4rs::Node = row
            .get("l")
            .map_err(|e| RepoError::database("query", format!("Failed to get 'l' node: {}", e)))?;
        let fallback = self.clock.now();

        let id: LoreId = parse_typed_id(&node, "id")
            .map_err(|e| RepoError::database("query", format!("Failed to parse LoreId: {}", e)))?;
        let world_id: WorldId = parse_typed_id(&node, "world_id").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to parse WorldId for Lore {}: {}", id, e),
            )
        })?;
        let title: String = node.get("title").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get 'title' for Lore {}: {}", id, e),
            )
        })?;
        let summary: String = node.get_string_or("summary", "");
        let category_str: String = node.get_string_or("category", "");
        let category: LoreCategory = if category_str.is_empty() {
            return Err(RepoError::database(
                "parse",
                format!("Missing required field 'category' for lore: {}", id),
            ));
        } else {
            category_str.parse().map_err(|e| {
                RepoError::database(
                    "parse",
                    format!("Invalid LoreCategory '{}': {}", category_str, e),
                )
            })?
        };
        let is_common_knowledge: bool = node.get_bool_or("is_common_knowledge", false);
        let created_at = node.get_datetime_or("created_at", fallback);
        let updated_at = node.get_datetime_or("updated_at", fallback);

        // Parse tags from JSON - fail-fast on invalid JSON
        let tags_str = node
            .get_optional_string("tags")
            .ok_or_else(|| RepoError::database("query", format!("Missing tags for Lore {}", id)))?;
        let tags_raw: Vec<String> = serde_json::from_str(&tags_str).map_err(|e| {
            RepoError::database(
                "parse",
                format!(
                    "Invalid tags JSON for Lore {}: {} (value: '{}')",
                    id, e, tags_str
                ),
            )
        })?;

        let mut lore = Lore::new(world_id, title, category, created_at)
            .with_id(id)
            .with_summary(summary)
            .with_timestamps(created_at, updated_at);

        for tag_str in tags_raw {
            let tag =
                wrldbldr_domain::Tag::new(&tag_str).map_err(|e| RepoError::database("parse", e))?;
            lore = lore.with_tag(tag);
        }
        if is_common_knowledge {
            lore = lore.as_common_knowledge();
        }
        Ok(lore)
    }

    async fn row_to_lore(&self, row: Row) -> Result<Lore, RepoError> {
        let lore = self.row_to_lore_without_chunks(row)?;
        let chunks = self.fetch_chunks(lore.id()).await?;
        Ok(lore.with_chunks(chunks))
    }

    fn row_to_knowledge(&self, row: Row) -> Result<LoreKnowledge, RepoError> {
        // Get the relationship properties
        let lore_id_str: String = row
            .get("lore_id")
            .map_err(|e| RepoError::database("query", format!("Failed to get 'lore_id': {}", e)))?;
        let character_id_str: String = row.get("character_id").map_err(|e| {
            RepoError::database("query", format!("Failed to get 'character_id': {}", e))
        })?;
        let known_chunk_ids_json: String = row.get::<String>("known_chunk_ids").map_err(|e| {
            RepoError::database(
                "query",
                format!(
                    "Missing required column 'known_chunk_ids' for lore knowledge (lore_id: {}): {}",
                    lore_id_str, e
                ),
            )
        })?;
        let discovery_source_json: String = row
            .get("discovery_source")
            .map_err(|e| RepoError::database("query", format!("Failed to get 'discovery_source' for lore knowledge (lore_id: {}, character_id: {}): {}", lore_id_str, character_id_str, e)))?;
        let discovered_at_str: String = row
            .get("discovered_at")
            .map_err(|e| RepoError::database("query", format!("Failed to get 'discovered_at' for lore knowledge (lore_id: {}, character_id: {}): {}", lore_id_str, character_id_str, e)))?;
        let notes: Option<String> = row.get("notes").ok();

        let lore_id = LoreId::from_uuid(uuid::Uuid::parse_str(&lore_id_str).map_err(|e| {
            RepoError::database(
                "query",
                format!(
                    "Failed to parse LoreId for lore knowledge: '{}': {}",
                    lore_id_str, e
                ),
            )
        })?);
        let character_id =
            CharacterId::from_uuid(uuid::Uuid::parse_str(&character_id_str).map_err(|e| {
                RepoError::database(
                    "query",
                    format!(
                        "Failed to parse CharacterId for lore knowledge: '{}': {}",
                        character_id_str, e
                    ),
                )
            })?);
        let known_chunk_ids: Vec<LoreChunkId> = serde_json::from_str(&known_chunk_ids_json)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let discovery_source: LoreDiscoverySource = serde_json::from_str(&discovery_source_json)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let discovered_at = chrono::DateTime::parse_from_rfc3339(&discovered_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| self.clock.now());

        let mut knowledge = LoreKnowledge::partial(
            lore_id,
            character_id,
            known_chunk_ids,
            discovery_source,
            discovered_at,
        );
        if let Some(n) = notes {
            knowledge = knowledge.with_notes(n);
        }
        Ok(knowledge)
    }

    /// Create composite key for chunk order uniqueness constraint.
    fn make_lore_order_key(lore_id: &LoreId, order: u32) -> String {
        format!("{}_{}", lore_id, order)
    }

    /// Helper function to convert a row to LoreJournalEntry (use-case DTO).
    fn row_to_journal_entry(
        &self,
        row: Row,
    ) -> Result<LoreJournalEntry, RepoError> {
        // Get lore node with explicit type
        let lore_node: neo4rs::Node = row
            .get::<neo4rs::Node>("l")
            .map_err(|e| RepoError::database("query", format!("Failed to get 'l' node: {}", e)))?;

        // Get knowledge relationship with explicit type
        let k: neo4rs::Relation = row
            .get::<neo4rs::Relation>("k")
            .map_err(|e| RepoError::database("query", format!("Failed to get knowledge: {}", e)))?;

        let lore_id_str: String = k.get("lore_id").map_err(|e| {
            RepoError::database("query", format!("Failed to get lore_id from knowledge: {}", e))
        })?;
        let character_id_str: String = k.get("character_id").map_err(|e| {
            RepoError::database("query", format!("Failed to get character_id from knowledge: {}", e))
        })?;
        let known_chunk_ids_json: String = k.get("known_chunk_ids").map_err(|e| {
            RepoError::database(
                "query",
                format!("Missing known_chunk_ids in knowledge: {}", e),
            )
        })?;
        let discovery_source_json: String = k.get("discovery_source").map_err(|e| {
            RepoError::database(
                "query",
                format!("Missing discovery_source in knowledge: {}", e),
            )
        })?;
        let discovered_at_str: String = k.get("discovered_at").map_err(|e| {
            RepoError::database(
                "query",
                format!("Missing discovered_at in knowledge: {}", e),
            )
        })?;
        let notes: Option<String> = k.get("notes").ok();

        // Parse knowledge data
        let known_chunk_ids: Vec<String> = serde_json::from_str(&known_chunk_ids_json)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let discovery_source: LoreDiscoverySourceRepo =
            serde_json::from_str(&discovery_source_json)
                .map_err(|e| RepoError::Serialization(e.to_string()))?;

        let knowledge_data = LoreKnowledgeRepo {
            lore_id: lore_id_str.clone(),
            character_id: character_id_str,
            known_chunk_ids,
            discovery_source,
            discovered_at: discovered_at_str,
            notes,
        };

        // Build LoreDetailRepo from node + chunks
        let lore_data = self.node_to_lore_detail_repo(lore_node, &row)?;

        Ok(LoreJournalEntry {
            lore: lore_data,
            knowledge: knowledge_data,
        })
    }

    /// Helper function to convert a node to LoreDetailRepo with chunks.
    fn node_to_lore_detail_repo(
        &self,
        node: neo4rs::Node,
        row: &Row,
    ) -> Result<LoreDetailRepo, RepoError> {
        let _fallback = self.clock.now();

        let id: String = node.get("id").map_err(|e| {
            RepoError::database("query", format!("Failed to get lore id: {}", e))
        })?;
        let world_id: String = node.get("world_id").map_err(|e| {
            RepoError::database("query", format!("Failed to get world_id for lore: {}", e))
        })?;
        let title: String = node.get("title").map_err(|e| {
            RepoError::database("query", format!("Failed to get title for lore {}: {}", id, e))
        })?;
        let summary: String = node.get_string_or("summary", "");
        let category_str: String = node.get_string_or("category", "");
        let category: String = if category_str.is_empty() {
            return Err(RepoError::database(
                "parse",
                format!("Missing required field 'category' for lore: {}", id),
            ));
        } else {
            category_str
        };
        let is_common_knowledge: bool = node.get_bool_or("is_common_knowledge", false);

        // Parse tags from JSON
        let tags_str = node.get_optional_string("tags");
        let tags: Vec<String> = tags_str
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        // Get chunks from the collect result with explicit type
        let chunk_nodes: Vec<neo4rs::Node> = row.get("chunks").map_err(|e| {
            RepoError::database(
                "query",
                format!("Failed to get chunks for lore {}: {}", id, e),
            )
        })?;

        let chunks: Vec<LoreChunkRepo> = chunk_nodes
            .into_iter()
            .map(|c| {
                let chunk_id: String = c.get("id").unwrap_or_default();
                let order: i64 = c.get("order").unwrap_or(0);
                let title: Option<String> = c.get_optional_string("title");
                let content: String = c.get("content").unwrap_or_default();
                let discovery_hint: Option<String> = c.get_optional_string("discovery_hint");

                LoreChunkRepo {
                    id: chunk_id,
                    order: order as u32,
                    title,
                    content,
                    discovery_hint,
                }
            })
            .collect();

        Ok(LoreDetailRepo {
            id,
            world_id,
            title,
            summary,
            category,
            chunks,
            is_common_knowledge,
            tags,
        })
    }
}

#[async_trait]
impl LoreRepo for Neo4jLoreRepo {
    async fn get(&self, id: LoreId) -> Result<Option<Lore>, RepoError> {
        let q = query(&format!("{} RETURN l", match_lore_by_id())).param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            Ok(Some(self.row_to_lore(row).await?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, lore: &Lore) -> Result<(), RepoError> {
        let tags: Vec<String> = lore.tags().iter().map(|t| t.to_string()).collect();
        let tags_json =
            serde_json::to_string(&tags).map_err(|e| RepoError::Serialization(e.to_string()))?;

        // Save the Lore node (without chunks - they're separate nodes now)
        let q = query(
            "MERGE (l:Lore {id: $id})
            SET l.world_id = $world_id,
                l.title = $title,
                l.summary = $summary,
                l.category = $category,
                l.is_common_knowledge = $is_common_knowledge,
                l.tags = $tags,
                l.created_at = $created_at,
                l.updated_at = $updated_at
            WITH l
            MATCH (w:World {id: $world_id})
            MERGE (w)-[:HAS_LORE]->(l)
            RETURN l.id as id",
        )
        .param("id", lore.id().to_string())
        .param("world_id", lore.world_id().to_string())
        .param("title", lore.title().to_string())
        .param("summary", lore.summary().to_string())
        .param("category", lore.category().to_string())
        .param("is_common_knowledge", lore.is_common_knowledge())
        .param("tags", tags_json)
        .param("created_at", lore.created_at().to_rfc3339())
        .param("updated_at", lore.updated_at().to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Get existing chunk IDs to determine what to delete
        let existing_chunks = self.fetch_chunks(lore.id()).await?;
        let existing_ids: std::collections::HashSet<_> =
            existing_chunks.iter().map(|c| c.id).collect();
        let new_ids: std::collections::HashSet<_> = lore.chunks().iter().map(|c| c.id).collect();

        // Delete chunks that are no longer in the lore
        let to_delete: Vec<_> = existing_ids.difference(&new_ids).collect();
        for chunk_id in to_delete {
            let del_q = query(
                "MATCH (c:LoreChunk {id: $chunk_id})
                 DETACH DELETE c",
            )
            .param("chunk_id", chunk_id.to_string());

            self.graph
                .run(del_q)
                .await
                .map_err(|e| RepoError::database("query", e))?;
        }

        // Upsert each chunk as a separate node
        for chunk in lore.chunks() {
            let lore_order_key = Self::make_lore_order_key(&lore.id(), chunk.order);

            let chunk_q = query(
                "MATCH (l:Lore {id: $lore_id})
                 MERGE (c:LoreChunk {id: $chunk_id})
                 SET c.lore_id = $lore_id,
                     c.order = $order,
                     c.lore_order_key = $lore_order_key,
                     c.title = $title,
                     c.content = $content,
                     c.discovery_hint = $discovery_hint
                 MERGE (l)-[:HAS_CHUNK]->(c)
                 RETURN c.id as id",
            )
            .param("lore_id", lore.id().to_string())
            .param("chunk_id", chunk.id.to_string())
            .param("order", chunk.order as i64)
            .param("lore_order_key", lore_order_key)
            .param(
                "title",
                chunk.title.as_deref().unwrap_or_default().to_string(),
            )
            .param("content", chunk.content.to_string())
            .param(
                "discovery_hint",
                chunk
                    .discovery_hint
                    .as_deref()
                    .unwrap_or_default()
                    .to_string(),
            );

            self.graph.run(chunk_q).await.map_err(|e| {
                // Check for constraint violation
                let msg = e.to_string();
                if msg.contains("already exists") || msg.contains("ConstraintValidation") {
                    RepoError::ConstraintViolation(format!(
                        "Duplicate chunk order {} for lore {}",
                        chunk.order,
                        lore.id()
                    ))
                } else {
                    RepoError::database("save_lore", msg)
                }
            })?;
        }

        tracing::debug!(
            "Saved lore: {} with {} chunks",
            lore.title(),
            lore.chunks().len()
        );
        Ok(())
    }

    async fn delete(&self, id: LoreId) -> Result<(), RepoError> {
        // Delete all chunks first, then the lore
        let q = query(
            "MATCH (l:Lore {id: $id})
             OPTIONAL MATCH (l)-[:HAS_CHUNK]->(c:LoreChunk)
             DETACH DELETE c, l",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Deleted lore: {}", id);
        Ok(())
    }

    async fn list_for_world(&self, world_id: WorldId) -> Result<Vec<Lore>, RepoError> {
        let q = query(&format!("{} RETURN l ORDER BY l.title", match_lore_by_world()))
            .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut lore_entries = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            lore_entries.push(self.row_to_lore(row).await?);
        }

        Ok(lore_entries)
    }

    async fn list_by_category(
        &self,
        world_id: WorldId,
        category: LoreCategory,
    ) -> Result<Vec<Lore>, RepoError> {
        let q = query(
            "MATCH (l:Lore {world_id: $world_id, category: $category})
            RETURN l ORDER BY l.title",
        )
        .param("world_id", world_id.to_string())
        .param("category", category.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut lore_entries = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            lore_entries.push(self.row_to_lore(row).await?);
        }

        Ok(lore_entries)
    }

    async fn list_common_knowledge(&self, world_id: WorldId) -> Result<Vec<Lore>, RepoError> {
        let q = query(
            "MATCH (l:Lore {world_id: $world_id, is_common_knowledge: true})
            RETURN l ORDER BY l.title",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut lore_entries = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            lore_entries.push(self.row_to_lore(row).await?);
        }

        Ok(lore_entries)
    }

    async fn search_by_tags(
        &self,
        world_id: WorldId,
        tags: &[String],
    ) -> Result<Vec<Lore>, RepoError> {
        if tags.is_empty() {
            return Ok(vec![]);
        }

        // Search for lore entries that have ANY of the specified tags.
        // Tags are stored as JSON arrays, so we check if the JSON string contains
        // each tag as a quoted string (e.g., `"history"` for tag "history").
        // Build a WHERE clause with OR conditions for each tag.
        // SAFETY: tag_conditions are generated programmatically from enumerate(),
        // not from user input. The format string only contains numeric indices
        // like "$tag0", "$tag1", etc. Tag values themselves are passed as
        // parameterized values, preventing Cypher injection.
        let tag_conditions: Vec<String> = tags
            .iter()
            .enumerate()
            .map(|(i, _)| format!("l.tags CONTAINS $tag{}", i))
            .collect();
        let where_clause = tag_conditions.join(" OR ");

        // SAFETY: The cypher query uses format!() only to interpolate `where_clause`,
        // which contains only programmatically-generated conditions with numeric
        // parameter placeholders (e.g., "$tag0 OR $tag1"). No user input is
        // interpolated into the query string itself.
        let cypher = format!(
            "MATCH (l:Lore {{world_id: $world_id}})
            WHERE {}
            RETURN l ORDER BY l.title",
            where_clause
        );

        let mut q = query(&cypher).param("world_id", world_id.to_string());

        // SAFETY: Parameter names ("tag0", "tag1", etc.) are generated from
        // enumerate() indices, not user input. The tag values are properly
        // passed as parameterized values to neo4rs, which handles escaping.
        for (i, tag) in tags.iter().enumerate() {
            // Tags in JSON are stored as `["tag1", "tag2"]`, so we search for `"tag"`.
            // Use serde_json::to_string to properly escape quotes and special characters.
            let json_tag =
                serde_json::to_string(tag).map_err(|e| RepoError::Serialization(e.to_string()))?;
            q = q.param(&format!("tag{}", i), json_tag);
        }

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut lore_entries = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            lore_entries.push(self.row_to_lore(row).await?);
        }

        Ok(lore_entries)
    }

    async fn grant_knowledge(&self, knowledge: &LoreKnowledge) -> Result<(), RepoError> {
        let known_chunk_ids_json = serde_json::to_string(&knowledge.known_chunk_ids)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let discovery_source_json = serde_json::to_string(&knowledge.discovery_source)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        let q = query(&format!(
            "{} MATCH (l:Lore {{id: $lore_id}})
            MERGE (c)-[k:KNOWS_LORE]->(l)
            SET k.known_chunk_ids = $known_chunk_ids,
                k.discovery_source = $discovery_source,
                k.discovered_at = $discovered_at,
                k.notes = $notes
            RETURN c.id as character_id",
            match_character_by_param("character_id")
        ))
        .param("character_id", knowledge.character_id.to_string())
        .param("lore_id", knowledge.lore_id.to_string())
        .param("known_chunk_ids", known_chunk_ids_json)
        .param("discovery_source", discovery_source_json)
        .param("discovered_at", knowledge.discovered_at.to_rfc3339())
        .param(
            "notes",
            knowledge.notes.as_deref().unwrap_or_default().to_string(),
        );

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!(
            "Granted lore {} to character {}",
            knowledge.lore_id,
            knowledge.character_id
        );
        Ok(())
    }

    async fn revoke_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (c:Character {id: $character_id})-[k:KNOWS_LORE]->(l:Lore {id: $lore_id})
            DELETE k",
        )
        .param("character_id", character_id.to_string())
        .param("lore_id", lore_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Revoked lore {} from character {}", lore_id, character_id);
        Ok(())
    }

    async fn get_character_knowledge(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<LoreKnowledge>, RepoError> {
        let q = query(&format!(
            "{}-[k:KNOWS_LORE]->(l:Lore) {}",
            match_character_by_param("character_id"),
            return_knowledge_fields()
        ))
        .param("character_id", character_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut knowledge_list = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            knowledge_list.push(self.row_to_knowledge(row)?);
        }

        Ok(knowledge_list)
    }

    async fn get_knowledge_for_lore(
        &self,
        lore_id: LoreId,
    ) -> Result<Vec<LoreKnowledge>, RepoError> {
        let q = query(&format!(
            "MATCH (c:Character)-[k:KNOWS_LORE]->(l:Lore {{id: $lore_id}}) {}",
            return_knowledge_fields()
        ))
        .param("lore_id", lore_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut knowledge_list = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            knowledge_list.push(self.row_to_knowledge(row)?);
        }

        Ok(knowledge_list)
    }

    async fn character_knows_lore(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
    ) -> Result<Option<LoreKnowledge>, RepoError> {
        let q = query(&format!(
            "{}-[k:KNOWS_LORE]->(l:Lore {{id: $lore_id}}) {}",
            match_character_by_param("character_id"),
            return_knowledge_fields()
        ))
        .param("character_id", character_id.to_string())
        .param("lore_id", lore_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            Ok(Some(self.row_to_knowledge(row)?))
        } else {
            Ok(None)
        }
    }

    async fn add_chunks_to_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
        chunk_ids: &[LoreChunkId],
    ) -> Result<(), RepoError> {
        if chunk_ids.is_empty() {
            return Ok(());
        }

        // First, fetch the current known_chunk_ids (stored as JSON string)
        let fetch_q = query(&format!(
            "{}-[k:KNOWS_LORE]->(l:Lore {{id: $lore_id}})
            RETURN k.known_chunk_ids as known_chunk_ids",
            match_character_by_param("character_id")
        ))
        .param("character_id", character_id.to_string())
        .param("lore_id", lore_id.to_string());

        let mut result = self
            .graph
            .execute(fetch_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let current_chunks: Vec<LoreChunkId> = if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let json_str: String = row.get("known_chunk_ids").map_err(|e| {
                RepoError::database(
                    "query",
                    format!(
                        "Missing known_chunk_ids for knowledge (character: {}, lore: {}): {}",
                        character_id, lore_id, e
                    ),
                )
            })?;
            serde_json::from_str(&json_str).map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Invalid known_chunk_ids JSON for knowledge (character: {}, lore: {}): {} (value: '{}')",
                        character_id, lore_id, e, json_str
                    ),
                )
            })?
        } else {
            tracing::warn!(
                character_id = %character_id,
                lore_id = %lore_id,
                "Lore knowledge relationship not found"
            );
            return Err(RepoError::not_found(
                "LoreKnowledge",
                format!("char:{}/lore:{}", character_id, lore_id),
            ));
        };

        // Merge new chunks with existing, deduplicating
        let mut merged: Vec<LoreChunkId> = current_chunks;
        for chunk_id in chunk_ids {
            if !merged.contains(chunk_id) {
                merged.push(*chunk_id);
            }
        }

        // Serialize and update
        let merged_json =
            serde_json::to_string(&merged).map_err(|e| RepoError::Serialization(e.to_string()))?;

        let update_q = query(
            "MATCH (c:Character {id: $character_id})-[k:KNOWS_LORE]->(l:Lore {id: $lore_id})
            SET k.known_chunk_ids = $known_chunk_ids
            RETURN c.id as character_id",
        )
        .param("character_id", character_id.to_string())
        .param("lore_id", lore_id.to_string())
        .param("known_chunk_ids", merged_json);

        self.graph
            .run(update_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!(
            "Added {} chunks to character {} knowledge of lore {}",
            chunk_ids.len(),
            character_id,
            lore_id
        );
        Ok(())
    }

    async fn remove_chunks_from_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
        chunk_ids: &[LoreChunkId],
    ) -> Result<bool, RepoError> {
        if chunk_ids.is_empty() {
            return Ok(false);
        }

        // First, fetch the current known_chunk_ids (stored as JSON string)
        let fetch_q = query(&format!(
            "{}-[k:KNOWS_LORE]->(l:Lore {{id: $lore_id}})
            RETURN k.known_chunk_ids as known_chunk_ids",
            match_character_by_param("character_id")
        ))
        .param("character_id", character_id.to_string())
        .param("lore_id", lore_id.to_string());

        let mut result = self
            .graph
            .execute(fetch_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let current_chunks: Vec<LoreChunkId> = if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let json_str: String = row.get("known_chunk_ids").map_err(|e| {
                RepoError::database(
                    "query",
                    format!(
                        "Missing known_chunk_ids for knowledge (character: {}, lore: {}): {}",
                        character_id, lore_id, e
                    ),
                )
            })?;
            serde_json::from_str(&json_str).map_err(|e| {
                RepoError::database(
                    "parse",
                    format!(
                        "Invalid known_chunk_ids JSON for knowledge (character: {}, lore: {}): {} (value: '{}')",
                        character_id, lore_id, e, json_str
                    ),
                )
            })?
        } else {
            // No knowledge relationship exists
            return Ok(false);
        };

        // Remove specified chunks
        let remaining: Vec<LoreChunkId> = current_chunks
            .into_iter()
            .filter(|id| !chunk_ids.contains(id))
            .collect();

        // If no chunks remain, delete the relationship entirely
        if remaining.is_empty() {
            self.revoke_knowledge(character_id, lore_id).await?;
            tracing::debug!(
                "Removed all chunks from character {} knowledge of lore {}, relationship deleted",
                character_id,
                lore_id
            );
            return Ok(true);
        }

        // Otherwise, update with remaining chunks
        let remaining_json = serde_json::to_string(&remaining)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

        let update_q = query(&format!(
            "{}-[k:KNOWS_LORE]->(l:Lore {{id: $lore_id}})
            SET k.known_chunk_ids = $known_chunk_ids
            RETURN c.id as character_id",
            match_character_by_param("character_id")
        ))
        .param("character_id", character_id.to_string())
        .param("lore_id", lore_id.to_string())
        .param("known_chunk_ids", remaining_json);

        self.graph
            .run(update_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!(
            "Removed {} chunks from character {} knowledge of lore {}, {} chunks remaining",
            chunk_ids.len(),
            character_id,
            lore_id,
            remaining.len()
        );
        Ok(false)
    }

    async fn get_character_lore_journal(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<LoreJournalEntry>, RepoError> {
        // Single query to fetch both lore and knowledge (avoid N+1)
        // Fetches chunks separately for each lore (still N+1 for chunks, but acceptable for now)
        let q = query(&format!(
            "{}-[k:KNOWS_LORE]->(l:Lore)
            WITH c, k, l
            OPTIONAL MATCH (l)-[:HAS_CHUNK]->(ch:LoreChunk)
            RETURN l, k, collect(ch) as chunks
            ORDER BY l.title",
            match_character_by_param("character_id")
        ))
        .param("character_id", character_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut journal_entries = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            journal_entries.push(self.row_to_journal_entry(row)?);
        }

        Ok(journal_entries)
    }

    async fn link_lore_about(
        &self,
        lore_id: LoreId,
        target: wrldbldr_domain::value_objects::AboutTarget,
    ) -> Result<(), RepoError> {
        // Validate lore exists
        self.get(lore_id).await?.ok_or_else(|| {
            RepoError::not_found("Lore", lore_id.to_string())
        })?;

        let (edge_type, target_id, target_label) = match target {
            wrldbldr_domain::value_objects::AboutTarget::Character { character_id } => {
                ("ABOUT_CHARACTER", character_id.to_string(), "Character")
            }
            wrldbldr_domain::value_objects::AboutTarget::Location { location_id } => {
                ("ABOUT_LOCATION", location_id.to_string(), "Location")
            }
            wrldbldr_domain::value_objects::AboutTarget::Region { region_id } => {
                ("ABOUT_REGION", region_id.to_string(), "Region")
            }
            wrldbldr_domain::value_objects::AboutTarget::Item { item_id } => {
                ("ABOUT_ITEM", item_id.to_string(), "Item")
            }
        };

        let q = query(&format!(
            "MATCH (l:Lore {{id: $lore_id}})
            MATCH (t:{} {{id: $target_id}})
            MERGE (l)-[:{}]->(t)
            RETURN l.id AS lore_id",
            target_label, edge_type
        ))
        .param("lore_id", lore_id.to_string())
        .param("target_id", target_id.clone());

        // The query will fail silently if target doesn't exist
        // Check result count to verify target exists
        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("link_lore_about", e))?;

        let has_result = matches!(result.next().await, Ok(Some(_)));
        if !has_result {
            return Err(RepoError::not_found(target_label, target_id));
        }

        tracing::debug!("Linked lore {} about {}", lore_id, edge_type);
        Ok(())
    }

    async fn unlink_lore_about(
        &self,
        lore_id: LoreId,
        target: &wrldbldr_domain::value_objects::AboutTarget,
    ) -> Result<(), RepoError> {
        // Validate lore exists
        self.get(lore_id).await?.ok_or_else(|| {
            RepoError::not_found("Lore", lore_id.to_string())
        })?;

        let (edge_type, target_id, target_label) = match target {
            wrldbldr_domain::value_objects::AboutTarget::Character { character_id } => {
                ("ABOUT_CHARACTER", character_id.to_string(), "Character")
            }
            wrldbldr_domain::value_objects::AboutTarget::Location { location_id } => {
                ("ABOUT_LOCATION", location_id.to_string(), "Location")
            }
            wrldbldr_domain::value_objects::AboutTarget::Region { region_id } => {
                ("ABOUT_REGION", region_id.to_string(), "Region")
            }
            wrldbldr_domain::value_objects::AboutTarget::Item { item_id } => {
                ("ABOUT_ITEM", item_id.to_string(), "Item")
            }
        };

        let q = query(&format!(
            "MATCH (l:Lore {{id: $lore_id}})-[r:{}]->(t:{} {{id: $target_id}})
            DELETE r
            RETURN count(r) AS deleted_count",
            edge_type, target_label
        ))
        .param("lore_id", lore_id.to_string())
        .param("target_id", target_id);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("unlink_lore_about", e))?;

        tracing::debug!("Unlinked lore {} from {}", lore_id, edge_type);
        Ok(())
    }

    async fn get_lore_about_targets(
        &self,
        lore_id: LoreId,
    ) -> Result<Vec<wrldbldr_domain::value_objects::AboutTarget>, RepoError> {
        // Query only ABOUT_* relationships for a lore entry
        // Use label membership checks instead of labels(t)[0] for performance and clarity
        let q = query(
            "MATCH (l:Lore {id: $lore_id})-[r:ABOUT_CHARACTER|ABOUT_LOCATION|ABOUT_REGION|ABOUT_ITEM]->(t)
            RETURN type(r) AS rel_type, t.id AS target_id,
                   CASE WHEN t:Character THEN 'Character'
                        WHEN t:Location THEN 'Location'
                        WHEN t:Region THEN 'Region'
                        WHEN t:Item THEN 'Item'
                        ELSE 'Unknown' END AS target_label
        ",
        )
        .param("lore_id", lore_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("get_lore_about_targets", e))?;

        let mut targets = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let rel_type: String = row
                .get("rel_type")
                .map_err(|e| RepoError::database("query", format!("Failed to get rel_type: {}", e)))?;
            let target_label: String = row
                .get("target_label")
                .map_err(|e| RepoError::database("query", format!("Failed to get target_label: {}", e)))?;
            let target_id_str: String = row
                .get("target_id")
                .map_err(|e| RepoError::database("query", format!("Failed to get target_id: {}", e)))?;

            let target = match target_label.as_str() {
                "Character" => {
                    let id = uuid::Uuid::parse_str(&target_id_str)
                        .map_err(|e| RepoError::database("parse", format!("Invalid CharacterId: {}", e)))?;
                    wrldbldr_domain::value_objects::AboutTarget::Character {
                        character_id: CharacterId::from_uuid(id),
                    }
                }
                "Location" => {
                    let id = uuid::Uuid::parse_str(&target_id_str)
                        .map_err(|e| RepoError::database("parse", format!("Invalid LocationId: {}", e)))?;
                    wrldbldr_domain::value_objects::AboutTarget::Location {
                        location_id: LocationId::from_uuid(id),
                    }
                }
                "Region" => {
                    let id = uuid::Uuid::parse_str(&target_id_str)
                        .map_err(|e| RepoError::database("parse", format!("Invalid RegionId: {}", e)))?;
                    wrldbldr_domain::value_objects::AboutTarget::Region {
                        region_id: RegionId::from_uuid(id),
                    }
                }
                "Item" => {
                    let id = uuid::Uuid::parse_str(&target_id_str)
                        .map_err(|e| RepoError::database("parse", format!("Invalid ItemId: {}", e)))?;
                    wrldbldr_domain::value_objects::AboutTarget::Item {
                        item_id: ItemId::from_uuid(id),
                    }
                }
                _ => {
                    return Err(RepoError::database(
                        "parse",
                        format!("Unknown target label: {}", target_label),
                    ));
                }
            };

            // Verify relationship type matches expected ABOUT_* type
            let expected_rel = target.edge_type();
            if rel_type != expected_rel {
                return Err(RepoError::database(
                    "parse",
                    format!(
                        "Relationship type mismatch: got {}, expected {} for target {}",
                        rel_type, expected_rel, target_label
                    ),
                ));
            }

            targets.push(target);
        }

        Ok(targets)
    }

    async fn mark_common_lore(
        &self,
        location_id: LocationId,
        lore_id: LoreId,
        chunk_ids: Option<Vec<LoreChunkId>>,
    ) -> Result<(), RepoError> {
        // Validate lore exists and chunk_ids belong to it (if provided)
        let lore = self.get(lore_id).await?.ok_or_else(|| {
            RepoError::not_found("Lore", lore_id.to_string())
        })?;

        // Validate location exists
        let check_location_q = query(
            "MATCH (l:Location {id: $location_id}) RETURN l.id AS location_id",
        )
        .param("location_id", location_id.to_string());

        let mut location_result = self
            .graph
            .execute(check_location_q)
            .await
            .map_err(|e| RepoError::database("mark_common_lore", e))?;

        if !matches!(location_result.next().await, Ok(Some(_))) {
            return Err(RepoError::not_found("Location", location_id.to_string()));
        }

        // If chunk_ids provided, validate they all belong to this lore
        if let Some(ref ids) = chunk_ids {
            let valid_chunk_ids: std::collections::HashSet<_> =
                lore.chunks().iter().map(|c| c.id).collect();
            let invalid_ids: Vec<_> = ids
                .iter()
                .filter(|id| !valid_chunk_ids.contains(id))
                .collect();

            if !invalid_ids.is_empty() {
                return Err(RepoError::ConstraintViolation(format!(
                    "Invalid chunk IDs for lore {}: {}",
                    lore_id,
                    invalid_ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        }

        // Serialize chunk_ids - normalize empty list to None for consistency
        let chunk_ids_to_store = chunk_ids.filter(|ids| !ids.is_empty());
        let chunk_ids_json = chunk_ids_to_store
            .as_ref()
            .map(|ids| serde_json::to_string(ids).map_err(|e| RepoError::Serialization(e.to_string())))
            .transpose()?;

        let q = query(
            "MATCH (l:Location {id: $location_id})
            MATCH (lo:Lore {id: $lore_id})
            MERGE (l)-[r:COMMON_LORE]->(lo)
            SET r.chunk_ids = $chunk_ids
            RETURN l.id AS location_id",
        )
        .param("location_id", location_id.to_string())
        .param("lore_id", lore_id.to_string())
        .param("chunk_ids", chunk_ids_json); // Pass null when chunk_ids is None

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("mark_common_lore", e))?;

        tracing::debug!(
            "Marked lore {} as common at location {}",
            lore_id,
            location_id
        );
        Ok(())
    }

    async fn unmark_common_lore(
        &self,
        location_id: LocationId,
        lore_id: LoreId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (l:Location {id: $location_id})-[r:COMMON_LORE]->(lo:Lore {id: $lore_id})
            DELETE r
            RETURN count(r) AS deleted_count",
        )
        .param("location_id", location_id.to_string())
        .param("lore_id", lore_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("unmark_common_lore", e))?;

        tracing::debug!(
            "Unmarked lore {} as common at location {}",
            lore_id,
            location_id
        );
        Ok(())
    }

    async fn get_location_common_lore(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<(LoreId, Option<Vec<LoreChunkId>>)>, RepoError> {
        let q = query(
            "MATCH (l:Location {id: $location_id})-[r:COMMON_LORE]->(lo:Lore)
            RETURN lo.id AS lore_id, r.chunk_ids AS chunk_ids
        ",
        )
        .param("location_id", location_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("get_location_common_lore", e))?;

        let mut common_lore = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let lore_id_str: String = row
                .get("lore_id")
                .map_err(|e| RepoError::database("query", format!("Failed to get lore_id: {}", e)))?;

            let lore_id = LoreId::from_uuid(
                uuid::Uuid::parse_str(&lore_id_str)
                    .map_err(|e| RepoError::database("parse", format!("Invalid LoreId: {}", e)))?,
            );

            let chunk_ids: Option<Vec<LoreChunkId>> = match row.get_optional_string("chunk_ids") {
                Some(json_str) if json_str == "null" || json_str.is_empty() => None,
                Some(json_str) => {
                    let ids: Vec<String> = serde_json::from_str::<Vec<String>>(&json_str)
                        .map_err(|e| RepoError::database("parse", format!("Invalid chunk_ids JSON: {}", e)))?;
                    Some(
                        ids.into_iter()
                            .map(|s| {
                                let uuid = uuid::Uuid::parse_str(&s).map_err(|e| {
                                    RepoError::database("parse", format!("Invalid LoreChunkId: {}", e))
                                })?;
                                Ok(LoreChunkId::from_uuid(uuid))
                            })
                            .collect::<Result<Vec<_>, RepoError>>()?
                    )
                }
                None => None,
            };

            common_lore.push((lore_id, chunk_ids));
        }

        Ok(common_lore)
    }
}

// =============================================================================
// Integration Tests (P1: Lore linking + common lore)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use wrldbldr_domain::{LoreCategory, LocationId, ItemId, WorldId, CharacterId};

    // Global mutex to serialize Neo4j integration tests and prevent
    // concurrent DB cleanup races (each test wipes the database).
    static TEST_MUTEX: Mutex<()> = Mutex::const_new(());

    fn create_test_lore(world_id: WorldId) -> Lore {
        Lore::new(world_id, "Test Lore", LoreCategory::Common, chrono::Utc::now())
    }

    async fn create_test_world(graph: &Neo4jGraph, world_id: WorldId) {
        let q = query("CREATE (w:World {id: $id}) RETURN w.id AS id")
            .param("id", world_id.to_string());
        graph.run(q).await.expect("Failed to create test world");
    }

    async fn create_test_character(graph: &Neo4jGraph, character_id: CharacterId) {
        let q = query("CREATE (c:Character {id: $id}) RETURN c.id AS id")
            .param("id", character_id.to_string());
        graph.run(q).await.expect("Failed to create test character");
    }

    async fn create_test_location(graph: &Neo4jGraph, location_id: LocationId) {
        let q = query("CREATE (l:Location {id: $id}) RETURN l.id AS id")
            .param("id", location_id.to_string());
        graph.run(q).await.expect("Failed to create test location");
    }

    #[tokio::test]
    async fn link_lore_about_character() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let character_id = CharacterId::new();

        // Create prerequisite nodes
        create_test_world(&graph, world_id).await;
        create_test_character(&graph, character_id).await;

        // Save lore
        repo.save(&lore).await.expect("should save lore");

        // Link lore about character
        let target = wrldbldr_domain::value_objects::AboutTarget::Character {
            character_id,
        };
        repo.link_lore_about(lore.id(), target).await.expect("should link lore about character");

        // Verify link exists
        let targets = repo.get_lore_about_targets(lore.id()).await.expect("should get lore about targets");
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            wrldbldr_domain::value_objects::AboutTarget::Character { character_id: id } => {
                assert_eq!(*id, character_id);
            }
            _ => panic!("Expected Character target"),
        }
    }

    #[tokio::test]
    async fn link_lore_about_location() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let location_id = LocationId::new();

        // Create prerequisite nodes
        create_test_world(&graph, world_id).await;
        create_test_location(&graph, location_id).await;

        // Save lore
        repo.save(&lore).await.expect("should save lore");

        // Link lore about location
        let target = wrldbldr_domain::value_objects::AboutTarget::Location {
            location_id,
        };
        repo.link_lore_about(lore.id(), target).await.expect("should link lore about location");

        // Verify link exists
        let targets = repo.get_lore_about_targets(lore.id()).await.expect("should get lore about targets");
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            wrldbldr_domain::value_objects::AboutTarget::Location { location_id: id } => {
                assert_eq!(*id, location_id);
            }
            _ => panic!("Expected Location target"),
        }
    }

    #[tokio::test]
    async fn unlink_lore_about() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let character_id = CharacterId::new();

        // Create prerequisite nodes
        create_test_world(&graph, world_id).await;
        create_test_character(&graph, character_id).await;

        // Save lore and link
        repo.save(&lore).await.expect("should save lore");
        let target = wrldbldr_domain::value_objects::AboutTarget::Character {
            character_id,
        };
        repo.link_lore_about(lore.id(), target.clone()).await.expect("should link lore about character");

        // Verify link exists
        let targets = repo.get_lore_about_targets(lore.id()).await.expect("should get lore about targets");
        assert_eq!(targets.len(), 1);

        // Unlink
        repo.unlink_lore_about(lore.id(), &target).await.expect("should unlink lore");

        // Verify link removed
        let targets = repo.get_lore_about_targets(lore.id()).await.expect("should get lore about targets");
        assert_eq!(targets.len(), 0);
    }

    #[tokio::test]
    async fn mark_common_lore() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let location_id = LocationId::new();

        // Create prerequisite nodes
        create_test_world(&graph, world_id).await;
        create_test_location(&graph, location_id).await;

        // Save lore
        repo.save(&lore).await.expect("should save lore");

        // Mark as common lore
        repo.mark_common_lore(location_id, lore.id(), None).await.expect("should mark common lore");

        // Verify common lore exists
        let common_lore = repo.get_location_common_lore(location_id).await.expect("should get location common lore");
        assert_eq!(common_lore.len(), 1);
        assert_eq!(common_lore[0].0, lore.id());
        assert!(common_lore[0].1.is_none()); // No chunk_ids means all chunks
    }

    #[tokio::test]
    async fn mark_common_lore_with_chunks() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let mut lore = create_test_lore(world_id);
        lore = lore.with_chunks(vec![
            wrldbldr_domain::LoreChunk::new("Chunk 1").with_order(0),
            wrldbldr_domain::LoreChunk::new("Chunk 2").with_order(1),
        ]);
        let location_id = LocationId::new();

        // Create prerequisite nodes
        create_test_world(&graph, world_id).await;
        create_test_location(&graph, location_id).await;

        // Save lore with chunks
        repo.save(&lore).await.expect("should save lore");

        // Mark specific chunks as common
        let common_chunk_ids = vec![lore.chunks()[0].id];
        repo.mark_common_lore(location_id, lore.id(), Some(common_chunk_ids.clone())).await.expect("should mark common lore with chunks");

        // Verify common lore with specific chunks
        let common_lore = repo.get_location_common_lore(location_id).await.expect("should get location common lore");
        assert_eq!(common_lore.len(), 1);
        assert_eq!(common_lore[0].0, lore.id());
        match &common_lore[0].1 {
            Some(ids) if ids.len() == 1 => {
                assert_eq!(ids[0], common_chunk_ids[0]);
            }
            _ => panic!("Expected specific chunk_ids"),
        }
    }

    #[tokio::test]
    async fn unmark_common_lore() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let location_id = LocationId::new();

        // Create prerequisite nodes
        create_test_world(&graph, world_id).await;
        create_test_location(&graph, location_id).await;

        // Save lore and mark as common
        repo.save(&lore).await.expect("should save lore");
        repo.mark_common_lore(location_id, lore.id(), None).await.expect("should mark common lore");

        // Verify common lore exists
        let common_lore = repo.get_location_common_lore(location_id).await.expect("should get location common lore");
        assert_eq!(common_lore.len(), 1);

        // Unmark
        repo.unmark_common_lore(location_id, lore.id()).await.expect("should unmark common lore");

        // Verify common lore removed
        let common_lore = repo.get_location_common_lore(location_id).await.expect("should get location common lore");
        assert_eq!(common_lore.len(), 0);
    }

    #[tokio::test]
    async fn get_location_common_lore_empty() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let location_id = LocationId::new();

        // Query common lore for location with no common lore
        let common_lore = repo.get_location_common_lore(location_id).await.expect("should get location common lore");
        assert_eq!(common_lore.len(), 0);
    }

    async fn setup_test_graph() -> (Neo4jGraph, crate::infrastructure::clock::FixedClock, tokio::sync::MutexGuard<'static, ()>) {
        use crate::infrastructure::neo4j::Neo4jGraph;
        use crate::e2e_tests::SharedNeo4jHarness;
        use neo4rs::ConfigBuilder;

        // Acquire global lock to serialize tests against shared Neo4j instance
        let _guard = TEST_MUTEX.lock().await;

        // If NEO4J_TEST_URI is set, use it directly (override mode)
        // Otherwise, use the shared harness (default)
        let inner_graph = if let Ok(uri) = std::env::var("NEO4J_TEST_URI") {
            let config = ConfigBuilder::new()
                .uri(uri.as_str())
                .user("neo4j")
                .password("password")
                .build()
                .expect("Failed to build config");
            neo4rs::Graph::connect(config).await.expect("Failed to connect")
        } else {
            let harness = SharedNeo4jHarness::shared()
                .await
                .expect("Failed to get shared Neo4j harness");
            harness
                .create_graph()
                .await
                .expect("Failed to create graph from harness")
        };

        let graph = Neo4jGraph::new(inner_graph);
        let clock = crate::infrastructure::clock::FixedClock(chrono::Utc::now());

        // Clean up test data
        let cleanup_q = query("MATCH (n) DETACH DELETE n");
        let _ = graph.run(cleanup_q).await;

        (graph, clock, _guard)
    }

    // ==========================================================================
    // New Tests for ABOUT_* filtering, NotFound handling, and chunk ID validation
    // ==========================================================================

    #[tokio::test]
    async fn get_lore_about_targets_filters_about_relationships_only() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let character_id = CharacterId::new();
        let _item_id = ItemId::new();

        // Create prerequisite nodes
        create_test_world(&graph, world_id).await;
        create_test_character(&graph, character_id).await;

        repo.save(&lore).await.expect("should save lore");

        // Create a KNOWS_LORE relationship (should NOT appear in get_lore_about_targets)
        let q = query(
            "MATCH (c:Character {id: $character_id})
            MATCH (l:Lore {id: $lore_id})
            MERGE (c)-[:KNOWS_LORE]->(l)
            RETURN l.id AS lore_id"
        )
        .param("character_id", character_id.to_string())
        .param("lore_id", lore.id().to_string());
        graph.run(q).await.expect("should run query");

        // Create ABOUT_CHARACTER relationship (should appear)
        let target = wrldbldr_domain::value_objects::AboutTarget::Character {
            character_id,
        };
        repo.link_lore_about(lore.id(), target).await.expect("should link lore about character");

        // Get targets - should only return ABOUT_* relationships
        let targets = repo.get_lore_about_targets(lore.id()).await.expect("should get lore about targets");
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            wrldbldr_domain::value_objects::AboutTarget::Character { character_id: id } => {
                assert_eq!(*id, character_id);
            }
            _ => panic!("Expected Character target"),
        }
    }

    #[tokio::test]
    async fn link_lore_about_fails_when_target_missing() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let location_id = LocationId::new();

        // Create prerequisite World before saving lore
        create_test_world(&graph, world_id).await;
        repo.save(&lore).await.expect("should save lore");

        // Try to link to non-existent location - should fail with NotFound
        let target = wrldbldr_domain::value_objects::AboutTarget::Location {
            location_id,
        };
        let result = repo.link_lore_about(lore.id(), target).await;

        assert!(result.is_err());
        match result.expect_err("should be error") {
            RepoError::NotFound { entity_type, .. } => {
                assert_eq!(entity_type, "Location");
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn mark_common_lore_fails_when_location_missing() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let location_id = LocationId::new();

        repo.save(&lore).await.expect("should save lore");

        // Try to mark as common at non-existent location - should fail with NotFound
        let result = repo.mark_common_lore(location_id, lore.id(), None).await;

        assert!(result.is_err());
        match result.expect_err("should be error") {
            RepoError::NotFound { entity_type, .. } => {
                assert_eq!(entity_type, "Location");
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn mark_common_lore_fails_when_chunk_ids_invalid() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let mut lore = create_test_lore(world_id);
        lore = lore.with_chunks(vec![
            wrldbldr_domain::LoreChunk::new("Chunk 1").with_order(0),
        ]);

        // Lore with only one chunk
        let _chunk_id = lore.chunks()[0].id;
        let other_chunk_id = LoreChunkId::new(); // Not part of this lore

        // Create prerequisite nodes
        create_test_world(&graph, world_id).await;
        create_test_location(&graph, location_id).await;

        repo.save(&lore).await.expect("should save lore");

        // Try to mark with invalid chunk ID - should fail with ConstraintViolation
        let result = repo.mark_common_lore(
            location_id,
            lore.id(),
            Some(vec![other_chunk_id]),
        ).await;

        assert!(result.is_err());
        match result.expect_err("should be error") {
            RepoError::ConstraintViolation { .. } => {
                // Expected
            }
            _ => panic!("Expected ConstraintViolation error"),
        }
    }

    #[tokio::test]
    async fn mark_common_lore_normalizes_empty_list_to_none() {
        let (graph, clock, _guard) = setup_test_graph().await;
        let repo = Neo4jLoreRepo::new(graph.clone(), Arc::new(clock));

        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let location_id = LocationId::new();

        // Create prerequisite nodes
        create_test_world(&graph, world_id).await;
        create_test_location(&graph, location_id).await;

        repo.save(&lore).await.expect("should save lore");

        // Mark with empty list - should normalize to None (all chunks)
        repo.mark_common_lore(location_id, lore.id(), Some(vec![])).await.expect("should mark common lore");

        // Verify it's stored as None
        let common_lore = repo.get_location_common_lore(location_id).await.expect("should get location common lore");
        assert_eq!(common_lore.len(), 1);
        assert_eq!(common_lore[0].1, None); // None means all chunks
    }
}
