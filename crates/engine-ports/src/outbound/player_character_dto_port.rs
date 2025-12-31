use async_trait::async_trait;

use wrldbldr_domain::PlayerCharacterId;

use super::PcData;

/// Outbound port for fetching player-character data in use-case DTO form.
///
/// This is distinct from the domain-facing `PlayerCharacterServicePort`.
#[async_trait]
pub trait PlayerCharacterDtoPort: Send + Sync {
    /// Get PC by ID
    async fn get_pc(&self, pc_id: PlayerCharacterId) -> Result<Option<PcData>, String>;
}
