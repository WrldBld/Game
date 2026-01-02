use async_trait::async_trait;
use std::collections::HashMap;
use wrldbldr_domain::value_objects::AppSettings;
use wrldbldr_domain::WorldId;

/// Cache for settings.
///
/// This is an infrastructure concern (performance), so the implementation should live
/// in adapters (e.g., in-memory cache). Application services depend on this port to
/// avoid embedding concurrency primitives in the app layer.
#[async_trait]
pub trait SettingsCachePort: Send + Sync {
    async fn get_global(&self) -> Option<AppSettings>;
    async fn set_global(&self, settings: Option<AppSettings>);

    async fn get_world(&self, world_id: WorldId) -> Option<AppSettings>;
    async fn set_world(&self, world_id: WorldId, settings: AppSettings);

    async fn clear_world(&self);
    async fn remove_world(&self, world_id: WorldId);
}

/// Snapshot type for debugging/testing adapters.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SettingsCacheSnapshot {
    pub global: Option<AppSettings>,
    pub per_world: HashMap<WorldId, AppSettings>,
}
