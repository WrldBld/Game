//! Exit to location use case.
//!
//! Handles player character movement to a different location entirely.
//! Determines the arrival region and coordinates with staging/narrative/scene/time systems.

use std::sync::Arc;
use wrldbldr_domain::{LocationId, PlayerCharacterId, RegionId};

use crate::infrastructure::ports::RepoError;
use crate::repositories::location::Location;
use crate::repositories::scene::Scene;
use crate::repositories::staging::Staging;
use crate::repositories::{Flag, Inventory, Observation, PlayerCharacter, World};
use crate::use_cases::narrative_operations::Narrative;
use crate::use_cases::time::SuggestTime;

use super::enter_region::EnterRegionResult;
use super::{resolve_scene_for_region, resolve_staging_for_region, suggest_time_for_movement};

/// Exit to location use case.
///
/// Handles moving to a different location entirely.
pub struct ExitLocation {
    player_character: Arc<PlayerCharacter>,
    location: Arc<Location>,
    staging: Arc<Staging>,
    observation: Arc<Observation>,
    narrative: Arc<Narrative>,
    scene: Arc<Scene>,
    inventory: Arc<Inventory>,
    flag: Arc<Flag>,
    world: Arc<World>,
    suggest_time: Arc<SuggestTime>,
}

impl ExitLocation {
    pub fn new(
        player_character: Arc<PlayerCharacter>,
        location: Arc<Location>,
        staging: Arc<Staging>,
        observation: Arc<Observation>,
        narrative: Arc<Narrative>,
        scene: Arc<Scene>,
        inventory: Arc<Inventory>,
        flag: Arc<Flag>,
        world: Arc<World>,
        suggest_time: Arc<SuggestTime>,
    ) -> Self {
        Self {
            player_character,
            location,
            staging,
            observation,
            narrative,
            scene,
            inventory,
            flag,
            world,
            suggest_time,
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
            .ok_or(ExitLocationError::PlayerCharacterNotFound)?;

        // 2. Get the target location
        let location = self
            .location
            .get(target_location_id)
            .await?
            .ok_or(ExitLocationError::LocationNotFound)?;

        // 3. Determine arrival region
        let region_id = self
            .determine_arrival_region(target_location_id, arrival_region_id)
            .await?;

        // 4. Get the arrival region
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(ExitLocationError::RegionNotFound)?;

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
            .ok_or(ExitLocationError::PlayerCharacterNotFound)?;

        // 7. Get the world to access game time for TTL checks and observations
        let world_data = self
            .world
            .get(pc.world_id())
            .await?
            .ok_or(ExitLocationError::WorldNotFound)?;
        let current_game_time = world_data.game_time().current();

        // 8. Check for valid staging (with TTL check using game time)
        let (npcs, staging_status) = resolve_staging_for_region(
            &self.staging,
            region_id,
            region.location_id(),
            pc.world_id(),
            current_game_time,
        )
        .await?;

        // 9. Update observation (only if staging ready)
        // Use game time for when the observation occurred in-game
        if !npcs.is_empty() {
            self.observation
                .record_visit(pc_id, region_id, &npcs, current_game_time)
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
            &self.scene,
            &self.inventory,
            &self.observation,
            &self.flag,
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
                .ok_or(ExitLocationError::RegionNotFound)?;

            if region.location_id() != location_id {
                return Err(ExitLocationError::RegionLocationMismatch);
            }

            return Ok(region_id);
        }

        // Try location's default arrival region
        let location = self
            .location
            .get(location_id)
            .await?
            .ok_or(ExitLocationError::LocationNotFound)?;

        if let Some(default_region_id) = location.default_region_id() {
            if self.location.get_region(default_region_id).await?.is_some() {
                return Ok(default_region_id);
            }
        }

        // Fall back to first spawn point in location
        let regions = self.location.list_regions_in_location(location_id).await?;

        regions
            .into_iter()
            .find(|r| r.is_spawn_point())
            .map(|r| r.id())
            .ok_or(ExitLocationError::NoArrivalRegion)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExitLocationError {
    #[error("Player character not found")]
    PlayerCharacterNotFound,
    #[error("Location not found")]
    LocationNotFound,
    #[error("World not found")]
    WorldNotFound,
    #[error("Region not found")]
    RegionNotFound,
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

    use chrono::Utc;
    use wrldbldr_domain::{
        value_objects::{CharacterName, LocationName},
        Description, LocationId, LocationType, PlayerCharacterId, Region, RegionId, WorldId,
    };

    use crate::infrastructure::ports::{
        ClockPort, MockChallengeRepo, MockCharacterRepo, MockFlagRepo, MockItemRepo,
        MockLocationRepo, MockNarrativeRepo, MockObservationRepo, MockPlayerCharacterRepo,
        MockSceneRepo, MockStagingRepo, MockWorldRepo,
    };
    use crate::repositories;
    use crate::repositories::{Inventory, Staging as StagingOp};
    use crate::use_cases::{Location, Narrative, Scene};

    struct FixedClock(chrono::DateTime<chrono::Utc>);

    impl ClockPort for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
    }

    fn build_use_case(
        player_character_repo: MockPlayerCharacterRepo,
        location_repo: MockLocationRepo,
        world_repo: MockWorldRepo,
        clock_port: Arc<dyn ClockPort>,
    ) -> super::ExitLocation {
        let clock = Arc::new(repositories::Clock::new(clock_port.clone()));
        let player_character = Arc::new(repositories::PlayerCharacter::new(Arc::new(
            player_character_repo,
        )));

        let location_repo = Arc::new(location_repo);
        let location = Arc::new(Location::new(location_repo.clone()));

        let staging = Arc::new(StagingOp::new(Arc::new(MockStagingRepo::new())));

        let observation = Arc::new(repositories::Observation::new(
            Arc::new(MockObservationRepo::new()),
            location_repo.clone(),
            clock_port.clone(),
        ));

        let scene = Arc::new(Scene::new(Arc::new(MockSceneRepo::new())));
        let inventory = Arc::new(Inventory::new(
            Arc::new(MockItemRepo::new()),
            Arc::new(MockCharacterRepo::new()),
            Arc::new(MockPlayerCharacterRepo::new()),
        ));
        let flag = Arc::new(repositories::Flag::new(Arc::new(MockFlagRepo::new())));

        let world = Arc::new(repositories::World::new(
            Arc::new(world_repo),
            clock_port.clone(),
        ));
        let narrative = Arc::new(Narrative::new(
            Arc::new(repositories::Narrative::new(
                Arc::new(MockNarrativeRepo::new()),
                clock_port.clone(),
            )),
            location.clone(),
            world.clone(),
            player_character.clone(),
            Arc::new(repositories::Character::new(Arc::new(
                MockCharacterRepo::new(),
            ))),
            observation.clone(),
            Arc::new(repositories::Challenge::new(Arc::new(
                MockChallengeRepo::new(),
            ))),
            flag.clone(),
            scene.clone(),
            clock.clone(),
        ));
        let suggest_time = Arc::new(crate::use_cases::time::SuggestTime::new(
            world.clone(),
            clock,
        ));

        super::ExitLocation::new(
            player_character,
            location,
            staging,
            observation,
            narrative,
            scene,
            inventory,
            flag,
            world,
            suggest_time,
        )
    }

    #[tokio::test]
    async fn when_pc_missing_then_returns_player_character_not_found() {
        let pc_id = PlayerCharacterId::new();
        let location_id = LocationId::new();

        let mut pc_repo = MockPlayerCharacterRepo::new();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(|_| Ok(None));

        let use_case = build_use_case(
            pc_repo,
            MockLocationRepo::new(),
            MockWorldRepo::new(),
            Arc::new(FixedClock(Utc::now())),
        );

        let err = use_case
            .execute(pc_id, location_id, None)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            super::ExitLocationError::PlayerCharacterNotFound
        ));
    }

