//! Exit to location use case.
//!
//! Handles player character movement to a different location entirely.
//! Determines the arrival region and coordinates with staging/narrative/scene/time systems.

use std::sync::Arc;
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId, WorldId};

use crate::infrastructure::ports::{
    ClockPort, FlagRepo, LocationRepo, LocationStateRepo, ObservationRepo, PlayerCharacterRepo,
    RegionStateRepo, RepoError, SceneRepo, StagingRepo, WorldRepo,
};
use crate::use_cases::narrative_operations::NarrativeOps;
use crate::use_cases::observation::RecordVisit;
use crate::use_cases::scene::ResolveScene;
use crate::use_cases::time::SuggestTime;

use super::enter_region::EnterRegionResult;
use super::{resolve_scene_for_region, resolve_staging_for_region, suggest_time_for_movement};

/// Exit to location use case.
///
/// Handles moving to a different location entirely.
pub struct ExitLocation {
    player_character: Arc<dyn PlayerCharacterRepo>,
    location: Arc<dyn LocationRepo>,
    staging: Arc<dyn StagingRepo>,
    location_state: Arc<dyn LocationStateRepo>,
    region_state: Arc<dyn RegionStateRepo>,
    observation: Arc<dyn ObservationRepo>,
    record_visit: Arc<RecordVisit>,
    narrative: Arc<NarrativeOps>,
    resolve_scene: Arc<ResolveScene>,
    scene: Arc<dyn SceneRepo>,
    flag: Arc<dyn FlagRepo>,
    world: Arc<dyn WorldRepo>,
    suggest_time: Arc<SuggestTime>,
    clock: Arc<dyn ClockPort>,
}

