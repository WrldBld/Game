//! Lore use cases.

use std::sync::Arc;

use serde::Serialize;
use uuid::Uuid;

use crate::infrastructure::ports::RepoError;
use crate::repositories::Lore;
use wrldbldr_domain::{
    CharacterId, LoreCategory, LoreChunkId, LoreDiscoverySource, LoreId, LoreKnowledge, WorldId,
};

// =============================================================================
// Domain Result Types
// =============================================================================

/// Result of creating a lore entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLoreResult {
    pub id: String,
    pub title: String,
}

/// Result of updating a lore entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLoreResult {
    pub id: String,
    pub title: String,
}

/// Result of deleting a lore entry.
#[derive(Debug, Clone, Serialize)]
pub struct DeleteLoreResult {
    pub deleted: bool,
}

/// Result of adding a chunk to lore.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddChunkResult {
    pub chunk_id: String,
}

/// Result of updating a chunk.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateChunkResult {
    pub lore_id: String,
    pub chunk_id: String,
}

/// Result of deleting a chunk.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteChunkResult {
    pub deleted: bool,
    pub lore_id: String,
    pub chunk_id: String,
}

/// Result of granting knowledge.
#[derive(Debug, Clone, Serialize)]
pub struct GrantKnowledgeResult {
    pub granted: bool,
}

/// Result of revoking knowledge.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeKnowledgeResult {
    pub revoked: bool,
    pub partial: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunks_removed: Option<usize>,
    pub relationship_deleted: bool,
}

/// Lore knowledge info for a character.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterLoreInfo {
    pub lore_id: String,
    pub character_id: String,
    pub known_chunk_ids: Vec<String>,
    pub discovered_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Info about a character who knows some lore.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreKnowerInfo {
    pub character_id: String,
    pub known_chunk_ids: Vec<String>,
    pub discovered_at: String,
}

/// Summary of a lore entry (for list views).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreSummary {
    pub id: String,
    pub world_id: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub is_common_knowledge: bool,
    pub tags: Vec<String>,
    pub chunk_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Detailed lore entry (for single item view).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreDetail {
    pub id: String,
    pub world_id: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub is_common_knowledge: bool,
    pub tags: Vec<String>,
    pub chunks: Vec<LoreChunkDetail>,
    pub created_at: String,
    pub updated_at: String,
}

/// Detail of a lore chunk.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreChunkDetail {
    pub id: String,
    pub order: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_hint: Option<String>,
}

// =============================================================================
// Domain Input Types
// =============================================================================

/// Input for creating a lore entry (domain representation).
#[derive(Debug, Clone, Default)]
pub struct CreateLoreInput {
    pub title: String,
    pub summary: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_common_knowledge: Option<bool>,
    pub chunks: Option<Vec<CreateLoreChunkInput>>,
}

/// Input for updating a lore entry (domain representation).
#[derive(Debug, Clone, Default)]
pub struct UpdateLoreInput {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_common_knowledge: Option<bool>,
}

/// Input for creating a lore chunk (domain representation).
#[derive(Debug, Clone)]
pub struct CreateLoreChunkInput {
    pub title: Option<String>,
    pub content: String,
    pub order: Option<u32>,
    pub discovery_hint: Option<String>,
}

/// Input for updating a lore chunk (domain representation).
#[derive(Debug, Clone, Default)]
pub struct UpdateLoreChunkInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub order: Option<u32>,
    pub discovery_hint: Option<String>,
}

