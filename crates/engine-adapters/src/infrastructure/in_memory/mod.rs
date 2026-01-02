use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

use wrldbldr_domain::value_objects::{AppSettings, ChallengeOutcomeData};
use wrldbldr_domain::{BatchId, WorldId};
use wrldbldr_engine_ports::outbound::{
    ActiveGenerationBatch, ActiveGenerationBatchesPort, ChallengeOutcomePendingPort,
    PromptTemplateCachePort, ResolvedPromptTemplate, SettingsCachePort,
};

pub struct InMemorySettingsCache {
    global: RwLock<Option<AppSettings>>,
    per_world: RwLock<HashMap<WorldId, AppSettings>>,
}

impl InMemorySettingsCache {
    pub fn new() -> Self {
        Self {
            global: RwLock::new(None),
            per_world: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl SettingsCachePort for InMemorySettingsCache {
    async fn get_global(&self) -> Option<AppSettings> {
        self.global.read().await.clone()
    }

    async fn set_global(&self, settings: Option<AppSettings>) {
        *self.global.write().await = settings;
    }

    async fn get_world(&self, world_id: WorldId) -> Option<AppSettings> {
        self.per_world.read().await.get(&world_id).cloned()
    }

    async fn set_world(&self, world_id: WorldId, settings: AppSettings) {
        self.per_world.write().await.insert(world_id, settings);
    }

    async fn clear_world(&self) {
        self.per_world.write().await.clear();
    }

    async fn remove_world(&self, world_id: WorldId) {
        self.per_world.write().await.remove(&world_id);
    }
}

pub struct InMemoryPromptTemplateCache {
    global: RwLock<HashMap<String, ResolvedPromptTemplate>>,
    per_world: RwLock<HashMap<WorldId, HashMap<String, ResolvedPromptTemplate>>>,
}

impl InMemoryPromptTemplateCache {
    pub fn new() -> Self {
        Self {
            global: RwLock::new(HashMap::new()),
            per_world: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl PromptTemplateCachePort for InMemoryPromptTemplateCache {
    async fn get_global(&self, key: &str) -> Option<ResolvedPromptTemplate> {
        self.global.read().await.get(key).cloned()
    }

    async fn set_global(&self, key: String, resolved: ResolvedPromptTemplate) {
        self.global.write().await.insert(key, resolved);
    }

    async fn remove_global(&self, key: &str) {
        self.global.write().await.remove(key);
    }

    async fn clear_global(&self) {
        self.global.write().await.clear();
    }

    async fn get_for_world(&self, world_id: WorldId, key: &str) -> Option<ResolvedPromptTemplate> {
        self.per_world
            .read()
            .await
            .get(&world_id)
            .and_then(|m| m.get(key))
            .cloned()
    }

    async fn set_for_world(
        &self,
        world_id: WorldId,
        key: String,
        resolved: ResolvedPromptTemplate,
    ) {
        self.per_world
            .write()
            .await
            .entry(world_id)
            .or_insert_with(HashMap::new)
            .insert(key, resolved);
    }

    async fn remove_for_world(&self, world_id: WorldId, key: &str) {
        if let Some(world_templates) = self.per_world.write().await.get_mut(&world_id) {
            world_templates.remove(key);
        }
    }

    async fn clear_world(&self) {
        self.per_world.write().await.clear();
    }

    async fn remove_world(&self, world_id: WorldId) {
        self.per_world.write().await.remove(&world_id);
    }
}

pub struct InMemoryActiveGenerationBatches {
    by_id: RwLock<HashMap<BatchId, ActiveGenerationBatch>>,
}

impl InMemoryActiveGenerationBatches {
    pub fn new() -> Self {
        Self {
            by_id: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ActiveGenerationBatchesPort for InMemoryActiveGenerationBatches {
    async fn len(&self) -> usize {
        self.by_id.read().await.len()
    }

    async fn insert(&self, batch_id: BatchId, tracker: ActiveGenerationBatch) {
        self.by_id.write().await.insert(batch_id, tracker);
    }

    async fn get(&self, batch_id: BatchId) -> Option<ActiveGenerationBatch> {
        self.by_id.read().await.get(&batch_id).cloned()
    }

    async fn update_prompt_ids(&self, batch_id: BatchId, prompt_ids: Vec<String>) {
        if let Some(tracker) = self.by_id.write().await.get_mut(&batch_id) {
            tracker.prompt_ids = prompt_ids;
        }
    }

    async fn remove(&self, batch_id: BatchId) {
        self.by_id.write().await.remove(&batch_id);
    }

    async fn list_for_world(&self, world_id: WorldId) -> Vec<ActiveGenerationBatch> {
        self.by_id
            .read()
            .await
            .values()
            .filter(|t| t.batch.world_id == world_id)
            .cloned()
            .collect()
    }
}

pub struct InMemoryChallengeOutcomePendingStore {
    by_resolution_id: RwLock<HashMap<String, ChallengeOutcomeData>>,
}

impl InMemoryChallengeOutcomePendingStore {
    pub fn new() -> Self {
        Self {
            by_resolution_id: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ChallengeOutcomePendingPort for InMemoryChallengeOutcomePendingStore {
    async fn insert(&self, item: ChallengeOutcomeData) {
        self.by_resolution_id
            .write()
            .await
            .insert(item.resolution_id.clone(), item);
    }

    async fn get(&self, resolution_id: &str) -> Option<ChallengeOutcomeData> {
        self.by_resolution_id
            .read()
            .await
            .get(resolution_id)
            .cloned()
    }

    async fn remove(&self, resolution_id: &str) {
        self.by_resolution_id.write().await.remove(resolution_id);
    }

    async fn list_for_world(&self, world_id: WorldId) -> Vec<ChallengeOutcomeData> {
        self.by_resolution_id
            .read()
            .await
            .values()
            .filter(|item| item.world_id == world_id)
            .cloned()
            .collect()
    }

    async fn set_generating_suggestions(&self, resolution_id: &str, generating: bool) {
        if let Some(item) = self.by_resolution_id.write().await.get_mut(resolution_id) {
            item.is_generating_suggestions = generating;
        }
    }

    async fn set_suggestions(&self, resolution_id: &str, suggestions: Option<Vec<String>>) {
        if let Some(item) = self.by_resolution_id.write().await.get_mut(resolution_id) {
            item.suggestions = suggestions;
        }
    }
}
