//! Enter region use case.
//!
//! Handles player character movement to a region within the same location.
//! Coordinates with staging, observation, scene resolution, narrative, and time systems.

use std::sync::Arc;
use wrldbldr_domain::{
    NarrativeEvent, PlayerCharacter as DomainPlayerCharacter, PlayerCharacterId, Region, RegionId,
    Scene as DomainScene, StagedNpc, Staging as DomainStaging, WorldId,
};

use crate::infrastructure::ports::{
    ClockPort, FlagRepo, LocationRepo, LocationStateRepo, ObservationRepo, PlayerCharacterRepo,
    RegionStateRepo, RepoError, SceneRepo, StagingRepo, WorldRepo,
};
use crate::use_cases::narrative_operations::NarrativeOps;
use crate::use_cases::observation::RecordVisit;
use crate::use_cases::scene::ResolveScene;
use crate::use_cases::time::{SuggestTime, TimeSuggestion};

use super::{resolve_scene_for_region, resolve_staging_for_region, suggest_time_for_movement};

/// Result of entering a region.
#[derive(Debug)]
pub struct EnterRegionResult {
    /// The region entered
    pub region: Region,
    /// NPCs present in region (empty if staging pending)
    pub npcs: Vec<StagedNpc>,
    /// Narrative events triggered by entry
    pub triggered_events: Vec<NarrativeEvent>,
    /// Staging status for this region
    pub staging_status: StagingStatus,
    /// The player character who moved (for context in pending staging)
    pub pc: DomainPlayerCharacter,
    /// Resolved scene for this region (if any)
    pub resolved_scene: Option<DomainScene>,
    /// Time suggestion for this movement (if time mode is Suggested)
    pub time_suggestion: Option<TimeSuggestion>,
    /// Visual state from active staging (if any)
    pub visual_state: Option<crate::use_cases::staging::ResolvedVisualState>,
}

/// Status of staging for a region.
#[derive(Debug)]
pub enum StagingStatus {
    /// Valid staging exists, NPCs are resolved
    Ready,
    /// No valid staging, DM approval required
    Pending {
        /// Previous staging if it exists (may be expired)
        previous_staging: Box<Option<DomainStaging>>,
    },
}

/// Enter region use case.
///
/// Orchestrates: Movement validation, staging resolution, scene resolution, observation updates, trigger checks, time suggestions.
pub struct EnterRegion {
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

impl EnterRegion {
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

    /// Execute the enter region use case.
    ///
    /// # Arguments
    /// * `pc_id` - The player character moving
    /// * `region_id` - The target region to enter
    ///
    /// # Returns
    /// * `Ok(EnterRegionResult)` - Successfully entered region with scene data
    /// * `Err(EnterRegionError)` - Failed to enter region
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        region_id: RegionId,
    ) -> Result<EnterRegionResult, EnterRegionError> {
        // 1. Get the player character to validate and get current location
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(EnterRegionError::PlayerCharacterNotFound(pc_id))?;

        // 2. Get the target region
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(EnterRegionError::RegionNotFound(region_id))?;

        // 3. Verify region is in the same location (for move_to_region)
        if region.location_id() != pc.current_location_id() {
            return Err(EnterRegionError::RegionNotInCurrentLocation);
        }

        // 4. Validate connection exists and is not locked (if PC has a current region)
        // Skip validation for initial spawn when PC has no current region
        if let Some(current_region_id) = pc.current_region_id() {
            // Don't require path if already in target region
            if current_region_id != region_id {
                let connection_result = self.check_connection(current_region_id, region_id).await?;
                match connection_result {
                    ConnectionCheckResult::NoConnection => {
                        return Err(EnterRegionError::NoPathToRegion);
                    }
                    ConnectionCheckResult::Locked(reason) => {
                        return Err(EnterRegionError::MovementBlocked(reason));
                    }
                    ConnectionCheckResult::Open => {
                        // Connection exists and is unlocked - proceed
                    }
                }
            }
        }

        // 5. Get the world to access game time for TTL checks and observations
        let world_data = self
            .world
            .get(pc.world_id())
            .await?
            .ok_or(EnterRegionError::WorldNotFound(pc.world_id()))?;
        let current_game_time_seconds = world_data.game_time().total_seconds();
        let real_timestamp = self.clock.now();

        // 6. Check for valid staging (with TTL check using game time)
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

        // 7. Update player's observation state (even if staging pending, record the visit)
        // Use game time for when the observation occurred in-game
        if !npcs.is_empty() {
            self.record_visit
                .execute(pc_id, region_id, &npcs, current_game_time_seconds)
                .await?;
        }

        // 8. Resolve scene for this region (use world's game time for time-of-day checks)
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
                "Scene resolved for region entry"
            );
        }

        // 9. Check for triggered narrative events
        let triggered_events = self.narrative.check_triggers(region_id, pc_id).await?;

        // 10. Update player character position
        self.player_character
            .update_position(pc_id, pc.current_location_id(), region_id)
            .await?;

        // 11. Generate time suggestion for movement
        // This is a region-to-region move within the same location (travel_region)
        let time_suggestion = suggest_time_for_movement(
            &self.suggest_time,
            pc.world_id(),
            pc_id,
            pc.name().to_string(),
            "travel_region",
            region.name().as_str(),
        )
        .await;

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

    /// Check if a valid connection exists between regions.
    ///
    /// Returns:
    /// - `Open` if connection exists and is unlocked
    /// - `Locked(reason)` if connection exists but is locked
    /// - `NoConnection` if no path exists between regions
    async fn check_connection(
        &self,
        from_region_id: RegionId,
        to_region_id: RegionId,
    ) -> Result<ConnectionCheckResult, EnterRegionError> {
        let connections = self.location.get_connections(from_region_id, None).await?;

        // Find connection to target region
        match connections.iter().find(|c| c.to_region == to_region_id) {
            Some(connection) if connection.is_locked => {
                let reason = connection
                    .lock_description
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "The way is blocked".to_string());
                Ok(ConnectionCheckResult::Locked(reason))
            }
            Some(_) => Ok(ConnectionCheckResult::Open),
            None => Ok(ConnectionCheckResult::NoConnection),
        }
    }
}

