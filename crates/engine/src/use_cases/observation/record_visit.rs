//! Record visit use case - tracks NPC observations when a PC enters a region.

use std::sync::Arc;
use wrldbldr_domain::{NpcObservation, PlayerCharacterId, RegionId, StagedNpc};

use crate::infrastructure::ports::{ClockPort, LocationRepo, ObservationRepo, RepoError};

/// Records observations when a PC visits a region and sees NPCs.
///
/// Creates direct observations for each present, visible NPC in the region.
/// Skips NPCs the player has already observed to avoid duplicate records.
pub struct RecordVisit {
    observation_repo: Arc<dyn ObservationRepo>,
    location_repo: Arc<dyn LocationRepo>,
    clock: Arc<dyn ClockPort>,
}

impl RecordVisit {
    pub fn new(
        observation_repo: Arc<dyn ObservationRepo>,
        location_repo: Arc<dyn LocationRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            observation_repo,
            location_repo,
            clock,
        }
    }

    /// Record that a PC has visited a region and seen its NPCs.
    ///
    /// # Arguments
    /// * `pc_id` - The player character who visited
    /// * `region_id` - The region that was visited
    /// * `npcs` - NPCs present in the region
    /// * `game_time_minutes` - Current game time in total minutes since epoch
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        region_id: RegionId,
        npcs: &[StagedNpc],
        game_time_minutes: i64,
    ) -> Result<(), RepoError> {
        // Get the region to find its location_id
        let region = self
            .location_repo
            .get_region(region_id)
            .await?
            .ok_or_else(|| RepoError::not_found("Region", region_id.to_string()))?;
        let location_id = region.location_id();

        let now = self.clock.now(); // Real time for created_at

        // Convert game time minutes to DateTime for observation storage
        // (observations still use DateTime for compatibility)
        let game_time = wrldbldr_domain::GameTime::from_minutes(game_time_minutes).to_datetime();

        // Create observations for each present, visible NPC
        for npc in npcs
            .iter()
            .filter(|n| n.is_present() && !n.is_hidden_from_players())
        {
            // Check if already observed to avoid duplicates
            let already_observed = self
                .observation_repo
                .has_observed(pc_id, npc.character_id)
                .await?;

            if !already_observed {
                let observation = NpcObservation::direct(
                    pc_id,
                    npc.character_id,
                    location_id,
                    region_id,
                    game_time, // Game time for when the observation occurred in-game
                    now,       // Real time for when record was created
                );

                self.observation_repo.save_observation(&observation).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{MockClockPort, MockLocationRepo, MockObservationRepo};
    use chrono::TimeZone;
    use mockall::predicate::*;
    use wrldbldr_domain::{CharacterId, LocationId, Region, RegionName, WorldId};

    fn test_region(location_id: LocationId) -> Region {
        Region::new(location_id, RegionName::new("Test Region").unwrap())
    }

    fn test_staged_npc(character_id: CharacterId, is_present: bool, is_hidden: bool) -> StagedNpc {
        StagedNpc::new(character_id, "Test NPC", is_present, "test reasoning")
            .with_hidden_from_players(is_hidden)
    }

    #[tokio::test]
    async fn creates_observations_for_visible_npcs() {
        use chrono::{TimeZone, Utc};

        let mut observation_repo = MockObservationRepo::new();
        let mut location_repo = MockLocationRepo::new();
        let mut clock = MockClockPort::new();

        let _world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let game_time_minutes: i64 = 720; // 12 hours = 720 minutes from epoch
        let real_time = Utc.with_ymd_and_hms(2025, 6, 15, 10, 30, 0).unwrap();

        location_repo
            .expect_get_region()
            .with(eq(region_id))
            .returning(move |_| Ok(Some(test_region(location_id))));

        clock.expect_now().returning(move || real_time);

        observation_repo
            .expect_has_observed()
            .with(eq(pc_id), eq(npc_id))
            .returning(|_, _| Ok(false));

        observation_repo
            .expect_save_observation()
            .returning(|_| Ok(()));

        let use_case = RecordVisit::new(
            Arc::new(observation_repo),
            Arc::new(location_repo),
            Arc::new(clock),
        );

        let npcs = vec![test_staged_npc(npc_id, true, false)];
        let result = use_case
            .execute(pc_id, region_id, &npcs, game_time_minutes)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn skips_hidden_npcs() {
        use chrono::{TimeZone, Utc};

        let mut observation_repo = MockObservationRepo::new();
        let mut location_repo = MockLocationRepo::new();
        let mut clock = MockClockPort::new();

        let _world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let game_time_minutes: i64 = 720;
        let real_time = Utc.with_ymd_and_hms(2025, 6, 15, 10, 30, 0).unwrap();

        location_repo
            .expect_get_region()
            .with(eq(region_id))
            .returning(move |_| Ok(Some(test_region(location_id))));

        clock.expect_now().returning(move || real_time);

        // No has_observed or save_observation calls expected for hidden NPCs

        let use_case = RecordVisit::new(
            Arc::new(observation_repo),
            Arc::new(location_repo),
            Arc::new(clock),
        );

        let npcs = vec![test_staged_npc(npc_id, true, true)]; // hidden
        let result = use_case
            .execute(pc_id, region_id, &npcs, game_time_minutes)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn skips_already_observed_npcs() {
        use chrono::{TimeZone, Utc};

        let mut observation_repo = MockObservationRepo::new();
        let mut location_repo = MockLocationRepo::new();
        let mut clock = MockClockPort::new();

        let _world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let game_time_minutes: i64 = 720;
        let real_time = Utc.with_ymd_and_hms(2025, 6, 15, 10, 30, 0).unwrap();

        location_repo
            .expect_get_region()
            .with(eq(region_id))
            .returning(move |_| Ok(Some(test_region(location_id))));

        clock.expect_now().returning(move || real_time);

        observation_repo
            .expect_has_observed()
            .with(eq(pc_id), eq(npc_id))
            .returning(|_, _| Ok(true)); // Already observed

        // No save_observation call expected

        let use_case = RecordVisit::new(
            Arc::new(observation_repo),
            Arc::new(location_repo),
            Arc::new(clock),
        );

        let npcs = vec![test_staged_npc(npc_id, true, false)];
        let result = use_case
            .execute(pc_id, region_id, &npcs, game_time_minutes)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn returns_error_for_missing_region() {
        let observation_repo = MockObservationRepo::new();
        let mut location_repo = MockLocationRepo::new();
        let clock = MockClockPort::new();

        let _world_id = WorldId::new();
        let _location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let game_time_minutes: i64 = 720;

        location_repo
            .expect_get_region()
            .with(eq(region_id))
            .returning(|_| Ok(None)); // Region not found

        let use_case = RecordVisit::new(
            Arc::new(observation_repo),
            Arc::new(location_repo),
            Arc::new(clock),
        );

        let npcs = vec![test_staged_npc(npc_id, true, false)];
        let result = use_case
            .execute(pc_id, region_id, &npcs, game_time_minutes)
            .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(RepoError::NotFound { .. })));
    }
}
