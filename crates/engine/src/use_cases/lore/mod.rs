//! Lore use cases.

use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::entities::Lore;
use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::{
    CharacterId, LoreCategory, LoreChunkId, LoreDiscoverySource, LoreId, LoreKnowledge, WorldId,
};
use wrldbldr_protocol::requests::{
    CreateLoreChunkData, CreateLoreData, UpdateLoreChunkData, UpdateLoreData,
};
use wrldbldr_protocol::types::LoreDiscoverySourceData;

/// Container for lore use cases.
pub struct LoreUseCases {
    pub ops: Arc<LoreOps>,
}

impl LoreUseCases {
    pub fn new(ops: Arc<LoreOps>) -> Self {
        Self { ops }
    }
}

/// Lore operations.
pub struct LoreOps {
    lore: Arc<Lore>,
}

impl LoreOps {
    pub fn new(lore: Arc<Lore>) -> Self {
        Self { lore }
    }

    pub async fn list(&self, world_id: WorldId) -> Result<Vec<Value>, LoreError> {
        let lore_list = self.lore.list_for_world(world_id).await?;
        Ok(lore_list.into_iter().map(lore_summary_to_json).collect())
    }

    pub async fn get(&self, lore_id: LoreId) -> Result<Option<Value>, LoreError> {
        let lore = self.lore.get(lore_id).await?;
        Ok(lore.map(lore_to_json))
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        data: CreateLoreData,
    ) -> Result<Value, LoreError> {
        let category = match data.category.as_deref() {
            Some(cat_str) => cat_str
                .parse::<LoreCategory>()
                .map_err(|e| LoreError::InvalidCategory(e))?,
            None => LoreCategory::Common,
        };

        let now = chrono::Utc::now();
        let mut lore = wrldbldr_domain::Lore::new(world_id, &data.title, category, now);

        if let Some(summary) = data.summary.as_ref() {
            lore = lore.with_summary(summary);
        }
        if let Some(tags) = data.tags.as_ref() {
            lore = lore.with_tags(tags.clone());
        }
        if data.is_common_knowledge.unwrap_or(false) {
            lore = lore.as_common_knowledge();
        }

        if let Some(chunks) = data.chunks.as_ref() {
            let mut domain_chunks = Vec::new();
            for (i, chunk_data) in chunks.iter().enumerate() {
                let mut chunk = wrldbldr_domain::LoreChunk::new(&chunk_data.content)
                    .with_order(chunk_data.order.unwrap_or(i as u32));
                if let Some(title) = chunk_data.title.as_ref() {
                    chunk = chunk.with_title(title);
                }
                if let Some(hint) = chunk_data.discovery_hint.as_ref() {
                    chunk = chunk.with_discovery_hint(hint);
                }
                domain_chunks.push(chunk);
            }
            lore = lore.with_chunks(domain_chunks);
        }

        self.lore.save(&lore).await?;

        Ok(serde_json::json!({
            "id": lore.id.to_string(),
            "title": lore.title,
        }))
    }

    pub async fn update(&self, lore_id: LoreId, data: UpdateLoreData) -> Result<Value, LoreError> {
        let mut lore = self.lore.get(lore_id).await?.ok_or(LoreError::NotFound)?;

        if let Some(title) = data.title.as_ref() {
            lore.title = title.clone();
        }
        if let Some(summary) = data.summary.as_ref() {
            lore.summary = summary.clone();
        }
        if let Some(category_str) = data.category.as_ref() {
            lore.category = category_str
                .parse::<LoreCategory>()
                .map_err(|e| LoreError::InvalidCategory(e))?;
        }
        if let Some(tags) = data.tags.as_ref() {
            lore.tags = tags.clone();
        }
        if let Some(is_common) = data.is_common_knowledge {
            lore.is_common_knowledge = is_common;
        }
        lore.updated_at = chrono::Utc::now();

        self.lore.save(&lore).await?;

        Ok(serde_json::json!({
            "id": lore.id.to_string(),
            "title": lore.title,
        }))
    }

    pub async fn delete(&self, lore_id: LoreId) -> Result<Value, LoreError> {
        self.lore.delete(lore_id).await?;
        Ok(serde_json::json!({ "deleted": true }))
    }

    pub async fn add_chunk(
        &self,
        lore_id: LoreId,
        data: CreateLoreChunkData,
    ) -> Result<Value, LoreError> {
        let mut lore = self.lore.get(lore_id).await?.ok_or(LoreError::NotFound)?;

        // Determine the order: use provided order or auto-assign next sequential
        let order = match data.order {
            Some(provided_order) => {
                // Validate that the provided order is unique
                if lore.chunks.iter().any(|c| c.order == provided_order) {
                    return Err(LoreError::DuplicateChunkOrder(provided_order));
                }
                provided_order
            }
            None => {
                // Auto-assign next sequential order
                lore.chunks.iter().map(|c| c.order).max().map_or(0, |max| max + 1)
            }
        };

        let mut chunk = wrldbldr_domain::LoreChunk::new(&data.content).with_order(order);
        if let Some(title) = data.title.as_ref() {
            chunk = chunk.with_title(title);
        }
        if let Some(hint) = data.discovery_hint.as_ref() {
            chunk = chunk.with_discovery_hint(hint);
        }

        let chunk_id = chunk.id.to_string();
        lore.chunks.push(chunk);
        lore.updated_at = chrono::Utc::now();

        self.lore.save(&lore).await?;

        Ok(serde_json::json!({ "chunkId": chunk_id }))
    }

