use async_trait::async_trait;

use wrldbldr_domain::WorldId;

use super::DirectorialContextData;

/// Outbound port for persisting directorial context in use-case DTO form.
///
/// This is intentionally distinct from the domain-facing
/// `DirectorialContextRepositoryPort`.
#[async_trait]
pub trait DirectorialContextDtoRepositoryPort: Send + Sync {
    /// Save directorial context
    async fn save(
        &self,
        world_id: &WorldId,
        context: &DirectorialContextData,
    ) -> Result<(), String>;
}
