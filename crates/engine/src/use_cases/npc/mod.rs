//! NPC use cases.
//!
//! Handles NPC disposition, mood, and region relationship operations.

use std::sync::Arc;

use crate::infrastructure::ports::{NpcDispositionInfo, RepoError};
use crate::repositories::character::Character;
use crate::repositories::location::Location;
use crate::repositories::staging::Staging;
use crate::repositories::{Clock, Observation};
use wrldbldr_domain::{
    CharacterId, DispositionLevel, LocationId, MoodState, NpcDispositionState, PlayerCharacterId,
    RegionId, RelationshipLevel,
};

/// Container for NPC use cases.
pub struct NpcUseCases {
    pub disposition: Arc<NpcDisposition>,
    pub mood: Arc<NpcMood>,
    pub region_relationships: Arc<NpcRegionRelationships>,
    pub location_sharing: Arc<NpcLocationSharing>,
    pub approach_events: Arc<NpcApproachEvents>,
}

impl NpcUseCases {
    pub fn new(
        disposition: Arc<NpcDisposition>,
        mood: Arc<NpcMood>,
        region_relationships: Arc<NpcRegionRelationships>,
        location_sharing: Arc<NpcLocationSharing>,
        approach_events: Arc<NpcApproachEvents>,
    ) -> Self {
        Self {
            disposition,
            mood,
            region_relationships,
            location_sharing,
            approach_events,
        }
    }
}

/// Disposition and relationship operations.
pub struct NpcDisposition {
    character: Arc<Character>,
    clock: Arc<Clock>,
}

impl NpcDisposition {
    pub fn new(character: Arc<Character>, clock: Arc<Clock>) -> Self {
        Self { character, clock }
    }

    pub async fn set_disposition(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        disposition: DispositionLevel,
        reason: Option<String>,
    ) -> Result<NpcDispositionUpdate, NpcError> {
        let now = self.clock.now();
        let state = match self.character.get_disposition(npc_id, pc_id).await? {
            Some(existing) => existing,
            None => NpcDispositionState::new(npc_id, pc_id, now),
        };

        let state = state.updating_disposition(disposition, reason.clone(), now);
        self.character.save_disposition(&state).await?;

        let npc_name = self
            .character
            .get(npc_id)
            .await
            .ok()
            .flatten()
            .map(|npc| npc.name().to_string())
            .unwrap_or_else(|| "Unknown NPC".to_string());

        Ok(NpcDispositionUpdate {
            npc_id,
            npc_name,
            pc_id,
            disposition: state.disposition(),
            relationship: state.relationship(),
            reason,
        })
    }

    pub async fn set_relationship(
        &self,
        npc_id: CharacterId,
        pc_id: PlayerCharacterId,
        relationship: RelationshipLevel,
    ) -> Result<NpcDispositionUpdate, NpcError> {
        let now = self.clock.now();
        let state = match self.character.get_disposition(npc_id, pc_id).await? {
            Some(existing) => existing,
            None => NpcDispositionState::new(npc_id, pc_id, now),
        };

        let state = state.updating_relationship(relationship, now);
        self.character.save_disposition(&state).await?;

        let npc_name = self
            .character
            .get(npc_id)
            .await
            .ok()
            .flatten()
            .map(|npc| npc.name().to_string())
            .unwrap_or_else(|| "Unknown NPC".to_string());

        Ok(NpcDispositionUpdate {
            npc_id,
            npc_name,
            pc_id,
            disposition: state.disposition(),
            relationship: state.relationship(),
            reason: None,
        })
    }

    pub async fn list_for_pc(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionInfo>, NpcError> {
        let dispositions = self.character.list_dispositions_for_pc(pc_id).await?;
        let mut response = Vec::with_capacity(dispositions.len());

        for disposition in dispositions {
            let npc_name = self
                .character
                .get(disposition.npc_id())
                .await
                .ok()
                .flatten()
                .map(|npc| npc.name().to_string())
                .unwrap_or_else(|| "Unknown NPC".to_string());

            response.push(NpcDispositionInfo {
                npc_id: disposition.npc_id().to_string(),
                npc_name,
                disposition: disposition.disposition().to_string(),
                relationship: disposition.relationship().to_string(),
                sentiment: disposition.sentiment(),
                last_reason: disposition.disposition_reason().map(|s| s.to_string()),
            });
        }

        Ok(response)
    }
}

/// Mood operations for staged NPCs.
pub struct NpcMood {
    staging: Arc<Staging>,
    character: Arc<Character>,
}

impl NpcMood {
    pub fn new(staging: Arc<Staging>, character: Arc<Character>) -> Self {
        Self { staging, character }
    }

    pub async fn set_mood(
        &self,
        region_id: RegionId,
        npc_id: CharacterId,
        mood: MoodState,
    ) -> Result<NpcMoodChange, NpcError> {
        let npc = self
            .character
            .get(npc_id)
            .await
            .ok()
            .flatten()
            .ok_or(NpcError::NotFound)?;

        let old_mood = self
            .staging
            .get_npc_mood(region_id, npc_id)
            .await
            .unwrap_or(npc.default_mood().clone());

        self.staging.set_npc_mood(region_id, npc_id, mood).await?;

        Ok(NpcMoodChange {
            npc_id,
            npc_name: npc.name().to_string(),
            old_mood,
            new_mood: mood,
            region_id,
        })
    }

