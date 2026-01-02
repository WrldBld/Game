//! Connection Use Case Adapters
//!
//! Implements connection-related ports by wrapping existing services.
//!
//! # Implementation Notes
//!
//! `ConnectionManagerPort` is implemented directly on `WorldConnectionManager`.
//! This file contains the remaining adapters for:
//! - WorldSnapshotJsonPort
//! - PlayerCharacterDtoPort
//! - DirectorialContextQueryPort
//! - WorldStatePort (consolidated from ConnectionWorldStatePort and SceneWorldStatePort)

use std::sync::Arc;

use wrldbldr_domain::value_objects::PacingGuidance;
use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::outbound::{
    DirectorialContextData, DirectorialContextQueryPort,
    DirectorialContextRepositoryPort as PortDirectorialContextRepositoryPort, NpcMotivation,
    PcData, PlayerCharacterDtoPort,
    PlayerCharacterServicePort as OutboundPlayerCharacterServicePort,
    WorldServicePort as OutboundWorldServicePort, WorldSnapshotJsonPort,
};

/// Adapter for WorldServicePort implementing WorldSnapshotJsonPort.
pub struct WorldServiceAdapter {
    service: Arc<dyn OutboundWorldServicePort>,
}

impl WorldServiceAdapter {
    pub fn new(service: Arc<dyn OutboundWorldServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl WorldSnapshotJsonPort for WorldServiceAdapter {
    async fn export_world_snapshot(&self, world_id: WorldId) -> Result<serde_json::Value, String> {
        match self.service.export_world_snapshot(world_id).await {
            Ok(snapshot) => {
                // Convert PlayerWorldSnapshot to serde_json::Value
                serde_json::to_value(&snapshot).map_err(|e| e.to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Adapter for PlayerCharacterServicePort implementing PlayerCharacterDtoPort.
pub struct PlayerCharacterServiceAdapter {
    service: Arc<dyn OutboundPlayerCharacterServicePort>,
}

impl PlayerCharacterServiceAdapter {
    pub fn new(service: Arc<dyn OutboundPlayerCharacterServicePort>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl PlayerCharacterDtoPort for PlayerCharacterServiceAdapter {
    async fn get_pc(&self, pc_id: PlayerCharacterId) -> Result<Option<PcData>, String> {
        match self.service.get_pc(pc_id).await {
            Ok(Some(pc)) => Ok(Some(PcData {
                id: pc.id.to_string(),
                name: pc.name,
                user_id: pc.user_id,
                world_id: pc.world_id.to_string(),
                current_location_id: pc.current_location_id.to_string(),
                current_region_id: pc.current_region_id.map(|id| id.to_string()),
                description: pc.description,
                sprite_asset: pc.sprite_asset,
                portrait_asset: pc.portrait_asset,
            })),
            Ok(None) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Adapter for DirectorialContextRepositoryPort implementing DirectorialContextQueryPort.
pub struct ConnectionDirectorialContextAdapter {
    repo: Arc<dyn PortDirectorialContextRepositoryPort>,
}

impl ConnectionDirectorialContextAdapter {
    pub fn new(repo: Arc<dyn PortDirectorialContextRepositoryPort>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl DirectorialContextQueryPort for ConnectionDirectorialContextAdapter {
    async fn get(&self, world_id: &WorldId) -> Result<Option<DirectorialContextData>, String> {
        match self.repo.get(world_id).await {
            Ok(Some(notes)) => {
                // Convert domain DirectorialNotes to use case DirectorialContextData
                let pacing_str = match notes.pacing {
                    PacingGuidance::Natural => None,
                    PacingGuidance::Fast => Some("fast".to_string()),
                    PacingGuidance::Slow => Some("slow".to_string()),
                    PacingGuidance::Building => Some("building".to_string()),
                    PacingGuidance::Urgent => Some("urgent".to_string()),
                };

                let tone_str = notes.tone.description().to_string();

                Ok(Some(DirectorialContextData {
                    npc_motivations: notes
                        .npc_motivations
                        .into_iter()
                        .map(|(char_id, m)| NpcMotivation {
                            character_id: char_id,
                            motivation: m.immediate_goal,
                            emotional_state: if m.current_mood.is_empty() {
                                None
                            } else {
                                Some(m.current_mood)
                            },
                        })
                        .collect(),
                    scene_mood: if tone_str.is_empty()
                        || tone_str == "Neutral - balanced, conversational"
                    {
                        None
                    } else {
                        Some(tone_str)
                    },
                    pacing: pacing_str,
                    dm_notes: if notes.general_notes.is_empty() {
                        None
                    } else {
                        Some(notes.general_notes)
                    },
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pc_data_fields() {
        let pc_data = PcData {
            id: "pc1".to_string(),
            name: "Hero".to_string(),
            user_id: "user1".to_string(),
            world_id: "world1".to_string(),
            current_location_id: "loc1".to_string(),
            current_region_id: Some("region1".to_string()),
            description: Some("A brave hero".to_string()),
            sprite_asset: None,
            portrait_asset: None,
        };

        assert_eq!(pc_data.name, "Hero");
        assert!(pc_data.description.is_some());
    }
}