/// Source of lore discovery (domain representation).
#[derive(Debug, Clone)]
pub enum LoreDiscoverySourceInput {
    ReadBook { book_name: String },
    Conversation { npc_id: String, npc_name: String },
    Investigation,
    DmGranted { reason: Option<String> },
    CommonKnowledge,
    LlmDiscovered { context: String },
    Unknown,
}

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

    pub async fn list(&self, world_id: WorldId) -> Result<Vec<LoreSummary>, LoreError> {
        let lore_list = self.lore.list_for_world(world_id).await?;
        Ok(lore_list.into_iter().map(lore_to_summary).collect())
    }

    pub async fn get(&self, lore_id: LoreId) -> Result<Option<LoreDetail>, LoreError> {
        let lore = self.lore.get(lore_id).await?;
        Ok(lore.map(lore_to_detail))
    }

    pub async fn create(
        &self,
        world_id: WorldId,
        input: CreateLoreInput,
    ) -> Result<CreateLoreResult, LoreError> {
        let category = match input.category.as_deref() {
            Some(cat_str) => cat_str
                .parse::<LoreCategory>()
                .map_err(|e| LoreError::InvalidCategory(e))?,
            None => LoreCategory::Common,
        };

        let now = chrono::Utc::now();
        let mut lore = wrldbldr_domain::Lore::new(world_id, &input.title, category, now);

        if let Some(summary) = input.summary.as_ref() {
            lore = lore.with_summary(summary);
        }
        if let Some(tags) = input.tags.as_ref() {
            lore = lore.with_tags(tags.clone());
        }
        if input.is_common_knowledge.unwrap_or(false) {
            lore = lore.as_common_knowledge();
        }

        if let Some(chunks) = input.chunks.as_ref() {
            let mut domain_chunks = Vec::new();
            let mut used_orders = std::collections::HashSet::new();
            let mut next_auto_order = 0u32;

            for chunk_data in chunks.iter() {
                // Determine order: use provided order or auto-assign next available from 0
                let order = match chunk_data.order {
                    Some(provided_order) => {
                        // Validate that the provided order is unique
                        if used_orders.contains(&provided_order) {
                            return Err(LoreError::DuplicateChunkOrder(provided_order));
                        }
                        provided_order
                    }
                    None => {
                        // Auto-assign: find the first available order starting from 0
                        while used_orders.contains(&next_auto_order) {
                            next_auto_order += 1;
                        }
                        let assigned = next_auto_order;
                        next_auto_order += 1; // Advance for next auto-assign
                        assigned
                    }
                };
                used_orders.insert(order);

                let mut chunk =
                    wrldbldr_domain::LoreChunk::new(&chunk_data.content).with_order(order);
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

        Ok(CreateLoreResult {
            id: lore.id.to_string(),
            title: lore.title,
        })
    }

    pub async fn update(
        &self,
        lore_id: LoreId,
        input: UpdateLoreInput,
    ) -> Result<UpdateLoreResult, LoreError> {
        let mut lore = self.lore.get(lore_id).await?.ok_or(LoreError::NotFound)?;

        if let Some(title) = input.title.as_ref() {
            lore.title = title.clone();
        }
        if let Some(summary) = input.summary.as_ref() {
            lore.summary = summary.clone();
        }
        if let Some(category_str) = input.category.as_ref() {
            lore.category = category_str
                .parse::<LoreCategory>()
                .map_err(|e| LoreError::InvalidCategory(e))?;
        }
        if let Some(tags) = input.tags.as_ref() {
            lore.tags = tags.clone();
        }
        if let Some(is_common) = input.is_common_knowledge {
            lore.is_common_knowledge = is_common;
        }
        lore.updated_at = chrono::Utc::now();

        self.lore.save(&lore).await?;

        Ok(UpdateLoreResult {
            id: lore.id.to_string(),
            title: lore.title,
        })
    }

    pub async fn delete(&self, lore_id: LoreId) -> Result<DeleteLoreResult, LoreError> {
        self.lore.delete(lore_id).await?;
        Ok(DeleteLoreResult { deleted: true })
    }

    /// Add a chunk to existing lore.
    ///
    /// # Order validation
    /// - If `order` is provided, validates it's unique among existing chunks
    /// - If `order` is None, auto-assigns the next sequential value (max + 1)
    ///
    /// Order uniqueness is enforced at the database level via a unique constraint
    /// on the composite key (lore_id, order), preventing race conditions.
    pub async fn add_chunk(
        &self,
        lore_id: LoreId,
        input: CreateLoreChunkInput,
    ) -> Result<AddChunkResult, LoreError> {
        let mut lore = self.lore.get(lore_id).await?.ok_or(LoreError::NotFound)?;

        // Determine the order: use provided order or auto-assign next sequential
        let order = match input.order {
            Some(provided_order) => {
                // Validate that the provided order is unique
                if lore.chunks.iter().any(|c| c.order == provided_order) {
                    return Err(LoreError::DuplicateChunkOrder(provided_order));
                }
                provided_order
            }
            None => {
                // Auto-assign next sequential order
                lore.chunks
                    .iter()
                    .map(|c| c.order)
                    .max()
                    .map_or(0, |max| max + 1)
            }
        };

        let mut chunk = wrldbldr_domain::LoreChunk::new(&input.content).with_order(order);
        if let Some(title) = input.title.as_ref() {
            chunk = chunk.with_title(title);
        }
        if let Some(hint) = input.discovery_hint.as_ref() {
            chunk = chunk.with_discovery_hint(hint);
        }

        let chunk_id = chunk.id.to_string();
        lore.chunks.push(chunk);
        lore.updated_at = chrono::Utc::now();

        self.lore.save(&lore).await?;

        Ok(AddChunkResult { chunk_id })
    }

    pub async fn update_chunk(
        &self,
        world_id: WorldId,
        chunk_id: LoreChunkId,
        input: UpdateLoreChunkInput,
    ) -> Result<UpdateChunkResult, LoreError> {
        let mut lore = self
            .lore
            .list_for_world(world_id)
            .await?
            .into_iter()
            .find(|l| l.chunks.iter().any(|c| c.id == chunk_id))
            .ok_or(LoreError::ChunkNotFound)?;

        // Validate that new order doesn't conflict with other chunks
        if let Some(new_order) = input.order {
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

        if let Some(title) = input.title.as_ref() {
            chunk.title = Some(title.clone());
        }
        if let Some(content) = input.content.as_ref() {
            chunk.content = content.clone();
        }
        if let Some(order) = input.order {
            chunk.order = order;
        }
        if let Some(hint) = input.discovery_hint.as_ref() {
            chunk.discovery_hint = Some(hint.clone());
        }

        lore.updated_at = chrono::Utc::now();
        self.lore.save(&lore).await?;

        Ok(UpdateChunkResult {
            lore_id: lore.id.to_string(),
            chunk_id: chunk_id.to_string(),
        })
    }

    pub async fn delete_chunk(
        &self,
        world_id: WorldId,
        chunk_id: LoreChunkId,
    ) -> Result<DeleteChunkResult, LoreError> {
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

        Ok(DeleteChunkResult {
            deleted: true,
            lore_id: lore.id.to_string(),
            chunk_id: chunk_id.to_string(),
        })
    }

    pub async fn grant_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
        chunk_ids: Option<Vec<LoreChunkId>>,
        discovery_source: LoreDiscoverySourceInput,
    ) -> Result<GrantKnowledgeResult, LoreError> {
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
        Ok(GrantKnowledgeResult { granted: true })
    }

    pub async fn revoke_knowledge(
        &self,
        character_id: CharacterId,
        lore_id: LoreId,
        chunk_ids: Option<Vec<LoreChunkId>>,
    ) -> Result<RevokeKnowledgeResult, LoreError> {
        match chunk_ids {
            // Explicit empty list is an error - use None for full revocation
            Some(ref ids) if ids.is_empty() => {
                return Err(LoreError::EmptyChunkList);
            }
            Some(ids) => {
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

                Ok(RevokeKnowledgeResult {
                    revoked: true,
                    partial: !fully_revoked,
                    chunks_removed: Some(ids.len()),
                    relationship_deleted: fully_revoked,
                })
            }
            None => {
                // Full revocation - remove entire knowledge relationship
                self.lore.revoke_knowledge(character_id, lore_id).await?;
                Ok(RevokeKnowledgeResult {
                    revoked: true,
                    partial: false,
                    chunks_removed: None,
                    relationship_deleted: true,
                })
            }
        }
    }

    pub async fn get_character_lore(
        &self,
        character_id: CharacterId,
    ) -> Result<Vec<CharacterLoreInfo>, LoreError> {
        let knowledge_list = self.lore.get_character_knowledge(character_id).await?;
        Ok(knowledge_list
            .into_iter()
            .map(|k| CharacterLoreInfo {
                lore_id: k.lore_id.to_string(),
                character_id: k.character_id.to_string(),
                known_chunk_ids: k.known_chunk_ids.iter().map(|id| id.to_string()).collect(),
                discovered_at: k.discovered_at.to_rfc3339(),
                notes: k.notes,
            })
            .collect())
    }

    pub async fn get_lore_knowers(
        &self,
        lore_id: LoreId,
    ) -> Result<Vec<LoreKnowerInfo>, LoreError> {
        let knowledge_list = self.lore.get_knowledge_for_lore(lore_id).await?;
        Ok(knowledge_list
            .into_iter()
            .map(|k| LoreKnowerInfo {
                character_id: k.character_id.to_string(),
                known_chunk_ids: k.known_chunk_ids.iter().map(|id| id.to_string()).collect(),
                discovered_at: k.discovered_at.to_rfc3339(),
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
    #[error("Empty chunk list provided - omit chunkIds for full revocation")]
    EmptyChunkList,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

fn lore_discovery_source(
    input: LoreDiscoverySourceInput,
) -> Result<LoreDiscoverySource, LoreError> {
    match input {
        LoreDiscoverySourceInput::ReadBook { book_name } => {
            Ok(LoreDiscoverySource::ReadBook { book_name })
        }
        LoreDiscoverySourceInput::Conversation { npc_id, npc_name } => {
            let npc_uuid = Uuid::parse_str(&npc_id)
                .map(CharacterId::from_uuid)
                .map_err(|e| {
                    tracing::debug!(input = %npc_id, error = %e, "NPC ID parsing failed");
                    LoreError::InvalidNpcId(npc_id)
                })?;
            Ok(LoreDiscoverySource::Conversation {
                npc_id: npc_uuid,
                npc_name,
            })
        }
        LoreDiscoverySourceInput::Investigation => Ok(LoreDiscoverySource::Investigation),
        LoreDiscoverySourceInput::DmGranted { reason } => {
            Ok(LoreDiscoverySource::DmGranted { reason })
        }
        LoreDiscoverySourceInput::CommonKnowledge => Ok(LoreDiscoverySource::CommonKnowledge),
        LoreDiscoverySourceInput::LlmDiscovered { context } => {
            Ok(LoreDiscoverySource::LlmDiscovered { context })
        }
        LoreDiscoverySourceInput::Unknown => Ok(LoreDiscoverySource::DmGranted {
            reason: Some("Unknown source type".to_string()),
        }),
    }
}

fn lore_to_summary(lore: wrldbldr_domain::Lore) -> LoreSummary {
    LoreSummary {
        id: lore.id.to_string(),
        world_id: lore.world_id.to_string(),
        title: lore.title,
        summary: lore.summary,
        category: format!("{}", lore.category),
        is_common_knowledge: lore.is_common_knowledge,
        tags: lore.tags,
        chunk_count: lore.chunks.len(),
        created_at: lore.created_at.to_rfc3339(),
        updated_at: lore.updated_at.to_rfc3339(),
    }
}

fn lore_to_detail(lore: wrldbldr_domain::Lore) -> LoreDetail {
    let chunks: Vec<LoreChunkDetail> = lore
        .chunks
        .iter()
        .map(|c| LoreChunkDetail {
            id: c.id.to_string(),
            order: c.order,
            title: c.title.clone(),
            content: c.content.clone(),
            discovery_hint: c.discovery_hint.clone(),
        })
        .collect();

    LoreDetail {
        id: lore.id.to_string(),
        world_id: lore.world_id.to_string(),
        title: lore.title,
        summary: lore.summary,
        category: format!("{}", lore.category),
        is_common_knowledge: lore.is_common_knowledge,
        tags: lore.tags,
        chunks,
        created_at: lore.created_at.to_rfc3339(),
        updated_at: lore.updated_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::MockLoreRepo;
    use crate::repositories::Lore as LoreEntity;
    use std::sync::Arc;

    fn create_test_lore(world_id: WorldId) -> wrldbldr_domain::Lore {
        wrldbldr_domain::Lore::new(
            world_id,
            "Test Lore",
            LoreCategory::Common,
            chrono::Utc::now(),
        )
    }

    fn create_test_lore_with_chunks(
        world_id: WorldId,
        chunk_orders: &[u32],
    ) -> wrldbldr_domain::Lore {
        let mut lore = create_test_lore(world_id);
        for &order in chunk_orders {
            let chunk =
                wrldbldr_domain::LoreChunk::new(&format!("Content {}", order)).with_order(order);
            lore.chunks.push(chunk);
        }
        lore
    }

    // ==========================================================================
    // Chunk Order Validation Tests
    // ==========================================================================

    #[tokio::test]
    async fn add_chunk_rejects_duplicate_order() {
        let world_id = WorldId::new();
        let lore = create_test_lore_with_chunks(world_id, &[0, 1, 2]);
        let lore_id = lore.id;

        let mut mock_repo = MockLoreRepo::new();
        mock_repo
            .expect_get()
            .with(mockall::predicate::eq(lore_id))
            .returning(move |_| Ok(Some(lore.clone())));

        let lore_entity = Arc::new(LoreEntity::new(Arc::new(mock_repo)));
        let ops = LoreOps::new(lore_entity);

        // Try to add chunk with order 1 (already exists)
        let input = CreateLoreChunkInput {
            content: "New content".to_string(),
            title: None,
            order: Some(1),
            discovery_hint: None,
        };

        let result = ops.add_chunk(lore_id, input).await;
        assert!(matches!(result, Err(LoreError::DuplicateChunkOrder(1))));
    }

    #[tokio::test]
    async fn add_chunk_auto_assigns_next_order() {
        let world_id = WorldId::new();
        let lore = create_test_lore_with_chunks(world_id, &[0, 1, 2]);
        let lore_id = lore.id;

        let mut mock_repo = MockLoreRepo::new();
        mock_repo
            .expect_get()
            .with(mockall::predicate::eq(lore_id))
            .returning(move |_| Ok(Some(lore.clone())));
        mock_repo.expect_save().returning(|saved_lore| {
            // Verify the new chunk has order 3 (next after 0,1,2)
            let new_chunk = saved_lore.chunks.last().unwrap();
            assert_eq!(new_chunk.order, 3);
            Ok(())
        });

        let lore_entity = Arc::new(LoreEntity::new(Arc::new(mock_repo)));
        let ops = LoreOps::new(lore_entity);

        let input = CreateLoreChunkInput {
            content: "New content".to_string(),
            title: None,
            order: None, // Auto-assign
            discovery_hint: None,
        };

        let result = ops.add_chunk(lore_id, input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_chunk_rejects_conflicting_order() {
        let world_id = WorldId::new();
        let lore = create_test_lore_with_chunks(world_id, &[0, 1, 2]);
        let chunk_id = lore.chunks[2].id; // Chunk with order 2

        let mut mock_repo = MockLoreRepo::new();
        mock_repo
            .expect_list_for_world()
            .returning(move |_| Ok(vec![lore.clone()]));

        let lore_entity = Arc::new(LoreEntity::new(Arc::new(mock_repo)));
        let ops = LoreOps::new(lore_entity);

        // Try to update chunk 2's order to 0 (already taken)
        let input = UpdateLoreChunkInput {
            title: None,
            content: None,
            order: Some(0),
            discovery_hint: None,
        };

        let result = ops.update_chunk(world_id, chunk_id, input).await;
        assert!(matches!(result, Err(LoreError::DuplicateChunkOrder(0))));
    }

    #[tokio::test]
    async fn delete_chunk_reindexes_remaining() {
        let world_id = WorldId::new();
        let lore = create_test_lore_with_chunks(world_id, &[0, 1, 2]);
        let chunk_to_delete = lore.chunks[1].id; // Delete middle chunk (order 1)

        let mut mock_repo = MockLoreRepo::new();
        mock_repo
            .expect_list_for_world()
            .returning(move |_| Ok(vec![lore.clone()]));
        mock_repo.expect_save().returning(|saved_lore| {
            // After deleting chunk with order 1, remaining should be reindexed to 0, 1
            assert_eq!(saved_lore.chunks.len(), 2);
            assert_eq!(saved_lore.chunks[0].order, 0);
            assert_eq!(saved_lore.chunks[1].order, 1);
            Ok(())
        });

        let lore_entity = Arc::new(LoreEntity::new(Arc::new(mock_repo)));
        let ops = LoreOps::new(lore_entity);

        let result = ops.delete_chunk(world_id, chunk_to_delete).await;
        assert!(result.is_ok());
    }

    // ==========================================================================
    // Partial Revocation Tests
    // ==========================================================================

    #[tokio::test]
    async fn revoke_with_empty_chunk_list_fails() {
        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let lore_id = lore.id;
        let character_id = CharacterId::new();

        let mock_repo = MockLoreRepo::new();
        let lore_entity = Arc::new(LoreEntity::new(Arc::new(mock_repo)));
        let ops = LoreOps::new(lore_entity);

        // Empty chunk list should fail
        let result = ops
            .revoke_knowledge(character_id, lore_id, Some(vec![]))
            .await;

        assert!(matches!(result, Err(LoreError::EmptyChunkList)));
    }

    #[tokio::test]
    async fn revoke_with_none_does_full_revocation() {
        let world_id = WorldId::new();
        let lore = create_test_lore(world_id);
        let lore_id = lore.id;
        let character_id = CharacterId::new();

        let mut mock_repo = MockLoreRepo::new();
        mock_repo
            .expect_revoke_knowledge()
            .with(
                mockall::predicate::eq(character_id),
                mockall::predicate::eq(lore_id),
            )
            .returning(|_, _| Ok(()));

        let lore_entity = Arc::new(LoreEntity::new(Arc::new(mock_repo)));
        let ops = LoreOps::new(lore_entity);

        let result = ops.revoke_knowledge(character_id, lore_id, None).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.revoked);
        assert!(!result.partial);
        assert!(result.relationship_deleted);
    }

    #[tokio::test]
    async fn revoke_with_invalid_chunk_ids_fails() {
        let world_id = WorldId::new();
        let lore = create_test_lore_with_chunks(world_id, &[0, 1]);
        let lore_id = lore.id;
        let character_id = CharacterId::new();
        let invalid_chunk_id = LoreChunkId::new();

        let mut mock_repo = MockLoreRepo::new();
        mock_repo
            .expect_get()
            .with(mockall::predicate::eq(lore_id))
            .returning(move |_| Ok(Some(lore.clone())));

        let lore_entity = Arc::new(LoreEntity::new(Arc::new(mock_repo)));
        let ops = LoreOps::new(lore_entity);

        let result = ops
            .revoke_knowledge(character_id, lore_id, Some(vec![invalid_chunk_id]))
            .await;

        assert!(matches!(result, Err(LoreError::InvalidChunkIds(_))));
    }

    // ==========================================================================
    // Creation Order Validation Tests
    // ==========================================================================

    #[tokio::test]
    async fn create_with_duplicate_chunk_orders_fails() {
        let world_id = WorldId::new();

        let mock_repo = MockLoreRepo::new();
        let lore_entity = Arc::new(LoreEntity::new(Arc::new(mock_repo)));
        let ops = LoreOps::new(lore_entity);

        // Create with duplicate orders
        let data = CreateLoreInput {
            title: "Test".to_string(),
            summary: None,
            category: None,
            tags: None,
            is_common_knowledge: None,
            chunks: Some(vec![
                CreateLoreChunkInput {
                    content: "First".to_string(),
                    title: None,
                    order: Some(0),
                    discovery_hint: None,
                },
                CreateLoreChunkInput {
                    content: "Second".to_string(),
                    title: None,
                    order: Some(0), // Duplicate!
                    discovery_hint: None,
                },
            ]),
        };

        let result = ops.create(world_id, data).await;
        assert!(matches!(result, Err(LoreError::DuplicateChunkOrder(0))));
    }

    #[tokio::test]
    async fn create_auto_assigns_sequential_orders() {
        let world_id = WorldId::new();

        let mut mock_repo = MockLoreRepo::new();
        mock_repo.expect_save().returning(|saved_lore| {
            // Verify auto-assigned orders are sequential starting from 0
            assert_eq!(saved_lore.chunks.len(), 3);
            assert_eq!(saved_lore.chunks[0].order, 0);
            assert_eq!(saved_lore.chunks[1].order, 1);
            assert_eq!(saved_lore.chunks[2].order, 2);
            Ok(())
        });

        let lore_entity = Arc::new(LoreEntity::new(Arc::new(mock_repo)));
        let ops = LoreOps::new(lore_entity);

        let data = CreateLoreInput {
            title: "Test".to_string(),
            summary: None,
            category: None,
            tags: None,
            is_common_knowledge: None,
            chunks: Some(vec![
                CreateLoreChunkInput {
                    content: "First".to_string(),
                    title: None,
                    order: None, // Auto-assign
                    discovery_hint: None,
                },
                CreateLoreChunkInput {
                    content: "Second".to_string(),
                    title: None,
                    order: None, // Auto-assign
                    discovery_hint: None,
                },
                CreateLoreChunkInput {
                    content: "Third".to_string(),
                    title: None,
                    order: None, // Auto-assign
                    discovery_hint: None,
                },
            ]),
        };

        let result = ops.create(world_id, data).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_with_mixed_explicit_and_auto_orders() {
        let world_id = WorldId::new();

        let mut mock_repo = MockLoreRepo::new();
        mock_repo.expect_save().returning(|saved_lore| {
            // First chunk: explicit order 5
            // Second chunk: auto-assign (should get 0, first available)
            // Third chunk: explicit order 2
            assert_eq!(saved_lore.chunks.len(), 3);
            assert_eq!(saved_lore.chunks[0].order, 5);
            assert_eq!(saved_lore.chunks[1].order, 0);
            assert_eq!(saved_lore.chunks[2].order, 2);
            Ok(())
        });

        let lore_entity = Arc::new(LoreEntity::new(Arc::new(mock_repo)));
        let ops = LoreOps::new(lore_entity);

        let data = CreateLoreInput {
            title: "Test".to_string(),
            summary: None,
            category: None,
            tags: None,
            is_common_knowledge: None,
            chunks: Some(vec![
                CreateLoreChunkInput {
                    content: "First".to_string(),
                    title: None,
                    order: Some(5), // Explicit
                    discovery_hint: None,
                },
                CreateLoreChunkInput {
                    content: "Second".to_string(),
                    title: None,
                    order: None, // Auto-assign (should get 0)
                    discovery_hint: None,
                },
                CreateLoreChunkInput {
                    content: "Third".to_string(),
                    title: None,
                    order: Some(2), // Explicit
                    discovery_hint: None,
                },
            ]),
        };

        let result = ops.create(world_id, data).await;
        assert!(result.is_ok());
    }
}
