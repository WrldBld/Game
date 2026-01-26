//! Approve staging request use case.

use std::sync::Arc;

use uuid::Uuid;
use wrldbldr_domain::{LocationId, NpcPresence, RegionId, StagingSource, WorldId};

use crate::infrastructure::ports::{
    CharacterRepo, ClockPort, LocationRepo, LocationStateRepo, RegionStateRepo, RepoError,
    StagingRepo, WorldRepo,
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

        // Resolve visual state IDs: use provided IDs or fall back to active states
        let (resolved_location_state_id, resolved_region_state_id, visual_state_source) =
            self.resolve_visual_state_ids(
                location_id,
                input.region_id,
                &input.location_state_id,
                &input.region_state_id,
                &input.source,
            )
            .await?;

        let mut staging = wrldbldr_domain::Staging::new(
            input.region_id,
            location_id,
            input.world_id,
            current_game_time_seconds,
            input.approved_by.clone(),
            input.source,
            input.ttl_hours,
            approved_at,
        )
        .with_npcs(staged_npcs)
        .with_visual_state_source(visual_state_source);

        // Set visual state IDs on staging (they've already been validated)
        if let Some(loc_state_id) = resolved_location_state_id {
            staging = staging.with_location_state(loc_state_id);
        }

        if let Some(reg_state_id) = resolved_region_state_id {
            staging = staging.with_region_state(reg_state_id);
        }

        // Use atomic save_and_activate with state updates to ensure all operations succeed or fail together
        self.staging
            .save_and_activate_pending_staging_with_states(
                &staging,
                input.region_id,
                resolved_location_state_id,
                resolved_region_state_id,
            )
            .await?;

        let npcs_present = self.build_npcs_present(&input.approved_npcs).await?;

        // Build visual state response using the resolved IDs from approval
        // This ensures the response matches what was actually saved
        let visual_state = self
            .build_visual_state_for_staging(resolved_location_state_id, resolved_region_state_id)
            .await?;

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
        location_state_id: Option<wrldbldr_domain::LocationStateId>,
        region_state_id: Option<wrldbldr_domain::RegionStateId>,
    ) -> Result<Option<ResolvedVisualState>, StagingError> {
        let location_state = match location_state_id {
            Some(id) => {
                let state = self
                    .location_state
                    .get(id)
                    .await
                    .map_err(StagingError::Repo)?
                    .ok_or_else(|| {
                        StagingError::Repo(RepoError::not_found("LocationState", id.to_string()))
                    })?;
                Some(state)
            }
            None => None,
        };

        let region_state = match region_state_id {
            Some(id) => {
                let state = self
                    .region_state
                    .get(id)
                    .await
                    .map_err(StagingError::Repo)?
                    .ok_or_else(|| {
                        StagingError::Repo(RepoError::not_found("RegionState", id.to_string()))
                    })?;
                Some(state)
            }
            None => None,
        };

        // Both being None is a data integrity error - staging was approved without resolving visual state IDs
        if location_state.is_none() && region_state.is_none() {
            return Err(StagingError::Repo(RepoError::database(
                "build_visual_state_for_staging",
                "Both location_state_id and region_state_id are None - staging was approved without resolving visual state IDs",
            )));
        }

        Ok(Some(ResolvedVisualState {
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
        }))
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

    /// Resolve visual state IDs for staging approval.
    ///
    /// Returns (location_state_id, region_state_id, visual_state_source).
    ///
    /// Logic:
    /// - If IDs provided by DM: validate they exist, use them, set source to DmOverride
    /// - If no IDs provided (manual DM approval or auto-approval): resolve active states, use them, set source to Default
    ///   - If no active states exist, return Validation error (at least one state must be active)
    async fn resolve_visual_state_ids(
        &self,
        location_id: LocationId,
        region_id: RegionId,
        provided_location_state_id: &Option<String>,
        provided_region_state_id: &Option<String>,
        source: &StagingSource,
    ) -> Result<(
        Option<wrldbldr_domain::LocationStateId>,
        Option<wrldbldr_domain::RegionStateId>,
        wrldbldr_domain::VisualStateSource,
    ), StagingError> {
        // Determine visual state source based on whether states were provided
        if provided_location_state_id.is_some() || provided_region_state_id.is_some() {
            // DM provided explicit IDs - validate they exist
            let resolved_location_id = if let Some(loc_state_str) = provided_location_state_id {
                let loc_uuid = Uuid::parse_str(loc_state_str).map_err(|e| {
                    StagingError::Validation(format!("Invalid location_state_id UUID: {}", e))
                })?;
                let loc_state_id = wrldbldr_domain::LocationStateId::from_uuid(loc_uuid);

                // Validate that the location state exists
                self.location_state
                    .get(loc_state_id)
                    .await
                    .map_err(|e| StagingError::Repo(e))?
                    .ok_or_else(|| {
                        StagingError::Validation(format!(
                            "Location state not found: {}",
                            loc_state_str
                        ))
                    })?;

                Some(loc_state_id)
            } else {
                None
            };

            let resolved_region_id = if let Some(reg_state_str) = provided_region_state_id {
                let reg_uuid = Uuid::parse_str(reg_state_str).map_err(|e| {
                    StagingError::Validation(format!("Invalid region_state_id UUID: {}", e))
                })?;
                let reg_state_id = wrldbldr_domain::RegionStateId::from_uuid(reg_uuid);

                // Validate that the region state exists
                self.region_state
                    .get(reg_state_id)
                    .await
                    .map_err(|e| StagingError::Repo(e))?
                    .ok_or_else(|| {
                        StagingError::Validation(format!("Region state not found: {}", reg_state_str))
                    })?;

                Some(reg_state_id)
            } else {
                None
            };

            Ok((
                resolved_location_id,
                resolved_region_id,
                wrldbldr_domain::VisualStateSource::DmOverride,
            ))
        } else {
            // No IDs provided - resolve active states
            let resolved_location_id = self
                .location_state
                .get_active(location_id)
                .await
                .map_err(|e| StagingError::Repo(e))?
                .map(|state| state.id());

            let resolved_region_id = self
                .region_state
                .get_active(region_id)
                .await
                .map_err(|e| StagingError::Repo(e))?
                .map(|state| state.id());

            // Require at least one active state for all approvals (no backcompat)
            if resolved_location_id.is_none() && resolved_region_id.is_none() {
                return Err(StagingError::Validation(
                    "No visual state IDs provided and no active states exist. Please select a location or region state."
                        .to_string(),
                ));
            }

            tracing::debug!(
                location_id = %location_id,
                region_id = %region_id,
                resolved_location_id = ?resolved_location_id,
                resolved_region_id = ?resolved_region_id,
                source = ?source,
                "Resolved visual states for approval"
            );

            Ok((
                resolved_location_id,
                resolved_region_id,
                wrldbldr_domain::VisualStateSource::Default,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{
        MockCharacterRepo, MockClockPort, MockLocationRepo, MockLocationStateRepo,
        MockRegionStateRepo, MockStagingRepo, MockWorldRepo, RepoError,
    };

    /// Test: build_visual_state_for_staging fails when both IDs are None.
    ///
    /// This verifies fail-fast behavior when staging is approved without
    /// resolving visual state IDs (no backcompat - at least one state must be resolved).
    #[tokio::test]
    async fn test_build_visual_state_for_staging_fails_when_both_ids_none() {
        // Setup mocks (only mock_location_state and mock_region_state are called by build_visual_state_for_staging)
        let mock_staging = MockStagingRepo::new();
        let mock_world = MockWorldRepo::new();
        let mock_character = MockCharacterRepo::new();
        let mock_location = MockLocationRepo::new();
        let mock_location_state = MockLocationStateRepo::new();
        let mock_region_state = MockRegionStateRepo::new();
        let mock_clock = MockClockPort::new();

        // Create use case instance
        let use_case = ApproveStagingRequest::new(
            Arc::new(mock_staging),
            Arc::new(mock_world),
            Arc::new(mock_character),
            Arc::new(mock_location),
            Arc::new(mock_location_state),
            Arc::new(mock_region_state),
            Arc::new(mock_clock),
        );

        // Call build_visual_state_for_staging with both IDs as None
        let result = use_case
            .build_visual_state_for_staging(None, None)
            .await;

        // Assert it returns the expected error
        assert!(result.is_err());
        match result {
            Err(StagingError::Repo(RepoError::Database { operation, message })) => {
                assert_eq!(operation, "build_visual_state_for_staging");
                assert!(message.contains("Both location_state_id and region_state_id are None"));
                assert!(message.contains("staging was approved without resolving visual state IDs"));
            }
            Err(other) => panic!("Expected StagingError::Repo(Database), got: {:?}", other),
            Ok(_) => panic!("Expected error, got Ok result"),
        }
    }
}
