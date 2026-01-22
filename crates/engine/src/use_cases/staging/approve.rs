//! Approve staging request use case.

use std::sync::Arc;

use uuid::Uuid;
use wrldbldr_domain::{LocationId, NpcPresence, RegionId, StagingSource, WorldId};

use crate::infrastructure::ports::{
    CharacterRepo, ClockPort, LocationRepo, LocationStateRepo, RegionStateRepo, StagingRepo,
    WorldRepo,
};

use super::types::{ApprovedNpc, NpcPresent, ResolvedStateInfo, ResolvedVisualState};
use super::StagingError;

/// Input for approving a staging request.
pub struct ApproveStagingInput {
    pub region_id: RegionId,
    pub location_id: Option<LocationId>,
    pub world_id: WorldId,
    pub approved_by: String,
    pub ttl_hours: i32,
    pub source: StagingSource,
    pub approved_npcs: Vec<ApprovedNpc>,
    pub location_state_id: Option<String>,
    pub region_state_id: Option<String>,
}

/// Result of staging approval.
pub struct StagingReadyPayload {
    pub region_id: RegionId,
    pub npcs_present: Vec<NpcPresent>,
    pub visual_state: Option<ResolvedVisualState>,
}

/// Use case for applying DM staging approvals.
pub struct ApproveStagingRequest {
    staging: Arc<dyn StagingRepo>,
    world: Arc<dyn WorldRepo>,
    character: Arc<dyn CharacterRepo>,
    location: Arc<dyn LocationRepo>,
    location_state: Arc<dyn LocationStateRepo>,
    region_state: Arc<dyn RegionStateRepo>,
    clock: Arc<dyn ClockPort>,
}

