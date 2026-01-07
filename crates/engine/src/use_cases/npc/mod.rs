//! NPC use cases.
//!
//! Handles NPC disposition, mood, and region relationship operations.

use std::sync::Arc;

use crate::entities::{Character, Staging};
use crate::infrastructure::ports::{ClockPort, RepoError};
use wrldbldr_domain::{
    CharacterId, DispositionLevel, MoodState, NpcDispositionState, PlayerCharacterId,
    RelationshipLevel, RegionId,
};
use wrldbldr_protocol::NpcDispositionData;

/// Container for NPC use cases.
pub struct NpcUseCases {
    pub disposition: Arc<NpcDisposition>,
    pub mood: Arc<NpcMood>,
    pub region_relationships: Arc<NpcRegionRelationships>,
}

impl NpcUseCases {
    pub fn new(
        disposition: Arc<NpcDisposition>,
        mood: Arc<NpcMood>,
        region_relationships: Arc<NpcRegionRelationships>,
    ) -> Self {
        Self {
            disposition,
            mood,
            region_relationships,
        }
    }
}

/// Disposition and relationship operations.
pub struct NpcDisposition {
    character: Arc<Character>,
    clock: Arc<dyn ClockPort>,
}

impl NpcDisposition {
    pub fn new(character: Arc<Character>, clock: Arc<dyn ClockPort>) -> Self {
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
        let mut state = match self.character.get_disposition(npc_id, pc_id).await? {
            Some(existing) => existing,
            None => NpcDispositionState::new(npc_id, pc_id, now),
        };

        state.set_disposition(disposition, reason.clone(), now);
        self.character.save_disposition(&state).await?;

        let npc_name = self
            .character
            .get(npc_id)
            .await
            .ok()
            .flatten()
            .map(|npc| npc.name)
            .unwrap_or_else(|| "Unknown NPC".to_string());

        Ok(NpcDispositionUpdate {
            npc_id,
            npc_name,
            pc_id,
            disposition: state.disposition,
            relationship: state.relationship,
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
        let mut state = match self.character.get_disposition(npc_id, pc_id).await? {
            Some(existing) => existing,
            None => NpcDispositionState::new(npc_id, pc_id, now),
        };

        state.relationship = relationship;
        state.updated_at = now;
        self.character.save_disposition(&state).await?;

        let npc_name = self
            .character
            .get(npc_id)
            .await
            .ok()
            .flatten()
            .map(|npc| npc.name)
            .unwrap_or_else(|| "Unknown NPC".to_string());

        Ok(NpcDispositionUpdate {
            npc_id,
            npc_name,
            pc_id,
            disposition: state.disposition,
            relationship: state.relationship,
            reason: None,
        })
    }

    pub async fn list_for_pc(
        &self,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<NpcDispositionData>, NpcError> {
        let dispositions = self.character.list_dispositions_for_pc(pc_id).await?;
        let mut response = Vec::with_capacity(dispositions.len());

        for disposition in dispositions {
            let npc_name = self
                .character
                .get(disposition.npc_id)
                .await
                .ok()
                .flatten()
                .map(|npc| npc.name)
                .unwrap_or_else(|| "Unknown NPC".to_string());

            response.push(NpcDispositionData {
                npc_id: disposition.npc_id.to_string(),
                npc_name,
                disposition: disposition.disposition.to_string(),
                relationship: disposition.relationship.to_string(),
                sentiment: disposition.sentiment,
                last_reason: disposition.disposition_reason,
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
            .unwrap_or(npc.default_mood);

        self.staging.set_npc_mood(region_id, npc_id, mood).await?;

        Ok(NpcMoodChange {
            npc_id,
            npc_name: npc.name,
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
        self.character.set_work_region(npc_id, region_id, None).await?;
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

#[derive(Debug, thiserror::Error)]
pub enum NpcError {
    #[error("NPC not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
