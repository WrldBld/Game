use async_trait::async_trait;
use std::collections::HashMap;
use wrldbldr_domain::WorldId;

use crate::outbound::ResolvedPromptTemplate;

/// Cache for resolved prompt templates.
///
/// Resolution involves DB lookups + env + defaults, so caching is purely an infrastructure
/// concern. Implementations should live in adapters.
#[async_trait]
pub trait PromptTemplateCachePort: Send + Sync {
    async fn get_global(&self, key: &str) -> Option<ResolvedPromptTemplate>;
    async fn set_global(&self, key: String, resolved: ResolvedPromptTemplate);
    async fn remove_global(&self, key: &str);
    async fn clear_global(&self);

    async fn get_for_world(&self, world_id: WorldId, key: &str) -> Option<ResolvedPromptTemplate>;
    async fn set_for_world(&self, world_id: WorldId, key: String, resolved: ResolvedPromptTemplate);
    async fn remove_for_world(&self, world_id: WorldId, key: &str);
    async fn clear_world(&self);
    async fn remove_world(&self, world_id: WorldId);
}

#[derive(Clone, Debug, Default)]
pub struct PromptTemplateCacheSnapshot {
    pub global: HashMap<String, ResolvedPromptTemplate>,
    pub per_world: HashMap<WorldId, HashMap<String, ResolvedPromptTemplate>>,
}
