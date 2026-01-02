use async_trait::async_trait;
use std::collections::HashMap;

use wrldbldr_domain::entities::GenerationBatch;
use wrldbldr_domain::{BatchId, WorldId};

/// Active generation batch state tracked in-process.
///
/// This is runtime state and must live behind an adapter-managed concurrency primitive.
#[derive(Clone, Debug)]
pub struct ActiveGenerationBatch {
    pub batch: GenerationBatch,
    pub prompt_ids: Vec<String>,
}

#[async_trait]
pub trait ActiveGenerationBatchesPort: Send + Sync {
    async fn len(&self) -> usize;

    async fn insert(&self, batch_id: BatchId, tracker: ActiveGenerationBatch);
    async fn get(&self, batch_id: BatchId) -> Option<ActiveGenerationBatch>;
    async fn update_prompt_ids(&self, batch_id: BatchId, prompt_ids: Vec<String>);
    async fn remove(&self, batch_id: BatchId);

    async fn list_for_world(&self, world_id: WorldId) -> Vec<ActiveGenerationBatch>;
}

#[derive(Clone, Debug, Default)]
pub struct ActiveGenerationBatchesSnapshot {
    pub by_id: HashMap<BatchId, ActiveGenerationBatch>,
}
