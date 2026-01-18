// Observation management - methods for future observation features
#![allow(dead_code)]

//! Observation management operations.

use std::sync::Arc;

use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId};

use crate::infrastructure::ports::{
    CharacterRepo, ClockPort, LocationRepo, ObservationRepo, PlayerCharacterRepo, WorldRepo,
};

use super::ManagementError;

pub struct ObservationManagement {
    observation: Arc<dyn ObservationRepo>,
    player_character: Arc<dyn PlayerCharacterRepo>,
    character: Arc<dyn CharacterRepo>,
    location: Arc<dyn LocationRepo>,
    world: Arc<dyn WorldRepo>,
    clock: Arc<dyn ClockPort>,
}

impl ObservationManagement {
    pub fn new(
        observation: Arc<dyn ObservationRepo>,
        player_character: Arc<dyn PlayerCharacterRepo>,
        character: Arc<dyn CharacterRepo>,
        location: Arc<dyn LocationRepo>,
        world: Arc<dyn WorldRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            observation,
            player_character,
            character,
            location,
            world,
            clock,
        }
    }

    pub async fn list(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<wrldbldr_domain::NpcObservation>, ManagementError> {
        Ok(self.observation.get_observations(pc_id).await?)
    }

    pub async fn list_summaries(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<ObservationSummaryData>, ManagementError> {
        let observations = self.observation.get_observations(pc_id).await?;
        let mut summaries = Vec::new();

        for observation in observations {
            let npc = self.character.get(observation.npc_id()).await?;
            let region = self.location.get_region(observation.region_id()).await?;
            let location = self
                .location
                .get_location(observation.location_id())
                .await?;

            let (npc_name, npc_portrait) = if observation.is_revealed_to_player() {
                (
                    npc.as_ref()
                        .map(|n| n.name().to_string())
                        .unwrap_or_else(|| "Unknown NPC".to_string()),
                    npc.as_ref()
                        .and_then(|n| n.portrait_asset().map(|s| s.to_string())),
                )
            } else {
                ("Unknown Figure".to_string(), None)
            };

            let location_name = location
                .as_ref()
                .map(|l| l.name().to_string())
                .unwrap_or_else(|| "Unknown Location".to_string());
            let region_name = region
                .as_ref()
                .map(|r| r.name().to_string())
                .unwrap_or_else(|| "Unknown Region".to_string());

            let (obs_type, obs_icon) = match observation.observation_type() {
                wrldbldr_domain::ObservationType::Direct => ("direct", "eye"),
                wrldbldr_domain::ObservationType::HeardAbout => ("heard_about", "ear"),
                wrldbldr_domain::ObservationType::Deduced => ("deduced", "brain"),
            };

            summaries.push(ObservationSummaryData {
                npc_id: observation.npc_id().to_string(),
                npc_name,
                npc_portrait,
                location_name,
                region_name,
                game_time: observation.game_time().to_rfc3339(),
                observation_type: obs_type.to_string(),
                observation_type_icon: obs_icon.to_string(),
                notes: observation.notes().map(|s| s.to_string()),
            });
        }

        Ok(summaries)
    }

    pub async fn create(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        observation_type: String,
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
        notes: Option<String>,
    ) -> Result<wrldbldr_domain::NpcObservation, ManagementError> {
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ManagementError::NotFound {
                entity_type: "PlayerCharacter",
                id: pc_id.to_string(),
            })?;

        let (location_id, region_id) = self
            .resolve_observation_location(location_id, region_id)
            .await?;

        let world = self
            .world
            .get(pc.world_id())
            .await?
            .ok_or(ManagementError::NotFound {
                entity_type: "World",
                id: pc.world_id().to_string(),
            })?;

        let obs_type = observation_type
            .parse::<wrldbldr_domain::ObservationType>()
            .map_err(|e| {
                ManagementError::InvalidInput(format!("Invalid observation type: {}", e))
            })?;

        let now = self.clock.now();
        let game_time = world.game_time().current();
        let observation = match obs_type {
            wrldbldr_domain::ObservationType::Direct => wrldbldr_domain::NpcObservation::direct(
                pc_id,
                npc_id,
                location_id,
                region_id,
                game_time,
                now,
            ),
            wrldbldr_domain::ObservationType::HeardAbout => {
                wrldbldr_domain::NpcObservation::heard_about(
                    pc_id,
                    npc_id,
                    location_id,
                    region_id,
                    game_time,
                    notes.clone(),
                    now,
                )
            }
            wrldbldr_domain::ObservationType::Deduced => wrldbldr_domain::NpcObservation::deduced(
                pc_id,
                npc_id,
                location_id,
                region_id,
                game_time,
                notes.clone(),
                now,
            ),
        };

        self.observation.save_observation(&observation).await?;
        Ok(observation)
    }

    pub async fn delete(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<(), ManagementError> {
        self.observation.delete_observation(pc_id, npc_id).await?;
        Ok(())
    }

    async fn resolve_observation_location(
        &self,
        location_id: Option<LocationId>,
        region_id: Option<RegionId>,
    ) -> Result<(LocationId, RegionId), ManagementError> {
        match (location_id, region_id) {
            (Some(location_id), Some(region_id)) => Ok((location_id, region_id)),
            (None, Some(region_id)) => {
                let region = self.location.get_region(region_id).await?.ok_or(
                    ManagementError::NotFound {
                        entity_type: "Region",
                        id: region_id.to_string(),
                    },
                )?;
                Ok((region.location_id(), region_id))
            }
            _ => Err(ManagementError::InvalidInput(
                "location_id and/or region_id required".to_string(),
            )),
        }
    }
}

/// Summary of an NPC observation for UI consumption.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObservationSummaryData {
    pub npc_id: String,
    pub npc_name: String,
    pub npc_portrait: Option<String>,
    pub location_name: String,
    pub region_name: String,
    pub game_time: String,
    pub observation_type: String,
    pub observation_type_icon: String,
    pub notes: Option<String>,
}
