use async_trait::async_trait;

use wrldbldr_domain::WorldId;

use super::DirectorialContextData;

/// Outbound port for loading directorial context in use-case DTO form.
///
/// This is distinct from the domain-facing `DirectorialContextRepositoryPort`.
#[async_trait]
pub trait DirectorialContextQueryPort: Send + Sync {
    /// Get directorial context
    async fn get(&self, world_id: &WorldId) -> Result<Option<DirectorialContextData>, String>;
}
