// NPC use cases - fields for future NPC mood features
#![allow(dead_code)]

//! NPC use cases.
//!
//! Handles NPC disposition, mood, and region relationship operations.

use std::sync::Arc;

use crate::infrastructure::ports::{
    CharacterRepo, ClockPort, LocationRepo, NpcDispositionInfo, NpcRegionRelationType,
    ObservationRepo, PlayerCharacterRepo, RepoError, StagingRepo, WorldRepo,
};
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
    character: Arc<dyn CharacterRepo>,
    clock: Arc<dyn ClockPort>,
}

impl NpcDisposition {
    pub fn new(character: Arc<dyn CharacterRepo>, clock: Arc<dyn ClockPort>) -> Self {
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

        let npc_name = match self.character.get(npc_id).await {
            Ok(Some(npc)) => npc.name().to_string(),
            Ok(None) => {
                tracing::warn!(npc_id = %npc_id, "NPC not found when updating disposition");
                "Unknown NPC".to_string()
            }
            Err(e) => {
                tracing::warn!(npc_id = %npc_id, error = %e, "Failed to fetch NPC name for disposition update");
                "Unknown NPC".to_string()
            }
        };

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

        let npc_name = match self.character.get(npc_id).await {
            Ok(Some(npc)) => npc.name().to_string(),
            Ok(None) => {
                tracing::warn!(npc_id = %npc_id, "NPC not found when updating relationship");
                "Unknown NPC".to_string()
            }
            Err(e) => {
                tracing::warn!(npc_id = %npc_id, error = %e, "Failed to fetch NPC name for relationship update");
                "Unknown NPC".to_string()
            }
        };

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
            let npc_name = match self.character.get(disposition.npc_id()).await {
                Ok(Some(npc)) => npc.name().to_string(),
                Ok(None) => {
                    tracing::warn!(npc_id = %disposition.npc_id(), "NPC not found when listing dispositions");
                    "Unknown NPC".to_string()
                }
                Err(e) => {
                    tracing::warn!(npc_id = %disposition.npc_id(), error = %e, "Failed to fetch NPC name for disposition list");
                    "Unknown NPC".to_string()
                }
            };

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
    staging: Arc<dyn StagingRepo>,
    character: Arc<dyn CharacterRepo>,
}

impl NpcMood {
    pub fn new(staging: Arc<dyn StagingRepo>, character: Arc<dyn CharacterRepo>) -> Self {
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
            .await?
            .ok_or(NpcError::NotFound {
                entity_type: "Character".to_string(),
                id: npc_id.to_string(),
            })?;

        let old_mood = match self.staging.get_npc_mood(region_id, npc_id).await {
            Ok(mood) => mood,
            Err(e) => {
                tracing::debug!(region_id = %region_id, npc_id = %npc_id, error = %e, "Failed to get staged mood, using default");
                *npc.default_mood()
            }
        };

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
    character: Arc<dyn CharacterRepo>,
}

impl NpcRegionRelationships {
    pub fn new(character: Arc<dyn CharacterRepo>) -> Self {
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
        relationship_type: NpcRegionRelationType,
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
    character: Arc<dyn CharacterRepo>,
    location: Arc<dyn LocationRepo>,
    observation: Arc<dyn ObservationRepo>,
    player_character: Arc<dyn PlayerCharacterRepo>,
    world: Arc<dyn WorldRepo>,
    clock: Arc<dyn ClockPort>,
}

impl NpcLocationSharing {
    pub fn new(
        character: Arc<dyn CharacterRepo>,
        location: Arc<dyn LocationRepo>,
        observation: Arc<dyn ObservationRepo>,
        player_character: Arc<dyn PlayerCharacterRepo>,
        world: Arc<dyn WorldRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            character,
            location,
            observation,
            player_character,
            world,
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

        // Validate that region belongs to location
        let region = self
            .location
            .get_region(region_id)
            .await?
            .ok_or(NpcError::NotFound {
                entity_type: "Region".to_string(),
                id: region_id.to_string(),
            })?;

        if region.location_id() != location_id {
            return Err(NpcError::InvalidInput(format!(
                "Region {} does not belong to location {}",
                region_id, location_id
            )));
        }

        let region_name = region.name().to_string();

        // Get world to use world game time
        let pc = self
            .player_character
            .get(pc_id)
            .await?
            .ok_or(NpcError::NotFound {
                entity_type: "PlayerCharacter".to_string(),
                id: pc_id.to_string(),
            })?;

        let world = self
            .world
            .get(pc.world_id())
            .await?
            .ok_or(NpcError::NotFound {
                entity_type: "World".to_string(),
                id: pc.world_id().to_string(),
            })?;

        let game_time = world.game_time().to_datetime();
        let now = self.clock.now();

        let observation = wrldbldr_domain::NpcObservation::heard_about(
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
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
    character: Arc<dyn CharacterRepo>,
    location: Arc<dyn LocationRepo>,
    observation: Arc<dyn ObservationRepo>,
    player_character: Arc<dyn PlayerCharacterRepo>,
    clock: Arc<dyn ClockPort>,
    world: Arc<dyn WorldRepo>,
}

impl NpcApproachEvents {
    pub fn new(
        character: Arc<dyn CharacterRepo>,
        location: Arc<dyn LocationRepo>,
        observation: Arc<dyn ObservationRepo>,
        player_character: Arc<dyn PlayerCharacterRepo>,
        world: Arc<dyn WorldRepo>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            character,
            location,
            observation,
            player_character,
            world,
            clock,
        }
    }

    pub async fn build_event(
        &self,
        npc_id: CharacterId,
        reveal: bool,
        pc_id: PlayerCharacterId,
        location_id: LocationId,
        region_id: RegionId,
    ) -> Result<NpcApproachEventResult, NpcError> {
        // Get NPC details
        let npc_name = self
            .character
            .get(npc_id)
            .await?
            .map(|npc| npc.name().to_string())
            .unwrap_or_else(|| "Unknown NPC".to_string());

        // When reveal=false, create unrevealed direct observation
        if !reveal {
            let now = self.clock.now();

            // Get world for game time
            let pc = self
                .player_character
                .get(pc_id)
                .await?
                .ok_or(NpcError::NotFound {
                    entity_type: "PlayerCharacter".to_string(),
                    id: pc_id.to_string(),
                })?;

            let world = self
                .world
                .get(pc.world_id())
                .await?
                .ok_or(NpcError::NotFound {
                    entity_type: "World".to_string(),
                    id: pc.world_id().to_string(),
                })?;

            let game_time = world.game_time().to_datetime();

            // Validate region belongs to location
            let region = self
                .location
                .get_region(region_id)
                .await?
                .ok_or(NpcError::NotFound {
                    entity_type: "Region".to_string(),
                    id: region_id.to_string(),
                })?;

            if region.location_id() != location_id {
                return Err(NpcError::InvalidInput(format!(
                    "Region {} does not belong to location {}",
                    region_id, location_id
                )));
            }

            // Create unrevealed direct observation
            let observation = wrldbldr_domain::NpcObservation::direct_unrevealed(
                pc_id,
                npc_id,
                location_id,
                region_id,
                game_time,
                now,
            );

            let observation_error = match self.observation.save_observation(&observation).await {
                Ok(()) => None,
                Err(e) => {
                    tracing::error!(
                        pc_id = %pc_id,
                        npc_id = %npc_id,
                        error = %e,
                        "Failed to save observation for approach event"
                    );
                    Some(e.to_string())
                }
            };

            return Ok(NpcApproachEventResult {
                npc_name: "Unknown Figure".to_string(),
                npc_sprite: None,
                lookup_error: None,
                observation_error,
            });
        }

        // reveal=true: return NPC details
        let npc_sprite = match self.character.get(npc_id).await {
            Ok(Some(npc)) => npc.sprite_asset().map(|s| s.to_string()),
            Ok(None) => None,
            Err(_) => None,
        };

        Ok(NpcApproachEventResult {
            npc_name,
            npc_sprite,
            lookup_error: None,
            observation_error: None,
        })
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
    pub observation_error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum NpcError {
    #[error("{entity_type} {id} not found")]
    NotFound {
        entity_type: String,
        id: String,
    },
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use wrldbldr_domain::{
        CampbellArchetype, Character, CharacterName, NpcDispositionState, Region, RegionFrequency,
        RegionName, WorldId,
    };

    use crate::infrastructure::ports::{
        ClockPort, MockCharacterRepo, MockLocationRepo, MockObservationRepo, MockPlayerCharacterRepo, MockStagingRepo,
        MockWorldRepo, NpcRegionRelationType, NpcRegionRelationship, NpcWithRegionInfo, RepoError,
    };

    // =========================================================================
    // Test Helpers
    // =========================================================================

    struct FixedClock(chrono::DateTime<chrono::Utc>);

    impl ClockPort for FixedClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
    }

    fn fixed_time() -> chrono::DateTime<chrono::Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    fn build_clock(now: chrono::DateTime<chrono::Utc>) -> Arc<dyn ClockPort> {
        Arc::new(FixedClock(now))
    }

    fn create_test_character(id: CharacterId, name: &str) -> Character {
        Character::new(
            WorldId::new(),
            CharacterName::new(name).expect("valid character name"),
            CampbellArchetype::Mentor,
        )
        .with_id(id)
    }

    // =========================================================================
    // NpcDisposition Tests
    // =========================================================================

    mod disposition_ops {
        use super::*;

        #[tokio::test]
        async fn when_set_disposition_succeeds() {
            let now = fixed_time();
            let npc_id = CharacterId::new();
            let pc_id = PlayerCharacterId::new();

            // Mock character repo - no existing disposition, save succeeds, get NPC name
            let mut character_repo = MockCharacterRepo::new();

            // get_disposition returns None (new disposition)
            character_repo
                .expect_get_disposition()
                .withf(move |n, p| *n == npc_id && *p == pc_id)
                .returning(|_, _| Ok(None));

            // save_disposition succeeds
            character_repo
                .expect_save_disposition()
                .returning(|_| Ok(()));

            // get NPC for name lookup
            let npc = create_test_character(npc_id, "TestNPC");
            let npc_clone = npc.clone();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(move |_| Ok(Some(npc_clone.clone())));

            let use_case = NpcDisposition::new(Arc::new(character_repo), build_clock(now));

            let result = use_case
                .set_disposition(
                    npc_id,
                    pc_id,
                    DispositionLevel::Friendly,
                    Some("Helped them".to_string()),
                )
                .await
                .expect("set_disposition should succeed");

            assert_eq!(result.npc_id, npc_id);
            assert_eq!(result.pc_id, pc_id);
            assert_eq!(result.npc_name, "TestNPC");
            assert_eq!(result.disposition, DispositionLevel::Friendly);
            assert_eq!(result.reason, Some("Helped them".to_string()));
        }

        #[tokio::test]
        async fn when_set_disposition_updates_existing() {
            let now = fixed_time();
            let npc_id = CharacterId::new();
            let pc_id = PlayerCharacterId::new();

            let existing_state = NpcDispositionState::new(npc_id, pc_id, now)
                .with_disposition(DispositionLevel::Neutral);

            let mut character_repo = MockCharacterRepo::new();

            // get_disposition returns existing state
            let state_clone = existing_state.clone();
            character_repo
                .expect_get_disposition()
                .withf(move |n, p| *n == npc_id && *p == pc_id)
                .returning(move |_, _| Ok(Some(state_clone.clone())));

            character_repo
                .expect_save_disposition()
                .returning(|_| Ok(()));

            let npc = create_test_character(npc_id, "UpdatedNPC");
            let npc_clone = npc.clone();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(move |_| Ok(Some(npc_clone.clone())));

            let use_case = NpcDisposition::new(Arc::new(character_repo), build_clock(now));

            let result = use_case
                .set_disposition(npc_id, pc_id, DispositionLevel::Hostile, None)
                .await
                .expect("set_disposition should succeed");

            assert_eq!(result.disposition, DispositionLevel::Hostile);
        }

        #[tokio::test]
        async fn when_set_relationship_succeeds() {
            let now = fixed_time();
            let npc_id = CharacterId::new();
            let pc_id = PlayerCharacterId::new();

            let mut character_repo = MockCharacterRepo::new();

            character_repo
                .expect_get_disposition()
                .returning(|_, _| Ok(None));

            character_repo
                .expect_save_disposition()
                .returning(|_| Ok(()));

            let npc = create_test_character(npc_id, "RelNPC");
            let npc_clone = npc.clone();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(move |_| Ok(Some(npc_clone.clone())));

            let use_case = NpcDisposition::new(Arc::new(character_repo), build_clock(now));

            let result = use_case
                .set_relationship(npc_id, pc_id, RelationshipLevel::Friend)
                .await
                .expect("set_relationship should succeed");

            assert_eq!(result.relationship, RelationshipLevel::Friend);
            assert_eq!(result.npc_name, "RelNPC");
        }

        #[tokio::test]
        async fn when_list_for_pc_succeeds() {
            let now = fixed_time();
            let npc_id = CharacterId::new();
            let pc_id = PlayerCharacterId::new();

            let disposition = NpcDispositionState::new(npc_id, pc_id, now)
                .with_disposition(DispositionLevel::Friendly);

            let mut character_repo = MockCharacterRepo::new();

            let disp_clone = disposition.clone();
            character_repo
                .expect_list_dispositions_for_pc()
                .withf(move |p| *p == pc_id)
                .returning(move |_| Ok(vec![disp_clone.clone()]));

            let npc = create_test_character(npc_id, "ListedNPC");
            let npc_clone = npc.clone();
            character_repo
                .expect_get()
                .returning(move |_| Ok(Some(npc_clone.clone())));

            let use_case = NpcDisposition::new(Arc::new(character_repo), build_clock(now));

            let result = use_case
                .list_for_pc(pc_id)
                .await
                .expect("list_for_pc should succeed");

            assert_eq!(result.len(), 1);
            assert_eq!(result[0].npc_name, "ListedNPC");
            assert_eq!(result[0].disposition, "Friendly");
        }
    }

    // =========================================================================
    // NpcMood Tests
    // =========================================================================

    mod mood_ops {
        use super::*;

        #[tokio::test]
        async fn when_get_mood_npc_not_found_returns_error() {
            let region_id = RegionId::new();
            let npc_id = CharacterId::new();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(|_| Ok(None));

            let use_case = NpcMood::new(Arc::new(MockStagingRepo::new()), Arc::new(character_repo));

            let err = use_case
                .set_mood(region_id, npc_id, MoodState::Happy)
                .await
                .expect_err("should fail");

            assert!(matches!(err, NpcError::NotFound { .. }));
        }

        #[tokio::test]
        async fn when_get_mood_succeeds() {
            let region_id = RegionId::new();
            let npc_id = CharacterId::new();

            let mut staging_repo = MockStagingRepo::new();
            staging_repo
                .expect_get_npc_mood()
                .withf(move |r, n| *r == region_id && *n == npc_id)
                .returning(|_, _| Ok(MoodState::Anxious));

            let use_case = NpcMood::new(Arc::new(staging_repo), Arc::new(MockCharacterRepo::new()));

            let result = use_case
                .get_mood(region_id, npc_id)
                .await
                .expect("get_mood should succeed");

            assert_eq!(result, MoodState::Anxious);
        }

        #[tokio::test]
        async fn when_set_mood_succeeds() {
            let region_id = RegionId::new();
            let npc_id = CharacterId::new();

            let mut staging_repo = MockStagingRepo::new();

            // get_npc_mood returns old mood
            staging_repo
                .expect_get_npc_mood()
                .withf(move |r, n| *r == region_id && *n == npc_id)
                .returning(|_, _| Ok(MoodState::Calm));

            // set_npc_mood succeeds
            staging_repo
                .expect_set_npc_mood()
                .withf(move |r, n, m| *r == region_id && *n == npc_id && *m == MoodState::Excited)
                .returning(|_, _, _| Ok(()));

            let mut character_repo = MockCharacterRepo::new();
            let npc = create_test_character(npc_id, "MoodNPC");
            let npc_clone = npc.clone();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(move |_| Ok(Some(npc_clone.clone())));

            let use_case = NpcMood::new(Arc::new(staging_repo), Arc::new(character_repo));

            let result = use_case
                .set_mood(region_id, npc_id, MoodState::Excited)
                .await
                .expect("set_mood should succeed");

            assert_eq!(result.npc_id, npc_id);
            assert_eq!(result.npc_name, "MoodNPC");
            assert_eq!(result.old_mood, MoodState::Calm);
            assert_eq!(result.new_mood, MoodState::Excited);
            assert_eq!(result.region_id, region_id);
        }

        #[tokio::test]
        async fn when_set_mood_uses_default_if_staging_fails() {
            let region_id = RegionId::new();
            let npc_id = CharacterId::new();

            let mut staging_repo = MockStagingRepo::new();

            // get_npc_mood fails - should use NPC's default_mood
            staging_repo
                .expect_get_npc_mood()
                .returning(|_, _| Err(RepoError::not_found("Staging", "not-found")));

            staging_repo
                .expect_set_npc_mood()
                .returning(|_, _, _| Ok(()));

            let mut character_repo = MockCharacterRepo::new();
            let npc =
                create_test_character(npc_id, "DefaultMoodNPC").with_default_mood(MoodState::Alert);
            let npc_clone = npc.clone();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(move |_| Ok(Some(npc_clone.clone())));

            let use_case = NpcMood::new(Arc::new(staging_repo), Arc::new(character_repo));

            let result = use_case
                .set_mood(region_id, npc_id, MoodState::Happy)
                .await
                .expect("set_mood should succeed");

            // old_mood should be the NPC's default_mood since staging lookup failed
            assert_eq!(result.old_mood, MoodState::Alert);
            assert_eq!(result.new_mood, MoodState::Happy);
        }
    }

    // =========================================================================
    // NpcRegionRelationships Tests
    // =========================================================================

    mod region_relationship_ops {
        use super::*;

        #[tokio::test]
        async fn when_list_for_character_succeeds() {
            let npc_id = CharacterId::new();
            let region_id = RegionId::new();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_get_region_relationships()
                .withf(move |id| *id == npc_id)
                .returning(move |_| {
                    Ok(vec![NpcRegionRelationship {
                        region_id,
                        relationship_type: NpcRegionRelationType::HomeRegion,
                        shift: None,
                        frequency: None,
                        time_of_day: None,
                        reason: None,
                    }])
                });

            let use_case = NpcRegionRelationships::new(Arc::new(character_repo));

            let result = use_case
                .list_for_character(npc_id)
                .await
                .expect("list_for_character should succeed");

            assert_eq!(result.len(), 1);
            assert_eq!(result[0].region_id, region_id);
            assert_eq!(
                result[0].relationship_type,
                NpcRegionRelationType::HomeRegion
            );
        }

        #[tokio::test]
        async fn when_set_home_region_succeeds() {
            let npc_id = CharacterId::new();
            let region_id = RegionId::new();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_set_home_region()
                .withf(move |n, r| *n == npc_id && *r == region_id)
                .returning(|_, _| Ok(()));

            let use_case = NpcRegionRelationships::new(Arc::new(character_repo));

            use_case
                .set_home_region(npc_id, region_id)
                .await
                .expect("set_home_region should succeed");
        }

        #[tokio::test]
        async fn when_set_work_region_succeeds() {
            let npc_id = CharacterId::new();
            let region_id = RegionId::new();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_set_work_region()
                .withf(move |n, r, _| *n == npc_id && *r == region_id)
                .returning(|_, _, _| Ok(()));

            let use_case = NpcRegionRelationships::new(Arc::new(character_repo));

            use_case
                .set_work_region(npc_id, region_id)
                .await
                .expect("set_work_region should succeed");
        }

        #[tokio::test]
        async fn when_remove_relationship_succeeds() {
            let npc_id = CharacterId::new();
            let region_id = RegionId::new();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_remove_region_relationship()
                .withf(move |n, r, t| {
                    *n == npc_id && *r == region_id && *t == NpcRegionRelationType::WorksAt
                })
                .returning(|_, _, _| Ok(()));

            let use_case = NpcRegionRelationships::new(Arc::new(character_repo));

            use_case
                .remove_relationship(npc_id, region_id, NpcRegionRelationType::WorksAt)
                .await
                .expect("remove_relationship should succeed");
        }

        #[tokio::test]
        async fn when_list_region_npcs_succeeds() {
            let region_id = RegionId::new();
            let npc_id = CharacterId::new();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_get_npcs_for_region()
                .withf(move |r| *r == region_id)
                .returning(move |_| {
                    Ok(vec![NpcWithRegionInfo {
                        character_id: npc_id,
                        name: "RegionNPC".to_string(),
                        sprite_asset: None,
                        portrait_asset: None,
                        relationship_type: NpcRegionRelationType::Frequents,
                        shift: None,
                        frequency: Some(RegionFrequency::Often),
                        time_of_day: None,
                        reason: None,
                        default_mood: MoodState::Calm,
                    }])
                });

            let use_case = NpcRegionRelationships::new(Arc::new(character_repo));

            let result = use_case
                .list_region_npcs(region_id)
                .await
                .expect("list_region_npcs should succeed");

            assert_eq!(result.len(), 1);
            assert_eq!(result[0].name, "RegionNPC");
            assert_eq!(result[0].frequency, Some(RegionFrequency::Often));
        }
    }

    // =========================================================================
    // NpcApproachEvents Tests
    // =========================================================================

    mod approach_events {
        use super::*;

        #[tokio::test]
        async fn when_reveal_false_returns_unknown_figure() {
            use wrldbldr_domain::UserId;

            let npc_id = CharacterId::new();
            let pc_id = PlayerCharacterId::new();
            let location_id = LocationId::new();
            let region_id = RegionId::new();
            let world_id = WorldId::new();

            // Setup mocks for the reveal=false path which needs PC and World
            let mut player_character_repo = MockPlayerCharacterRepo::new();
            let pc = wrldbldr_domain::PlayerCharacter::new(
                UserId::new("test-user").expect("valid user id"),
                world_id,
                CharacterName::new("Test PC").expect("valid character name"),
                location_id,
                fixed_time(),
            );
            player_character_repo
                .expect_get()
                .withf(move |id| *id == pc_id)
                .returning(move |_| Ok(Some(pc.clone())));

            let mut world_repo = MockWorldRepo::new();
            let world = wrldbldr_domain::World::new(
                wrldbldr_domain::value_objects::WorldName::new("Test World").expect("valid world name"),
                fixed_time(),
            )
            .with_id(world_id);
            world_repo
                .expect_get()
                .withf(move |id| *id == world_id)
                .returning(move |_| Ok(Some(world.clone())));

            let mut location_repo = MockLocationRepo::new();
            let region = Region::from_storage(
                region_id,
                location_id,
                RegionName::new("Test Region").expect("valid region name"),
                wrldbldr_domain::value_objects::Description::default(),
                None,
                None,
                None,
                false,
                0,
            );
            location_repo
                .expect_get_region()
                .withf(move |id| *id == region_id)
                .returning(move |_| Ok(Some(region.clone())));

            // character_repo.get is called at the start of build_event
            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(|_| Ok(None)); // NPC not found, returns "Unknown NPC"
            let character_repo = Arc::new(character_repo);

            // When reveal=false, the code now creates an unrevealed direct observation
            let mut observation_repo = MockObservationRepo::new();
            observation_repo
                .expect_save_observation()
                .returning(|_| Ok(()));
            let observation_repo = Arc::new(observation_repo);

            let clock = build_clock(fixed_time());

            let use_case = NpcApproachEvents::new(
                character_repo,
                Arc::new(location_repo),
                observation_repo,
                Arc::new(player_character_repo),
                Arc::new(world_repo),
                clock,
            );

            let result = use_case
                .build_event(npc_id, false, pc_id, location_id, region_id)
                .await
                .expect("build_event should succeed");

            assert_eq!(result.npc_name, "Unknown Figure");
            assert!(result.npc_sprite.is_none());
            assert!(result.lookup_error.is_none());
            // When reveal=false, observation is saved successfully (no error)
            assert!(result.observation_error.is_none());
        }

        #[tokio::test]
        async fn when_reveal_true_and_npc_found_returns_details() {
            let npc_id = CharacterId::new();
            let pc_id = PlayerCharacterId::new();
            let location_id = LocationId::new();
            let region_id = RegionId::new();

            let mut character_repo = MockCharacterRepo::new();
            let npc = create_test_character(npc_id, "RevealedNPC");
            let npc_clone = npc.clone();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(move |_| Ok(Some(npc_clone.clone())));

            let character_repo_arc = Arc::new(character_repo);
            let location_repo = Arc::new(MockLocationRepo::new());
            let observation_repo = Arc::new(MockObservationRepo::new());
            let player_character_repo = Arc::new(MockPlayerCharacterRepo::new());
            let world_repo = Arc::new(MockWorldRepo::new());
            let clock = build_clock(fixed_time());

            let use_case = NpcApproachEvents::new(
                character_repo_arc,
                location_repo,
                observation_repo,
                player_character_repo,
                world_repo,
                clock,
            );

            let result = use_case
                .build_event(npc_id, true, pc_id, location_id, region_id)
                .await
                .expect("build_event should succeed");

            assert_eq!(result.npc_name, "RevealedNPC");
            assert!(result.lookup_error.is_none());
            assert!(result.observation_error.is_none());
        }

        #[tokio::test]
        async fn when_reveal_true_but_npc_not_found_returns_unknown() {
            let npc_id = CharacterId::new();
            let pc_id = PlayerCharacterId::new();
            let location_id = LocationId::new();
            let region_id = RegionId::new();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(|_| Ok(None));

            let character_repo_arc = Arc::new(character_repo);
            let location_repo = Arc::new(MockLocationRepo::new());
            let observation_repo = Arc::new(MockObservationRepo::new());
            let player_character_repo = Arc::new(MockPlayerCharacterRepo::new());
            let world_repo = Arc::new(MockWorldRepo::new());
            let clock = build_clock(fixed_time());

            let use_case = NpcApproachEvents::new(
                character_repo_arc,
                location_repo,
                observation_repo,
                player_character_repo,
                world_repo,
                clock,
            );

            let result = use_case
                .build_event(npc_id, true, pc_id, location_id, region_id)
                .await
                .expect("build_event should succeed");

            assert_eq!(result.npc_name, "Unknown NPC");
            assert!(result.lookup_error.is_none());
            assert!(result.observation_error.is_none());
        }

        #[tokio::test]
        async fn when_reveal_true_and_repo_fails_returns_error() {
            let npc_id = CharacterId::new();
            let pc_id = PlayerCharacterId::new();
            let location_id = LocationId::new();
            let region_id = RegionId::new();

            let mut character_repo = MockCharacterRepo::new();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(|_| Err(RepoError::database("get", "Connection failed")));

            let character_repo_arc = Arc::new(character_repo);
            let location_repo = Arc::new(MockLocationRepo::new());
            let observation_repo = Arc::new(MockObservationRepo::new());
            let player_character_repo = Arc::new(MockPlayerCharacterRepo::new());
            let world_repo = Arc::new(MockWorldRepo::new());
            let clock = build_clock(fixed_time());

            let use_case = NpcApproachEvents::new(
                character_repo_arc,
                location_repo,
                observation_repo,
                player_character_repo,
                world_repo,
                clock,
            );

            // The use case now returns an error on repo failure (fail-fast behavior)
            let result = use_case
                .build_event(npc_id, true, pc_id, location_id, region_id)
                .await;

            assert!(result.is_err());
        }
    }

    // =========================================================================
    // NpcLocationSharing Tests
    // =========================================================================

    mod location_sharing {
        use super::*;
        use wrldbldr_domain::{CharacterName, Description, PlayerCharacter, UserId, World, WorldName};

        #[tokio::test]
        async fn when_share_location_succeeds() {
            let _now = fixed_time();
            let pc_id = PlayerCharacterId::new();
            let npc_id = CharacterId::new();
            let location_id = LocationId::new();
            let region_id = RegionId::new();
            let world_id = WorldId::new();

            let mut character_repo = MockCharacterRepo::new();
            let npc = create_test_character(npc_id, "SharingNPC");
            let npc_clone = npc.clone();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(move |_| Ok(Some(npc_clone.clone())));

            let mut location_repo = MockLocationRepo::new();
            // Use from_parts to set the region ID
            let region_with_id = Region::from_storage(
                region_id,
                location_id,
                RegionName::new("SharedRegion").expect("valid region name"),
                Description::default(),
                None,
                None,
                None,
                false,
                0,
            );
            let region_clone = region_with_id.clone();
            location_repo
                .expect_get_region()
                .withf(move |id| *id == region_id)
                .returning(move |_| Ok(Some(region_clone.clone())));

            let mut player_character_repo = MockPlayerCharacterRepo::new();
            let now = fixed_time();
            let user_id = UserId::new("test-user").expect("valid user id");
            let pc = PlayerCharacter::new(
                user_id,
                world_id,
                CharacterName::new("Test PC").expect("valid character name"),
                LocationId::new(),
                now,
            );
            let pc_clone = pc.clone();
            player_character_repo
                .expect_get()
                .withf(move |id| *id == pc_id)
                .returning(move |_| Ok(Some(pc_clone.clone())));

            let mut world_repo = MockWorldRepo::new();
            let world = World::new(WorldName::new("Test World").expect("valid world name"), now).with_id(world_id);
            let world_clone = world.clone();
            world_repo
                .expect_get()
                .withf(move |id| *id == world_id)
                .returning(move |_| Ok(Some(world_clone.clone())));

            let mut observation_repo = MockObservationRepo::new();
            observation_repo
                .expect_save_observation()
                .returning(|_| Ok(()));

            let use_case = NpcLocationSharing::new(
                Arc::new(character_repo),
                Arc::new(location_repo),
                Arc::new(observation_repo),
                Arc::new(player_character_repo),
                Arc::new(world_repo),
                build_clock(now),
            );

            let result = use_case
                .share_location(
                    pc_id,
                    npc_id,
                    location_id,
                    region_id,
                    Some("Found here".to_string()),
                )
                .await
                .expect("share_location should succeed");

            assert_eq!(result.pc_id, pc_id);
            assert_eq!(result.npc_id, npc_id);
            assert_eq!(result.npc_name, "SharingNPC");
            assert_eq!(result.region_name, "SharedRegion");
            assert_eq!(result.notes, Some("Found here".to_string()));
            assert!(result.observation_error.is_none());
        }

        #[tokio::test]
        async fn when_share_location_observation_fails_still_succeeds_with_error() {
            let now = fixed_time();
            let pc_id = PlayerCharacterId::new();
            let npc_id = CharacterId::new();
            let location_id = LocationId::new();
            let region_id = RegionId::new();
            let world_id = WorldId::new();

            let mut character_repo = MockCharacterRepo::new();
            let npc = create_test_character(npc_id, "SharingNPC");
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(move |_| Ok(Some(npc.clone())));

            let mut location_repo = MockLocationRepo::new();
            // Region exists and belongs to location
            let region = Region::from_storage(
                region_id,
                location_id,
                RegionName::new("SharedRegion").expect("valid region name"),
                wrldbldr_domain::value_objects::Description::default(),
                None,
                None,
                None,
                false,
                0,
            );
            location_repo
                .expect_get_region()
                .withf(move |id| *id == region_id)
                .returning(move |_| Ok(Some(region.clone())));

            let mut player_character_repo = MockPlayerCharacterRepo::new();
            let user_id = UserId::new("test-user").expect("valid user id");
            let pc = PlayerCharacter::new(
                user_id,
                world_id,
                CharacterName::new("Test PC").expect("valid character name"),
                LocationId::new(),
                now,
            );
            let pc_clone = pc.clone();
            player_character_repo
                .expect_get()
                .withf(move |id| *id == pc_id)
                .returning(move |_| Ok(Some(pc_clone.clone())));

            let mut world_repo = MockWorldRepo::new();
            let world = World::new(WorldName::new("Test World").expect("valid world name"), now).with_id(world_id);
            let world_clone = world.clone();
            world_repo
                .expect_get()
                .withf(move |id| *id == world_id)
                .returning(move |_| Ok(Some(world_clone.clone())));

            let mut observation_repo = MockObservationRepo::new();
            observation_repo
                .expect_save_observation()
                .returning(|_| Err(RepoError::database("save_observation", "Write failed")));

            let use_case = NpcLocationSharing::new(
                Arc::new(character_repo),
                Arc::new(location_repo),
                Arc::new(observation_repo),
                Arc::new(player_character_repo),
                Arc::new(world_repo),
                build_clock(now),
            );

            let result = use_case
                .share_location(pc_id, npc_id, location_id, region_id, None)
                .await
                .expect("share_location should succeed even if observation fails");

            assert_eq!(result.npc_name, "SharingNPC");
            assert_eq!(result.region_name, "SharedRegion");
            assert!(result.observation_error.is_some());
        }

        #[tokio::test]
        async fn when_region_not_belonging_to_location_fails() {
            let now = fixed_time();
            let pc_id = PlayerCharacterId::new();
            let npc_id = CharacterId::new();
            let location_id = LocationId::new();
            let region_id = RegionId::new();
            let mismatched_location_id = LocationId::new();
            let world_id = WorldId::new();

            let mut character_repo = MockCharacterRepo::new();
            let npc = create_test_character(npc_id, "SharingNPC");
            let npc_clone = npc.clone();
            character_repo
                .expect_get()
                .withf(move |id| *id == npc_id)
                .returning(move |_| Ok(Some(npc_clone.clone())));

            let mut location_repo = MockLocationRepo::new();
            // Region belongs to different location
            let region_with_mismatch = Region::from_storage(
                region_id,
                mismatched_location_id, // Different location ID
                RegionName::new("SharedRegion").expect("valid region name"),
                Description::default(),
                None,
                None,
                None,
                false,
                0,
            );
            let region_clone = region_with_mismatch.clone();
            location_repo
                .expect_get_region()
                .withf(move |id| *id == region_id)
                .returning(move |_| Ok(Some(region_clone.clone())));

            let mut player_character_repo = MockPlayerCharacterRepo::new();
            let user_id = UserId::new("test-user").expect("valid user id");
            let pc = PlayerCharacter::new(
                user_id,
                world_id,
                CharacterName::new("Test PC").expect("valid character name"),
                location_id, // Current location
                now,
            );
            let pc_clone = pc.clone();
            player_character_repo
                .expect_get()
                .withf(move |id| *id == pc_id)
                .returning(move |_| Ok(Some(pc_clone.clone())));

            let mut world_repo = MockWorldRepo::new();
            let world = World::new(WorldName::new("Test World").expect("valid world name"), now).with_id(world_id);
            let world_clone = world.clone();
            world_repo
                .expect_get()
                .withf(move |id| *id == world_id)
                .returning(move |_| Ok(Some(world_clone.clone())));

            let mut observation_repo = MockObservationRepo::new();
            observation_repo
                .expect_save_observation()
                .returning(|_| Ok(()));

            let use_case = NpcLocationSharing::new(
                Arc::new(character_repo),
                Arc::new(location_repo),
                Arc::new(observation_repo),
                Arc::new(player_character_repo),
                Arc::new(world_repo),
                build_clock(now),
            );

            let result = use_case
                .share_location(pc_id, npc_id, location_id, region_id, None)
                .await;

            // Should fail with InvalidInput error
            assert!(result.is_err());
            assert!(matches!(
                result.expect_err("should fail"),
                NpcError::InvalidInput(_)
            ));
        }
    }
}