    pub async fn get_mood(
        &self,
        region_id: RegionId,
        npc_id: CharacterId,
    ) -> Result<MoodState, NpcError> {
        let mood = self.staging.get_npc_mood(region_id, npc_id).await?;
        Ok(mood)
    }
}

/// NPC region relationship operations.
pub struct NpcRegionRelationships {
    character: Arc<Character>,
}

impl NpcRegionRelationships {
    pub fn new(character: Arc<Character>) -> Self {
        Self { character }
    }

    pub async fn list_for_character(
        &self,
        npc_id: CharacterId,
    ) -> Result<Vec<crate::infrastructure::ports::NpcRegionRelationship>, NpcError> {
        Ok(self.character.get_region_relationships(npc_id).await?)
    }

    pub async fn set_home_region(
        &self,
        npc_id: CharacterId,
        region_id: RegionId,
    ) -> Result<(), NpcError> {
        self.character.set_home_region(npc_id, region_id).await?;
        Ok(())
    }

    pub async fn set_work_region(
        &self,
        npc_id: CharacterId,
        region_id: RegionId,
    ) -> Result<(), NpcError> {
        self.character
            .set_work_region(npc_id, region_id, None)
            .await?;
        Ok(())
    }

    pub async fn remove_relationship(
        &self,
        npc_id: CharacterId,
        region_id: RegionId,
        relationship_type: &str,
    ) -> Result<(), NpcError> {
        self.character
            .remove_region_relationship(npc_id, region_id, relationship_type)
            .await?;
        Ok(())
    }

    pub async fn list_region_npcs(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<crate::infrastructure::ports::NpcWithRegionInfo>, NpcError> {
        Ok(self.character.get_npcs_for_region(region_id).await?)
    }
}

/// Share NPC location knowledge with a PC (creates observation).
pub struct NpcLocationSharing {
    character: Arc<Character>,
    location: Arc<Location>,
    observation: Arc<Observation>,
    clock: Arc<Clock>,
}

impl NpcLocationSharing {
    pub fn new(
        character: Arc<Character>,
        location: Arc<Location>,
        observation: Arc<Observation>,
        clock: Arc<Clock>,
    ) -> Self {
        Self {
            character,
            location,
            observation,
            clock,
        }
    }

    pub async fn share_location(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        location_id: LocationId,
        region_id: RegionId,
        notes: Option<String>,
    ) -> Result<NpcLocationShareResult, NpcError> {
        let npc_name = self
            .character
            .get(npc_id)
            .await?
            .map(|npc| npc.name().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let region_name = self
            .location
            .get_region(region_id)
            .await?
            .map(|region| region.name)
            .unwrap_or_else(|| "Unknown".to_string());

        let now = self.clock.now();
        let observation = wrldbldr_domain::NpcObservation::heard_about(
            pc_id,
            npc_id,
            location_id,
            region_id,
            now,
            notes.clone(),
            now,
        );

        let observation_error = match self.observation.save_observation(&observation).await {
            Ok(()) => None,
            Err(e) => {
                tracing::error!(
                    pc_id = %pc_id,
                    npc_id = %npc_id,
                    location_id = %location_id,
                    error = %e,
                    "Failed to save NPC observation during location share"
                );
                Some(e.to_string())
            }
        };

        Ok(NpcLocationShareResult {
            pc_id,
            npc_id,
            location_id,
            region_id,
            npc_name,
            region_name,
            notes,
            observation_error,
        })
    }
}

/// Build NPC approach event details.
pub struct NpcApproachEvents {
    character: Arc<Character>,
}

impl NpcApproachEvents {
    pub fn new(character: Arc<Character>) -> Self {
        Self { character }
    }

    pub async fn build_event(
        &self,
        npc_id: CharacterId,
        reveal: bool,
    ) -> Result<NpcApproachEventResult, NpcError> {
        if !reveal {
            return Ok(NpcApproachEventResult {
                npc_name: "Unknown Figure".to_string(),
                npc_sprite: None,
                lookup_error: None,
            });
        }

        match self.character.get(npc_id).await {
            Ok(Some(npc)) => Ok(NpcApproachEventResult {
                npc_name: npc.name().to_string(),
                npc_sprite: npc.sprite_asset().map(|s| s.to_string()),
                lookup_error: None,
            }),
            Ok(None) => Ok(NpcApproachEventResult {
                npc_name: "Unknown NPC".to_string(),
                npc_sprite: None,
                lookup_error: None,
            }),
            Err(e) => Ok(NpcApproachEventResult {
                npc_name: "Unknown NPC".to_string(),
                npc_sprite: None,
                lookup_error: Some(e.to_string()),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NpcDispositionUpdate {
    pub npc_id: CharacterId,
    pub npc_name: String,
    pub pc_id: PlayerCharacterId,
    pub disposition: DispositionLevel,
    pub relationship: RelationshipLevel,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NpcMoodChange {
    pub npc_id: CharacterId,
    pub npc_name: String,
    pub old_mood: MoodState,
    pub new_mood: MoodState,
    pub region_id: RegionId,
}

#[derive(Debug, Clone)]
pub struct NpcLocationShareResult {
    pub pc_id: PlayerCharacterId,
    pub npc_id: CharacterId,
    pub location_id: LocationId,
    pub region_id: RegionId,
    pub npc_name: String,
    pub region_name: String,
    pub notes: Option<String>,
    pub observation_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NpcApproachEventResult {
    pub npc_name: String,
    pub npc_sprite: Option<String>,
    pub lookup_error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum NpcError {
    #[error("NPC not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