    #[tokio::test]
    async fn when_location_missing_then_returns_location_not_found() {
        let now = Utc::now();
        let world_id = WorldId::new();
        let pc_location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let target_location_id = LocationId::new();

        let pc = wrldbldr_domain::PlayerCharacter::new(
            "user",
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
        assert!(matches!(err, super::ExitLocationError::LocationNotFound));
    }

    #[tokio::test]
    async fn when_specified_arrival_region_is_not_in_location_then_returns_region_location_mismatch(
    ) {
        let now = Utc::now();
        let world_id = WorldId::new();
        let pc_location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let target_location_id = LocationId::new();
        let other_location_id = LocationId::new();

        let pc = wrldbldr_domain::PlayerCharacter::new(
            "user",
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
            "Arrival".to_string(),
            String::new(),
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
        let now = Utc::now();
        let world_id = WorldId::new();
        let pc_location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let target_location_id = LocationId::new();

        let pc = wrldbldr_domain::PlayerCharacter::new(
            "user",
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
            .withf(move |id| *id == target_location_id)
            .returning(|_| Ok(vec![]));

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
        let now = Utc::now();
        let world_id = WorldId::new();
        let pc_location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();

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
            "Arrival".to_string(),
            String::new(),
            None,
            None,
            None,
            false,
            0,
        );

        let pc = wrldbldr_domain::PlayerCharacter::new(
            "user",
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
        assert!(matches!(err, super::ExitLocationError::WorldNotFound));
    }
}
