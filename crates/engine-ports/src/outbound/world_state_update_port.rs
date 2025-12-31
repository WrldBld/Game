use wrldbldr_domain::WorldId;

use super::DirectorialContextData;

/// Outbound port for updating minimal world state needed by use cases.
///
/// This is intentionally separate from the domain world-state ports in
/// `outbound::world_state::*`.
pub trait WorldStateUpdatePort: Send + Sync {
    /// Set the current scene for a world
    fn set_current_scene(&self, world_id: &WorldId, scene_id: Option<String>);

    /// Set directorial context for a world
    fn set_directorial_context(&self, world_id: &WorldId, context: DirectorialContextData);
}
