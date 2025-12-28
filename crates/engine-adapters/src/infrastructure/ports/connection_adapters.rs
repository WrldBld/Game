//! Connection Use Case Adapters
//!
//! Implements connection-related ports by wrapping existing services.
//!
//! # Implementation Notes
//!
//! ConnectionManagerAdapter is in a separate file (connection_manager_adapter.rs)
//! as it was created earlier. This file contains the remaining adapters for:
//! - WorldServicePort
//! - PlayerCharacterServicePort
//! - DirectorialContextPort
//! - ConnectionWorldStatePort (same as SceneWorldStatePort but with different trait)

use std::sync::Arc;

use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_domain::value_objects::{DirectorialNotes, DomainNpcMotivation, PacingGuidance};
use wrldbldr_engine_app::application::services::{PlayerCharacterService, WorldService};
use wrldbldr_engine_app::application::use_cases::{
    DirectorialContextData, DirectorialContextPort, NpcMotivation, PcData,
    PlayerCharacterServicePort, ConnectionWorldStatePort, WorldServicePort,
};
use wrldbldr_engine_ports::outbound::DirectorialContextRepositoryPort as PortDirectorialContextRepositoryPort;

use crate::infrastructure::websocket::directorial_converters::parse_tone;
use crate::infrastructure::WorldStateManager;

/// Adapter for WorldService implementing WorldServicePort
pub struct WorldServiceAdapter {
    service: Arc<dyn WorldService>,
}

impl WorldServiceAdapter {
    pub fn new(service: Arc<dyn WorldService>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl WorldServicePort for WorldServiceAdapter {
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

/// Adapter for PlayerCharacterService implementing PlayerCharacterServicePort
pub struct PlayerCharacterServiceAdapter {
    service: Arc<dyn PlayerCharacterService>,
}

impl PlayerCharacterServiceAdapter {
    pub fn new(service: Arc<dyn PlayerCharacterService>) -> Self {
        Self { service }
    }
}

#[async_trait::async_trait]
impl PlayerCharacterServicePort for PlayerCharacterServiceAdapter {
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

/// Adapter for DirectorialContextRepositoryPort implementing DirectorialContextPort
pub struct ConnectionDirectorialContextAdapter {
    repo: Arc<dyn PortDirectorialContextRepositoryPort>,
}

impl ConnectionDirectorialContextAdapter {
    pub fn new(repo: Arc<dyn PortDirectorialContextRepositoryPort>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl DirectorialContextPort for ConnectionDirectorialContextAdapter {
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
                    scene_mood: if tone_str.is_empty() || tone_str == "Neutral - balanced, conversational" {
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

/// Adapter for WorldStateManager implementing ConnectionWorldStatePort
pub struct ConnectionWorldStateAdapter {
    state: Arc<WorldStateManager>,
}

impl ConnectionWorldStateAdapter {
    pub fn new(state: Arc<WorldStateManager>) -> Self {
        Self { state }
    }
}

impl ConnectionWorldStatePort for ConnectionWorldStateAdapter {
    fn set_directorial_context(&self, world_id: &WorldId, context: DirectorialContextData) {
        // Convert use case DirectorialContextData to domain DirectorialNotes
        let npc_motivations = context
            .npc_motivations
            .into_iter()
            .map(|m| {
                let motivation = DomainNpcMotivation::new(
                    m.emotional_state.unwrap_or_default(),
                    m.motivation,
                );
                (m.character_id, motivation)
            })
            .collect();

        let notes = DirectorialNotes {
            general_notes: context.dm_notes.unwrap_or_default(),
            tone: parse_tone(&context.scene_mood.unwrap_or_default()),
            npc_motivations,
            forbidden_topics: Vec::new(),
            allowed_tools: Vec::new(),
            suggested_beats: Vec::new(),
            pacing: context
                .pacing
                .as_ref()
                .map(|p| match p.to_lowercase().as_str() {
                    "fast" => PacingGuidance::Fast,
                    "slow" => PacingGuidance::Slow,
                    "building" => PacingGuidance::Building,
                    "urgent" => PacingGuidance::Urgent,
                    _ => PacingGuidance::Natural,
                })
                .unwrap_or(PacingGuidance::Natural),
        };

        self.state.set_directorial_context(world_id, notes);
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
