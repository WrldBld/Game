use async_trait::async_trait;
use wrldbldr_domain::value_objects::ChallengeOutcomeData;
use wrldbldr_domain::WorldId;

/// In-memory pending store for challenge outcome approvals.
///
/// This is operational state (cache of active approvals) and must not require app-layer
/// concurrency primitives.
#[async_trait]
pub trait ChallengeOutcomePendingPort: Send + Sync {
    async fn insert(&self, item: ChallengeOutcomeData);
    async fn get(&self, resolution_id: &str) -> Option<ChallengeOutcomeData>;
    async fn remove(&self, resolution_id: &str);

    async fn list_for_world(&self, world_id: WorldId) -> Vec<ChallengeOutcomeData>;

    async fn set_generating_suggestions(&self, resolution_id: &str, generating: bool);
    async fn set_suggestions(&self, resolution_id: &str, suggestions: Option<Vec<String>>);
}