    pub async fn update_chunk(
        &self,
        world_id: WorldId,
        chunk_id: LoreChunkId,
        data: UpdateLoreChunkData,
    ) -> Result<Value, LoreError> {
        let mut lore = self
            .lore
            .list_for_world(world_id)
            .await?
            .into_iter()
            .find(|l| l.chunks.iter().any(|c| c.id == chunk_id))
            .ok_or(LoreError::ChunkNotFound)?;

        // Validate that new order doesn't conflict with other chunks
        if let Some(new_order) = data.order {
            let conflicts = lore
                .chunks
                .iter()
                .any(|c| c.id != chunk_id && c.order == new_order);
            if conflicts {
                return Err(LoreError::DuplicateChunkOrder(new_order));
            }
        }

        let chunk = lore
            .chunks
            .iter_mut()
            .find(|c| c.id == chunk_id)
            .ok_or(LoreError::ChunkNotFound)?;

        if let Some(title) = data.title.as_ref() {
            chunk.title = Some(title.clone());
        }
        if let Some(content) = data.content.as_ref() {
            chunk.content = content.clone();
        }
        if let Some(order) = data.order {
            chunk.order = order;
        }
        if let Some(hint) = data.discovery_hint.as_ref() {
            chunk.discovery_hint = Some(hint.clone());
        }

        lore.updated_at = chrono::Utc::now();
        self.lore.save(&lore).await?;

        Ok(serde_json::json!({
            "loreId": lore.id.to_string(),
            "chunkId": chunk_id.to_string(),
        }))
    }

    pub async fn delete_chunk(
        &self,
        world_id: WorldId,
        chunk_id: LoreChunkId,
    ) -> Result<Value, LoreError> {
        let mut lore = self
            .lore
            .list_for_world(world_id)
            .await?
            .into_iter()
            .find(|l| l.chunks.iter().any(|c| c.id == chunk_id))
            .ok_or(LoreError::ChunkNotFound)?;

        let before = lore.chunks.len();
        lore.chunks.retain(|c| c.id != chunk_id);
        if lore.chunks.len() == before {
            return Err(LoreError::ChunkNotFound);
        }

        // Re-index remaining chunks to maintain sequential order (0, 1, 2, ...)
        lore.chunks.sort_by_key(|c| c.order);
        for (i, chunk) in lore.chunks.iter_mut().enumerate() {
            chunk.order = i as u32;
        }

        lore.updated_at = chrono::Utc::now();
        self.lore.save(&lore).await?;

        Ok(serde_json::json!({
            "deleted": true,
            "loreId": lore.id.to_string(),
            "chunkId": chunk_id.to_string(),
        }))
    }