/// Result of checking a connection between regions.
enum ConnectionCheckResult {
    /// Connection exists and is open
    Open,
    /// Connection exists but is locked
    Locked(String),
    /// No connection exists between regions
    NoConnection,
}

#[derive(Debug, thiserror::Error)]
pub enum EnterRegionError {
    #[error("Player character not found: {0}")]
    PlayerCharacterNotFound(PlayerCharacterId),
    #[error("Region not found: {0}")]
    RegionNotFound(RegionId),
    #[error("World not found: {0}")]
    WorldNotFound(WorldId),
    #[error("Region is not in the current location")]
    RegionNotInCurrentLocation,
    #[error("No path exists to that region")]
    NoPathToRegion,
    #[error("Movement blocked: {0}")]
    MovementBlocked(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{TimeZone, Utc};
    use wrldbldr_domain::{
        value_objects::RegionName, CharacterName, Description, LocationId, PlayerCharacterId,
        Region, RegionConnection, RegionId, UserId, WorldId,
    };

    use crate::infrastructure::ports::{
        ClockPort, MockChallengeRepo, MockCharacterRepo, MockFlagRepo, MockLocationRepo,
        MockLocationStateRepo, MockNarrativeRepo, MockObservationRepo,
        MockPlayerCharacterRepo, MockRegionStateRepo, MockSceneRepo, MockStagingRepo,
        MockWorldRepo, RepoError,
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
    ) -> super::EnterRegion {
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

        super::EnterRegion::new(
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
        let region_id = RegionId::new();
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

        let err = use_case.execute(pc_id, region_id).await.unwrap_err();
        assert!(matches!(
            err,
            super::EnterRegionError::PlayerCharacterNotFound(_)
        ));
    }

    #[tokio::test]
    async fn when_region_missing_then_returns_region_not_found() {
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let region_id = RegionId::new();
        let now = fixed_time();

        let pc = wrldbldr_domain::PlayerCharacter::new(
            UserId::new("user").unwrap(),
            world_id,
            CharacterName::new("PC").unwrap(),
            location_id,
            now,
        );

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut location_repo = MockLocationRepo::new();
        location_repo
            .expect_get_region()
            .withf(move |id| *id == region_id)
            .returning(|_| Ok(None));

        let use_case = build_use_case(
            pc_repo,
            location_repo,
            MockWorldRepo::new(),
            Arc::new(FixedClock(now)),
        );

        let err = use_case.execute(pc_id, region_id).await.unwrap_err();
        assert!(matches!(err, super::EnterRegionError::RegionNotFound(_)));
    }

    #[tokio::test]
    async fn when_region_in_different_location_then_returns_region_not_in_current_location() {
        let world_id = WorldId::new();
        let pc_location_id = LocationId::new();
        let other_location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let now = fixed_time();

        let pc = wrldbldr_domain::PlayerCharacter::new(
            UserId::new("user").unwrap(),
            world_id,
            CharacterName::new("PC").unwrap(),
            pc_location_id,
            now,
        );
        let region_id = RegionId::new();
        let region = Region::from_storage(
            region_id,
            other_location_id,
            RegionName::new("Target").unwrap(),
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
        let region_for_get = region.clone();
        location_repo
            .expect_get_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(Some(region_for_get.clone())));

        let use_case = build_use_case(
            pc_repo,
            location_repo,
            MockWorldRepo::new(),
            Arc::new(FixedClock(now)),
        );

        let err = use_case.execute(pc_id, region_id).await.unwrap_err();
        assert!(matches!(
            err,
            super::EnterRegionError::RegionNotInCurrentLocation
        ));
    }

    #[tokio::test]
    async fn when_no_connection_then_returns_no_path_to_region() {
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let now = fixed_time();

        let from_region_id = RegionId::new();
        let to_region_id = RegionId::new();
        let to_region = Region::from_storage(
            to_region_id,
            location_id,
            RegionName::new("Target").unwrap(),
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
            location_id,
            now,
        )
        .with_starting_region(from_region_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut location_repo = MockLocationRepo::new();
        let to_region_for_get = to_region.clone();
        location_repo
            .expect_get_region()
            .withf(move |id| *id == to_region_id)
            .returning(move |_| Ok(Some(to_region_for_get.clone())));
        location_repo
            .expect_get_connections()
            .withf(move |id, _limit| *id == from_region_id)
            .returning(|_, _| Ok(vec![]));

        let use_case = build_use_case(
            pc_repo,
            location_repo,
            MockWorldRepo::new(),
            Arc::new(FixedClock(now)),
        );

        let err = use_case.execute(pc_id, to_region_id).await.unwrap_err();
        assert!(matches!(err, super::EnterRegionError::NoPathToRegion));
    }

    #[tokio::test]
    async fn when_get_connections_repo_error_then_propagates() {
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let from_region_id = RegionId::new();
        let to_region_id = RegionId::new();
        let now = fixed_time();

        let to_region = Region::from_storage(
            to_region_id,
            location_id,
            RegionName::new("Target").unwrap(),
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
            location_id,
            now,
        )
        .with_starting_region(from_region_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut location_repo = MockLocationRepo::new();
        let to_region_for_get = to_region.clone();
        location_repo
            .expect_get_region()
            .withf(move |id| *id == to_region_id)
            .returning(move |_| Ok(Some(to_region_for_get.clone())));
        location_repo
            .expect_get_connections()
            .withf(move |id, _limit| *id == from_region_id)
            .returning(|_, _| Err(RepoError::database("get_connections", "Database error")));

        let use_case = build_use_case(
            pc_repo,
            location_repo,
            MockWorldRepo::new(),
            Arc::new(FixedClock(now)),
        );

        let err = use_case.execute(pc_id, to_region_id).await.unwrap_err();
        assert!(matches!(err, super::EnterRegionError::Repo(_)));
    }

    #[tokio::test]
    async fn when_connection_locked_then_returns_movement_blocked() {
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let now = fixed_time();

        let from_region_id = RegionId::new();
        let to_region_id = RegionId::new();
        let to_region = Region::from_storage(
            to_region_id,
            location_id,
            RegionName::new("Target").unwrap(),
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
            location_id,
            now,
        )
        .with_starting_region(from_region_id);

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut location_repo = MockLocationRepo::new();
        let to_region_for_get = to_region.clone();
        location_repo
            .expect_get_region()
            .withf(move |id| *id == to_region_id)
            .returning(move |_| Ok(Some(to_region_for_get.clone())));

        let conn = RegionConnection {
            from_region: from_region_id,
            to_region: to_region_id,
            description: None,
            bidirectional: false,
            is_locked: true,
            lock_description: Some("Locked".to_string()),
        };
        location_repo
            .expect_get_connections()
            .withf(move |id, _limit| *id == from_region_id)
            .returning(move |_, _| Ok(vec![conn.clone()]));

        let use_case = build_use_case(
            pc_repo,
            location_repo,
            MockWorldRepo::new(),
            Arc::new(FixedClock(now)),
        );

        let err = use_case.execute(pc_id, to_region_id).await.unwrap_err();
        let super::EnterRegionError::MovementBlocked(reason) = err else {
            panic!("expected MovementBlocked");
        };
        assert_eq!(reason, "Locked".to_string());
    }

    #[tokio::test]
    async fn when_world_missing_then_returns_world_not_found() {
        let world_id = WorldId::new();
        let location_id = LocationId::new();
        let pc_id = PlayerCharacterId::new();
        let now = fixed_time();

        let region_id = RegionId::new();
        let region = Region::from_storage(
            to_region_id,
            location_id,
            RegionName::new("Target").unwrap(),
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
            location_id,
            now,
        );

        let mut pc_repo = MockPlayerCharacterRepo::new();
        let pc_for_get = pc.clone();
        pc_repo
            .expect_get()
            .withf(move |id| *id == pc_id)
            .returning(move |_| Ok(Some(pc_for_get.clone())));

        let mut location_repo = MockLocationRepo::new();
        let region_for_get = region.clone();
        location_repo
            .expect_get_region()
            .withf(move |id| *id == region_id)
            .returning(move |_| Ok(Some(region_for_get.clone())));

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

        let err = use_case.execute(pc_id, region_id).await.unwrap_err();
        assert!(matches!(err, super::EnterRegionError::WorldNotFound(_)));
    }
}