impl ApproveStagingRequest {
    pub fn new(
        staging: Arc<dyn StagingRepo>,
        world: Arc<dyn WorldRepo>,
        character: Arc<dyn CharacterRepo>,
        location: Arc<dyn LocationRepo>,
        location_state: Arc<dyn LocationStateRepo>,
        region_state: Arc<dyn RegionStateRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            staging,
            world,
            character,
            location,
            location_state,
            region_state,
            clock,
        }
    }

    pub async fn execute(
        &self,
        input: ApproveStagingInput,
    ) -> Result<StagingReadyPayload, StagingError> {
        // Validate approved_npcs array
        // Note: Empty approved_npcs is explicitly allowed - it represents a staging
        // with no NPCs present (e.g., an empty room or wilderness area)
        self.validate_approved_npcs(&input.approved_npcs)?;

        let world = self
            .world
            .get(input.world_id)
            .await?
            .ok_or(StagingError::WorldNotFound(input.world_id))?;

        let location_id = match input.location_id {
            Some(id) => id,
            None => {
                let region = self
                    .location
                    .get_region(input.region_id)
                    .await?
                    .ok_or(StagingError::RegionNotFound(input.region_id))?;
                region.location_id()
            }
        };

        let current_game_time_seconds = world.game_time().total_seconds();
        let approved_at = self.clock.now();

        let staged_npcs = self.build_staged_npcs(&input.approved_npcs).await?;

        let staging = wrldbldr_domain::Staging::new(
            input.region_id,
            location_id,
            input.world_id,
            current_game_time_seconds,
            input.approved_by.clone(),
            input.source,
            input.ttl_hours,
            approved_at,
        )
        .with_npcs(staged_npcs);

        // Use atomic save_and_activate to ensure both operations succeed or fail together
        self.staging
            .save_and_activate_pending_staging(&staging, input.region_id)
            .await?;

        if let Some(loc_state_str) = &input.location_state_id {
            if let Ok(loc_uuid) = Uuid::parse_str(loc_state_str) {
                let loc_state_id = wrldbldr_domain::LocationStateId::from_uuid(loc_uuid);
                // Validate that the location state exists before setting it as active
                match self.location_state.get(loc_state_id).await {
                    Ok(Some(_)) => {
                        self.location_state
                            .set_active(location_id, loc_state_id)
                            .await?;
                    }
                    Ok(None) => {
                        tracing::warn!(
                            location_state_id = %loc_state_str,
                            location_id = %location_id,
                            "Location state ID provided but not found in database, skipping"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            location_state_id = %loc_state_str,
                            "Failed to validate location state existence"
                        );
                    }
                }
            }
        }

        if let Some(reg_state_str) = &input.region_state_id {
            if let Ok(reg_uuid) = Uuid::parse_str(reg_state_str) {
                let reg_state_id = wrldbldr_domain::RegionStateId::from_uuid(reg_uuid);
                // Validate that the region state exists before setting it as active
                match self.region_state.get(reg_state_id).await {
                    Ok(Some(_)) => {
                        self.region_state
                            .set_active(input.region_id, reg_state_id)
                            .await?;
                    }
                    Ok(None) => {
                        tracing::warn!(
                            region_state_id = %reg_state_str,
                            region_id = %input.region_id,
                            "Region state ID provided but not found in database, skipping"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            region_state_id = %reg_state_str,
                            "Failed to validate region state existence"
                        );
                    }
                }
            }
        }

        let npcs_present = self.build_npcs_present(&input.approved_npcs).await?;
        let visual_state = self
            .build_visual_state_for_staging(location_id, input.region_id)
            .await;

        Ok(StagingReadyPayload {
            region_id: input.region_id,
            npcs_present,
            visual_state,
        })
    }

    async fn build_staged_npcs(
        &self,
        approved_npcs: &[ApprovedNpc],
    ) -> Result<Vec<wrldbldr_domain::StagedNpc>, StagingError> {
        let mut staged_npcs = Vec::new();

        for npc_info in approved_npcs {
            let character = self
                .character
                .get(npc_info.character_id)
                .await
                .map_err(|e| StagingError::Repo(e))?
                .ok_or(StagingError::CharacterNotFound(npc_info.character_id))?;

            let name = character.name().to_string();
            let sprite_asset = character.sprite_asset().map(|s| s.to_string());
            let portrait_asset = character.portrait_asset().map(|s| s.to_string());
            let default_mood = *character.default_mood();

            let mood = match npc_info.mood.as_deref() {
                Some(mood_str) => mood_str
                    .parse::<wrldbldr_domain::MoodState>()
                    .map_err(|e| {
                        StagingError::Validation(format!(
                            "Invalid mood state '{}' for character {}: {}",
                            mood_str, npc_info.character_id, e
                        ))
                    })?,
                None => default_mood,
            };

            let mut staged_npc = wrldbldr_domain::StagedNpc::new(
                npc_info.character_id,
                name,
                npc_info.is_present,
                npc_info.reasoning.clone().unwrap_or_default(),
            );
            let presence = if npc_info.is_hidden_from_players {
                NpcPresence::Hidden
            } else if npc_info.is_present {
                NpcPresence::Visible
            } else {
                NpcPresence::Absent
            };
            staged_npc = staged_npc.with_presence(presence);
            staged_npc.mood = mood;
            if let Some(sprite_path) = sprite_asset {
                let sprite = wrldbldr_domain::AssetPath::new(sprite_path.clone()).map_err(|e| {
                    StagingError::Validation(format!(
                        "Invalid sprite asset path '{}' for character {}: {}",
                        sprite_path, npc_info.character_id, e
                    ))
                })?;
                staged_npc.sprite_asset = Some(sprite);
            }
            if let Some(portrait_path) = portrait_asset {
                let portrait =
                    wrldbldr_domain::AssetPath::new(portrait_path.clone()).map_err(|e| {
                        StagingError::Validation(format!(
                            "Invalid portrait asset path '{}' for character {}: {}",
                            portrait_path, npc_info.character_id, e
                        ))
                    })?;
                staged_npc.portrait_asset = Some(portrait);
            }
            staged_npcs.push(staged_npc);
        }

        Ok(staged_npcs)
    }

    async fn build_npcs_present(&self, approved_npcs: &[ApprovedNpc]) -> Result<Vec<NpcPresent>, StagingError> {
        let mut npcs_present = Vec::new();
        for npc_info in approved_npcs {
            if npc_info.is_present && !npc_info.is_hidden_from_players {
                let character = self
                    .character
                    .get(npc_info.character_id)
                    .await
                    .map_err(|e| StagingError::Repo(e))?
                    .ok_or(StagingError::CharacterNotFound(npc_info.character_id))?;

                let name = character.name().to_string();
                let sprite_asset = character.sprite_asset().map(|s| s.to_string());
                let portrait_asset = character.portrait_asset().map(|s| s.to_string());

                npcs_present.push(NpcPresent {
                    character_id: npc_info.character_id,
                    name,
                    sprite_asset,
                    portrait_asset,
                    is_hidden_from_players: npc_info.is_hidden_from_players,
                    mood: npc_info.mood.clone(),
                });
            }
        }

        Ok(npcs_present)
    }

    async fn build_visual_state_for_staging(
        &self,
        location_id: LocationId,
        region_id: RegionId,
    ) -> Option<ResolvedVisualState> {
        let location_state = match self.location_state.get_active(location_id).await {
            Ok(state) => state,
            Err(e) => {
                tracing::warn!(location_id = %location_id, error = %e, "Failed to fetch location state for staging");
                None
            }
        };
        let region_state = match self.region_state.get_active(region_id).await {
            Ok(state) => state,
            Err(e) => {
                tracing::warn!(region_id = %region_id, error = %e, "Failed to fetch region state for staging");
                None
            }
        };

        if location_state.is_none() && region_state.is_none() {
            return None;
        }

        Some(ResolvedVisualState {
            location_state: location_state.map(|s| ResolvedStateInfo {
                id: s.id().to_string(),
                name: s.name().to_string(),
                backdrop_override: s.backdrop_override().map(|s| s.to_string()),
                atmosphere_override: s.atmosphere_override().map(|s| s.to_string()),
                ambient_sound: s.ambient_sound().map(|s| s.to_string()),
            }),
            region_state: region_state.map(|s| ResolvedStateInfo {
                id: s.id().to_string(),
                name: s.name().to_string(),
                backdrop_override: s.backdrop_override().map(|s| s.to_string()),
                atmosphere_override: s.atmosphere_override().map(|s| s.to_string()),
                ambient_sound: s.ambient_sound().map(|s| s.to_string()),
            }),
        })
    }

    /// Validates the approved_npcs array.
    ///
    /// Validation rules:
    /// - Empty array is allowed (represents staging with no NPCs)
    /// - CharacterId is already a valid typed ID (no string parsing needed)
    fn validate_approved_npcs(&self, approved_npcs: &[ApprovedNpc]) -> Result<(), StagingError> {
        // Log when empty array is explicitly approved
        if approved_npcs.is_empty() {
            tracing::debug!("Staging approved with empty NPC list (no NPCs present)");
        }

        // CharacterId is already typed, no further validation needed
        Ok(())
    }
}