impl ExitLocation {
    pub fn new(
        player_character: Arc<dyn PlayerCharacterRepo>,
        location: Arc<dyn LocationRepo>,
        staging: Arc<dyn StagingRepo>,
        location_state: Arc<dyn LocationStateRepo>,
        region_state: Arc<dyn RegionStateRepo>,
        observation: Arc<dyn ObservationRepo>,
        record_visit: Arc<RecordVisit>,
        narrative: Arc<NarrativeOps>,
        resolve_scene: Arc<ResolveScene>,
        scene: Arc<dyn SceneRepo>,
        flag: Arc<dyn FlagRepo>,
        world: Arc<dyn WorldRepo>,
        suggest_time: Arc<SuggestTime>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            player_character,
            location,
            staging,
            location_state,
            region_state,
            observation,
            record_visit,
            narrative,
            resolve_scene,
            scene,
            flag,
            world,
            suggest_time,
            clock,
        }
    }

    /// Execute the exit to location use case.
    ///
    /// # Arguments
    /// * `pc_id` - The player character moving
    /// * `target_location_id` - The destination location
    /// * `arrival_region_id` - Optional specific region to arrive in
    ///
    /// # Returns
    /// * `Ok(EnterRegionResult)` - Successfully arrived at new location
    /// * `Err(ExitLocationError)` - Failed to move
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        target_location_id: LocationId,
        arrival_region_id: Option<RegionId>,
    ) -> Result<EnterRegionResult, ExitLocationError> {
        // 1. Validate player character exists
        let _pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ExitLocationError::PlayerCharacterNotFound(pc_id))?;

        // 2. Get the target location
        let location = self
            .location
            .get_location(target_location_id)
            .await?
            .ok_or(ExitLocationError::LocationNotFound(target_location_id))?;

        // 3. Determine arrival region
        let region_id = self
            .determine_arrival_region(target_location_id, arrival_region_id)
            .await?;

        // 4. Get the arrival region
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(ExitLocationError::RegionNotFound(region_id))?;

        // Verify region belongs to target location
        if region.location_id() != location.id() {
            return Err(ExitLocationError::RegionLocationMismatch);
        }

        // 5. Update player character position (both location and region)
        self.player_character
            .update_position(pc_id, target_location_id, region_id)
            .await?;

        // 6. Get fresh PC data after position update
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ExitLocationError::PlayerCharacterNotFound(pc_id))?;

        // 7. Get the world to access game time for TTL checks and observations
        let world_id = pc.world_id();
        let world_data = self
            .world
            .get(world_id)
            .await?
            .ok_or(ExitLocationError::WorldNotFound(world_id))?;
        let current_game_time_seconds = world_data.game_time().total_seconds();
        let real_timestamp = self.clock.now();

        // 8. Check for valid staging (with TTL check using game time)
        let (npcs, staging_status, visual_state) = resolve_staging_for_region(
            self.staging.as_ref(),
            self.location_state.as_ref(),
            self.region_state.as_ref(),
            region_id,
            region.location_id(),
            pc.world_id(),
            current_game_time_seconds,
            real_timestamp,
        )
        .await?;

        // 9. Update observation (only if staging ready)
        // Use game time for when the observation occurred in-game
        if !npcs.is_empty() {
            self.record_visit
                .execute(pc_id, region_id, &npcs, current_game_time_seconds)
                .await?;
        }

        // 10. Check triggers
        let triggered_events = self.narrative.check_triggers(region_id, pc_id).await?;

        // 11. Generate time suggestion for location travel
        let time_suggestion = suggest_time_for_movement(
            &self.suggest_time,
            pc.world_id(),
            pc_id,
            pc.name().to_string(),
            "travel_location",
            location.name().as_str(),
        )
        .await;

        // 12. Resolve scene for the arrival region
        let resolved_scene = resolve_scene_for_region(
            &self.resolve_scene,
            self.scene.as_ref(),
            self.player_character.as_ref(),
            self.observation.as_ref(),
            self.flag.as_ref(),
            pc_id,
            pc.world_id(),
            region_id,
            world_data.game_time(),
        )
        .await?;
        if let Some(ref scene) = resolved_scene {
            tracing::info!(
                pc_id = %pc_id,
                region_id = %region_id,
                scene_id = %scene.id(),
                scene_name = %scene.name(),
                "Scene resolved for location arrival"
            );
        }

        Ok(EnterRegionResult {
            region,
            npcs,
            triggered_events,
            staging_status,
            pc,
            resolved_scene,
            time_suggestion,
            visual_state,
        })
    }

    /// Determine the arrival region for a location.
    async fn determine_arrival_region(
        &self,
        location_id: LocationId,
        specified_region_id: Option<RegionId>,
    ) -> Result<RegionId, ExitLocationError> {
        // If a specific region was specified, use it
        if let Some(region_id) = specified_region_id {
            // Verify region exists and belongs to location
            let region = self
                .location
                .get_region(region_id)
                .await?
                .ok_or(ExitLocationError::RegionNotFound(region_id))?;

            if region.location_id() != location_id {
                return Err(ExitLocationError::RegionLocationMismatch);
            }

            return Ok(region_id);
        }

        // Try location's default arrival region
        let location = self
            .location
            .get_location(location_id)
            .await?
            .ok_or(ExitLocationError::LocationNotFound(location_id))?;

        if let Some(default_region_id) = location.default_region_id() {
            if self.location.get_region(default_region_id).await?.is_some() {
                return Ok(default_region_id);
            }
        }

        // Fall back to first spawn point in location
        let regions = self.location.list_regions_in_location(location_id, None, None).await?;

        regions
            .into_iter()
            .find(|r| r.is_spawn_point())
            .map(|r| r.id())
            .ok_or(ExitLocationError::NoArrivalRegion)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExitLocationError {
    #[error("Player character not found: {0}")]
    PlayerCharacterNotFound(PlayerCharacterId),
    #[error("Location not found: {0}")]
    LocationNotFound(LocationId),
    #[error("World not found: {0}")]
    WorldNotFound(WorldId),
    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),
    #[error("No arrival region specified and no default found")]
    NoArrivalRegion,
    #[error("Region does not belong to target location")]
    RegionLocationMismatch,
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{TimeZone, Utc};
    use wrldbldr_domain::{
        value_objects::{CharacterName, LocationName, RegionName},
        Description, LocationId, LocationType, PlayerCharacterId, Region, RegionId, UserId,
        WorldId,
    };

    use crate::infrastructure::ports::{
        ClockPort, MockChallengeRepo, MockCharacterRepo, MockFlagRepo, MockLocationRepo,
        MockLocationStateRepo, MockNarrativeRepo, MockObservationRepo,
        MockPlayerCharacterRepo, MockRegionStateRepo, MockSceneRepo, MockStagingRepo,
        MockWorldRepo,
    };
    use crate::use_cases::scene::ResolveScene;
    use crate::use_cases::NarrativeOps;

    fn fixed_time() -> chrono::DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    struct FixedClock(chrono::DateTime<Utc>);

    impl ClockPort for FixedClock {
        fn now(&self) -> chrono::DateTime<Utc> {
            self.0
        }
    }

    fn build_use_case(
        player_character_repo: MockPlayerCharacterRepo,
        location_repo: MockLocationRepo,
        world_repo: MockWorldRepo,
        clock_port: Arc<dyn ClockPort>,
    ) -> super::ExitLocation {
        let player_character_repo: Arc<dyn crate::infrastructure::ports::PlayerCharacterRepo> =
            Arc::new(player_character_repo);

        let location_repo: Arc<dyn crate::infrastructure::ports::LocationRepo> =
            Arc::new(location_repo);

        let staging_repo: Arc<dyn crate::infrastructure::ports::StagingRepo> =
            Arc::new(MockStagingRepo::new());

        let location_state_repo: Arc<dyn crate::infrastructure::ports::LocationStateRepo> =
            Arc::new(MockLocationStateRepo::new());

        let region_state_repo: Arc<dyn crate::infrastructure::ports::RegionStateRepo> =
            Arc::new(MockRegionStateRepo::new());

        let observation_repo: Arc<dyn crate::infrastructure::ports::ObservationRepo> =
            Arc::new(MockObservationRepo::new());
        let record_visit = Arc::new(crate::use_cases::observation::RecordVisit::new(
            observation_repo.clone(),
            location_repo.clone(),
            clock_port.clone(),
        ));

        let scene_repo: Arc<dyn crate::infrastructure::ports::SceneRepo> =
            Arc::new(MockSceneRepo::new());
        let resolve_scene = Arc::new(ResolveScene::new(scene_repo.clone()));
        let flag_repo: Arc<dyn crate::infrastructure::ports::FlagRepo> =
            Arc::new(MockFlagRepo::new());

        let world_repo: Arc<dyn crate::infrastructure::ports::WorldRepo> = Arc::new(world_repo);
        let narrative = Arc::new(NarrativeOps::new(
            Arc::new(MockNarrativeRepo::new()),
            location_repo.clone(),
            world_repo.clone(),
            player_character_repo.clone(),
            Arc::new(MockCharacterRepo::new()),
            observation_repo.clone(),
            Arc::new(MockChallengeRepo::new()),
            flag_repo.clone(),
            scene_repo.clone(),
            clock_port.clone(),
        ));
        let suggest_time = Arc::new(crate::use_cases::time::SuggestTime::new(
            world_repo.clone(),
            clock_port.clone(),
        ));

        super::ExitLocation::new(
            player_character_repo,
            location_repo,
            staging_repo,
            location_state_repo,
            region_state_repo,
            observation_repo,
            record_visit,
            narrative,
            resolve_scene,
            scene_repo,
            flag_repo,
            world_repo,
            suggest_time,
            clock_port,
        )
    }

    #[tokio::test]
    async fn when_pc_missing_then_returns_player_character_not_found() {
        let pc_id = PlayerCharacterId::new();
        let location_id = LocationId::new();
        let now = fixed_time();

        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(|_| Ok(None));

        let use_case = build_use_case(
            pc_repo,
            MockLocationRepo::new(),
            MockWorldRepo::new(),
            Arc::new(FixedClock(now)),
        );

        let err = use_case
            .execute(pc_id, location_id, None)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            super::ExitLocationError::PlayerCharacterNotFound(_)
        ));
    }

    #[tokio::test]
    async fn when_location_missing_then_returns_location_not_found() {
        let world_id = WorldId::new();
        let pc_location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let target_location_id = LocationId::new();
        let now = fixed_time();

        let pc = wrldbldr_domain::PlayerCharacter::new(
            UserId::new("user").unwrap(),
            world_id,
            CharacterName::new("PC").unwrap(),
            pc_location_id,
            now,
        )
        .with_id(pc_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut location_repo = MockLocationRepo::new();
        location_repo
            .expect_get_location()
            .withf(move |id| *id == target_location_id)
            .returning(|_| Ok(None));

        let use_case = build_use_case(
            pc_repo,
            location_repo,
            MockWorldRepo::new(),
            Arc::new(FixedClock(now)),
        );

        let err = use_case
            .execute(pc_id, target_location_id, None)
            .await
            .unwrap_err();
        assert!(matches!(err, super::ExitLocationError::LocationNotFound(_)));
    }

    #[tokio::test]
    async fn when_specified_arrival_region_is_not_in_location_then_returns_region_location_mismatch(
    ) {
        let world_id = WorldId::new();
        let pc_location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let target_location_id = LocationId::new();
        let other_location_id = LocationId::new();
        let now = fixed_time();

        let pc = wrldbldr_domain::PlayerCharacter::new(
            UserId::new("user").unwrap(),
            world_id,
            CharacterName::new("PC").unwrap(),
            pc_location_id,
            now,
        )
        .with_id(pc_id);

        let location_name = LocationName::new("Target").unwrap();
        let location =
            wrldbldr_domain::Location::new(world_id, location_name, LocationType::Interior)
                .with_description(Description::new("Desc").unwrap())
                .with_id(target_location_id);

        let arrival_region_id = RegionId::new();
        let arrival_region = Region::from_parts(
            arrival_region_id,
            other_location_id,
            RegionName::new("Arrival").unwrap(),
            Description::default(),
            None,
            None,
            None,
            false,
            0,
        );

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut location_repo = MockLocationRepo::new();
        let location_for_get = location.clone();
        location_repo
            .expect_get_location()
            .withf(move |id| *id == target_location_id)
            .returning(move |_| Ok(Some(location_for_get.clone())));

        let region_for_get = arrival_region.clone();
        location_repo
            .expect_get_region()
            .withf(move |id| *id == arrival_region_id)
            .returning(move |_| Ok(Some(region_for_get.clone())));

        let use_case = build_use_case(
            pc_repo,
            location_repo,
            MockWorldRepo::new(),
            Arc::new(FixedClock(now)),
        );

        let err = use_case
            .execute(pc_id, target_location_id, Some(arrival_region_id))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            super::ExitLocationError::RegionLocationMismatch
        ));
    }

    #[tokio::test]
    async fn when_no_arrival_region_possible_then_returns_no_arrival_region() {
        let world_id = WorldId::new();
        let pc_location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let target_location_id = LocationId::new();
        let now = fixed_time();

        let pc = wrldbldr_domain::PlayerCharacter::new(
            UserId::new("user").unwrap(),
            world_id,
            CharacterName::new("PC").unwrap(),
            pc_location_id,
            now,
        )
        .with_id(pc_id);

        let location_name = LocationName::new("Target").unwrap();
        // Location has default_region_id = None by default, no need to set it explicitly
        let location =
            wrldbldr_domain::Location::new(world_id, location_name, LocationType::Interior)
                .with_description(Description::new("Desc").unwrap())
                .with_id(target_location_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut location_repo = MockLocationRepo::new();
        let location_for_get = location.clone();
        location_repo
            .expect_get_location()
            .withf(move |id| *id == target_location_id)
            .returning(move |_| Ok(Some(location_for_get.clone())));

        // determine_arrival_region fetches location again
        let location_for_get_2 = location.clone();
        location_repo
            .expect_get_location()
            .withf(move |id| *id == target_location_id)
            .returning(move |_| Ok(Some(location_for_get_2.clone())));

        location_repo
            .expect_list_regions_in_location()
            .withf(move |id, _limit, _offset| *id == target_location_id)
            .returning(|_, _, _| Ok(vec![]));

        let use_case = build_use_case(
            pc_repo,
            location_repo,
            MockWorldRepo::new(),
            Arc::new(FixedClock(now)),
        );

        let err = use_case
            .execute(pc_id, target_location_id, None)
            .await
            .unwrap_err();
        assert!(matches!(err, super::ExitLocationError::NoArrivalRegion));
    }

    #[tokio::test]
    async fn when_world_missing_then_returns_world_not_found() {
        let world_id = WorldId::new();
        let pc_location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let now = fixed_time();

        let target_location_id = LocationId::new();
        let location_name = LocationName::new("Target").unwrap();
        let target_location =
            wrldbldr_domain::Location::new(world_id, location_name, LocationType::Interior)
                .with_description(Description::new("Desc").unwrap())
                .with_id(target_location_id);

        let arrival_region_id = RegionId::new();
        let arrival_region = Region::from_parts(
            arrival_region_id,
            target_location_id,
            RegionName::new("Arrival").unwrap(),
            Description::default(),
            None,
            None,
            None,
            false,
            0,
        );

        let pc = wrldbldr_domain::PlayerCharacter::new(
            UserId::new("user").unwrap(),
            world_id,
            CharacterName::new("PC").unwrap(),
            pc_location_id,
            now,
        )
        .with_id(pc_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get_1 = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .times(1)
            .returning(move |_| Ok(Some(pc_for_get_1.clone())));

        pc_repo
            .expect_update_position()
            .withf(move |id, loc, reg| {
                *id == pc_id && *loc == target_location_id && *reg == arrival_region_id
            })
            .returning(|_, _, _| Ok(()));

        let pc_for_get_2 = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .times(1)
            .returning(move |_| Ok(Some(pc_for_get_2.clone())));

        let mut location_repo = MockLocationRepo::new();
        let location_for_get = target_location.clone();
        location_repo
            .expect_get_location()
            .withf(move |id| *id == target_location_id)
            .returning(move |_| Ok(Some(location_for_get.clone())));

        let arrival_region_for_get_1 = arrival_region.clone();
        location_repo
            .expect_get_region()
            .withf(move |id| *id == arrival_region_id)
            .times(1)
            .returning(move |_| Ok(Some(arrival_region_for_get_1.clone())));

        let arrival_region_for_get_2 = arrival_region.clone();
        location_repo
            .expect_get_region()
            .withf(move |id| *id == arrival_region_id)
            .times(1)
            .returning(move |_| Ok(Some(arrival_region_for_get_2.clone())));

        let mut world_repo = MockWorldRepo::new();
        world_repo
            .expect_get()
            .withf(move |id| *id == world_id)
            .returning(|_| Ok(None));

        let use_case = build_use_case(
            pc_repo,
            location_repo,
            world_repo,
            Arc::new(FixedClock(now)),
        );

        let err = use_case
            .execute(pc_id, target_location_id, Some(arrival_region_id))
            .await
            .unwrap_err();
        assert!(matches!(err, super::ExitLocationError::WorldNotFound(_)));
    }
}
