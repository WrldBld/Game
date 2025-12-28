//! Data Transfer Objects (DTOs)
//!
//! Wire-format types for serialization/deserialization that are shared
//! between engine and player. These types use raw UUIDs and primitive types
//! for transport, rather than domain ID types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use wrldbldr_domain::value_objects::{DispositionLevel, NpcDispositionState, RelationshipLevel};
use wrldbldr_domain::{CharacterId, PlayerCharacterId};

// =============================================================================
// NPC Disposition DTOs
// =============================================================================

/// Wire-format disposition state for protocol serialization
///
/// This DTO is used to transfer NPC disposition state over WebSocket/HTTP.
/// It uses raw UUIDs instead of domain ID types for serialization compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcDispositionStateDto {
    /// The NPC's UUID
    pub npc_id: Uuid,
    /// The PC's UUID
    pub pc_id: Uuid,
    /// Current emotional stance
    pub disposition: DispositionLevel,
    /// Long-term relationship level
    pub relationship: RelationshipLevel,
    /// Fine-grained sentiment score (-1.0 to 1.0)
    pub sentiment: f32,
    /// When this state was last updated (RFC 3339)
    pub updated_at: String,
    /// Reason for the last disposition change
    pub disposition_reason: Option<String>,
    /// Accumulated relationship points
    pub relationship_points: i32,
}

impl From<&NpcDispositionState> for NpcDispositionStateDto {
    fn from(state: &NpcDispositionState) -> Self {
        Self {
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
}

impl NpcDispositionStateDto {
    /// Convert back to domain type
    pub fn to_domain(&self) -> NpcDispositionState {
        use chrono::Utc;
        NpcDispositionState {
            npc_id: CharacterId::from_uuid(self.npc_id),
            pc_id: PlayerCharacterId::from_uuid(self.pc_id),
            disposition: self.disposition,
            relationship: self.relationship,
            sentiment: self.sentiment,
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            disposition_reason: self.disposition_reason.clone(),
            relationship_points: self.relationship_points,
        }
    }
}