    pub async fn grant_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
        chunk_ids: Option<Vec<LoreChunkId>>,
        discovery_source: LoreDiscoverySourceData,
    ) -> Result<Value, LoreError> {
        // Validate that the lore exists and chunk IDs are valid
        let lore = self.lore.get(lore_id).await?.ok_or(LoreError::NotFound)?;

        // If chunk_ids are provided, validate they exist in the lore
        if let Some(ref ids) = chunk_ids {
            let valid_chunk_ids: std::collections::HashSet<_> =
                lore.chunks.iter().map(|c| c.id).collect();
            let invalid_ids: Vec<_> = ids
                .iter()
                .filter(|id| !valid_chunk_ids.contains(id))
                .map(|id| id.to_string())
                .collect();

            if !invalid_ids.is_empty() {
                return Err(LoreError::InvalidChunkIds(invalid_ids.join(", ")));
            }
        }

        let domain_source = lore_discovery_source(discovery_source)?;
        let now = chrono::Utc::now();
        let knowledge = if let Some(ids) = chunk_ids {
            LoreKnowledge::partial(lore_id, character_id, ids, domain_source, now)
        } else {
            LoreKnowledge::full(lore_id, character_id, domain_source, now)
        };

        self.lore.grant_knowledge(&knowledge).await?;
        Ok(serde_json::json!({ "granted": true }))
    }

    pub async fn revoke_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
        chunk_ids: Option<Vec<LoreChunkId>>,
    ) -> Result<Value, LoreError> {
        match chunk_ids {
            Some(ids) if !ids.is_empty() => {
                // Validate that the lore exists and chunk IDs are valid
                let lore = self.lore.get(lore_id).await?.ok_or(LoreError::NotFound)?;

                let valid_chunk_ids: std::collections::HashSet<_> =
                    lore.chunks.iter().map(|c| c.id).collect();
                let invalid_ids: Vec<_> = ids
                    .iter()
                    .filter(|id| !valid_chunk_ids.contains(id))
                    .map(|id| id.to_string())
                    .collect();

                if !invalid_ids.is_empty() {
                    return Err(LoreError::InvalidChunkIds(invalid_ids.join(", ")));
                }

                // Partial revocation - remove specific chunks
                let fully_revoked = self
                    .lore
                    .remove_chunks_from_knowledge(character_id, lore_id, &ids)
                    .await?;

                Ok(serde_json::json!({
                    "revoked": true,
                    "partial": !fully_revoked,
                    "chunksRemoved": ids.len(),
                    "relationshipDeleted": fully_revoked,
                }))
            }
            _ => {
                // Full revocation - remove entire knowledge relationship
                self.lore.revoke_knowledge(character_id, lore_id).await?;
                Ok(serde_json::json!({
                    "revoked": true,
                    "partial": false,
                    "relationshipDeleted": true,
                }))
            }
        }
    }

    pub async fn get_character_lore(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<Value>, LoreError> {
        let knowledge_list = self.lore.get_character_knowledge(character_id).await?;
        Ok(knowledge_list
            .into_iter()
            .map(|k| {
                serde_json::json!({
                    "loreId": k.lore_id.to_string(),
                    "characterId": k.character_id.to_string(),
                    "knownChunkIds": k
                        .known_chunk_ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>(),
                    "discoveredAt": k.discovered_at.to_rfc3339(),
                    "notes": k.notes,
                })
            })
            .collect())
    }

    pub async fn get_lore_knowers(&self, lore_id: LoreId) -> Result<Vec<Value>, LoreError> {
        let knowledge_list = self.lore.get_knowledge_for_lore(lore_id).await?;
        Ok(knowledge_list
            .into_iter()
            .map(|k| {
                serde_json::json!({
                    "characterId": k.character_id.to_string(),
                    "knownChunkIds": k
                        .known_chunk_ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>(),
                    "discoveredAt": k.discovered_at.to_rfc3339(),
                })
            })
            .collect())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoreError {
    #[error("Lore not found")]
    NotFound,
    #[error("Lore chunk not found")]
    ChunkNotFound,
    #[error("{0}")]
    InvalidCategory(String),
    #[error("Invalid chunk IDs: {0}")]
    InvalidChunkIds(String),
    #[error("Invalid NPC ID in conversation source: {0}")]
    InvalidNpcId(String),
    #[error("Duplicate chunk order: {0}")]
    DuplicateChunkOrder(u32),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

fn lore_discovery_source(data: LoreDiscoverySourceData) -> Result<LoreDiscoverySource, LoreError> {
    match data {
        LoreDiscoverySourceData::ReadBook { book_name } => {
            Ok(LoreDiscoverySource::ReadBook { book_name })
        }
        LoreDiscoverySourceData::Conversation { npc_id, npc_name } => {
            let npc_uuid = Uuid::parse_str(&npc_id)
                .map(CharacterId::from_uuid)
                .map_err(|_| LoreError::InvalidNpcId(npc_id))?;
            Ok(LoreDiscoverySource::Conversation {
                npc_id: npc_uuid,
                npc_name,
            })
        }
        LoreDiscoverySourceData::Investigation => Ok(LoreDiscoverySource::Investigation),
        LoreDiscoverySourceData::DmGranted { reason } => {
            Ok(LoreDiscoverySource::DmGranted { reason })
        }
        LoreDiscoverySourceData::CommonKnowledge => Ok(LoreDiscoverySource::CommonKnowledge),
        LoreDiscoverySourceData::LlmDiscovered { context } => {
            Ok(LoreDiscoverySource::LlmDiscovered { context })
        }
        LoreDiscoverySourceData::Unknown => Ok(LoreDiscoverySource::DmGranted {
            reason: Some("Unknown source type".to_string()),
        }),
    }
}

fn lore_summary_to_json(lore: wrldbldr_domain::Lore) -> Value {
    serde_json::json!({
        "id": lore.id.to_string(),
        "worldId": lore.world_id.to_string(),
        "title": lore.title,
        "summary": lore.summary,
        "category": format!("{}", lore.category),
        "isCommonKnowledge": lore.is_common_knowledge,
        "tags": lore.tags,
        "chunkCount": lore.chunks.len(),
        "createdAt": lore.created_at.to_rfc3339(),
        "updatedAt": lore.updated_at.to_rfc3339(),
    })
}

fn lore_to_json(lore: wrldbldr_domain::Lore) -> Value {
    let chunks: Vec<Value> = lore
        .chunks
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id.to_string(),
                "order": c.order,
                "title": c.title,
                "content": c.content,
                "discoveryHint": c.discovery_hint,
            })
        })
        .collect();

    serde_json::json!({
        "id": lore.id.to_string(),
        "worldId": lore.world_id.to_string(),
        "title": lore.title,
        "summary": lore.summary,
        "category": format!("{}", lore.category),
        "isCommonKnowledge": lore.is_common_knowledge,
        "tags": lore.tags,
        "chunks": chunks,
        "createdAt": lore.created_at.to_rfc3339(),
        "updatedAt": lore.updated_at.to_rfc3339(),
    })
}
