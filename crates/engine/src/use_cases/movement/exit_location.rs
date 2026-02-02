//! Exit to location use case.
//!
//! Handles player character movement to a different location entirely.
//! Determines the arrival region and coordinates with staging/narrative/scene/time systems.

use std::sync::Arc;
use wrldbldr_domain::{PlayerCharacterId, RegionId, WorldId};

// Import LocationId for use in return types
use wrldbldr_domain::LocationId;

use crate::infrastructure::ports::{
    ClockPort, FlagRepo, LocationRepo, LocationStateRepo, ObservationRepo, PlayerCharacterRepo,
    RegionStateRepo, SceneRepo, StagingRepo, WorldRepo,
};
use crate::use_cases::narrative_operations::NarrativeOps;
use crate::use_cases::observation::RecordVisit;
use crate::use_cases::scene::ResolveScene;
use crate::use_cases::time::SuggestTime;

use super::enter_region::EnterRegionResult;
use super::{resolve_scene_for_region, resolve_staging_for_region, suggest_time_for_movement};

// StagingStatus is now defined in mod.rs and re-exported from there
use super::StagingStatus;

/// Exit to location use case.
///
/// Handles moving to a different location entirely.
pub struct ExitLocation {
    player_character: Arc<dyn PlayerCharacterRepo>,
    location: Arc<dyn LocationRepo>,
    arrival_determiner: LocationArrivalDeterminer,
    staging_resolver: MovementStagingResolver,
    scene_resolver: MovementSceneResolver,
    narrative_checker: MovementNarrativeChecker,
    time_suggester: MovementTimeSuggester,
    record_visit: Arc<RecordVisit>,
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
        let arrival_determiner = LocationArrivalDeterminer { location: location.clone() };
        let staging_resolver = MovementStagingResolver {
            staging,
            location_state,
            region_state,
            world: world.clone(),
            clock: clock.clone(),
        };
        let scene_resolver = MovementSceneResolver {
            resolve_scene,
            scene,
            player_character: player_character.clone(),
            observation,
            flag,
        };
        let narrative_checker = MovementNarrativeChecker { narrative };
        let time_suggester = MovementTimeSuggester { suggest_time };
        Self {
            player_character,
            location,
            arrival_determiner,
            staging_resolver,
            scene_resolver,
            narrative_checker,
            time_suggester,
            record_visit,
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

        // 2. Determine arrival region
        let region_id = self
            .arrival_determiner
            .determine(target_location_id, arrival_region_id)
            .await?;

        // 3. Get the arrival region
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(ExitLocationError::RegionNotFound(region_id))?;

        // Verify region belongs to target location
        if region.location_id() != target_location_id {
            return Err(ExitLocationError::RegionLocationMismatch);
        }

        // 4. Update player character position (both location and region)
        self.player_character
            .update_position(pc_id, target_location_id, region_id)
            .await?;

        // 5. Get fresh PC data after position update
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(ExitLocationError::PlayerCharacterNotFound(pc_id))?;

        // 6. Resolve staging, scene, narrative triggers, and time suggestion
        let world_id = pc.world_id();
        let location_id = region.location_id();

        let (npcs, staging_status, visual_state) = self
            .staging_resolver
            .resolve_for_region(region_id, location_id, world_id)
            .await?;

        // 7. Update observation (only if staging ready)
        // Use game time for when the observation occurred in-game
        if !npcs.is_empty() {
            let current_game_time_seconds = self.staging_resolver.get_game_time(world_id).await?;
            self.record_visit
                .execute(pc_id, region_id, &npcs, current_game_time_seconds)
                .await?;
        }

        let game_time = self.staging_resolver.get_game_time(world_id).await?;

        let triggered_events = self
            .narrative_checker
            .check_triggers(region_id, pc_id)
            .await?;

        let time_suggestion = self
            .time_suggester
            .suggest_for_movement(
                pc_id,
                world_id,
                &pc.name().to_string(),
                "travel_location",
                self.location
                    .get_location(target_location_id)
                    .await?
                    .ok_or(ExitLocationError::LocationNotFound(target_location_id))?
                    .name()
                    .as_str(),
            )
            .await;

        let resolved_scene = self
            .scene_resolver
            .resolve_for_region(pc_id, world_id, region_id, game_time)
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
}

/// Helper for determining arrival region when exiting to a location.
///
/// Encapsulates arrival region selection logic.
struct LocationArrivalDeterminer {
    location: Arc<dyn LocationRepo>,
}

impl LocationArrivalDeterminer {
    /// Determine the arrival region for a location.
    ///
    /// Returns the region ID where the player should arrive.
    async fn determine(
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

/// Helper for resolving movement staging.
///
/// Encapsulates staging resolution logic, including TTL checking and visual state loading.
/// Shared between EnterRegion and ExitLocation.
struct MovementStagingResolver {
    staging: Arc<dyn StagingRepo>,
    location_state: Arc<dyn LocationStateRepo>,
    region_state: Arc<dyn RegionStateRepo>,
    world: Arc<dyn WorldRepo>,
    clock: Arc<dyn ClockPort>,
}

impl MovementStagingResolver {
    /// Resolve staging for a region.
    ///
    /// Returns the visible NPCs, staging status, and optional visual state.
    async fn resolve_for_region(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        world_id: WorldId,
    ) -> Result<(Vec<wrldbldr_domain::StagedNpc>, StagingStatus, Option<crate::use_cases::staging::ResolvedVisualState>), ExitLocationError>
    {
        let current_game_time_seconds = self.get_game_time(world_id).await?;
        let real_timestamp = self.clock.now();

        resolve_staging_for_region(
            self.staging.as_ref(),
            self.location_state.as_ref(),
            self.region_state.as_ref(),
            region_id,
            location_id,
            world_id,
            current_game_time_seconds,
            real_timestamp,
        )
        .await
        .map_err(ExitLocationError::from)
    }

    /// Get the current game time in seconds for the given world.
    async fn get_game_time(&self, world_id: WorldId) -> Result<i64, ExitLocationError> {
        let world_data = self
            .world
            .get(world_id)
            .await?
            .ok_or(ExitLocationError::WorldNotFound(world_id))?;
        Ok(world_data.game_time().total_seconds())
    }
}

/// Helper for resolving scenes during movement.
///
/// Encapsulates scene resolution logic, including building evaluation context.
/// Shared between EnterRegion and ExitLocation.
struct MovementSceneResolver {
    resolve_scene: Arc<ResolveScene>,
    scene: Arc<dyn SceneRepo>,
    player_character: Arc<dyn PlayerCharacterRepo>,
    observation: Arc<dyn ObservationRepo>,
    flag: Arc<dyn FlagRepo>,
}

impl MovementSceneResolver {
    /// Resolve which scene to display for a PC entering a region.
    async fn resolve_for_region(
        &self,
        pc_id: PlayerCharacterId,
        world_id: WorldId,
        region_id: RegionId,
        game_time: i64,
    ) -> Result<Option<wrldbldr_domain::Scene>, ExitLocationError> {
        use wrldbldr_domain::GameTime;

        let game_time = GameTime::from_seconds(game_time);

        resolve_scene_for_region(
            &self.resolve_scene,
            self.scene.as_ref(),
            self.player_character.as_ref(),
            self.observation.as_ref(),
            self.flag.as_ref(),
            pc_id,
            world_id,
            region_id,
            &game_time,
        )
        .await
        .map_err(ExitLocationError::from)
    }
}

/// Helper for checking narrative triggers during movement.
///
/// Encapsulates narrative trigger checking logic.
/// Shared between EnterRegion and ExitLocation.
struct MovementNarrativeChecker {
    narrative: Arc<NarrativeOps>,
}

impl MovementNarrativeChecker {
    /// Check for triggered narrative events for a region/PC pair.
    async fn check_triggers(
        &self,
        region_id: RegionId,
        pc_id: PlayerCharacterId,
    ) -> Result<Vec<wrldbldr_domain::NarrativeEvent>, ExitLocationError> {
        self.narrative
            .check_triggers(region_id, pc_id)
            .await
            .map_err(ExitLocationError::from)
    }
}

/// Helper for generating time suggestions during movement.
///
/// Encapsulates time suggestion logic for movement actions.
/// Shared between EnterRegion and ExitLocation.
struct MovementTimeSuggester {
    suggest_time: Arc<SuggestTime>,
}

impl MovementTimeSuggester {
    /// Generate a time suggestion for movement.
    async fn suggest_for_movement(
        &self,
        pc_id: PlayerCharacterId,
        world_id: WorldId,
        pc_name: &str,
        action_type: &str,
        destination_name: &str,
    ) -> Option<crate::use_cases::time::TimeSuggestion> {
        suggest_time_for_movement(
            &self.suggest_time,
            world_id,
            pc_id,
            pc_name.to_string(),
            action_type,
            destination_name,
        )
        .await
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
    Repo(#[from] crate::infrastructure::ports::RepoError),
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
        let arrival_region = Region::from_storage(
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
        // Region belongs to target_location_id so we can test world not found
        let arrival_region = Region::from_storage(
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
            .returning(move |_| Ok(Some(arrival_region_for_get_1.clone())));

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
