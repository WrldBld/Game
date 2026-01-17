//! Record visit use case - tracks NPC observations when a PC enters a region.

use chrono::{DateTime, Utc};
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
    /// * `game_time` - Current game time (from World.game_time.current())
    pub async fn execute(
        &self,
        pc_id: PlayerCharacterId,
        region_id: RegionId,
        npcs: &[StagedNpc],
        game_time: DateTime<Utc>,
    ) -> Result<(), RepoError> {
        // Get the region to find its location_id
        let region = self.location_repo.get_region(region_id).await?;
        let location_id = match region {
            Some(r) => r.location_id(),
            None => {
                tracing::warn!(
                    region_id = %region_id,
                    pc_id = %pc_id,
                    "Cannot record visit: region not found"
                );
                return Ok(()); // Region not found, nothing to record
            }
        };

        let now = self.clock.now(); // Real time for created_at

        // Create observations for each present, visible NPC
        for npc in npcs
            .iter()
            .filter(|n| n.is_present && !n.is_hidden_from_players)
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
        let mut npc = StagedNpc::new(character_id, "Test NPC", is_present, "test reasoning");
        npc.is_hidden_from_players = is_hidden;
        npc
    }

    #[tokio::test]
    async fn creates_observations_for_visible_npcs() {
        let mut observation_repo = MockObservationRepo::new();
        let mut location_repo = MockLocationRepo::new();
        let mut clock = MockClockPort::new();

        let _world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let game_time = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
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
        let result = use_case.execute(pc_id, region_id, &npcs, game_time).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn skips_hidden_npcs() {
        let mut observation_repo = MockObservationRepo::new();
        let mut location_repo = MockLocationRepo::new();
        let mut clock = MockClockPort::new();

        let _world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let game_time = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
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
        let result = use_case.execute(pc_id, region_id, &npcs, game_time).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn skips_already_observed_npcs() {
        let mut observation_repo = MockObservationRepo::new();
        let mut location_repo = MockLocationRepo::new();
        let mut clock = MockClockPort::new();

        let _world_id = WorldId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let game_time = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
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
        let result = use_case.execute(pc_id, region_id, &npcs, game_time).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn handles_missing_region_gracefully() {
        let observation_repo = MockObservationRepo::new();
        let mut location_repo = MockLocationRepo::new();
        let clock = MockClockPort::new();

        let _world_id = WorldId::new();
        let _location_id = LocationId::new();
        let region_id = RegionId::new();
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let game_time = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();

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
        let result = use_case.execute(pc_id, region_id, &npcs, game_time).await;

        // Should succeed without creating observations
        assert!(result.is_ok());
    }
}
