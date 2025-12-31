use async_trait::async_trait;

use wrldbldr_domain::WorldId;

/// Outbound port for exporting a world snapshot as JSON.
///
/// This is a use-case-oriented contract (DTO/JSON), distinct from the domain-facing
/// `WorldServicePort` and `WorldExporterPort`.
#[async_trait]
pub trait WorldSnapshotJsonPort: Send + Sync {
    /// Export world snapshot
    async fn export_world_snapshot(&self, world_id: WorldId) -> Result<serde_json::Value, String>;
}
