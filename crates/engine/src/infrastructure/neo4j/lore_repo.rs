//! Neo4j lore repository implementation.
//!
//! Lore chunks are stored as separate `:LoreChunk` nodes with a relationship
//! `(Lore)-[:HAS_CHUNK]->(LoreChunk)`. A unique constraint on `lore_order_key`
//! (composite of lore_id + order) enforces order uniqueness at the database level.

use std::sync::Arc;

use async_trait::async_trait;
use neo4rs::{query, Row};
use crate::infrastructure::neo4j::Neo4jGraph;
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{ClockPort, LoreRepo, RepoError};

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
        let node: neo4rs::Node = row.get("c").map_err(|e| RepoError::database("query", e))?;

        let id: LoreChunkId =
            parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
        let order: i64 = node
            .get("order")
            .map_err(|e| RepoError::database("query", e))?;
        let content: String = node
            .get("content")
            .map_err(|e| RepoError::database("query", e))?;
        let title: Option<String> = node.get_optional_string("title");
        let discovery_hint: Option<String> = node.get_optional_string("discovery_hint");

        Ok(LoreChunk {
            id,
            order: order as u32,
            title,
            content,
            discovery_hint,
        })
    }

    fn row_to_lore_without_chunks(&self, row: Row) -> Result<Lore, RepoError> {
        let node: neo4rs::Node = row.get("l").map_err(|e| RepoError::database("query", e))?;
        let fallback = self.clock.now();

        let id: LoreId =
            parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
        let world_id: WorldId =
            parse_typed_id(&node, "world_id").map_err(|e| RepoError::database("query", e))?;
        let title: String = node
            .get("title")
            .map_err(|e| RepoError::database("query", e))?;
        let summary: String = node.get_string_or("summary", "");
        let category_str: String = node.get_string_or("category", "common");
        let category: LoreCategory = category_str.parse().unwrap_or(LoreCategory::Common);
        let is_common_knowledge: bool = node.get_bool_or("is_common_knowledge", false);
        let created_at = node.get_datetime_or("created_at", fallback);
        let updated_at = node.get_datetime_or("updated_at", fallback);

        // Parse tags from JSON
        let tags: Vec<String> = node
            .get_optional_string("tags")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Ok(Lore {
            id,
            world_id,
            title,
            summary,
            category,
            chunks: Vec::new(), // Will be populated separately
            is_common_knowledge,
            tags,
            created_at,
            updated_at,
        })
    }

    async fn row_to_lore(&self, row: Row) -> Result<Lore, RepoError> {
        let mut lore = self.row_to_lore_without_chunks(row)?;
        lore.chunks = self.fetch_chunks(lore.id).await?;
        Ok(lore)
    }

    fn row_to_knowledge(&self, row: Row) -> Result<LoreKnowledge, RepoError> {
        // Get the relationship properties
        let lore_id_str: String = row
            .get("lore_id")
            .map_err(|e| RepoError::database("query", e))?;
        let character_id_str: String = row
            .get("character_id")
            .map_err(|e| RepoError::database("query", e))?;
        let known_chunk_ids_json: String = row
            .get::<String>("known_chunk_ids")
            .unwrap_or_else(|_| "[]".to_string());
        let discovery_source_json: String = row
            .get("discovery_source")
            .map_err(|e| RepoError::database("query", e))?;
        let discovered_at_str: String = row
            .get("discovered_at")
            .map_err(|e| RepoError::database("query", e))?;
        let notes: Option<String> = row.get("notes").ok();

        let lore_id = LoreId::from_uuid(
            uuid::Uuid::parse_str(&lore_id_str).map_err(|e| RepoError::database("query", e))?,
        );
        let character_id = CharacterId::from_uuid(
            uuid::Uuid::parse_str(&character_id_str)
                .map_err(|e| RepoError::database("query", e))?,
        );
        let known_chunk_ids: Vec<LoreChunkId> = serde_json::from_str(&known_chunk_ids_json)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let discovery_source: LoreDiscoverySource = serde_json::from_str(&discovery_source_json)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;
        let discovered_at = chrono::DateTime::parse_from_rfc3339(&discovered_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| self.clock.now());

        Ok(LoreKnowledge {
            lore_id,
            character_id,
            known_chunk_ids,
            discovery_source,
            discovered_at,
            notes,
        })
    }

    /// Create composite key for chunk order uniqueness constraint.
    fn make_lore_order_key(lore_id: &LoreId, order: u32) -> String {
        format!("{}_{}", lore_id, order)
    }
}

