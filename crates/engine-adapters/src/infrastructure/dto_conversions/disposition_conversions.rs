//! Disposition-related DTO conversions

use wrldbldr_domain::value_objects::NpcDispositionState;
use wrldbldr_protocol::NpcDispositionStateDto;

/// Convert NpcDispositionState to NpcDispositionStateDto
pub fn npc_disposition_to_dto(state: &NpcDispositionState) -> NpcDispositionStateDto {
    NpcDispositionStateDto {
        npc_id: state.npc_id.to_uuid(),
        pc_id: state.pc_id.to_uuid(),
        disposition: state.disposition,
        relationship: state.relationship,
        sentiment: state.sentiment,
        updated_at: state.updated_at.to_rfc3339(),
        disposition_reason: state.disposition_reason.clone(),
        relationship_points: state.relationship_points,
    }
}
