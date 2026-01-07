//! Lore use cases.

use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::entities::Lore;
use crate::infrastructure::ports::RepoError;
use wrldbldr_domain::{CharacterId, LoreCategory, LoreChunkId, LoreDiscoverySource, LoreId, LoreKnowledge, WorldId};
use wrldbldr_protocol::requests::{CreateLoreChunkData, CreateLoreData, UpdateLoreChunkData, UpdateLoreData};
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
        let category = data
            .category
            .as_deref()
            .unwrap_or("common")
            .parse::<LoreCategory>()
            .unwrap_or(LoreCategory::Common);

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
        let mut lore = self
            .lore
            .get(lore_id)
            .await?
            .ok_or(LoreError::NotFound)?;

        if let Some(title) = data.title.as_ref() {
            lore.title = title.clone();
        }
        if let Some(summary) = data.summary.as_ref() {
            lore.summary = summary.clone();
        }
        if let Some(category_str) = data.category.as_ref() {
            if let Ok(cat) = category_str.parse::<LoreCategory>() {
                lore.category = cat;
            }
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
        let mut lore = self
            .lore
            .get(lore_id)
            .await?
            .ok_or(LoreError::NotFound)?;

        let mut chunk = wrldbldr_domain::LoreChunk::new(&data.content)
            .with_order(data.order.unwrap_or(lore.chunks.len() as u32));
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
        let domain_source = lore_discovery_source(discovery_source);
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
    ) -> Result<Value, LoreError> {
        self.lore.revoke_knowledge(character_id, lore_id).await?;
        Ok(serde_json::json!({ "revoked": true }))
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
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

fn lore_discovery_source(data: LoreDiscoverySourceData) -> LoreDiscoverySource {
    match data {
        LoreDiscoverySourceData::ReadBook { book_name } => LoreDiscoverySource::ReadBook { book_name },
        LoreDiscoverySourceData::Conversation { npc_id, npc_name } => {
            let npc_uuid = Uuid::parse_str(&npc_id)
                .map(CharacterId::from_uuid)
                .unwrap_or_else(|_| CharacterId::new());
            LoreDiscoverySource::Conversation { npc_id: npc_uuid, npc_name }
        }
        LoreDiscoverySourceData::Investigation => LoreDiscoverySource::Investigation,
        LoreDiscoverySourceData::DmGranted { reason } => LoreDiscoverySource::DmGranted { reason },
        LoreDiscoverySourceData::CommonKnowledge => LoreDiscoverySource::CommonKnowledge,
        LoreDiscoverySourceData::LlmDiscovered { context } => {
            LoreDiscoverySource::LlmDiscovered { context }
        }
        LoreDiscoverySourceData::Unknown => LoreDiscoverySource::DmGranted {
            reason: Some("Unknown source type".to_string()),
        },
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