#[async_trait]
impl LoreRepo for Neo4jLoreRepo {
    async fn get(&self, id: LoreId) -> Result<Option<Lore>, RepoError> {
        let q = query("MATCH (l:Lore {id: $id}) RETURN l").param("id", id.to_string());

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
        let tags_json = serde_json::to_string(&lore.tags)
            .map_err(|e| RepoError::Serialization(e.to_string()))?;

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
        .param("id", lore.id.to_string())
        .param("world_id", lore.world_id.to_string())
        .param("title", lore.title.clone())
        .param("summary", lore.summary.clone())
        .param("category", lore.category.to_string())
        .param("is_common_knowledge", lore.is_common_knowledge)
        .param("tags", tags_json)
        .param("created_at", lore.created_at.to_rfc3339())
        .param("updated_at", lore.updated_at.to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Get existing chunk IDs to determine what to delete
        let existing_chunks = self.fetch_chunks(lore.id).await?;
        let existing_ids: std::collections::HashSet<_> =
            existing_chunks.iter().map(|c| c.id).collect();
        let new_ids: std::collections::HashSet<_> = lore.chunks.iter().map(|c| c.id).collect();

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
        for chunk in &lore.chunks {
            let lore_order_key = Self::make_lore_order_key(&lore.id, chunk.order);

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
            .param("lore_id", lore.id.to_string())
            .param("chunk_id", chunk.id.to_string())
            .param("order", chunk.order as i64)
            .param("lore_order_key", lore_order_key)
            .param("title", chunk.title.clone().unwrap_or_default())
            .param("content", chunk.content.clone())
            .param(
                "discovery_hint",
                chunk.discovery_hint.clone().unwrap_or_default(),
            );

            self.graph.run(chunk_q).await.map_err(|e| {
                // Check for constraint violation
                let msg = e.to_string();
                if msg.contains("already exists") || msg.contains("ConstraintValidation") {
                    RepoError::ConstraintViolation(format!(
                        "Duplicate chunk order {} for lore {}",
                        chunk.order, lore.id
                    ))
                } else {
                    RepoError::database("save_lore", msg)
                }
            })?;
        }

        tracing::debug!(
            "Saved lore: {} with {} chunks",
            lore.title,
            lore.chunks.len()
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
        let q = query(
            "MATCH (l:Lore {world_id: $world_id})
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
            // Tags in JSON are stored as `["tag1", "tag2"]`, so we search for `"tag"`
            q = q.param(&format!("tag{}", i), format!("\"{}\"", tag));
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

        let q = query(
            "MATCH (c:Character {id: $character_id})
            MATCH (l:Lore {id: $lore_id})
            MERGE (c)-[k:KNOWS_LORE]->(l)
            SET k.known_chunk_ids = $known_chunk_ids,
                k.discovery_source = $discovery_source,
                k.discovered_at = $discovered_at,
                k.notes = $notes
            RETURN c.id as character_id",
        )
        .param("character_id", knowledge.character_id.to_string())
        .param("lore_id", knowledge.lore_id.to_string())
        .param("known_chunk_ids", known_chunk_ids_json)
        .param("discovery_source", discovery_source_json)
        .param("discovered_at", knowledge.discovered_at.to_rfc3339())
        .param("notes", knowledge.notes.clone().unwrap_or_default());

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
        let q = query(
            "MATCH (c:Character {id: $character_id})-[k:KNOWS_LORE]->(l:Lore)
            RETURN l.id as lore_id, c.id as character_id,
                   k.known_chunk_ids as known_chunk_ids,
                   k.discovery_source as discovery_source,
                   k.discovered_at as discovered_at,
                   k.notes as notes",
        )
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
        let q = query(
            "MATCH (c:Character)-[k:KNOWS_LORE]->(l:Lore {id: $lore_id})
            RETURN l.id as lore_id, c.id as character_id,
                   k.known_chunk_ids as known_chunk_ids,
                   k.discovery_source as discovery_source,
                   k.discovered_at as discovered_at,
                   k.notes as notes",
        )
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
        let q = query(
            "MATCH (c:Character {id: $character_id})-[k:KNOWS_LORE]->(l:Lore {id: $lore_id})
            RETURN l.id as lore_id, c.id as character_id,
                   k.known_chunk_ids as known_chunk_ids,
                   k.discovery_source as discovery_source,
                   k.discovered_at as discovered_at,
                   k.notes as notes",
        )
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
        let fetch_q = query(
            "MATCH (c:Character {id: $character_id})-[k:KNOWS_LORE]->(l:Lore {id: $lore_id})
            RETURN k.known_chunk_ids as known_chunk_ids",
        )
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
            let json_str: String = row
                .get("known_chunk_ids")
                .unwrap_or_else(|_| "[]".to_string());
            serde_json::from_str(&json_str).unwrap_or_default()
        } else {
            return Err(RepoError::not_found("Entity", "unknown"));
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
        let fetch_q = query(
            "MATCH (c:Character {id: $character_id})-[k:KNOWS_LORE]->(l:Lore {id: $lore_id})
            RETURN k.known_chunk_ids as known_chunk_ids",
        )
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
            let json_str: String = row
                .get("known_chunk_ids")
                .unwrap_or_else(|_| "[]".to_string());
            serde_json::from_str(&json_str).unwrap_or_default()
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

        let update_q = query(
            "MATCH (c:Character {id: $character_id})-[k:KNOWS_LORE]->(l:Lore {id: $lore_id})
            SET k.known_chunk_ids = $known_chunk_ids
            RETURN c.id as character_id",
        )
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
}